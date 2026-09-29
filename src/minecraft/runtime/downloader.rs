//! Java runtime downloader — generic over `FileDownloader`.

use mine_data_structs::minecraft::RUNTIMES_URL;
use mine_data_structs::minecraft::{RuntimeFiles, Runtimes, get_minecraft_path};
use reqwest::Client;

use crate::common::constants::EXECUTABLE_MODE;
use crate::engine::{DownloadableObject, Downloader, FileDownloader, HashType};
use crate::error::{Result, UraniumError};

/// Downloader for Java runtimes. Generic over `D: FileDownloader` so callers
/// can inject a custom downloader (DIP). `RuntimeDownloader` without turbofish
/// defaults to `Downloader`.
pub struct RuntimeDownloader<D: FileDownloader = Downloader> {
    runtime: String,
    _marker: std::marker::PhantomData<D>,
}

impl<D: FileDownloader> RuntimeDownloader<D> {
    pub fn new(runtime: String) -> Self {
        Self {
            runtime,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates with an explicit downloader instance (for callers that
    /// pre-configure `D`).
    pub fn with_downloader(runtime: String, _downloader: D) -> Self {
        Self::new(runtime)
    }

    /// Fetches the runtime manifest, downloads all required files, and sets
    /// executable bits.
    pub async fn start(&mut self) -> Result<()> {
        let mut dl = D::new();
        self.start_with(&mut dl).await
    }

    /// Same as `start` but re-uses an existing downloader instance.
    pub async fn start_with(&mut self, dl: &mut D) -> Result<()> {
        let client = Client::new();
        let runtimes = client
            .get(RUNTIMES_URL)
            .send()
            .await?
            .text()
            .await?;
        let val: Runtimes = serde_json::from_str(&runtimes)
            .map_err(|e| UraniumError::other(format!("Failed to parse runtimes JSON: {e}")))?;
        let runtime_url = self.get_runtime_url(&val)?;
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

        let executables = runtime_files
            .files
            .iter()
            .filter(|(_, item)| item.executable)
            .map(|(s, _)| runtime_path.join(s));

        let objects = runtime_files
            .files
            .iter()
            .filter(|(_, s)| s.file_type == "file")
            .flat_map(|(k, s)| -> Result<DownloadableObject> {
                let raw = s
                    .downloads
                    .get("raw")
                    .ok_or(UraniumError::other(format!(
                        "No raw download for runtime file {k:?}"
                    )))?;
                Ok(DownloadableObject::new(
                    &raw.url,
                    &runtime_path.join(k),
                    Some(HashType::Sha1(raw.sha1.to_string())),
                ))
            });

        dl.add_objects(objects);
        dl.start().await?;

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in executables {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE_MODE))?;
            }
        }

        Ok(())
    }

    fn get_runtime_url<'a>(&self, val: &'a Runtimes) -> Result<&'a str> {
        let runtime_url = val
            .linux
            .get(&self.runtime)
            .ok_or(UraniumError::other("No runtime found"))?
            .first()
            .ok_or(UraniumError::other(
                "Mojang doesn't know about their own runtime",
            ))?
            .get_url();
        Ok(runtime_url)
    }
}

impl RuntimeDownloader<Downloader> {
    /// Convenience: creates and runs with the default `Downloader`.
    pub async fn download(runtime: String) -> Result<()> {
        let mut r = Self::new(runtime);
        r.start().await
    }
}
