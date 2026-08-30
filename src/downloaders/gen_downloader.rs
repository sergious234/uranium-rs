use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;
use futures_util::future::join_all;
use futures_util::stream::FuturesUnordered;
use log::{error, info};
use mine_data_structs::minecraft::{AssetIndex, Library, ObjectData};
use rayon::iter::{ParallelBridge, ParallelIterator};
use reqwest::Response;
use sha1::Digest;
use tokio::io::AsyncWriteExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::error::Result;
use crate::hashes::bytes_to_hex;
use crate::{code_functions::N_THREADS, error::UraniumError};

const BUFFER_SIZER: usize = 1024 * 512;
const MAX_RETRIES: u8 = 3;
const DEFAULT_CHUNK_SIZE: usize = 32;

/// A trait for asynchronous file downloading.
///
/// This trait provides a generic interface that allows different downloader
/// implementations to be used interchangeably. This promotes a flexible and
/// modular design.
#[allow(async_fn_in_trait)]
pub trait FileDownloader {
    /// Builds a new struct.
    fn new() -> Self;

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

    /// Calls `Self::progress()` repeatedly until `DownloadState::Completed`.
    async fn start(&mut self) -> Result<()> {
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

impl std::convert::From<&Library> for DownloadableObject {
    fn from(value: &Library) -> Self {
        let d = value
            .downloads
            .as_ref()
            .expect("Library must have downloads to convert to DownloadableObject");

        Self {
            url: d.artifact.url.clone(),
            hash: Some(HashType::Sha1(d.artifact.sha1.clone())),
            path: d.artifact.path.clone(),
        }
    }
}

impl std::convert::From<&ObjectData> for DownloadableObject {
    fn from(obj: &ObjectData) -> Self {
        Self {
            url: obj.get_link(),
            hash: Some(HashType::Sha1(obj.hash.clone())),
            path: obj.get_path(),
        }
    }
}

impl std::convert::From<&AssetIndex> for DownloadableObject {
    fn from(value: &AssetIndex) -> Self {
        let (url, path, hash) = (value.url.clone(), value.id.clone(), value.sha1.clone());
        Self {
            url,
            hash: Some(HashType::Sha1(hash)),
            path: path.into(),
        }
    }
}

/// A concrete implementation of `FileDownloader` that
/// manages a pool of download tasks, ensuring that a limited number of
/// requests are active at any time.
///
/// # Concurrency model (double-buffered)
///
/// The downloader maintains two regions:
/// - **`files[cursor..]`** — files not yet dispatched (the backlog).
/// - **`tasks`** — spawned tokio tasks downloading chunks of up to 16 files.
///
/// `cursor` advances monotonically through the initial file list as files are
/// verified and dispatched. Failed files (retries) and files added via
/// `add_object` / `add_objects` are pushed to the end of `files`.
/// `get_next_chunk` pulls up to `DEFAULT_CHUNK_SIZE` items from `cursor..`,
/// skipping files whose hashes already match on disk.
/// `make_requests` spawns each chunk as an independent task. On completion,
/// the task returns a list of failed files (`FilesDontMatch`), which the
/// retry loop re-queues at the end of `files` or gives up after `MAX_RETRIES`.
pub struct Downloader {
    /// Queue of files to download. `cursor` divides dispatched vs. pending.
    files: Vec<DownloadableObject>,
    requester: reqwest::Client,
    /// Index of the first not-yet-dispatched file in `files`.
    cursor: usize,
    /// Semaphore limiting concurrent download tasks to `N_THREADS()`.
    s: Arc<Semaphore>,
    /// In-flight download tasks.
    tasks: FuturesUnordered<JoinHandle<Result<()>>>,
    /// Retry count per URL — entries > `MAX_RETRIES` are silently dropped.
    retries: HashMap<String, u8>,
}

impl FileDownloader for Downloader {
    fn new() -> Self {
        info!("{} available permits", N_THREADS());

        let client = reqwest::Client::new();

        Downloader {
            files: vec![],
            requester: client,
            cursor: 0,
            s: Arc::new(Semaphore::new(N_THREADS())),
            tasks: FuturesUnordered::new(),
            retries: HashMap::new(),
        }
    }

    async fn progress(&mut self) -> Result<DownloadState> {
        while self.cursor != self.files.len() && self.s.available_permits() > 0 {
            self.make_requests().await?;
        }

        let Some(task) = self.tasks.next().await else {
            return Ok(DownloadState::Completed);
        };

        match task? {
            Err(UraniumError::FilesDontMatch(objects)) => {
                let mut exhausted = Vec::new();
                for obj in objects {
                    let entry = self
                        .retries
                        .entry(obj.url.clone())
                        .or_insert(0);
                    *entry += 1;
                    if *entry > MAX_RETRIES {
                        error!("Giving up on {:?} after {MAX_RETRIES} retries", obj.path);
                        exhausted.push(obj);
                    } else {
                        self.files.push(obj);
                    }
                }
                if !exhausted.is_empty() {
                    return Err(UraniumError::FilesDontMatch(exhausted.into_boxed_slice()));
                }
            }
            Err(e) => return Err(e),
            Ok(_) => {}
        }
        info!("Reporting state");
        Ok(DownloadState::Downloading)
    }

    /// Returns how many requests are left.
    fn requests_left(&self) -> usize {
        self.files.len() - self.cursor + self.tasks.len()
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
            .map_err(|e| UraniumError::other(format!("Failed to acquire semaphore: {e}")))
    }

    async fn get_next_chunk(&mut self) -> Vec<DownloadableObject> {
        let remaining = self.files.len() - self.cursor;
        if remaining == 0 {
            return vec![];
        }
        let chunk_size = DEFAULT_CHUNK_SIZE.min(remaining);

        let to_download: Vec<_> = self.files[self.cursor..]
            .iter()
            .take(chunk_size)
            .par_bridge()
            .filter(|obj| !matches!(verify_file_hash(&obj.path, &obj.hash), Ok(Some(true))))
            .cloned()
            .collect();

        self.cursor += chunk_size;
        let skipped = chunk_size - to_download.len();
        if skipped != 0 {
            info!(
                "Skipping {} objs, already exists and hash matches",
                skipped
            );
        }
        to_download
    }

    async fn make_requests(&mut self) -> Result<DownloadState> {
        let chunk = self.get_next_chunk().await;
        let chunk_len = chunk.len();
        if chunk.is_empty() {
            return Ok(DownloadState::Completed);
        }

        let sem = self
            .acquire_semaphore()
            .await?;
        let client = self.requester.clone();
        // let task = tokio::spawn(async move { download_and_write(chunk, client, sem).await });
        let task = tokio::spawn(download_and_write(chunk, client, sem));
        self.tasks.push(task);

        info!("Pushing new task with {} objects", chunk_len);
        Ok(DownloadState::MakingRequests)
    }
}

async fn download_and_write(
    objects: Vec<DownloadableObject>,
    requester: reqwest::Client,
    _sem: OwnedSemaphorePermit,
) -> Result<()> {
    let tasks = objects
        .into_iter()
        .map(|obj| async {
            let response = match requester
                .get(&obj.url)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("Request to {} failed: {e}", obj.url);
                    return Err(UraniumError::FileNotMatch(obj));
                }
            };

            download_single_file(response, obj).await
        });

    let errors: Box<[DownloadableObject]> = join_all(tasks)
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

/// Returns:
/// - `Ok(Some(true))` — file exists and hash matches
/// - `Ok(Some(false))` — file exists but hash doesn't match
/// - `Ok(None)` — no hash provided, can't verify
/// - `Err(e)` — IO error reading the file
fn verify_file_hash(path: &Path, expected_hash: &Option<HashType>) -> Result<Option<bool>> {
    if !path.exists() {
        return Ok(None);
    }

    let Some(HashType::Sha1(expected)) = expected_hash else {
        return Ok(None);
    };

    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha1::Sha1::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let actual = bytes_to_hex(&hasher.finalize());

    Ok(Some(&actual == expected))
}

async fn download_single_file(response: Response, obj: DownloadableObject) -> Result<()> {
    if !response.status().is_success() {
        error!(
            "{} returned status {} for {:?}",
            obj.url,
            response.status(),
            obj.path
        );
        return Err(UraniumError::FileNotMatch(obj));
    }

    let expected_size = response.content_length();

    let mut bytes_stream = response.bytes_stream();

    if let Some(parent) = obj.path.parent()
        && let Err(e) = create_dir_all(parent)
    {
        error!("Can not create directories for {:?}: {}", obj.path, e);
        return Err(UraniumError::FileNotMatch(obj));
    }

    let file = match tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&obj.path)
        .await
    {
        Err(e) => {
            error!("Can not open {:?}: {}", obj.path, e);
            return Err(UraniumError::FileNotMatch(obj));
        }
        Ok(f) => f,
    };

    let mut file = tokio::io::BufWriter::with_capacity(BUFFER_SIZER, file);

    let mut total = 0;
    let mut hasher = sha1::Sha1::new();

    while let Some(item) = bytes_stream.next().await {
        let chunk = match item {
            Err(e) => {
                error!("Error reading stream for {:?}: {}", obj.path, e);
                return Err(UraniumError::FileNotMatch(obj));
            }
            Ok(c) => c,
        };
        match file.write_all(&chunk).await {
            Err(e) => {
                error!("Can not write in {:?}: {}", obj.path, e);
                return Err(UraniumError::FileNotMatch(obj));
            }
            Ok(_) => total += chunk.len(),
        };
        hasher.update(chunk);
    }
    if let Err(e) = file.flush().await {
        error!("Can not flush {:?}: {}", obj.path, e);
        return Err(UraniumError::FileNotMatch(obj));
    }
    let file_hash = bytes_to_hex(&hasher.finalize());

    let hash_ok = obj
        .hash
        .as_ref()
        .is_none_or(|x| match x {
            HashType::Sha1(h) => h == &file_hash,
        });
    let size_ok = expected_size.is_none_or(|expected| total == expected as usize);

    if hash_ok && size_ok {
        Ok(())
    } else {
        error!("{:?}'s hash doesn't match!", obj.path);
        Err(UraniumError::FileNotMatch(obj))
    }
}

#[cfg(test)]
mod tests {
    use mine_data_structs::minecraft::{
        Artifact, AssetIndex, Library, LibraryDownloads, ObjectData,
    };

    use super::*;

    #[test]
    fn from_library_with_downloads() {
        let lib = Library {
            name: "org.example:test:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("org/example/test/1.0/test.jar"),
                    sha1: "abcdef1234567890".into(),
                    size: 1024,
                    url: "https://example.com/test.jar".into(),
                },
                classifiers: None,
            }),
            rules: None,
        };

        let obj = DownloadableObject::from(&lib);
        assert_eq!(obj.url, "https://example.com/test.jar");
        assert_eq!(obj.path, PathBuf::from("org/example/test/1.0/test.jar"));
        assert!(matches!(obj.hash, Some(HashType::Sha1(ref h)) if h == "abcdef1234567890"));
    }

    #[test]
    fn from_object_data_constructs_correctly() {
        let data = ObjectData {
            hash: "a1b2c3d4e5f6a7b8c9d0".into(),
            size: 512,
        };

        let obj = DownloadableObject::from(&data);
        assert_eq!(
            obj.url,
            "https://resources.download.minecraft.net/a1/a1b2c3d4e5f6a7b8c9d0"
        );
        assert_eq!(obj.path, PathBuf::from("a1/a1b2c3d4e5f6a7b8c9d0"));
        assert!(matches!(obj.hash, Some(HashType::Sha1(ref h)) if h == "a1b2c3d4e5f6a7b8c9d0"));
    }

    #[test]
    fn from_asset_index_uses_id_as_path() {
        let index = AssetIndex {
            id: "19".into(),
            sha1: "fff000fff000".into(),
            size: 4096,
            total_size: 100_000,
            url: "https://example.com/index.json".into(),
        };

        let obj = DownloadableObject::from(&index);
        assert_eq!(obj.url, "https://example.com/index.json");
        assert_eq!(obj.path, PathBuf::from("19"));
        assert!(matches!(obj.hash, Some(HashType::Sha1(ref h)) if h == "fff000fff000"));
    }

    #[test]
    fn downloadable_object_name_returns_filename() {
        let obj = DownloadableObject::new(
            "https://example.com/test.jar",
            &PathBuf::from("/some/path/test.jar"),
            None,
        );
        assert_eq!(obj.name(), Some("test.jar"));
    }

    #[test]
    fn downloadable_object_name_on_dir_returns_none() {
        let obj = DownloadableObject::new(
            "https://example.com/test",
            &PathBuf::from("/some/path/"),
            None,
        );
        assert!(obj.name().is_some());
    }

    #[test]
    fn verify_file_hash_none_for_missing_file() {
        let result = verify_file_hash(
            &PathBuf::from("/nonexistent/path/file.jar"),
            &Some(HashType::Sha1("abc".into())),
        );
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn verify_file_hash_none_when_no_hash_provided() {
        let result = verify_file_hash(
            &PathBuf::from("/nonexistent/path/file.jar"),
            &None,
        );
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn downloadable_object_new_sets_fields() {
        let obj = DownloadableObject::new(
            "https://example.com/file.txt",
            &PathBuf::from("/tmp/file.txt"),
            Some(HashType::Sha1("deadbeef".into())),
        );
        assert_eq!(obj.url, "https://example.com/file.txt");
        assert_eq!(obj.path, PathBuf::from("/tmp/file.txt"));
        assert!(matches!(obj.hash, Some(HashType::Sha1(ref h)) if h == "deadbeef"));
    }
}
