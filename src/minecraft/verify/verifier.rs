use std::path::{Path, PathBuf};

use log::{error, info, warn};
use mine_data_structs::minecraft::{
    AssetIndex, DownloadData, Library, ObjectData, Resources, Root,
};
use rayon::iter::{ParallelBridge, ParallelIterator};

use super::super::list_instances;
use crate::common::hash::rinth_hash;
use crate::error::{Result, UraniumError};
use crate::minecraft::verify::types::VersionCheckResult;

const ASSETS_PATH: &str = "assets/";
const OBJECTS_PATH: &str = "objects";

pub struct InstallationVerifier {
    minecraft_path: PathBuf,
    minecraft_instance: Root,
    resources: Resources,
}

impl InstallationVerifier {
    pub async fn new(minecraft_dir: &Path, version_id: &str) -> Result<Self> {
        let instances = list_instances().await?;
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
            error!("Wrong hash for {:?}, {}", client_path, client.sha1);
            Some(client)
        } else {
            None
        }
    }

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
        let data = fs::read_to_string(&index_path)
            .ok()?
            .replace(":", ": ")
            .replace(",", ", ");
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(data.as_bytes());
        let h = format!("{:x}", hasher.finalize());
        if index.sha1 != h {
            error!("Wrong hash for {:?}, {}-{}", index_path, index.sha1, h);
            return Some(index);
        }
        None
    }

    fn verify_libs(&self) -> Box<[&Library]> {
        let os_libs = self
            .minecraft_instance
            .libraries
            .iter()
            .filter(|l| l.applies());
        let raw_data = os_libs.filter_map(|lib| {
            lib.downloads
                .as_ref()
                .map(|d| (&d.artifact.path, &d.artifact.sha1, lib))
        });
        let fixed_data = raw_data.map(|(path, sha1, lib)| {
            (
                self.minecraft_path
                    .join("libraries")
                    .join(path),
                sha1,
                lib,
            )
        });
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
        bad_objects.into_boxed_slice()
    }
}

fn verify_file_hash(file_path: &Path, expected_hash: &str) -> Result<bool> {
    if !file_path.exists() {
        return Err(UraniumError::FileNotFound(
            file_path
                .to_string_lossy()
                .to_string(),
        ));
    }
    let actual_hash = rinth_hash(file_path)?;
    Ok(actual_hash.to_lowercase() == expected_hash.to_lowercase())
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, LazyLock, Mutex};

    use super::*;
    use crate::{
        common::constants::TEMP_DIR,
        engine::{Downloader, FileDownloader},
        minecraft::MinecraftDownloader,
    };

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
        let client_path = TEMP_DIR
            .join("versions")
            .join(VERSION)
            .join(format!("{VERSION}.jar"));
        if let Err(e) = std::fs::remove_file(&client_path) {
            panic!("Could not remove {client_path:?}: {e}");
        }
        let result = checker.verify();
        assert_eq!(result.total_problems(), 1);
    }
}
