use crate::{code_functions::N_THREADS, error::UraniumError};
use futures::{future::join_all, StreamExt};
use log::{debug, error, info, warn};
use reqwest::Response;
use sha1::Digest;
use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tokio::{io::AsyncWriteExt, task::JoinHandle};

/// Download files asynchronously.  
///
/// This trait allows the user to make their own `FileDownloader` and use it with
/// the differents downloader such us:
/// - `MinecraftDownloader`
/// - `RinthDownloader`
/// - `CurseDownloader`
///
#[allow(async_fn_in_trait)]
pub trait FileDownloader {
    /// This method is responsible for managing the progress of downloads and
    /// tasks in the Uranium library.
    ///
    /// It returns the current `DownloadState`, which represents the state of
    /// the download process.
    ///
    /// If there are pending `DownlodableObject` and the number of active tasks is less than
    /// the maximum allowed threads, this method will make additional requests to fetch data.
    ///
    /// If there are active tasks, it will check their status and handle
    /// completed tasks accordingly.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `UraniumError` in the following cases:
    ///
    /// - If there is an error while making requests or processing tasks.
    ///
    /// # Returns
    ///
    /// This method returns a `Result<DownloadState, UraniumError>`, where
    /// `DownloadState` represents the current state of the download process, and `UraniumError`
    /// is the error type that occurs in case of failure.
    async fn progress(&mut self) -> Result<DownloadState, UraniumError>;

    /// This method calls `Self::progress()` repeatdly until it returns `DownloadState::Completed`.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `UraniumError` in the following cases:
    ///
    /// - If there is an error while making requests or processing tasks.
    ///
    /// # Returns
    ///
    /// This method returns a `Result<(), UraniumError>`.
    async fn complete(&mut self) -> Result<(), UraniumError> {
        loop {
            match self.progress().await {
                Err(e) => return Err(e),
                Ok(DownloadState::Completed) => return Ok(()),
                Ok(_) => {}
            }
        }
    }

    /// Builds a new struct from a vec of `DownlodableObject`s.
    fn new(files: Vec<DownlodableObject>) -> Self;

    /// Return how many requests are left.
    fn requests_left(&self) -> usize;
}

/// Indicates the state of the downloader
#[derive(Debug)]
pub enum DownloadState {
    MakingRequests,
    Downloading,
    Completed,
}

#[derive(Debug, Clone)]
pub enum HashType {
    Sha1(String),
}

/// Simple struct with the necessary data to download a file
///
/// Fields:
/// - url : http://somerandomurl.com
/// - name: my_filename.whatever
/// - path: /path/to/something/mods
///
/// The join between path and name MUST result in the final path e.g:
///
/// `name`: MyMinecraftMod.jar
/// `path`: /home/sergio/.minecraft/Fabric1.18/mods/
#[derive(Debug, Clone)]
pub struct DownlodableObject {
    pub url: String,
    pub name: String,
    pub path: PathBuf,
    pub hash: Option<HashType>,
}

impl DownlodableObject {
    pub fn new(url: &str, name: &str, path: &Path, hash: Option<HashType>) -> Self {
        Self {
            url: url.to_owned(),
            name: name.to_owned(),
            path: path.to_owned(),
            hash,
        }
    }
}

/// Basic downloader
///
/// `Downloader` is a basic implementation of `FileDownloader` trait.
///
/// It uses `reqwest::Client` for the HTTP requests.
pub struct Downloader {
    files: Vec<DownlodableObject>,
    requester: reqwest::Client,
    max_threads: usize,
    start: usize,
    tasks: VecDeque<JoinHandle<Result<(), UraniumError>>>,
}

impl FileDownloader for Downloader {
    fn new(files: Vec<DownlodableObject>) -> Self {
        let n_files = files.len();
        info!("{n_files} files to download");

        let client = reqwest::ClientBuilder::new()
            .resolve(
                "resources.download.minecraft.net",
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(13, 107, 246, 43)), 80),
            )
            .build()
            .unwrap();

        Downloader {
            files,
            requester: client,
            max_threads: 32,
            start: 0,
            tasks: VecDeque::with_capacity(n_files),
        }
    }

    /// Returns how many requests are left.
    fn requests_left(&self) -> usize {
        self.files.len()
    }

    async fn progress(&mut self) -> Result<DownloadState, UraniumError> {
        debug!("Start: {} || Tasks: {}", self.start, self.tasks.len());

        while self.files.len() != self.start && self.tasks.len() < self.max_threads {
            self.make_requests().await?;
        }

        if !self.tasks.is_empty() {
            let mut guard = true;
            while guard {
                guard = false;
                for i in 0..self.tasks.len() {
                    // SAFETY: There is no way this unwraps fails since we are
                    // iterating over the len of the queue and no other thread
                    // is modifing the queue.
                    if self.tasks.get(i).unwrap().is_finished() {
                        let task = self.tasks.remove(i).unwrap();
                        guard = true;
                        match task.await.unwrap() {
                            Err(UraniumError::FilesDontMatch(objects)) => {
                                self.files.extend(objects)
                            }
                            Err(e) => Err(e)?,
                            Ok(_) => {}
                        }
                        break;
                    }
                }
            }

            if guard {
                return Ok(DownloadState::Downloading);
            }

            // In case no task is finished yet we wait for the first one
            if !self.tasks.is_empty() {
                warn!("Waiting the first one...");
                // UNWRAP SAFETY: Can't be empty since we are checking. 
                match self.tasks.pop_front().unwrap().await.unwrap() {
                    Err(UraniumError::FilesDontMatch(objects)) => self.files.extend(objects),
                    Err(e) => Err(e)?,
                    _ => {}
                };
                return Ok(DownloadState::Downloading);
            }
        }
        Ok(DownloadState::Completed)
    }
}

impl Downloader {
    async fn make_requests(&mut self) -> Result<DownloadState, UraniumError> {
        let mut chunk_size = 16; //self.max_threads;

        if self.max_threads < N_THREADS() * 2 {
            self.max_threads += 4;
        }

        if self.start + chunk_size > self.files.len() {
            chunk_size = self.files.len() - self.start;
        }

        let files = &self.files[self.start..self.start + chunk_size];

        let mut requests_vec = Vec::new();
        for file in files {
            let rq = self.requester.clone();
            let file_url = file.url.to_owned();

            requests_vec.push(tokio::task::spawn(
                async move { rq.get(&file_url).send().await },
            ));
        }

        let responses: Vec<Result<Response, reqwest::Error>> =
            join_all(requests_vec).await.into_iter().flatten().collect();

        if let Some(e) = responses.iter().find(|e| e.is_err()) {
            error!("{:?}", e);
            return Err(UraniumError::RequestError);
        }

        let responses = responses.into_iter().flatten().collect();

        let files = self.files[self.start..self.start + chunk_size]
            .iter()
            .cloned()
            .collect();
        let task = tokio::spawn(async move { download_and_write(files, responses).await });

        info!("Pushing new task {}", self.start);
        self.tasks.push_back(task);
        self.start += chunk_size;
        Ok(DownloadState::MakingRequests)
    }
}

async fn download_and_write(
    files: Vec<DownlodableObject>,
    responses: Vec<Response>,
) -> Result<(), UraniumError> {
    assert_eq!(responses.len(), files.len());

    info!("Downloading data");
    let mut bytes_from_res = Vec::with_capacity(responses.len());

    for (response, obj) in responses.into_iter().zip(files.into_iter()) {
        bytes_from_res.push(tokio::spawn(async move {
            let file_path = obj.path.join(&obj.name);
            let content_length = response
                .content_length()
                .map(|e| e as usize)
                .unwrap_or_default();

            let mut bytes_stream = response.bytes_stream();
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&file_path)
                .await
                .inspect_err(|e| error!("An error ocurred trying to open {:?}: {:?}", file_path, e))?;

            let mut total = 0;
            let mut buffer = Vec::with_capacity(content_length);
            while let Some(item) = bytes_stream.next().await {
                let chunk = item.unwrap();
                match file.write(&chunk).await {
                    Err(e) => {
                        error!("Can not write in {:?}: {}", file_path, e);
                        return Err(UraniumError::DownloadError);
                    }
                    Ok(n) => total += n,
                };
                buffer.extend(chunk);
            }

            let good_hash = match obj.hash {
                Some(HashType::Sha1(ref expected)) => {
                    let mut hasher = sha1::Sha1::new();
                    hasher.update(&buffer);
                    let actual = hex::encode(hasher.finalize());
                    &actual == expected
                }
                None => true,
            };

            if total == content_length && good_hash {
                Ok(())
            } else {
                Err(UraniumError::FileNotMatch(obj))
            }
        }));
    }

    let errors: Vec<_> = join_all(bytes_from_res)
        .await
        .into_iter()
        .flatten()
        .filter_map(|e| e.err())
        .collect();

    if !errors.is_empty() {
        warn!("Some files are broken");
        let objects: Vec<DownlodableObject> = errors
            .into_iter()
            .filter_map(|e| match e {
                UraniumError::FileNotMatch(obj) => Some(obj),
                _ => None,
            })
            .collect();
        return Err(UraniumError::FilesDontMatch(objects));
    }

    info!("Chunk wrote succesfully!");
    Ok(())
}
