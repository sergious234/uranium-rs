use std::fs::create_dir_all;
use std::sync::Arc;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use futures::{future::join_all, StreamExt};
use log::{error, info};
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
        info!("{} available permits", N_THREADS());

        let client = reqwest::ClientBuilder::new()
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
        while self.start != self.files.len() && self.s.available_permits() > 0 {
            self.make_requests().await?;
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
                            error!("Trying again {} files", objects.len());
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
                info!("Waiting the first one...");
                // let _ = join_all(&mut self.tasks).await;
                // self.tasks.clear();
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

    /// Returns how many files are in the files vector. The already downloaded
    /// files are also taking into account.
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

    async fn get_next_chunk(&mut self) -> Vec<DownloadableObject> {
        const DEFAULT_CHUNK_SIZE: usize = 16;

        let remaining = self.files.len() - self.start;
        if remaining == 0 {
            return vec![];
        }

        let chunk_size = DEFAULT_CHUNK_SIZE.min(remaining);
        let end = self.start + chunk_size;

        let mut objects = vec![];

        loop {
            if objects.len() >= DEFAULT_CHUNK_SIZE {
                break;
            }

            if self.start == end {
                break;
            }

            let obj = &self.files[self.start];

            // Check if the file already exists so we can skit it.
            if let Ok(true) = verify_file_hash(&obj.path, &obj.hash).await {
                info!("Skipping {:?}, already exists", obj.path);
            } else {
                objects.push(obj.clone());
            }
            self.start += 1;
        }
        objects
    }

    async fn make_requests(&mut self) -> Result<DownloadState> {
        let chunk = self.get_next_chunk().await;
        if chunk.is_empty() {
            return Ok(DownloadState::Completed);
        }

        let sem = self
            .acquire_semaphore()
            .await?;
        let client = self.requester.clone();
        let task = tokio::spawn(async move { download_and_write(chunk, client, sem).await });

        info!("Pushing new task {}", self.start);
        self.tasks.push_back(task);
        Ok(DownloadState::MakingRequests)
    }
}

async fn download_and_write(
    objects: Vec<DownloadableObject>,
    requester: reqwest::Client,
    _sem: OwnedSemaphorePermit,
) -> Result<()> {
    let x = objects
        .into_iter()
        .map(|obj| async {
            let response = match requester
                .get(&obj.url)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => return Err(UraniumError::from(e)),
            };

            download_single_file(response, obj).await
        });

    let errors: Vec<DownloadableObject> = join_all(x)
        .await
        .into_iter()
        .flat_map(|e| match e {
            Err(UraniumError::FileNotMatch(obj)) => Some(obj),
            Err(error) => {
                error!("Error with the response: {}", error);
                None
            }
            _ => None,
        })
        .collect();

    if !errors.is_empty() {
        return Err(UraniumError::FilesDontMatch(errors));
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
                .ok_or(UraniumError::OtherWithReason(format!(
                    "Cant create {:?} path",
                    obj.path
                )))?,
        )?;
    }

    let mut file = tokio::io::BufWriter::with_capacity(
        1024 * 512,
        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&obj.path)
            .await?,
    );

    let mut total = 0;
    let mut hasher = sha1::Sha1::new();

    while let Some(item) = bytes_stream.next().await {
        let chunk = item?;
        match file.write_all(&chunk).await {
            Err(e) => {
                error!("Can not write in {:?}: {}", obj.path, e);
                return Err(e.into());
            }
            Ok(_) => total += chunk.len(),
        };
        hasher.update(chunk);
    }
    file.flush().await?;
    let actual = hex::encode(hasher.finalize());

    if total == content_length
        && obj
            .hash
            .as_ref()
            .is_none_or(|x| match x {
                HashType::Sha1(h) => h == &actual,
            })
    {
        Ok(())
    } else {
        error!("{:?}'s hash doesn't match!", &obj.path);
        Err(UraniumError::FileNotMatch(obj))
    }
}
