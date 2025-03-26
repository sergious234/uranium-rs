use std::io::Write;
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::RwLock,
};

use log::{error, info, warn};
use mine_data_structs::minecraft::{
    Library, MinecraftVersions, ObjectData, Profile, ProfilesJson, Resources, Root
};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use super::gen_downloader::{DownloadState, DownloadableObject, FileDownloader, HashType};
use super::RuntimeDownloader;
use crate::{
    code_functions::N_THREADS,
    error::{Result, UraniumError},
    variables::constants::PROFILES_FILE,
};

const ASSETS_PATH: &str = "assets/";
const OBJECTS_PATH: &str = "objects";
const INSTANCES_LIST: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

/*

   MINECRAFT INSTANCES VERSIONS/LIST ?

*/

/// Function that returns a list `Result<MinecraftInstances, UraniumError>`
///
/// Returns a `Result<_, _>` where the `Ok()` value is a `MinecraftInstances`
/// struct and the `Err()` value a `UraniumError`.
///
/// # Errors
/// This function can fail when fetching the minecraft versions from Microsoft
/// page. In that case this function will return an
/// `Err(UraniumError::RequestError)`
pub async fn list_instances() -> Result<MinecraftVersions> {
    let requester = reqwest::Client::new();

    let instances = requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?;

    Ok(instances)
}

/// Function that returns the latest Minecraft snapshot version as a
/// `Result<String, UraniumError>`.
///
/// Returns a `uranium_rs::error::Result<_, _>` where the `Ok()` value is a
/// `String` representing the latest snapshot version, and the `Err()` value is
/// a `UraniumError`.
///
/// # Errors
/// This function can fail when fetching the Minecraft versions from the
/// Microsoft page. In such a case, this function will return an
/// `Err(UraniumError::RequestError)`.
pub async fn get_last_snapshot() -> Result<String> {
    let requester = reqwest::Client::new();
    Ok(requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?
        .latest
        .snapshot)
}

/// Function that returns the latest Minecraft release version as a
/// `Result<String, UraniumError>`.
///
/// Returns a `uranium_rs::error::Result<_, _>` where the `Ok()` value is a
/// `String` representing the latest release version, and the `Err()` value is a
/// `UraniumError`.
///
/// # Errors
/// This function can fail when fetching the Minecraft versions from the
/// Microsoft page. In such a case, this function will return an
/// `Err(UraniumError::RequestError)`.
pub async fn get_last_release() -> Result<String> {
    let requester = reqwest::Client::new();
    Ok(requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?
        .latest
        .release)
}

/*

        DOWNLOAD MINECRAFT RESOURCES CODE SECTION

*/

/// Indicates the download state of a Minecraft instance.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MinecraftDownloadState {
    GettingSources,
    DownloadingIndexes,
    DownloadingAssests,
    DownloadingLibraries,
    CheckingFiles,
    Completed,
}

/// This struct is responsible for downloading Minecraft and it's libraries.
///
///
/// # Example:
///
/// ```no_run
/// use uranium_rs::downloaders::{FileDownloader, MinecraftDownloader, MinecraftDownloadState};
/// use uranium_rs::error::Result;
///
/// async fn foo<T: FileDownloader + Send + Sync>() -> Result<()> {
///     // T: FileDownloader + Send + Sync
///     let mut minecraft_down = MinecraftDownloader::<T>::init("my/path", "1.20.1").await?;
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
///                 return Err(e);
///            },
///         }
///     }
///     Ok(())
/// }
/// ```
pub struct MinecraftDownloader<T: FileDownloader + Send> {
    requester: reqwest::Client,
    dot_minecraft_path: PathBuf,
    resources: Vec<DownloadableObject>,
    minecraft_instance: Root,
    download_state: MinecraftDownloadState,
    downloader: Option<T>,

    #[allow(unused)]
    bad_files: RwLock<Vec<ObjectData>>,
}

impl<T: FileDownloader + Send + Sync> MinecraftDownloader<T> {
    /// Makes a new `MinecraftDownloader` struct.
    ///
    /// - `destination_path`: Where minecraft will be downloaded. (THIS IS
    ///   USUALLY `.minecraft` DIRECTORY)
    /// - `minecraft_version`: Which versions is going to be downloaded.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use uranium_rs::downloaders::MinecraftDownloader;
    /// use uranium_rs::downloaders::FileDownloader;
    /// use uranium_rs::error::Result;
    ///
    /// async fn foo<T: FileDownloader + Send + Sync>() -> Result<()>{
    ///
    ///     // This will result in an error since "league of legends" is mental illness.
    ///     // (and also a game)
    ///     MinecraftDownloader::<T>::init("my/mine/path", "league of legends").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn init<I: AsRef<Path>>(
        destination_path: I,
        minecraft_version: &str,
    ) -> Result<Self> {
        let requester = reqwest::Client::new();
        let instances = list_instances().await?;

        let instance_url = instances
            .get_instance_url(minecraft_version)
            .ok_or(UraniumError::OtherWithReason(format!(
                "Version {minecraft_version} doesn't exist"
            )))?;

        let minecraft_instance: Root = requester
            .get(instance_url)
            .send()
            .await?
            .json()
            .await?;

        let destination_path = destination_path
            .as_ref()
            .to_path_buf();

        Ok(MinecraftDownloader::new(
            destination_path,
            minecraft_instance,
        ))
    }

    /// WIP
    fn new(destination_path: PathBuf, minecraft_instance: Root) -> Self {
        MinecraftDownloader {
            requester: reqwest::Client::new(),
            dot_minecraft_path: destination_path,
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
    /// This method will call `self.progress()` repeatedly. If there is any
    /// error, this method will propagate it.
    pub async fn start(&mut self) -> Result<MinecraftDownloadState> {
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

    /// This function will make progress in the installation. It will go through
    /// all the installations steps (`MinecraftDownloadState`) so the user
    /// can know what is the downloader doing and can show a progress bar,
    /// info logs...
    ///
    ///
    /// Every time a step is completed `self.download_state` will change to the
    /// next step working like a FSM.
    ///
    /// # Errors
    ///
    /// Because this struct works like a State Machine this function can fail in
    /// many steps. Each step will return the corresponding
    /// `Err(UraniumError)` if an error occurs.
    ///
    /// # Panics
    ///
    /// This function should not panic
    pub async fn progress(&mut self) -> Result<MinecraftDownloadState> {
        match self.download_state {
            MinecraftDownloadState::GettingSources => {
                self.get_sources().await?;
                self.download_state = MinecraftDownloadState::DownloadingIndexes;
            }

            MinecraftDownloadState::DownloadingIndexes => {
                if self
                    .create_assess_folders(&self.resources)
                    .is_err()
                {
                    error!("Error creating assets folders");
                    return Err(UraniumError::CantCreateDir("assets"));
                };

                let mut files = vec![];
                std::mem::swap(&mut files, self.resources.as_mut());
                self.downloader = Some(T::new(files));

                self.download_state = MinecraftDownloadState::DownloadingAssests;
            }

            MinecraftDownloadState::DownloadingAssests => {
                // SAFETY: The previous step will ALWAYS init the downloader
                // into Some(Downloader).
                let download_state = self
                    .downloader
                    .as_mut()
                    .unwrap()
                    .progress()
                    .await;

                match download_state {
                    // Here we prepare to download minecraft libs.
                    Ok(DownloadState::Completed) => {
                        /*
                            Write inside .minecraft the client and version manual

                            .minecraft
                                | ...
                                |
                                | versions
                                    | X.XX.X            < Write this
                                        | X.XX.X.jar    < And this
                                        | X.XX.X.json   < And despite what everyone might think, this too

                        */
                        let url = self
                            .minecraft_instance
                            .downloads
                            .get("client")
                            .map(|i| i.url.clone())
                            .ok_or(UraniumError::OtherWithReason(
                                "Client .jar not found in the minecraft instance".to_owned(),
                            ))?;

                        let content = self
                            .requester
                            .get(url)
                            .send()
                            .await
                            .unwrap()
                            .bytes()
                            .await?;
                        let instance_folder = self
                            .dot_minecraft_path
                            .join("versions")
                            .join(&self.minecraft_instance.id);

                        if !instance_folder.exists() {
                            std::fs::create_dir_all(&instance_folder)?;
                        }

                        let client_path = instance_folder
                            .as_path()
                            .join(
                                self.minecraft_instance
                                    .id
                                    .clone()
                                    + ".jar",
                            );
                        if !client_path.exists() {
                            info!("Writing client!");
                            let mut client_file = File::create(client_path)?;
                            client_file.write_all(&content)?;
                        }

                        let manual_path = instance_folder.join(
                            self.minecraft_instance
                                .id
                                .clone()
                                + ".json",
                        );
                        if !manual_path.exists() {
                            info!("Writing client json!");
                            let mut manual_file = File::create(manual_path)?;
                            manual_file.write_all(
                                serde_json::to_string(&self.minecraft_instance)
                                    .unwrap()
                                    .as_bytes(),
                            )?;
                        }
                        self.prepare_libraries()?;
                        self.download_state = MinecraftDownloadState::DownloadingLibraries;
                    }
                    Err(e) => {
                        if let UraniumError::WriteError(io_err) = &e {
                            error!("Io error: {io_err}");
                        }
                        error!("Error downloading assets: {e}");
                        return Err(e);
                    }
                    _ => {}
                }
            }

            MinecraftDownloadState::DownloadingLibraries => {
                // Again the same process of:
                // While not completed or no error keep doing progress
                let download_state = self
                    .downloader
                    .as_mut()
                    .unwrap()
                    .progress()
                    .await;

                match download_state {
                    Ok(DownloadState::Completed) => {
                        let runtime_res = RuntimeDownloader::new(
                            self.minecraft_instance
                                .java_version
                                .component
                                .to_string(),
                        )
                        .download()
                        .await;

                        if let Err(err) = runtime_res {
                            error!("Error downloading runtime: {}", err);
                        }
                        self.download_state = MinecraftDownloadState::CheckingFiles;
                    }
                    Err(e) => {
                        if let UraniumError::WriteError(io_err) = &e {
                            error!("Io error: {io_err}");
                        }
                        error!("Error downloading assets: {e}");
                        return Err(e);
                    }
                    _ => {}
                }
                //self.download_state = MinecraftDownloadState::CheckingFiles;
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

    /// Returns the number of requests left to be processed by the downloader,
    /// taking into account the configured number of threads for concurrent
    /// processing.
    ///
    /// This method checks if a downloader is associated with the current
    /// instance, and if so, it queries the number of requests left from the
    /// downloader. The result is then adjusted to distribute the workload
    /// evenly among the configured number of threads.
    ///
    /// # Returns
    /// The adjusted number of requests left to be processed by the downloader.
    /// If there is no downloader associated with the current instance, it
    /// returns 0.
    pub fn requests_left(&self) -> usize {
        self.downloader
            .as_ref()
            .map(|d| (d.requests_left() as f64 / N_THREADS() as f64).ceil() as usize)
            .unwrap_or_default()
    }

    /// Returns the number of chunks of libs to download: `libs.len() /
    /// N_THREADS()`
    pub fn lib_chunks(&self) -> usize {
        let n = self
            .minecraft_instance
            .libraries
            .len() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    /// Return the number of chunks to download.
    ///
    /// If the downloader is empty, then this method will download 0.
    pub fn chunks(&self) -> usize {
        let n = self
            .downloader
            .as_ref()
            .map(|d| d.len())
            .unwrap_or_default() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    /// If a call to this function success it will set
    /// `self.resources` to `Some(Resources)`.
    ///
    /// If fails it will return the error in `Err()`.
    async fn get_sources(&mut self) -> Result<()> {
        let resources: Resources = self
            .requester
            .get(
                &self
                    .minecraft_instance
                    .asset_index
                    .url,
            )
            .send()
            .await?
            .json::<Resources>()
            .await?;

        tokio::fs::create_dir_all(
            self.dot_minecraft_path
                .join("assets/indexes"),
        )
        .await
        .map_err(|err| {
            error!("Cant create assets/indexes");
            UraniumError::OtherWithReason(format!("assets/indexes: [{}]", err))
        })?;

        if tokio::fs::create_dir_all(
            self.dot_minecraft_path
                .join("assets/objects"),
        )
        .await
        .is_err()
        {
            error!("Cant create assets/objects");
            return Err(UraniumError::CantCreateDir("assets/objects"));
        }

        self.create_indexes(&resources)
            .await?;

        let base = PathBuf::from(ASSETS_PATH).join(OBJECTS_PATH);

        for obj in resources.objects.values() {
            let url = obj.get_link();
            let path = base
                .join(&obj.hash[..2])
                .join(&obj.hash);
            self.resources
                .push(DownloadableObject::new(
                    &url,
                    &self
                        .dot_minecraft_path
                        .join(path),
                    Some(HashType::Sha1(obj.hash.to_owned())),
                ));
        }

        Ok(())
    }

    /// Makes the minecraft index.json file
    async fn create_indexes(&self, resources: &Resources) -> Result<()> {
        let indexes_path = self
            .dot_minecraft_path
            .join(ASSETS_PATH)
            .join("indexes")
            .join(
                self.minecraft_instance
                    .get_index_name(),
            );

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

    /// When success all the assets folder are created
    fn create_assess_folders(&self, names: &[DownloadableObject]) -> Result<()> {
        for p in names {
            std::fs::create_dir_all(
                self.dot_minecraft_path
                    .join(
                        p.name()
                            .ok_or(UraniumError::other("No filename"))?,
                    )
                    .parent()
                    .ok_or(UraniumError::other("Error creating assests forlder"))?,
            )?;
        }

        Ok(())
    }

    /// Return a `Vec<String>` with the urls of the libraries for the current.
    /// If the lib has no specified Os then it will be inside the vector too.
    fn get_os_libraries(libraries: &[Library]) -> Vec<String> {
        let current_os = match std::env::consts::OS {
            "linux" => mine_data_structs::minecraft::Os::Linux,
            "macos" => mine_data_structs::minecraft::Os::Other,
            // "windows" => mine_data_structs::minecraft::Os::Windows,
            _ => mine_data_structs::minecraft::Os::Windows,
        };

        libraries
            .iter()
            .filter(|lib| {
                lib.get_os().is_none()
                    || lib
                        .get_os()
                        .is_some_and(|os| os == current_os)
            })
            .map(|lib| lib.get_url().to_owned())
            .collect()
    }

    /// This function sets `self.downloader` with the urls and paths in order to
    /// download minecraft libraries corresponding to the user OS.
    ///
    /// This function **WILL NOT** start the download in any way.
    fn prepare_libraries(&mut self) -> Result<()> {
        let libraries = &self
            .minecraft_instance
            .libraries;

        let lib_path = self
            .dot_minecraft_path
            .join("libraries");


        let files = libraries
            .iter()
            .map(|l| {
                &l.downloads
                    .as_ref()
                    .unwrap()
                    .artifact
            })
            .map(|art| {
                DownloadableObject::new(
                    &art.url,
                    &lib_path.join(&art.path),
                    Some(HashType::Sha1(art.sha1.clone())),
                )
            }).collect();

        self.downloader = Some(T::new(files));

        Ok(())
    }

    #[allow(clippy::await_holding_lock)]
    async fn _fix_wrong_file(&mut self) -> Result<()> {
        while !self
            .bad_files
            .read()
            .map_err(|_| UraniumError::AsyncRuntimeError)?
            .is_empty()
        {
            let mut guard = self
                .bad_files
                .write()
                .map_err(|_| UraniumError::AsyncRuntimeError)?;
            warn!("{} wrong files, trying to fix them", guard.len());

            let objects: Vec<ObjectData> = guard.drain(..).collect();
            drop(guard);

            let _names: Vec<PathBuf> = objects
                .iter()
                .map(|obj| {
                    PathBuf::from(ASSETS_PATH)
                        .join(OBJECTS_PATH)
                        .join(&obj.hash[..2])
                        .join(&obj.hash)
                })
                .collect();

            let _urls: Vec<String> = objects
                .iter()
                .map(ObjectData::get_link)
                .collect();

            T::new(
                // TODO, FIXME
                vec![],
            )
            .complete()
            .await?;

            // God forgive me until I found a better way to do this.
            let _aux: Vec<&ObjectData> = objects.iter().collect();
        }

        Ok(())
    }

    /// This function will add a new minecraft profile to
    /// `launcher_profiles.json` file located in `minecraft_path` dir.
    ///
    /// If `icon` is not specified the default Grass icon will be set.
    ///
    /// # Errors
    /// If the `minecraft_path` doesn't exit or is not valid then
    /// `Err(UraniumError::FileNotFound)` will be returned.
    ///
    /// Also, if the profile file is not valid
    /// `Err(UraniumError::WrongFileFormat)` will be returned
    ///
    /// In case it is not possible to write into the file then
    /// `Err(UraniumError::WriteError)` will be returned
    pub fn add_instance<I: AsRef<Path>>(
        &self,
        minecraft_path: I,
        instance_name: &str,
        icon: Option<&str>,
    ) -> Result<()> {
        let profiles_path = minecraft_path
            .as_ref()
            .to_path_buf()
            .join(PROFILES_FILE);

        if !profiles_path.exists() {
            error!("{profiles_path:?} doesn't exist!");
            return Err(UraniumError::FileNotFound(
                profiles_path
                    .display()
                    .to_string(),
            ));
        }

        // let Ok(mut profiles): std::result::Result<ProfilesJson, _> =
        //     serde_json::from_reader(File::open(&profiles_path)?)
        // else {
        //     return Err(UraniumError::OtherWithReason("Cant deserialize
        // profile file".to_owned())); };

        let mut profiles: ProfilesJson = match serde_json::from_reader(File::open(&profiles_path)?)
        {
            Ok(v) => v,
            Err(e) => Err(UraniumError::OtherWithReason(e.to_string()))?,
        };

        let icon = icon.unwrap_or("Grass");

        let new_profile = Profile::new(
            icon,
            &self.minecraft_instance.id,
            instance_name,
            "custom",
            Some(&self.dot_minecraft_path),
        );

        profiles.insert(instance_name, new_profile);

        info!("Writing new profile");

        let Ok(content) = serde_json::to_string_pretty(&profiles) else {
            return Err(UraniumError::WrongFileFormat);
        };

        if let Err(err) = std::fs::write(profiles_path, content) {
            error!("Error writing the new profile");
            return Err(err.into());
        }

        info!("Profile added!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloaders::Downloader;
    use crate::error::Result;

    #[tokio::test(flavor = "multi_thread")]
    pub async fn download_minecraft() -> Result<()> {
        let mut downloader =
            MinecraftDownloader::<Downloader>::init("/home/sergio/.minecraft", "1.20.1").await?;

        let mut stdout = tokio::io::stdout();
        let r = loop {
            let state = if let Ok(x) = downloader.progress().await {
                x
            } else {
                break None;
            };

            if let MinecraftDownloadState::Completed = state {
                downloader.add_instance("/home/sergio/.minecraft", "Vanilla 1.20.1", None)?;
                break Some(());
            }
            stdout
                .write_all(format!("{:?}  [{:?}]\n", state, downloader.requests_left()).as_bytes())
                .await?;
            tokio::io::stdout()
                .flush()
                .await?;
        };
        let exits = Path::new("/home/sergio/.minecraft/versions/1.20.1/1.20.1.jar").exists();

        if r.is_some() {
            assert!(exits);
        }
        Ok(())
    }
}
