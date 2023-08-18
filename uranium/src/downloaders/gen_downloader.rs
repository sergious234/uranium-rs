use crate::{code_functions::N_THREADS, error::UraniumError};
use futures::future::join_all;
use log::{error, info};
use requester::{mod_searcher::Method, requester::request_maker::Req};
use reqwest::Response;
use std::{collections::VecDeque, path::PathBuf, sync::Arc};
use tokio::task::JoinHandle;

async fn download_and_write(
    path: Arc<PathBuf>,
    responses: Vec<Response>,
    names: Vec<PathBuf>,
) -> Result<(), UraniumError> {
    assert_eq!(responses.len(), names.len());

    let mut bytes_from_res = Vec::with_capacity(responses.len());

    for response in responses {
        bytes_from_res.push(async move { response.bytes().await });
    }

    // TODO! Manage potentials errors
    let bytes_from_res: Vec<bytes::Bytes> = join_all(bytes_from_res)
        .await
        .into_iter()
        .flatten()
        .collect();

    for (i, bytes) in bytes_from_res.into_iter().enumerate() {
        let file_path = path.join(&names[i]);
        if let Err(e) = std::fs::write(&file_path, &bytes) {
            error!(
                "Can not write in {:?}: {}",
                file_path.file_name().unwrap_or_default(),
                e
            );
            return Err(e.into());
        };
    }

    Ok(())
}

#[derive(Debug)]
pub enum DownloadState {
    MakingRequests,
    Downloading,
    Completed,
}

/*
 *           +------------+
 *           | Downloader |
 *           +------------+
 *                 |                   path
 *                 V                   names
 *          +-----------------+        response_chunk       +--------------------+
 *          |   get_response  | --------------------------> | download_and_write |
 *          +-----------------+          *NEW TASK*         +--------------------+
 *              ^        |
 *              \       /
 *               \_____/
 *               ^^^^^^^
 *       for chunk in urls.chunk(N_THREADS)
 *
 * */

pub struct Downloader<T: Req + Clone + Send> {
    urls: Arc<Vec<String>>,
    names: Vec<PathBuf>,
    path: Arc<PathBuf>,
    requester: T,
    max_threads: usize,
    tasks: VecDeque<JoinHandle<Result<(), UraniumError>>>,
}

impl<T: Req + Clone + Send> Downloader<T> {
    pub fn new(
        urls: Arc<Vec<String>>,
        names: Vec<PathBuf>,
        path: Arc<PathBuf>,
        requester: T,
    ) -> Downloader<T> {
        info!("Downloader max threads: {}", N_THREADS());
        Downloader {
            urls,
            names,
            path,
            requester,
            max_threads: N_THREADS(),
            tasks: VecDeque::new(),
        }
    }
}

impl<T: Req + Clone + Send + Sync> Downloader<T> {
    /// This method will start the download and make progress until
    /// the download is completed.
    pub async fn start(&mut self) -> Result<(), UraniumError> {
        loop {
            match self.progress().await {
                Err(e) => return Err(e),
                Ok(DownloadState::Completed) => return Ok(()),
                Ok(_) => {}
            }
        }
    }

    /// Returns a reference to the urls.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    /// Returns how many requests are left.
    pub fn requests_left(&self) -> usize {
        self.names.len()
    }

    #[allow(unused)]
    async fn get_responses(&mut self) -> Result<(), UraniumError> {
        let chunk_size = self.max_threads;

        for url_chunk in self.urls.chunks(chunk_size) {
            let path_c = self.path.clone();
            let names: Vec<PathBuf> = self.names.drain(0..url_chunk.len()).collect();

            let mut requests_vec = Vec::new();
            for url in url_chunk {
                let rq = self.requester.clone();
                let u = url.clone();
                requests_vec.push(async move { rq.get(&u, Method::GET, "").await.unwrap() });
            }

            let responses = join_all(requests_vec).await.into_iter().flatten().collect();

            let task =
                tokio::task::spawn(async { download_and_write(path_c, responses, names).await });

            self.tasks.push_back(task);
        }

        Ok(())
    }

    async fn make_requests(&mut self) -> Result<DownloadState, UraniumError> {
        let mut chunk_size = N_THREADS();

        if chunk_size > self.names.len() {
            chunk_size = self.names.len();
        }

        let start = self.urls.len() - self.names.len();
        let urls = &self.urls[start..start + chunk_size];

        let mut requests_vec = Vec::new();
        for url in urls {
            let rq = self.requester.clone();
            //let u = url.clone();
            requests_vec.push(async move { rq.get(url, Method::GET, "").await });
        }

        let responses: Vec<Result<Response, reqwest::Error>> =
            join_all(requests_vec).await.into_iter().flatten().collect();

        if responses.iter().any(Result::is_err) {
            return Err(UraniumError::RequestError);
        }

        let responses = responses.into_iter().flatten().collect();

        let path = self.path.clone();
        let names = self.names.drain(0..chunk_size).collect();
        let task = tokio::spawn(async { download_and_write(path, responses, names).await });

        self.tasks.push_back(task);
        Ok(DownloadState::MakingRequests)
    }

    /// Calling this function will make **progress** in the download.
    ///
    /// When `DownloadState::Completed` is returned then the download is finish.
    ///
    /// If there are requests to do and free threads it will make them and return
    /// `DownloadState::MakingRequests` if success or `UraniumError` if failed.
    ///
    /// This function will only wait for the first completed task and then it will
    /// return `DownloadState::Downloading` in success or `UraniumError` if failed.
    pub async fn progress(&mut self) -> Result<DownloadState, UraniumError> {
        if !self.names.is_empty() && self.tasks.len() < self.max_threads {
            return self.make_requests().await;
        } else if !self.tasks.is_empty() {
            for i in 0..self.tasks.len() {
                // SAFETY: There is no way this unwraps fails since we are
                // iterating over the len of the queue and no other thread
                // is modifing the queue.
                if self.tasks.get(i).unwrap().is_finished() {
                    let task = self.tasks.remove(i).unwrap();
                    return task.await.unwrap().map(|_| DownloadState::Downloading);
                }
            }

            if !self.tasks.is_empty() {
                // SAFETY: Again, we are checking if the queue is not empty.
                let _ = self.tasks.pop_front().unwrap().await;
                return Ok(DownloadState::Downloading);
            }
        }
        Ok(DownloadState::Completed)
    }

    /// Calling this function will make **progress** in the download.
    ///
    /// When `DownloadState::Completed` is returned then the download is finish.
    ///
    /// If there are requests to do and free threads it will make them and return
    /// `DownloadState::MakingRequests` if success or `UraniumError` if failed.
    ///
    /// This function will check all the tasks, remove the completed ones and
    /// return `DownloadState::Downloading`. Also if no tasks have been completed it
    /// will also return `DownloadState::Downloading`.
    ///
    #[allow(unused)]
    pub async fn advance(&mut self) -> Result<DownloadState, UraniumError> {
        if !self.names.is_empty() && self.tasks.len() < self.max_threads {
            return self.make_requests().await;
        } else if !self.tasks.is_empty() {
            let mut removed = true;

            while removed {
                removed = false;
                for i in 0..self.tasks.len() {
                    // SAFETY: There is no way this unwraps fails since we are
                    // iterating over the len of the queue and no other thread
                    // is modifing the queue.
                    if self.tasks.get(i).unwrap().is_finished() {
                        let task = self.tasks.remove(i).unwrap();
                        task.await.unwrap()?;
                        removed = true;
                        break;
                    }
                }
            }
            return Ok(DownloadState::Downloading);
        }

        Ok(DownloadState::Completed)
    }
}
