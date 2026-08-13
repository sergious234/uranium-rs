use std::env::consts::OS;
use std::io::Write;
use std::{
    fs::File,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use log::{error, info};
use mine_data_structs::minecraft::{
    Artifact, Library, MinecraftVersions, ObjectData, Profile, ProfilesJson, Resources, Root,
};
use reqwest;
use tokio::io::AsyncWriteExt;

use super::RuntimeDownloader;
use super::gen_downloader::{DownloadState, DownloadableObject, FileDownloader, HashType};
use crate::{
    code_functions::N_THREADS,
    error::{Result, UraniumError},
    variables::constants::{EXECUTABLE_MODE, PROFILES_FILE},
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
#[derive(Debug, Clone)]
pub enum MinecraftDownloadState {
    GettingSources,
    DownloadingVersion,
    DownloadingAssets,
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
        MinecraftDownloader::with_downloader(destination_path, minecraft_version, T::new()).await
    }

    /// Makes a new `MinecraftDownloader` struct using an existing Downloader
    /// struct.
    ///
    /// - `destination_path`: Where minecraft will be downloaded. (THIS IS
    ///   USUALLY `.minecraft` DIRECTORY)
    /// - `minecraft_version`: Which versions is going to be downloaded.
    /// - `downloader`: Downloader that will be used
    ///
    /// This is usefull in case the **YOU** want to do something with the
    /// downloader before using it. Look at the example
    ///
    /// # Examples
    ///
    /// ```ignore
    /// async fn foo<T: FileDownloader + Send + Sync>() -> Result<()>{
    ///
    ///     let my_downloader = MyChanneledDownloader::new();
    ///
    ///     // MyChanneledDownloader has a mpsc channel inside which reports the progress of the downloader
    ///     let rx = my_downloader.get_channel();
    ///
    ///     // This will result in an error since "league of legends" is mental illness.
    ///     // (and also a game)
    ///     let downloader = MinecraftDownloader::with_downloader("my/mine/path", "league of legends", my_downloader).await?;
    ///
    ///     // Now I can send rx to another thread and recieve info from my custom downloader.
    ///
    ///     thread::spawn(move || {
    ///         while let Ok(r) = rx.recv() {
    ///             info!("{r}");
    ///         }
    ///     })
    ///
    ///     downloader.start().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_downloader<I: AsRef<Path>>(
        destination_path: I,
        minecraft_version: &str,
        downloader: T,
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
            downloader,
            requester,
        ))
    }

    fn new(
        destination_path: PathBuf,
        minecraft_instance: Root,
        downloader: T,
        requester: reqwest::Client,
    ) -> Self {
        MinecraftDownloader {
            requester,
            dot_minecraft_path: destination_path,
            minecraft_instance,
            download_state: MinecraftDownloadState::GettingSources,
            downloader,
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
                let resources = self.get_resources().await?;

                if let Err(err) = Self::create_indexes(
                    &self.dot_minecraft_path,
                    &self
                        .minecraft_instance
                        .get_index_name(),
                    &resources,
                )
                .await
                {
                    error!("Error creating index");
                    return Err(err);
                }

                if let Err(err) = Self::create_assets_folders(
                    &self.dot_minecraft_path,
                    resources.objects.values(),
                ) {
                    error!("Error creating assets folders");
                    return Err(err);
                }

                let objects = Self::get_assets_objects(&self.dot_minecraft_path, resources);
                self.downloader
                    .add_objects(objects);
                self.download_state = MinecraftDownloadState::DownloadingVersion;
            }

            MinecraftDownloadState::DownloadingVersion => {
                self.create_version_folder()
                    .await?;
                self.download_state = MinecraftDownloadState::DownloadingAssets;
            }

            MinecraftDownloadState::DownloadingAssets => {
                let download_state = self
                    .downloader
                    .progress()
                    .await;

                match download_state {
                    Ok(DownloadState::Completed) => {
                        let libs = Self::prepare_libraries(
                            &self
                                .minecraft_instance
                                .libraries,
                            &self.dot_minecraft_path,
                        )?;
                        self.downloader
                            .add_objects(libs);
                        self.download_state = MinecraftDownloadState::DownloadingLibraries;
                    }
                    Err(e) => {
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
                        error!("Error downloading libraries: {e}");
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
                .start()
                .await;

                if let Err(err) = runtime_res {
                    error!("Error downloading runtime: {}", err);
                    return Err(err);
                }
                self.download_state = MinecraftDownloadState::CheckingFiles;
            }

            MinecraftDownloadState::CheckingFiles => {
                // TODO: Check the files
                self.download_state = MinecraftDownloadState::Completed;
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

    /// Checks versions/version/version.jar file.
    async fn check_client(&mut self, instance_folder: &Path) -> Result<()> {
        let client_path = instance_folder.join(
            self.minecraft_instance
                .id
                .clone()
                + ".jar",
        );
        if !client_path.exists() {
            self.download_client(client_path)
                .await?;
        }
        Ok(())
    }

    async fn download_client(&mut self, client_path: PathBuf) -> Result<()> {
        info!("Downloading client!");
        const CLIENT: &str = "client";
        let (url, hash) = self
            .minecraft_instance
            .downloads
            .get(CLIENT)
            .map(|i| (&i.url, i.sha1.to_string()))
            .ok_or(UraniumError::OtherWithReason(
                "Client .jar not found in the minecraft instance".to_owned(),
            ))?;
        let obj = DownloadableObject::new(url, &client_path, Some(HashType::Sha1(hash)));
        self.downloader
            .add_object(obj);
        self.downloader
            .start()
            .await?;
        Ok(std::fs::set_permissions(
            &client_path,
            std::fs::Permissions::from_mode(EXECUTABLE_MODE),
        )?)
    }

    fn check_instance(&self, instance_folder: &Path) -> Result<()> {
        let instance_path = instance_folder.join(
            self.minecraft_instance
                .id
                .clone()
                + ".json",
        );
        if !instance_path.exists() {
            info!("Writing client json!");
            let mut instance_file = File::create(instance_path)?;
            let content = serde_json::to_string(&self.minecraft_instance)
                .map_err(|e| UraniumError::OtherWithReason(e.to_string()))?;
            instance_file.write_all(content.as_bytes())?;
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
        self.downloader
            .requests_left()
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
            .requests_left() as f64;
        (n / N_THREADS() as f64).ceil() as usize
    }

    async fn get_resources(&self) -> Result<Resources> {
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
        Ok(resources)
    }

    fn get_assets_objects(
        minecraft_path: &Path,
        resources: Resources,
    ) -> impl Iterator<Item = DownloadableObject> {
        let base = PathBuf::from(ASSETS_PATH).join(OBJECTS_PATH);

        resources
            .objects
            .into_values()
            .map(move |obj| {
                let url = obj.get_link();
                let path = base
                    .join(&obj.hash[..2])
                    .join(&obj.hash);
                DownloadableObject::new(
                    &url,
                    &minecraft_path.join(path),
                    Some(HashType::Sha1(obj.hash.to_owned())),
                )
            })
    }

    /// Makes the minecraft index.json file
    async fn create_indexes(
        minecraft_path: &Path,
        index_name: &str,
        resources: &Resources,
    ) -> Result<()> {
        let indexes_path = minecraft_path
            .join(ASSETS_PATH)
            .join("indexes")
            .join(index_name);

        let mut indexes = tokio::fs::File::create(indexes_path).await?;

        indexes
            .write_all(
                serde_json::to_string(resources)
                    .map_err(|err| UraniumError::OtherWithReason(err.to_string()))?
                    .as_bytes(),
            )
            .await?;

        Ok(())
    }

    fn create_assets_folders<'a>(
        minecraft_path: &Path,
        objects: impl Iterator<Item = &'a ObjectData>,
    ) -> Result<()> {
        std::fs::create_dir_all(minecraft_path.join("assets/indexes")).map_err(|err| {
            error!("Cant create assets/indexes: [{err}]");
            UraniumError::CantCreateDir("assets/indexes")
        })?;

        std::fs::create_dir_all(minecraft_path.join("assets/objects")).map_err(|err| {
            error!("Cant create assets/objects: [{err}]");
            UraniumError::CantCreateDir("assets/objects")
        })?;

        let obj_path = minecraft_path
            .join(ASSETS_PATH)
            .join(OBJECTS_PATH);
        for obj in objects {
            if let Some(parent) = obj.get_path().parent() {
                std::fs::create_dir_all(obj_path.join(parent))?;
            }
        }
        Ok(())
    }

    /// This function processes the minecraft instance libraries and creates a
    /// vector of `DownloadableObject` instances containing the URLs, paths,
    /// and SHA1 hashes needed for downloading the required libraries.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `impl Iterator<Item = DownloadableObject>` with
    /// all the library files that need to be downloaded, or an error if the
    /// operation fails.
    fn prepare_libraries(
        libs: &[Library],
        minecraft_path: &Path,
    ) -> Result<impl Iterator<Item = DownloadableObject>> {
        let lib_path = minecraft_path.join("libraries");

        fn extract_native(lib: &Library) -> Option<&Artifact> {
            if let Some(downloads) = &lib.downloads
                && let Some(classifiers) = &downloads.classifiers
            {
                return match OS {
                    "linux"
                        if classifiers
                            .natives_linux
                            .is_some() =>
                    {
                        classifiers
                            .natives_linux
                            .as_ref()
                    }
                    "windows"
                        if classifiers
                            .natives_windows
                            .is_some() =>
                    {
                        classifiers
                            .natives_windows
                            .as_ref()
                    }
                    _ => None,
                };
            }
            None
        }

        let value = lib_path.clone();
        let natives = libs
            .iter()
            .flat_map(|l| extract_native(l))
            .map(move |l| {
                DownloadableObject::new(
                    &l.url,
                    &value.join(&l.path),
                    Some(HashType::Sha1(l.sha1.clone())),
                )
            });

        Ok(libs
            .iter()
            .filter(|l| l.applies() && l.downloads.is_some())
            .flat_map(|l| l.downloads.as_ref())
            .map(|d| &d.artifact)
            .map(move |a| {
                DownloadableObject::new(
                    &a.url,
                    &lib_path.join(&a.path),
                    Some(HashType::Sha1(a.sha1.clone())),
                )
            })
            .chain(natives))
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

pub fn get_index_path(installation_path: &Path, index_name: &Path) -> PathBuf {
    installation_path
        .join(ASSETS_PATH)
        .join("indexes")
        .join(index_name)
}

pub fn get_lib_path(installation_path: &Path, lib_path: &Path) -> PathBuf {
    installation_path
        .join("libraries")
        .join(lib_path)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mine_data_structs::minecraft::{
        Arguments, Artifact, AssetIndex, Classifiers, JavaVersion, Library, LibraryDownloads, Os,
        OsName, Root, Rule,
    };

    use super::*;

    /// A mock `FileDownloader` for testing `MinecraftDownloader`'s state
    /// machine.
    ///
    /// Simulates downloading by returning `Downloading` once per batch of
    /// objects, then `Completed`. Objects added via
    /// `add_object`/`add_objects` are tracked and reported through
    /// `requests_left`.
    #[allow(dead_code)]
    struct MockDownloader {
        total: usize,
        remaining: usize,
        completed: bool,
        objects: Vec<DownloadableObject>,
    }

    impl FileDownloader for MockDownloader {
        fn new() -> Self {
            MockDownloader {
                total: 0,
                remaining: 0,
                completed: true,
                objects: vec![],
            }
        }

        async fn progress(&mut self) -> Result<DownloadState> {
            if self.completed {
                return Ok(DownloadState::Completed);
            }
            if self.remaining > 0 {
                self.remaining = 0;
                self.completed = true;
                Ok(DownloadState::Downloading)
            } else {
                Ok(DownloadState::Completed)
            }
        }

        fn requests_left(&self) -> usize {
            self.remaining
        }

        fn len(&self) -> usize {
            self.total
        }

        fn add_object(&mut self, obj: DownloadableObject) {
            self.objects.push(obj);
            self.total += 1;
            self.remaining += 1;
            self.completed = false;
        }

        fn is_empty(&self) -> bool {
            self.total == 0
        }
    }

    #[test]
    fn get_index_path_appends_assets_indexes() {
        let result = get_index_path(&PathBuf::from("/root"), &PathBuf::from("19"));
        assert_eq!(result, PathBuf::from("/root/assets/indexes/19"));
    }

    #[test]
    fn get_lib_path_appends_libraries() {
        let result = get_lib_path(&PathBuf::from("/root"), &PathBuf::from("a/b/c.jar"));
        assert_eq!(result, PathBuf::from("/root/libraries/a/b/c.jar"));
    }

    #[test]
    fn get_index_path_trailing_slash() {
        let result = get_index_path(&PathBuf::from("/root/"), &PathBuf::from("1.21.json"));
        assert_eq!(result, PathBuf::from("/root/assets/indexes/1.21.json"));
    }

    #[test]
    fn get_lib_path_with_nested_path() {
        let result = get_lib_path(
            &PathBuf::from("/minecraft"),
            &PathBuf::from("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"),
        );
        assert_eq!(
            result,
            PathBuf::from("/minecraft/libraries/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar")
        );
    }

    #[test]
    fn prepare_libraries_filters_by_os_rules() {
        let lib_always = Library {
            name: "always:lib:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("always/lib/1.0/lib.jar"),
                    sha1: "aaa111".into(),
                    size: 100,
                    url: "https://example.com/lib.jar".into(),
                },
                classifiers: None,
            }),
            rules: None,
        };
        let lib_disallowed = Library {
            name: "never:lib:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("never/lib/1.0/lib.jar"),
                    sha1: "bbb222".into(),
                    size: 100,
                    url: "https://example.com/lib.jar".into(),
                },
                classifiers: None,
            }),
            rules: Some(Box::new([Rule {
                action: "disallow".into(),
                os: Some(Os {
                    name: Some(OsName::Linux),
                    arch: None,
                }),
            }])),
        };

        let instance = Root {
            arguments: Arguments {
                game: Box::new([]),
                jvm: Box::new([]),
            },
            asset_index: AssetIndex {
                id: "test".into(),
                sha1: "000".into(),
                size: 0,
                total_size: 0,
                url: "https://example.com/index.json".into(),
            },
            assets: "test".into(),
            downloads: HashMap::new(),
            id: "test".into(),
            java_version: JavaVersion {
                component: "java-runtime-test".into(),
                major_version: 21,
            },
            libraries: Box::new([lib_always, lib_disallowed]),
            inherits_from: None,
            main_class: "net.minecraft.client.main.Main".into(),
            version_type: "release".into(),
        };

        // let downloader = make_minecraft_downloader(instance);
        let objects: Vec<DownloadableObject> =
            MinecraftDownloader::<MockDownloader>::prepare_libraries(
                &instance.libraries,
                &PathBuf::from("/test"),
            )
            .unwrap()
            .collect();

        let names: Vec<_> = objects
            .iter()
            .map(|o| o.path.to_string_lossy())
            .collect();
        assert!(
            names
                .iter()
                .any(|p| p.contains("always")),
            "expected always-applicable library to be included, got: {names:?}"
        );
        assert!(
            !names
                .iter()
                .any(|p| p.contains("never")),
            "expected disallowed library to be excluded, got: {names:?}"
        );
    }

    #[test]
    fn prepare_libraries_includes_natives() {
        let lib_with_natives = Library {
            name: "natives:test:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("natives/main.jar"),
                    sha1: "ccc333".into(),
                    size: 100,
                    url: "https://example.com/main.jar".into(),
                },
                classifiers: Some(Classifiers {
                    natives_linux: Some(Artifact {
                        path: PathBuf::from("natives/linux/native.so"),
                        sha1: "ddd444".into(),
                        size: 50,
                        url: "https://example.com/native.so".into(),
                    }),
                    natives_windows: None,
                    natives_macos: None,
                }),
            }),
            rules: None,
        };

        let instance = Root {
            arguments: Arguments {
                game: Box::new([]),
                jvm: Box::new([]),
            },
            asset_index: AssetIndex {
                id: "test".into(),
                sha1: "000".into(),
                size: 0,
                total_size: 0,
                url: "https://example.com/index.json".into(),
            },
            assets: "test".into(),
            downloads: HashMap::new(),
            id: "test".into(),
            java_version: JavaVersion {
                component: "java-runtime-test".into(),
                major_version: 21,
            },
            libraries: Box::new([lib_with_natives]),
            inherits_from: None,
            main_class: "net.minecraft.client.main.Main".into(),
            version_type: "release".into(),
        };

        // let downloader = make_minecraft_downloader(instance);
        let objects: Vec<DownloadableObject> =
            MinecraftDownloader::<MockDownloader>::prepare_libraries(
                &instance.libraries,
                &PathBuf::from("/test"),
            )
            .unwrap()
            .collect();

        let paths: Vec<_> = objects
            .iter()
            .map(|o| {
                o.path
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(
            paths
                .iter()
                .any(|p| p.contains("main.jar")),
            "expected main artifact, got: {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| p.contains("native.so")),
            "expected native classifier, got: {paths:?}"
        );
    }

    #[test]
    fn prepare_libraries_skips_library_without_downloads() {
        let lib_no_downloads = Library {
            name: "empty:lib:1.0".into(),
            downloads: None,
            rules: None,
        };
        let lib_with = Library {
            name: "has:downloads:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("has/downloads.jar"),
                    sha1: "eee555".into(),
                    size: 100,
                    url: "https://example.com/downloads.jar".into(),
                },
                classifiers: None,
            }),
            rules: None,
        };

        let instance = Root {
            arguments: Arguments {
                game: Box::new([]),
                jvm: Box::new([]),
            },
            asset_index: AssetIndex {
                id: "test".into(),
                sha1: "000".into(),
                size: 0,
                total_size: 0,
                url: "https://example.com/index.json".into(),
            },
            assets: "test".into(),
            downloads: HashMap::new(),
            id: "test".into(),
            java_version: JavaVersion {
                component: "java-runtime-test".into(),
                major_version: 21,
            },
            libraries: Box::new([lib_no_downloads, lib_with]),
            inherits_from: None,
            main_class: "net.minecraft.client.main.Main".into(),
            version_type: "release".into(),
        };

        // let downloader = make_minecraft_downloader(instance);
        let objects: Vec<DownloadableObject> =
            MinecraftDownloader::<MockDownloader>::prepare_libraries(
                &instance.libraries,
                &PathBuf::from("/test"),
            )
            .unwrap()
            .collect();

        let paths: Vec<_> = objects
            .iter()
            .map(|o| {
                o.path
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(
            !paths
                .iter()
                .any(|p| p.contains("empty")),
            "expected library without downloads to be skipped, got: {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| p.contains("has")),
            "expected library with downloads to be included, got: {paths:?}"
        );
    }

    #[cfg(feature = "integration-tests")]
    #[tokio::test(flavor = "multi_thread")]
    pub async fn download_minecraft() -> Result<()> {
        use super::super::gen_downloader::Downloader;
        let mut downloader =
            MinecraftDownloader::<Downloader>::init("/home/sergio/.minecraft", "1.20.1").await?;

        let _ = crate::init_logger();
        loop {
            match downloader.progress().await {
                Ok(MinecraftDownloadState::Completed) => break,
                Err(e) => return Err(e),
                _ => {}
            }
        }

        let client_path = PathBuf::from("/home/sergio/.minecraft/versions/1.20.1/1.20.1.jar");
        assert!(client_path.exists(), "Client jar was not downloaded");
        Ok(())
    }
}
