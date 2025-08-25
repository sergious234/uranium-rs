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
//! Here's a basic example of how to use the `RuntimeDownloader` to download a runtime.
//!
//! ```no_run
//! # use uranium_rs::downloaders::RuntimeDownloader;
//! # use uranium_rs::error::Result;
//! #
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut downloader = RuntimeDownloader::new("java-runtime-beta".to_string());
//!     downloader.download().await?;
//!     println!("Runtime downloaded and installed successfully!");
//!     Ok(())
//! }
//! ```

use std::fs;

use mine_data_structs::minecraft::RUNTIMES_URL;
use mine_data_structs::minecraft::{get_minecraft_path, RuntimeFiles, Runtimes};
use reqwest::Client;

use super::DownloadableObject;
use crate::downloaders::{Downloader, FileDownloader, HashType};
use crate::error::{Result, UraniumError};

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
    pub async fn download(&mut self) -> Result<()> {
        let client = Client::new();
        let x = client
            .get(RUNTIMES_URL)
            .send()
            .await?
            .text()
            .await?;

        let val: Runtimes = serde_json::from_str(&x).unwrap();

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

        let minecraft_root = get_minecraft_path().unwrap();
        let runtime_path =
            minecraft_root.join(format!("runtime/{}/{}/{}", self.runtime, os, self.runtime));

        let executables_files = runtime_files
            .files
            .iter()
            .filter(|(_, item)| item.executable)
            .map(|(s, _)| runtime_path.join(s));

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            executables_files
                .flat_map(fs::metadata)
                .for_each(|metadata| {
                    metadata
                        .permissions()
                        .set_mode(0o766)
                });
        }

        let objects: Vec<DownloadableObject> = runtime_files
            .files
            .into_iter()
            .filter(|(_, s)| s.file_type == "file")
            .map(|(k, mut s)| {
                let raw = s
                    .downloads
                    .remove("raw")
                    .unwrap();
                (runtime_path.join(k), raw.url, raw.sha1)
            })
            .map(|(k, s, h)| DownloadableObject::new(&s, &k, Some(HashType::Sha1(h.to_string()))))
            .collect();

        Downloader::new(objects).complete().await
    }
}
