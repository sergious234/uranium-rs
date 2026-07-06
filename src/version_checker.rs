use std::path::{Path, PathBuf};

use log::{error, info, warn};
use mine_data_structs::minecraft::{
    AssetIndex, DownloadData, Library, ObjectData, Os, Resources, Root,
};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::downloaders::list_instances;
use crate::error::{Result, UraniumError};

// I know this is duplicated, idc.
const ASSETS_PATH: &str = "assets/";
const OBJECTS_PATH: &str = "objects";

/// Manages Minecraft installation verification and integrity checks.
///
/// This struct owns the primary data structures needed for verifying
/// a Minecraft installation, including the installation path, instance
/// configuration, and game resources. It serves as the central component
/// for performing comprehensive verification operations on Minecraft
/// installations.
///
/// # Example Usage
///
/// Basic verification workflow:
/// ```ignore
///     let verifier = InstallationVerifier::new(minecraft_path);
///     let result = verifier.verify_version();
///
///     if result.is_valid() {
///         println!("Installation is valid!");
///     } else {
///         println!("Found problems: {}", result.total_problems());
///     }
/// ```
pub struct InstallationVerifier {
    minecraft_path: PathBuf,
    minecraft_instance: Root,
    resources: Resources,
}

impl InstallationVerifier {
    pub async fn new(minecraft_dir: &Path, version_id: &str) -> Result<Self> {
        let instances = list_instances()
            .await
            .unwrap();

        let instance_url = instances
            .get_instance_url(version_id)
            .ok_or(UraniumError::OtherWithReason(format!(
                "Version {version_id} doesn't exist"
            )))?;

        let requester = reqwest::Client::new();

        let minecraft_instance: Root = requester
            .get(instance_url)
            .send()
            .await?
            .json()
            .await?;

        let resources: Resources = requester
            .get(
                &minecraft_instance
                    .asset_index
                    .url,
            )
            .send()
            .await?
            .json::<Resources>()
            .await?;

        Ok(Self {
            minecraft_path: minecraft_dir.to_path_buf(),
            minecraft_instance,
            resources,
        })
    }

    /// Performs a comprehensive verification of the Minecraft installation.
    ///
    /// Verifies both libraries and objects in the installation and returns
    /// references to any problematic files found.
    ///
    /// # Returns
    ///
    /// A `VersionCheckResult` containing references to any problematic
    /// libraries and objects found during verification. If the installation
    /// is completely valid, both arrays in the result will be empty.
    ///
    /// # Example
    /// ```ignore
    /// let mut verifier = InstallationVerifier::new(minecraft_path);
    /// let result = verifier.verify();
    ///
    /// if result.is_valid() {
    ///     println!("Installation verified successfully!");
    /// } else {
    ///     println!("Verification failed: {}", result.summary());
    /// }
    /// ```
    pub fn verify(&self) -> VersionCheckResult<'_> {
        let libs = self.verify_libs();
        let objects = self.verify_objects();
        let index = self.very_index();
        let client = self.verify_client();
        info!("Wrong files: {}", libs.len() + objects.len());
        if let Some(index) = index {
            info!("Wrong index: {}", index.id);
        }
        if let Some(client) = client {
            info!("Wrong client: {}", client.sha1);
        }

        VersionCheckResult {
            objects,
            libs,
            index,
            client,
        }
    }

    /// Verifies the integrity of the Minecraft client JAR file and returns
    /// download data if verification fails.
    ///
    /// This function checks whether the client JAR file exists and verifies its
    /// SHA1 hash against the expected hash from the download data.
    ///
    /// # Returns
    ///
    /// * `Some(&DownloadData)` - When the client JAR file is missing or has an
    ///   incorrect hash, indicating that the client needs to be downloaded or
    ///   re-downloaded
    /// * `None` - When the client JAR file exists and passes hash verification,
    ///   meaning the local copy is valid and up-to-date, or if client download
    ///   data is not available
    fn verify_client(&self) -> Option<&DownloadData> {
        let client_path = self
            .minecraft_path
            .join("versions")
            .join(&self.minecraft_instance.id)
            .join(
                self.minecraft_instance
                    .id
                    .clone()
                    + ".jar",
            );

        let client = self
            .minecraft_instance
            .downloads
            .get("client")?;

        if !client_path.exists() {
            error!("Client doesn't exist: {client_path:?}");
            Some(client)
        } else if let Ok(false) = verify_file_hash(&client_path, &client.sha1) {
            error!("Wrong hash for {:?}, {}", &client_path, &client.sha1);
            Some(client)
        } else {
            None
        }
    }

    /// Verifies the integrity of the asset index file and returns it if
    /// verification fails.
    ///
    /// This function checks whether the asset index file exists and verifies
    /// its SHA1 hash against the expected hash from the Minecraft instance
    /// configuration.
    ///
    /// # Returns
    ///
    /// * `Some(&AssetIndex)` - When the asset index file is missing or has an
    ///   incorrect hash, indicating that the index needs to be downloaded or
    ///   re-downloaded
    /// * `None` - When the asset index file exists and passes hash
    ///   verification, meaning the local copy is valid and up-to-date
    fn very_index(&self) -> Option<&AssetIndex> {
        let index = &self
            .minecraft_instance
            .asset_index;

        let index_path = self
            .minecraft_path
            .join(ASSETS_PATH)
            .join("indexes")
            .join(&index.id)
            .with_extension("json");

        if !index_path.exists() {
            return Some(index);
        }

        use std::fs;

        // Mojang json comes with spaces after ',' and ':', so we need to
        // replace them with the trimmed version.
        let data = fs::read_to_string(&index_path)
            .ok()?
            .replace(":", ": ")
            .replace(",", ", ");

        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(data.as_bytes());

        let h = format!("{:x}", hasher.finalize());
        if index.sha1 != h {
            error!("Wrong hash for {:?}, {}-{}", &index_path, &index.sha1, h);
            return Some(index);
        }

        //if let Ok(false) = verify_file_hash(&index_path, &index.sha1) {
        //    error!("Wrong hash for {:?}, {}", &index_path, &index.sha1);
        //    return Some(index);
        //}
        None
    }

    fn verify_libs(&self) -> Box<[&Library]> {
        let current_os = match std::env::consts::OS {
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            _ => Os::Other,
        };

        // Extract libs for the current os
        let os_libs = self
            .minecraft_instance
            .libraries
            .iter()
            .filter(|l| {
                l.get_os()
                    .is_none_or(|os| os == current_os)
            });

        // Set up an iterator with all the data needed
        let raw_data = os_libs.filter_map(|lib| {
            lib.downloads
                .as_ref()
                .map(|d| (&d.artifact.path, &d.artifact.sha1, lib))
        });

        // Fix the lib's paths
        let fixed_data = raw_data.map(|(path, sha1, lib)| {
            (
                self.minecraft_path
                    .join("libraries")
                    .join(path),
                sha1,
                lib,
            )
        });

        // Verify libraries in parallel with rayon
        let bad_objects: Vec<_> = fixed_data
            .par_bridge()
            .filter_map(|(lib_path, hash, lib)| {
                if let Ok(false) = verify_file_hash(&lib_path, hash) {
                    error!("Wrong hash for {lib_path:?}, {hash}");
                    Some(lib)
                } else {
                    None
                }
            })
            .collect();

        Box::from(bad_objects)
    }

    /// This method verify the objects under `assets/objects`.
    ///
    /// Returns:
    /// Err(UraniumError) If something went wrong
    /// Ok(Box<[&str]>) A boxed array of the names of the wrong files, if the
    /// box is empty then all objects are ok.
    fn verify_objects(&self) -> Box<[&ObjectData]> {
        use rayon::prelude::*;
        let base = self
            .minecraft_path
            .join(ASSETS_PATH)
            .join(OBJECTS_PATH);

        let bad_objects = self
            .resources
            .objects
            .par_iter()
            .flat_map(|(_, data)| {
                let object_path = base.join(data.get_path());
                match verify_file_hash(&object_path, &data.hash) {
                    Ok(false) => {
                        warn!("Wrong hash for {object_path:?}, {}", data.hash);
                        Some(data)
                    }
                    Err(e) => {
                        error!("Error verifying: {}", e);
                        None
                    }
                    _ => None,
                }
            })
            .collect::<Vec<&ObjectData>>();
        Box::from(bad_objects)
    }
}

/// Result of a version check operation containing references to problematic
/// files.
///
/// This structure holds references to objects and libraries that were
/// identified as having errors or inconsistencies during the verification
/// process. The lifetime parameter 'a ensures that the references remain valid
/// as long as the original data in the InstallationVerifier exists.
///
/// # Fields
///
/// * objects - References to problematic object data files
/// * libs - References to problematic library files
///
/// # Example Usage
///
/// ```ignore
/// let verifier = InstallationVerifier::new(path);
/// let result = verifier.verify_version();
/// // Process problematic objects
/// for object in result.objects.iter() {
///      println!("Problematic object: {:?}", object);  
/// }
/// // Check problematic libs...
/// ```
pub struct VersionCheckResult<'a> {
    pub objects: Box<[&'a ObjectData]>,
    pub libs: Box<[&'a Library]>,
    pub index: Option<&'a AssetIndex>,
    pub client: Option<&'a DownloadData>,
}

impl VersionCheckResult<'_> {
    /// Returns true if the verification found no problems.
    ///
    /// This is a convenience method that checks if both the objects
    /// and libraries arrays are empty, indicating a successful verification.
    ///
    /// # Returns
    ///
    /// `true` if no problematic objects or libraries were found, `false`
    /// otherwise.
    ///
    /// # Example
    ///```ignore
    /// let result = verifier.verify_version();
    /// if result.is_valid() {
    ///     println!("Installation is clean!");
    /// }
    /// ```
    pub fn is_valid(&self) -> bool {
        self.objects.is_empty()
            && self.libs.is_empty()
            && self.index.is_none()
            && self.client.is_none()
    }

    /// Returns the total number of problematic items found.
    ///
    /// This combines the count of problematic objects and libraries
    /// into a single number for quick assessment of verification results.
    ///
    /// # Returns
    ///
    /// The total count of problematic files.
    pub fn total_problems(&self) -> usize {
        self.objects.len()
            + self.libs.len()
            + self
                .index
                .map(|_| 1)
                .unwrap_or_default()
            + self
                .client
                .map(|_| 1)
                .unwrap_or_default()
    }

    /// Returns the number of problematic objects found.
    ///
    /// # Returns
    ///
    /// The count of problematic objects.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Returns the number of problematic libraries found.
    ///
    /// # Returns
    ///
    /// The count of problematic libraries.
    pub fn lib_count(&self) -> usize {
        self.libs.len()
    }
}

// What do you think this function does eh ?
// Duh... of course it hashes the file verifier...
fn verify_file_hash(file_path: &Path, expected_hash: &str) -> Result<bool> {
    // Rinth hash is sha1
    use crate::hashes::rinth_hash;

    if !file_path.exists() {
        return Err(UraniumError::FileNotFound(
            file_path
                .to_string_lossy()
                .to_string(),
        ));
    }
    let actual_hash = rinth_hash(file_path);
    Ok(actual_hash.to_lowercase() == expected_hash.to_lowercase())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex, LazyLock};
    use tokio::sync::Notify;

    use crate::{
        downloaders::{Downloader, MinecraftDownloader},
        variables::constants::TEMP_DIR,
    };

    use super::*;

    static PAIR: LazyLock<Arc<(Mutex<bool>, Condvar)>> =
        LazyLock::new(|| Arc::new((Mutex::new(false), Condvar::new())));

    const VERSION: &str = "1.21.1";

    #[tokio::test]
    async fn a_download_minecraft() -> Result<()> {
        let (lock, cvar) = &*(PAIR.clone());
        let mut downloader = MinecraftDownloader::<Downloader>::init(TEMP_DIR.as_path(), VERSION)
            .await
            .unwrap();
        let _ = downloader.start().await;
        *lock.lock().unwrap() = true;
        cvar.notify_all();

        Ok(())
    }

    #[tokio::test]
    async fn check_file() {
        let (lock, cvar) = &*(PAIR.clone());
        let mut started = lock.lock().unwrap();
        while !*started {
            started = cvar.wait(started).unwrap();
        }

        let checker = InstallationVerifier::new(&TEMP_DIR, VERSION)
            .await
            .unwrap();
        let result = checker.verify();
        assert_eq!(result.total_problems(), 0);
    }

    #[tokio::test]
    async fn check_missing_client() {
        let (lock, cvar) = &*(PAIR.clone());
        let mut started = lock.lock().unwrap();
        while !*started {
            started = cvar.wait(started).unwrap();
        }

        let checker = InstallationVerifier::new(&TEMP_DIR, VERSION)
            .await
            .unwrap();
        if let Err(e) =
            std::fs::remove_file("/home/sergio/.local/state/uranium/versions/1.21.1/1.21.1.jar")
        {
            panic!("Could not remove {e}");
        }
        let result = checker.verify();
        assert_eq!(result.total_problems(), 1);
    }
}
