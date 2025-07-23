use std::io::Write;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

use log::{error, info};
use mine_data_structs::minecraft::{
    Library, MinecraftVersions, Profile, ProfilesJson, Resources, Root,
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
#[derive(Debug)]
pub enum InnerMinecraftDownloadState {
    GettingSources,
    DownloadingIndexes(Vec<DownloadableObject>),
    DownloadingAssests,
    DownloadingLibraries,
    CheckingFiles,
    Completed,
}

/// Indicates the download state of a Minecraft instance.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MinecraftDownloadState {
    GettingSources,
    DownloadingVersion,
    DownloadingAssests,
    DownloadingLibraries,
    DownloadingRuntime,
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
    minecraft_instance: Root,
    download_state: MinecraftDownloadState,
    downloader: T,
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
            minecraft_instance,
            download_state: MinecraftDownloadState::GettingSources,
            downloader: T::new(vec![]),
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
    pub async fn progress(&mut self) -> Result<MinecraftDownloadState> {
        match self.download_state {
            MinecraftDownloadState::GettingSources => {
                let files = self.get_sources().await?;

                if self
                    .create_assests_folders(&files)
                    .is_err()
                {
                    error!("Error creating assets folders");
                    return Err(UraniumError::CantCreateDir("assets"));
                };

                self.downloader
                    .add_objects(files);
                self.download_state = MinecraftDownloadState::DownloadingVersion;
            }

            MinecraftDownloadState::DownloadingVersion => {
                self.create_version_folder()
                    .await?;
                self.download_state = MinecraftDownloadState::DownloadingAssests;
            }

            MinecraftDownloadState::DownloadingAssests => {
                let download_state = self
                    .downloader
                    .progress()
                    .await;

                match download_state {
                    Ok(DownloadState::Completed) => {
                        let files = self.prepare_libraries()?;
                        self.downloader
                            .add_objects(files);
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
                let download_state = self
                    .downloader
                    .progress()
                    .await;

                match download_state {
                    Ok(DownloadState::Completed) => {
                        self.download_state = MinecraftDownloadState::DownloadingRuntime;
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

            MinecraftDownloadState::DownloadingRuntime => {
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

    /// Creates the version folder structure for a Minecraft instance and
    /// ensures required files are present.
    ///
    /// This method creates the necessary directory structure under
    /// `.minecraft/versions/` for the current Minecraft instance. It
    /// creates a folder named after the instance ID and ensures that both
    /// the client JAR file and instance JSON file are properly downloaded
    /// and validated.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful completion of all operations.
    async fn create_version_folder(&mut self) -> Result<()> {
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
        let instance_folder = self
            .dot_minecraft_path
            .join("versions")
            .join(&self.minecraft_instance.id);

        info!("Instance folder: {instance_folder:?}");

        if !instance_folder.exists() {
            std::fs::create_dir_all(&instance_folder)?;
        }

        // .minectaft/versions/version/version.jar
        self.check_client(&instance_folder)
            .await?;

        // .minectaft/versions/version/version.json
        self.check_instance(&instance_folder)?;
        Ok(())
    }

    async fn check_client(&mut self, instance_folder: &Path) -> Result<()> {
        let client_path = instance_folder
            .join(self.minecraft_instance.id.clone() + ".jar");
        if !client_path.exists() {
            info!("Downloading client!");
            let (url, hash) = self
                .minecraft_instance
                .downloads
                .get("client")
                .map(|i| (&i.url, i.sha1.to_string()))
                .ok_or(UraniumError::OtherWithReason(
                    "Client .jar not found in the minecraft instance".to_owned(),
                ))?;
            let obj = DownloadableObject::new(url, &client_path, Some(HashType::Sha1(hash)));
            self.downloader
                .add_object(obj);
            self.downloader
                .complete()
                .await?;
        }
        Ok(())
    }

    fn check_instance(&self, instance_folder: &Path) -> Result<()> {
        let instance_path = instance_folder
            .join(self.minecraft_instance.id.clone() + ".json");
        if !instance_path.exists() {
            info!("Writing client json!");
            let mut instance_file = File::create(instance_path)?;
            instance_file.write_all(
                serde_json::to_string(&self.minecraft_instance)
                    .unwrap()
                    .as_bytes(),
            )?;
        }
        Ok(())
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
        (self
            .downloader
            .requests_left() as f64
            / N_THREADS() as f64)
            .ceil() as usize
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
        let n = self.downloader.len() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    async fn get_sources(&mut self) -> Result<Box<[DownloadableObject]>> {
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
            UraniumError::OtherWithReason(format!("assets/indexes: [{err}]"))
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

        let mut files = vec![];

        for obj in resources.objects.values() {
            let url = obj.get_link();
            let path = base
                .join(&obj.hash[..2])
                .join(&obj.hash);
            files.push(DownloadableObject::new(
                &url,
                &self
                    .dot_minecraft_path
                    .join(path),
                Some(HashType::Sha1(obj.hash.to_owned())),
            ));
        }

        Ok(Box::from(files))
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
    fn create_assests_folders(&self, names: &[DownloadableObject]) -> Result<()> {
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

    // WIP
    #[allow(dead_code)]
    /// Return a `Vec<String>` with the urls of the libraries for the current.
    /// If the lib has no specified Os then it will be inside the vector too.
    fn get_os_libraries(&self, libraries: &[Library]) -> Vec<DownloadableObject> {
        let lib_path = self
            .dot_minecraft_path
            .join("libraries");

        let current_os = match std::env::consts::OS {
            "linux" => mine_data_structs::minecraft::Os::Linux,
            "macos" => mine_data_structs::minecraft::Os::Other,
            // "windows" => mine_data_structs::minecraft::Os::Windows,
            _ => mine_data_structs::minecraft::Os::Windows,
        };

        libraries
            .iter()
            .filter(|lib| {
                lib.get_os()
                    .is_none_or(|os| os == current_os)
            })
            .map(|lib| {
                DownloadableObject::new(
                    lib.get_url(),
                    &lib_path.join(lib.get_rel_path().unwrap()),
                    None,
                )
            })
            .collect()
    }

    /// This function processes the minecraft instance libraries and creates a
    /// vector of `DownloadableObject` instances containing the URLs, paths,
    /// and SHA1 hashes needed for downloading the required libraries.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<DownloadableObject>` with all the library
    /// files that need to be downloaded, or an error if the operation
    /// fails.
    fn prepare_libraries(&self) -> Result<Box<[DownloadableObject]>> {
        let lib_path = self
            .dot_minecraft_path
            .join("libraries");

        Ok(self
            .minecraft_instance
            .libraries
            .as_ref()
            .iter()
            .map(|l| {
                DownloadableObject::new(
                    l.get_url(),
                    &lib_path.join(
                        l.get_rel_path()
                            .unwrap_or_else(|| panic!("Missing download field for library {l:?}")),
                    ),
                    l.get_hash()
                        .map(|h| HashType::Sha1(h.to_string())),
                )
            })
            .collect::<Box<[DownloadableObject]>>())
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
    use crate::init_logger;

    #[tokio::test(flavor = "multi_thread")]
    pub async fn download_minecraft() -> Result<()> {
        let mut downloader =
            MinecraftDownloader::<Downloader>::init("/home/sergio/.minecraft", "1.20.1").await?;

        let mut stdout = tokio::io::stdout();
        let _ = init_logger();
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

        let exits = std::env::home_dir()
            .unwrap()
            .join(".minecraft/versions/1.20.1/1.20.1.jar")
            .exists();

        if r.is_some() {
            assert!(exits);
        }
        Ok(())
    }
}
