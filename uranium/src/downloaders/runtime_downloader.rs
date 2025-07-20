use std::fs;

use mine_data_structs::minecraft::{FileRelPath, RUNTIMES_URL};
use mine_data_structs::minecraft::{RuntimeFiles, Runtimes, get_minecraft_path};
use reqwest::Client;

use super::DownloadableObject;
use crate::downloaders::{Downloader, FileDownloader, HashType};
use crate::error::{Result, UraniumError};

pub struct RuntimeDownloader {
    runtime: String,
}

impl RuntimeDownloader {
    pub fn new(runtime: String) -> Self {
        Self { runtime }
    }

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

        let executables_files: Box<[FileRelPath]> = runtime_files
            .files
            .iter()
            .filter(|(_, item)| item.executable)
            .map(|(s, _)| runtime_path.join(s))
            .collect();

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            executables_files
                .iter()
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

        let mut downloader = Downloader::new(objects);
        downloader.complete().await?;

        Ok(())
    }
}
