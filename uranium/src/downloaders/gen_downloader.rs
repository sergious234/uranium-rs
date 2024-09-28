use std::sync::Arc;
use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use futures::{future::join_all, StreamExt};
use log::{error, info, warn};
use reqwest::Response;
use sha1::Digest;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::{io::AsyncWriteExt, task::JoinHandle};

use crate::error::Result;
use crate::{code_functions::N_THREADS, error::UraniumError};

/// Download files asynchronously.
///
/// This trait allows the user to make their own `FileDownloader` and use it
/// with the different downloader such us:
/// - `MinecraftDownloader`
/// - `RinthDownloader`
/// - `CurseDownloader`
#[allow(async_fn_in_trait)]
pub trait FileDownloader {
    /// Builds a new struct from a vec of `DownlodableObject`s.
    fn new(files: Vec<DownloadableObject>) -> Self;

    /// This method is responsible for managing the progress of downloads and
    /// tasks in the Uranium library.
    ///
    /// It returns the current `DownloadState`, which represents the state of
    /// the download process.
    ///
    /// If there are pending `DownlodableObject` and the number of active tasks
    /// is less than the maximum allowed threads, this method will make
    /// additional requests to fetch data.
    ///
    /// If there are active tasks, it will check their status and handle
    /// completed tasks accordingly.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `UraniumError` in the following
    /// cases:
    ///
    /// - If there is an error while making requests or processing tasks.
    ///
    /// # Returns
    ///
    /// This method returns a `Result<DownloadState, UraniumError>`, where
    /// `DownloadState` represents the current state of the download process,
    /// and `UraniumError` is the error type that occurs in case of failure.
    async fn progress(&mut self) -> Result<DownloadState>;

    /// This method calls `Self::progress()` repeatedly until it returns
    /// `DownloadState::Completed`.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `UraniumError` in the following
    /// cases:
    ///
    /// - If there is an error while making requests or processing tasks.
    ///
    /// # Returns
    ///
    /// This method returns a `Result<(), UraniumError>`.
    async fn complete(&mut self) -> Result<()> {
        loop {
            match self.progress().await {
                Err(e) => return Err(e),
                Ok(DownloadState::Completed) => return Ok(()),
                Ok(_) => {}
            }
        }
    }

    /// Return how many requests are left.
    ///
    /// This method is important when it comes to know the % of the
    /// already downloaded files.
    fn requests_left(&self) -> usize;

    /// Return how many requests the downloader has.
    fn len(&self) -> usize;
}

/// Indicates the state of the downloader
#[derive(Debug)]
pub enum DownloadState {
    MakingRequests,
    Downloading,
    Completed,
}

// TODO! : Add Sha5
/// Indicates which hash the file uses for verification.
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
/// `name`: MyMinecraftMod.jar <br>
/// `path`: /home/sergio/.minecraft/Fabric1.18/mods/
#[derive(Debug, Clone)]
pub struct DownloadableObject {
    pub url: String,
    pub name: String,
    pub path: PathBuf,
    pub hash: Option<HashType>,
}

impl DownloadableObject {
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
    files: Vec<DownloadableObject>,
    requester: reqwest::Client,
    start: usize,
    s: Arc<Semaphore>,
    tasks: VecDeque<JoinHandle<Result<()>>>,
}

impl FileDownloader for Downloader {
    fn new(files: Vec<DownloadableObject>) -> Self {
        let n_files = files.len();
        info!("{n_files} files to download");

        let client = reqwest::ClientBuilder::new()
            .resolve(
                "resources.download.minecraft.net",
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(13, 107, 246, 43)), 80),
            )
            .build()
            .expect("Error while creating the Downloader client, please report this error.");

        Downloader {
            files,
            requester: client,
            start: 0,
            s: Arc::new(Semaphore::new(N_THREADS())),
            tasks: VecDeque::with_capacity(n_files),
        }
    }

    async fn progress(&mut self) -> Result<DownloadState> {
        let mut x = N_THREADS();
        while x > 0 && self.start != self.files.len() && self.s.available_permits() > 0 {
            self.make_requests().await?;
            x -= 1;
        }

        if !self.tasks.is_empty() {
            let mut guard = true;
            let mut i = 0;
            while guard {
                guard = false;
                // SAFETY: There is no way this unwraps fails since we are
                // iterating over the len of the queue and no other thread
                // is modifying the queue, also the queue is not empty.
                if self
                    .tasks
                    .get(i)
                    .unwrap()
                    .is_finished()
                {
                    let task = self.tasks.remove(i).unwrap();
                    guard = true;
                    match task.await? {
                        Err(UraniumError::FilesDontMatch(objects)) => {
                            self.files.extend(objects);
                        }
                        Err(e) => Err(e)?,
                        Ok(_) => {}
                    }
                    break;
                }

                i = (i + 1) % self.tasks.len();
            }

            if guard {
                return Ok(DownloadState::Downloading);
            }

            // In case no task is finished yet, we wait for the first one
            if !self.tasks.is_empty() {
                warn!("Waiting the first one...");
                // UNWRAP SAFETY: Can't be empty since we are checking.
                match self
                    .tasks
                    .pop_front()
                    .unwrap()
                    .await?
                {
                    Err(UraniumError::FilesDontMatch(objects)) => self.files.extend(objects),
                    Err(e) => Err(e)?,
                    _ => {}
                };
                return Ok(DownloadState::Downloading);
            }
        }
        Ok(DownloadState::Completed)
    }

    /// Returns how many requests are left.
    #[must_use]
    fn requests_left(&self) -> usize {
        self.files.len() - self.start + self.tasks.len()
    }

    #[must_use]
    fn len(&self) -> usize {
        self.files.len()
    }
}

impl Downloader {

    pub fn mi_static() -> i32 {
        return -33;
    }

    async fn make_requests(&mut self) -> Result<DownloadState> {
        let mut chunk_size = 32;

        if self.start + chunk_size > self.files.len() {
            chunk_size = self.files.len() - self.start;
        }

        let files = &self.files[self.start..self.start + chunk_size];

        let mut requests_vec = Vec::new();
        for file in files {
            let rq = self.requester.clone();
            let file_url = file.url.to_owned();

            requests_vec.push(async move { rq.get(&file_url).send().await });
        }

        let responses: Vec<std::result::Result<Response, reqwest::Error>> = join_all(requests_vec)
            .await
            .into_iter()
            .collect();

        if let Some(i) = responses
            .iter()
            .position(|e| e.is_err())
        {
            error!("{:?}", responses[i]);
            return Err(UraniumError::Other);
        }

        let responses = responses
            .into_iter()
            .flatten()
            .collect();

        let files = self.files[self.start..self.start + chunk_size].to_vec();
        let sem = self
            .s
            .clone()
            .acquire_owned()
            .await
            .unwrap();
        let task = tokio::spawn(async move { download_and_write(files, responses, sem).await });

        info!("Pushing new task {}", self.start);
        self.tasks.push_back(task);
        self.start += chunk_size;
        Ok(DownloadState::MakingRequests)
    }
}

async fn download_and_write(
    files: Vec<DownloadableObject>,
    responses: Vec<Response>,
    _sem: OwnedSemaphorePermit,
) -> Result<()> {
    debug_assert_eq!(responses.len(), files.len());

    if responses.len() != files.len() {
        return Err(UraniumError::OtherWithReason(
            "Responses len doesn't match files len, this shouldn't happen...".into(),
        ));
    }

    info!("Downloading data");
    let mut bytes_from_res = Vec::with_capacity(responses.len());

    for (response, obj) in responses
        .into_iter()
        .zip(files.into_iter())
    {
        let file_path = obj.path.join(&obj.name);

        // If the file already exits check if its hash match, if so go for
        // the next file.
        if file_path.exists() {
            let content = tokio::fs::read(&file_path).await?;
            let good_hash = match obj.hash {
                Some(HashType::Sha1(ref expected)) => {
                    let mut hasher = sha1::Sha1::new();
                    hasher.update(&content);
                    let actual = hex::encode(hasher.finalize());
                    &actual == expected
                }
                None => false,
            };

            if good_hash {
                continue;
            }
        }

        bytes_from_res.push(async move {
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
                .await?;

            let mut total = 0;
            let mut buffer = Vec::with_capacity(content_length);

            while let Some(item) = bytes_stream.next().await {
                let chunk = item?;
                match file.write(&chunk).await {
                    Err(e) => {
                        error!("Can not write in {:?}: {}", file_path, e);
                        return Err(e.into());
                    }
                    Ok(n) => total += n,
                };
                buffer.extend(chunk);
            }

            let good_hash = match obj.hash {
                Some(HashType::Sha1(ref expected)) if total == content_length => {
                    let mut hasher = sha1::Sha1::new();
                    hasher.update(&buffer);
                    let actual = hex::encode(hasher.finalize());
                    &actual == expected
                }

                // If a hash is available but the download size doesn't match
                // the content length then something is wrong.
                Some(_) => false,

                None => true,
            };

            if total == content_length && good_hash {
                Ok(())
            } else {
                Err(UraniumError::FileNotMatch(obj))
            }
        });
    }

    let errors: Vec<_> = join_all(bytes_from_res)
        .await
        .into_iter()
        .filter_map(|e| e.err())
        .collect();

    if !errors.is_empty() {
        warn!("Some files are broken");
        let objects: Vec<DownloadableObject> = errors
            .into_iter()
            .filter_map(|e| match e {
                UraniumError::FileNotMatch(obj) => Some(obj),
                _ => None,
            })
            .collect();
        return Err(UraniumError::FilesDontMatch(objects));
    }

    info!("Chunk wrote successfully!");
    Ok(())
}
