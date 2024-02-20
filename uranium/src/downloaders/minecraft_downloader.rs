use super::gen_downloader::{DownloadState, DownlodableObject, FileDownloader, HashType};
use crate::{code_functions::N_THREADS, error::UraniumError, variables::constants::PROFILES_FILE};
use log::{error, info, warn};
use mine_data_strutcs::minecraft::{
    Lib, Libraries, MinecraftInstance, MinecraftInstances, ObjectData, ProfileData, ProfilesJson,
    Resources,
};
use reqwest;
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::RwLock,
};
use tokio::io::AsyncWriteExt;

const ASSESTS_PATH: &str = "assets/";
const OBJECTS_PATH: &str = "objects";
const INSTANCES_LIST: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

/*

   MINECRAFT INSTANCES VERSIONS/LIST ?

*/

/// Function that returns a list `Result<MinecraftInstances, UraniumError>`
///
/// Returns a `Result<_, _>` where the `Ok()` value is a `MinecraftInstances` struct
/// and the `Err()` value a `UraniumError`.
///
/// # Errors
/// This function can fail when fetching the minecraft versions from Microsoft page. In that case
/// this function will return an `Err(UraniumError::RequestError)`
pub async fn list_instances() -> Result<MinecraftInstances, UraniumError> {
    let requester = reqwest::Client::new();

    let instances = requester
        .get(INSTANCES_LIST)
        .send()
        .await
        .map_err(|_| UraniumError::RequestError)?
        .json::<MinecraftInstances>()
        .await
        .map_err(|_| UraniumError::RequestError)?;

    Ok(instances)
}

/*

        DOWNLOAD MINECRAFT RESOURCES CODE SECTION

*/

/// Indicates the download state of a Minecraft instance.
#[derive(Debug, Clone)]
pub enum MinecraftDownloadState {
    GettingSources,
    DownloadingIndexes,
    DownloadingAssests,
    DownloadingLibraries,
    CheckingFiles,
    Completed,
}

/// This struct is responsable of downloading Minecraft and it's libraries.
///
///
/// # Example:
///
/// ```no_run
/// use uranium::downloaders::{FileDownloader, MinecraftDownloader, MinecraftDownloadState};
///
/// # fn foo<T: FileDownloader + Send + Sync>() {
/// async {
///     // T: FileDownloader + Send + Sync
///     let mut minecraft_down = MinecraftDownloader::<T>::init("my/path", "1.20.1").await;
///
///     loop {
///         let state = minecraft_down.progress().await;
///
///         match state {
///             // If completed break
///             Ok(MinecraftDownloadState::Completed) => {
///                 println!("Instalation completed!");
///                 break;
///             },
///             // Doing progress
///             Ok(_) => {
///                 println!("Instaling...");
///             },
///
///             // Also if error break.
///             Err(e) => {
///                 eprintln!("Error while installing minecraft: {}", e);
///                 break;
///            },
///         }
///     }
/// };
/// # }
/// ```
pub struct MinecraftDownloader<T: FileDownloader> {
    requester: reqwest::Client,
    destination_path: PathBuf,
    resources: Vec<DownlodableObject>,
    minecraft_instance: MinecraftInstance,
    download_state: MinecraftDownloadState,
    downloader: Option<T>,

    #[allow(unused)]
    bad_files: RwLock<Vec<ObjectData>>,
}

impl<T: FileDownloader + Send + Sync> MinecraftDownloader<T> {
    /// Makes a new `MinecraftDownloader` struct.
    ///
    /// - `destination_path`: Where minecraft will be downloaded.
    /// - `minecraft_version`: Which versions is going to be downloaded.
    ///
    /// # Panics
    ///
    /// This function will panic if the `minecraft_version` does not exists
    ///
    /// # Examples
    ///
    /// This will panic:
    /// ```
    /// use uranium::downloaders::MinecraftDownloader;
    /// use uranium::downloaders::FileDownloader;
    ///
    /// # fn foo<T: FileDownloader + Send + Sync>() {
    /// // Where T: FileDownloader + Send + Sync
    /// MinecraftDownloader::<T>::init("my/mine/path", "league of legends");
    /// # }
    /// ```
    pub async fn init<I: AsRef<Path>>(destination_path: I, minecraft_version: &str) -> Self {
        let requester = reqwest::Client::new();
        let intances = list_instances()
            .await
            .expect("Couldnt get minecraft versions");

        let instance_url = intances
            .get_instance_url(minecraft_version)
            .unwrap_or_else(|| panic!("Couldnt find {minecraft_version} version"));

        let instance: MinecraftInstance = requester
            .get(instance_url)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        MinecraftDownloader::new(destination_path.as_ref().to_path_buf(), instance)
    }

    /// WIP
    fn new(destination_path: PathBuf, minecraft_instance: MinecraftInstance) -> Self {
        MinecraftDownloader {
            requester: reqwest::Client::new(),
            destination_path,
            resources: vec![],
            minecraft_instance,
            download_state: MinecraftDownloadState::GettingSources,
            downloader: None,
            bad_files: RwLock::new(vec![]),
        }
    }

    /// This function will start the download anb block until
    /// `Ok(MinecraftDownloadState::Completed)`is returned if success or
    /// `Err(UraniumError)` if failed.
    ///
    /// # Errors
    /// This method will call `self.progress()` repeatedly. If there is any error, this method will
    /// propagate it.
    pub async fn start(&mut self) -> Result<MinecraftDownloadState, UraniumError> {
        loop {
            let state = self.progress().await;

            match state {
                Ok(MinecraftDownloadState::Completed) => break,
                Err(e) => return Err(e),
                _ => {}
            }
        }
        Ok(MinecraftDownloadState::Completed)
    }

    /// This function will make progress in the installation. It will go through all the
    /// installations steps (`MinecraftDownloadState`) so the user can know what is the
    /// downloader doing and can show a progress bar, info logs...
    ///
    ///
    /// Every time a step is completed `self.download_state` will change to the next step
    /// working like a FSM.
    ///
    /// # Errors
    ///
    /// Because this struct works like a State Machine this function can fail in many steps. Each
    /// step will return the corresponding `Err(UraniumError)` if an error occurs.
    ///
    /// # Panics
    ///
    /// This function should not panic
    pub async fn progress(&mut self) -> Result<MinecraftDownloadState, UraniumError> {
        match self.download_state {
            MinecraftDownloadState::GettingSources => {
                self.get_sources().await?;
                self.download_state = MinecraftDownloadState::DownloadingIndexes;
            }

            MinecraftDownloadState::DownloadingIndexes => {
                if self.creater_assest_folders(&self.resources).is_err() {
                    error!("Error creating assests folders");
                    return Err(UraniumError::CantCreateDir);
                };

                let mut files = vec![];
                std::mem::swap(&mut files, self.resources.as_mut());
                self.downloader = Some(T::new(files));

                self.download_state = MinecraftDownloadState::DownloadingAssests;
            }

            MinecraftDownloadState::DownloadingAssests => {
                // SAFETY: The previous step will ALWAYS init the downloader into Some(Downloader).
                let download_state = self.downloader.as_mut().unwrap().progress().await;

                match download_state {
                    // Here we prepare to download minecraft libs.
                    Ok(DownloadState::Completed) => {
                        self.prepare_libraries()?;
                        self.download_state = MinecraftDownloadState::DownloadingLibraries;
                    }
                    Err(e) => {
                        if let UraniumError::WriteError(io_err) = &e {
                            error!("Io error: {io_err}");
                        }
                        error!("Error downloading assests: {e}");
                        return Err(e);
                    }
                    _ => {}
                }
            }

            MinecraftDownloadState::DownloadingLibraries => {
                // Again the same process of:
                // While not completed or no error keep doing progress
                let download_state = self.downloader.as_mut().unwrap().progress().await;

                match download_state {
                    Ok(DownloadState::Completed) => {
                        self.download_state = MinecraftDownloadState::CheckingFiles;
                    }
                    Err(e) => {
                        if let UraniumError::WriteError(io_err) = &e {
                            error!("Io error: {io_err}");
                        }
                        error!("Error downloading assests: {e}");
                        return Err(e);
                    }
                    _ => {}
                }
                self.download_state = MinecraftDownloadState::CheckingFiles;
            }

            MinecraftDownloadState::CheckingFiles => {
                self.download_state = MinecraftDownloadState::Completed;
                // self.fix_wrong_file().await?;
            }

            MinecraftDownloadState::Completed => {
                info!("Minecraft download complete!");
            }
        };

        Ok(self.download_state.clone())
    }

    /// Returns the number of requests left to be processed by the downloader, taking into account
    /// the configured number of threads for concurrent processing.
    ///
    /// This method checks if a downloader is associated with the current instance, and if so, it
    /// queries the number of requests left from the downloader. The result is then adjusted to
    /// distribute the workload evenly among the configured number of threads.
    ///
    /// # Returns
    /// The adjusted number of requests left to be processed by the downloader. If there is no
    /// downloader associated with the current instance, it returns 0.
    pub fn requests_left(&self) -> usize {
        self.downloader
            .as_ref()
            .map(|d| (d.requests_left() as f64 / N_THREADS() as f64).ceil() as usize)
            .unwrap_or_default()
    }

    /// Returns the number of chunks of libs to download: `libs.len() / N_THREADS()`
    pub fn lib_chunks(&self) -> usize {
        let n = self.minecraft_instance.get_libs().len() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    /// Return the number of chunks to download.
    ///
    /// If the downloader is empty, then this method will download 0.
    pub fn chunks(&self) -> usize {
        let n = self.downloader.as_ref().map(|d| d.len()).unwrap_or_default() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    /// If a call to this function success it will set
    /// `self.resources` to `Some(Resources)`.
    ///
    /// If fails it will return the error in `Err()`.
    async fn get_sources(&mut self) -> Result<(), UraniumError> {
        /*
        self.resources = Some(
            self.requester
                .get(self.minecraft_instance.get_assests_url())
                .send()
                .await
                .map_err(|_| UraniumError::RequestError)?
                .json::<Resources>()
                .await
                .map_err(|_| UraniumError::RequestError)?,
        );
        */

        //let mut objects: Vec<&ObjectData> = resources.objects.values().collect();
        let resources: Resources = self
            .requester
            .get(self.minecraft_instance.get_assests_url())
            .send()
            .await
            .map_err(|_| UraniumError::RequestError)?
            .json::<Resources>()
            .await
            .map_err(|_| UraniumError::RequestError)?;

        if tokio::fs::create_dir_all(self.destination_path.join("assets/indexes"))
            .await
            .is_err()
        {
            error!("Cant create assets/indexes");
            return Err(UraniumError::CantCreateDir);
        }

        if tokio::fs::create_dir_all(self.destination_path.join("assets/objects"))
            .await
            .is_err()
        {
            error!("Cant create assets/objects");
            return Err(UraniumError::CantCreateDir);
        }

        self.create_indexes(&resources).await?;

        let base = PathBuf::from(ASSESTS_PATH).join(OBJECTS_PATH);

        for obj in resources.objects.values() {
            let url = obj.get_link();
            let path = base.join(&obj.hash[..2]).join(&obj.hash);
            self.resources.push(DownlodableObject::new(
                &url,
                path.to_str().unwrap_or_default(),
                &self.destination_path,
                Some(HashType::Sha1(obj.hash.to_owned())),
            ));
        } 

        Ok(())
    }

    /// Makes the minecraft index.json file
    async fn create_indexes(&self, resources: &Resources) -> Result<(), UraniumError> {
        let indexes_path = self
            .destination_path
            .join(ASSESTS_PATH)
            .join("indexes")
            .join(self.minecraft_instance.get_index_name());

        let mut indexes = tokio::fs::File::create(indexes_path).await?;

        indexes
            .write_all(
                serde_json::to_string(resources)
                    .unwrap_or_default()
                    .as_bytes(),
            )
            .await?;

        Ok(())
    }

    /// When success all the assests folder are created
    fn creater_assest_folders(&self, names: &[DownlodableObject]) -> Result<(), UraniumError> {
        for p in names {
            std::fs::create_dir_all(self.destination_path.join(&p.name).parent().unwrap())?;
        }

        Ok(())
    }

    /// Return a `Vec<String>` with the urls of the libraries for the current.
    /// If the lib has no specified Os then it will be inside the vector too.
    fn get_os_libraries(libraries: &Libraries) -> Vec<String> {
        let current_os = match std::env::consts::OS {
            "linux" => mine_data_strutcs::minecraft::Os::Linux,
            "macos" => mine_data_strutcs::minecraft::Os::MacOS,
            // "windows" => mine_data_strutcs::minecraft::Os::Windows,
            _ => mine_data_strutcs::minecraft::Os::Windows,
        };

        libraries
            .iter()
            .filter(|lib| lib.get_os().is_none() || lib.get_os().is_some_and(|os| os == current_os))
            .map(|lib| lib.get_url().to_owned())
            .collect()
    }

    /// This function sets `self.downloader` with the urls and paths in order to
    /// download minecraft libraries corresponding to the user OS.
    ///
    /// This function **WILL NOT** start the download in any way.
    fn prepare_libraries(&mut self) -> Result<(), UraniumError> {
        let libraries = self.minecraft_instance.get_libs();
        let raw_paths = libraries.get_paths();
        let urls = Self::get_os_libraries(libraries);

        let good_paths: Vec<PathBuf> = raw_paths
            .iter()
            .map(|p| {
                self.destination_path
                    .join(PathBuf::from("libraries").join(p))
            })
            .collect();

        for p in &good_paths {
            std::fs::create_dir_all(p.parent().unwrap())?;
        }

        let files = good_paths
            .iter()
            .zip(&urls)
            .zip(raw_paths)
            .map(|((path, url), lib_path)| {
                DownlodableObject::new(
                    url,
                    lib_path.file_name().unwrap().to_str().unwrap_or_default(),
                    path.parent().unwrap(),
                    None,
                )
            })
            .collect();

        self.downloader = Some(T::new(files));

        Ok(())
    }

    #[allow(clippy::await_holding_lock)]
    async fn _fix_wrong_file(&mut self) -> Result<(), UraniumError> {
        while !self.bad_files.read().unwrap().is_empty() {
            let mut guard = self.bad_files.write().unwrap();
            warn!("{} wrong files, trying to fix them", guard.len());

            let objects: Vec<ObjectData> = guard.drain(..).collect();
            drop(guard);

            let _names: Vec<PathBuf> = objects
                .iter()
                .map(|obj| {
                    PathBuf::from(ASSESTS_PATH)
                        .join(OBJECTS_PATH)
                        .join(&obj.hash[..2])
                        .join(&obj.hash)
                })
                .collect();

            let _urls: Vec<String> = objects.iter().map(ObjectData::get_link).collect();

            T::new(
                // TODO, FIXME
                vec![],
            )
            .complete()
            .await?;

            // God forgive me until i found a better way to do this.
            let _aux: Vec<&ObjectData> = objects.iter().collect();
        }

        Ok(())
    }

    /// This function will add a new minecraft profile to `launcher_profiles.json` file
    /// located in `minecraft_path` dir.
    ///
    /// If `icon` is not specified the default Grass icon will be set.
    ///
    /// # Errors
    /// If the `minecraft_path` doesn't exits or is not valid then
    /// `Err(UraniumError::FileNotFound)` will be returned.
    ///
    /// Also, if the profile file is not valid `Err(UraniumError::WrongFileFormat)` will be
    /// returned
    ///
    /// In case it is not possible to write into the file then `Err(UraniumError::WriteError)` will
    /// be returned
    pub fn add_instance<I: AsRef<Path>>(
        &self,
        minecraft_path: I,
        instance_name: &str,
        icon: Option<&str>,
    ) -> Result<(), UraniumError> {
        let profiles_path = minecraft_path.as_ref().to_path_buf().join(PROFILES_FILE);

        if !profiles_path.exists() {
            return Err(UraniumError::FileNotFound);
        }

        let Ok(mut profiles): Result<ProfilesJson, _> =
            serde_json::from_reader(File::open(&profiles_path)?)
        else {
            return Err(UraniumError::FileNotFound);
        };

        let icon = icon.unwrap_or("Grass");

        let new_profile = ProfileData::new(
            icon,
            &self.minecraft_instance.id,
            instance_name,
            "custom",
            Some(&self.destination_path),
        );

        profiles.add_profile(instance_name, new_profile);

        info!("Writting new profile");

        let Ok(content) = serde_json::to_string_pretty(&profiles) else {
            return Err(UraniumError::WrongFileFormat);
        };

        if let Err(err) = std::fs::write(profiles_path, content) {
            error!("Error writting the new profile");
            return Err(err.into());
        }

        info!("Profile added!");
        Ok(())
    }
}
