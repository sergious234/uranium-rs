//! # Runtime Downloader
//!
//! This module is responsible for downloading the required Java runtimes
//! for Minecraft instances. It interacts with Mojang's launcher metadata to
//! find the correct runtime version based on the operating system and then
//! downloads all associated files.
//!
//! This module ensures that the downloaded runtime files are correctly placed
//! in the Minecraft root directory and that the executable files have the
//! necessary permissions.
//!
//! ## Example
//!
//! Here's a basic example of how to use the `RuntimeDownloader` to download a
//! runtime.
//!
//! ```no_run
//! # use uranium_rs::downloaders::RuntimeDownloader;
//! # use uranium_rs::error::Result;
//! #
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut downloader = RuntimeDownloader::new("java-runtime-beta".to_string());
//!     downloader.start().await?;
//!     println!("Runtime downloaded and installed successfully!");
//!     Ok(())
//! }
//! ```

use mine_data_structs::minecraft::RUNTIMES_URL;
use mine_data_structs::minecraft::{RuntimeFiles, Runtimes, get_minecraft_path};
use reqwest::Client;

use super::DownloadableObject;
use crate::downloaders::{Downloader, FileDownloader, HashType};
use crate::error::{Result, UraniumError};
use crate::variables::constants::EXECUTABLE_MODE;

/// A downloader specifically for Java runtimes.
pub struct RuntimeDownloader {
    runtime: String,
}

impl RuntimeDownloader {
    pub fn new(runtime: String) -> Self {
        Self { runtime }
    }

    /// Fetches the runtime manifest, downloads all required files, and sets
    /// permissions for executables.
    ///
    /// # Errors
    ///
    /// This function can return a `UraniumError` if:
    /// * There are issues with the network requests to Mojang's servers.
    /// * The requested runtime is not found in the manifest.
    /// * There are issues with creating directories or writing files to disk.
    pub async fn start(&mut self) -> Result<()> {
        let client = Client::new();
        let x = client
            .get(RUNTIMES_URL)
            .send()
            .await?
            .text()
            .await?;

        let val: Runtimes = serde_json::from_str(&x)
            .map_err(|e| UraniumError::other(format!("Failed to parse runtimes JSON: {e}")))?;

        let runtime_url = val
            .linux
            .get(&self.runtime)
            .ok_or(UraniumError::other("No runtime found"))?
            .first()
            .ok_or(UraniumError::other(
                "Mojang doesn't know about their own runtime",
            ))?
            .get_url();

        let runtime_files: RuntimeFiles = client
            .get(runtime_url)
            .send()
            .await?
            .json()
            .await?;

        let os = std::env::consts::OS;

        let minecraft_root = get_minecraft_path()
            .ok_or(UraniumError::other("Could not determine minecraft path"))?;
        let runtime_path =
            minecraft_root.join(format!("runtime/{}/{}/{}", self.runtime, os, self.runtime));

        let executables: Vec<_> = runtime_files
            .files
            .iter()
            .filter(|(_, item)| item.executable)
            .map(|(s, _)| runtime_path.join(s))
            .collect();

        let objects: Vec<DownloadableObject> = runtime_files
            .files
            .into_iter()
            .filter(|(_, s)| s.file_type == "file")
            .map(|(k, mut s)| {
                let raw = s
                    .downloads
                    .remove("raw")
                    .ok_or(UraniumError::other(format!(
                        "No raw download for runtime file {k:?}"
                    )))?;
                Ok::<_, UraniumError>(DownloadableObject::new(
                    &raw.url,
                    &runtime_path.join(k),
                    Some(HashType::Sha1(raw.sha1.to_string())),
                ))
            })
            .collect::<Result<_>>()?;

        let mut dl = Downloader::new();
        dl.add_objects(objects);
        dl.start().await?;

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in &executables {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE_MODE))?;
            }
        }

        Ok(())
    }
}
