use std::fs::create_dir_all;
use std::sync::Arc;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use futures::{StreamExt, future::join_all};
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

    /// Adds a single `DownloadableObject` to the downloader's queue.
    ///
    /// This method allows you to dynamically add new download tasks to an
    /// existing downloader instance. The object will be queued for download
    /// and processed according to the downloader's scheduling logic.
    fn add_object(&mut self, obj: DownloadableObject);

    /// Adds multiple `DownloadableObject`s to the downloader's queue.
    ///
    /// This is a convenience method that accepts any iterator of
    /// `DownloadableObject`s and adds them all to the download queue.
    /// Internally, it calls `add_object` for each item in the iterator.
    fn add_objects<T>(&mut self, objs: T)
    where
        T: IntoIterator<Item = DownloadableObject>,
    {
        objs.into_iter()
            .for_each(|f| self.add_object(f));
    }

    /// Returns `true` if the downloader has no downloadable objects.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
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
    pub path: PathBuf,
    pub hash: Option<HashType>,
}

impl DownloadableObject {
    pub fn new(url: &str, path: &Path, hash: Option<HashType>) -> Self {
        Self {
            url: url.to_owned(),
            path: path.to_owned(),
            hash,
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.path
            .file_name()
            .and_then(|f| f.to_str())
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
            // .resolve(
            //     "resources.download.minecraft.net",
            //     SocketAddr::new(IpAddr::V4(Ipv4Addr::new(13, 107, 246, 43)), 80),
            // )
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
        //let mut x = N_THREADS();
        while self.start != self.files.len() && self.s.available_permits() > 0 {
            self.make_requests().await?;
            //x -= 1;
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
    fn requests_left(&self) -> usize {
        self.files.len() - self.start + self.tasks.len()
    }

    fn len(&self) -> usize {
        self.files.len()
    }

    /// Add an object to the files vector.
    fn add_object(&mut self, obj: DownloadableObject) {
        self.files.push(obj);
    }

    fn add_objects<T>(&mut self, objs: T)
    where
        T: IntoIterator<Item = DownloadableObject>,
    {
        self.files.extend(objs);
    }
}

impl Downloader {
    /// Improved semaphore acquisition with proper error handling
    async fn acquire_semaphore(&self) -> Result<OwnedSemaphorePermit> {
        self.s
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| UraniumError::other(&format!("Failed to acquire semaphore: {e}")))
    }

    fn get_next_chunk(&mut self) -> Vec<DownloadableObject> {
        const DEFAULT_CHUNK_SIZE: usize = 32;

        let remaining = self.files.len() - self.start;
        if remaining == 0 {
            return vec![];
        }

        let chunk_size = DEFAULT_CHUNK_SIZE.min(remaining);
        let end = self.start + chunk_size;
        let chunk = self.files[self.start..end].to_vec();

        self.start = end;
        chunk
    }

    /// Fetches HTTP responses for a chunk of files
    async fn fetch_responses(&self, files: &[DownloadableObject]) -> Result<Vec<Response>> {
        let requests = files.iter().map(|file| {
            let requester = self.requester.clone();
            let url = file.url.clone();
            async move {
                requester
                    .get(&url)
                    .send()
                    .await
            }
        });

        let responses = join_all(requests).await;

        // Find first error and return it
        for (i, response) in responses.iter().enumerate() {
            if let Err(e) = response {
                error!("Request failed for file {}: {:?}", i, e);
                return Err(UraniumError::other(&format!("Request failed: {e:?}")));
            }
        }

        Ok(responses
            .into_iter()
            .flatten()
            .collect::<Vec<_>>())
    }

    async fn make_requests(&mut self) -> Result<DownloadState> {
        let chunk = self.get_next_chunk();
        if chunk.is_empty() {
            return Ok(DownloadState::Completed);
        }

        let responses = self
            .fetch_responses(&chunk)
            .await?;

        let sem = self
            .acquire_semaphore()
            .await?;
        let task = tokio::spawn(async move { download_and_write(chunk, responses, sem).await });

        info!("Pushing new task {}", self.start);
        self.tasks.push_back(task);
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

    let mut bytes_from_res = Vec::with_capacity(responses.len());

    for (response, obj) in responses
        .into_iter()
        .zip(files.into_iter())
    {
        // If the file already exits check if its hash match, if so go for
        // the next file.
        if verify_file_hash(&obj.path, &obj.hash).await? {
            continue;
        }

        bytes_from_res.push(download_single_file(response, obj));
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
                error => {
                    error!("{}", error);
                    None
                }
            })
            .collect();
        return Err(UraniumError::FilesDontMatch(objects));
    }

    info!("Chunk wrote successfully!");
    Ok(())
}

/// Verifies if a file matches its expected hash
async fn verify_file_hash(path: &Path, expected_hash: &Option<HashType>) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let Some(HashType::Sha1(expected)) = expected_hash else {
        return Ok(false);
    };

    let content = tokio::fs::read(path).await?;
    let mut hasher = sha1::Sha1::new();
    hasher.update(&content);
    let actual = hex::encode(hasher.finalize());

    Ok(&actual == expected)
}

async fn download_single_file(response: Response, obj: DownloadableObject) -> Result<()> {
    if !response.status().is_success() {
        return Err(UraniumError::other(&format!(
            "Error with response, status {}",
            response.status()
        )));
    }

    let content_length = response
        .content_length()
        .map(|e| e as usize)
        .unwrap_or_default();

    let mut bytes_stream = response.bytes_stream();

    if !obj.path.exists() {
        create_dir_all(
            obj.path
                .parent()
                .expect("Error getting parent path of lib"),
        )?;
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&obj.path)
        .await?;

    let mut total = 0;
    let mut buffer = Vec::with_capacity(content_length);

    while let Some(item) = bytes_stream.next().await {
        let chunk = item?;
        match file.write(&chunk).await {
            Err(e) => {
                error!("Can not write in {:?}: {}", obj.path, e);
                return Err(e.into());
            }
            Ok(n) => total += n,
        };
        buffer.extend(chunk);
    }

    if total == content_length
        && verify_file_hash(&obj.path, &obj.hash)
            .await
            .is_ok_and(|x| x)
    {
        Ok(())
    } else {
        Err(UraniumError::FileNotMatch(obj))
    }
}
