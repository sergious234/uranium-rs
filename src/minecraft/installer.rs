use std::path::{Path, PathBuf};

use log::{error, info};
use mine_data_structs::minecraft::{MinecraftVersions, Root};

use super::runtime::RuntimeDownloader;
use super::{
    assets::AssetsStep,
    client::ClientStep,
    libraries,
    steps::{InstallCtx, InstallStep},
    version,
};
use crate::config::num_threads;
use crate::engine::{DownloadState, FileDownloader};
use crate::error::Result;

// region:    --- Re-exports for backward compat (delegates to version.rs)

pub async fn list_instances() -> Result<MinecraftVersions> {
    version::list_instances().await
}

pub async fn get_last_snapshot() -> Result<String> {
    version::get_last_snapshot().await
}

pub async fn get_last_release() -> Result<String> {
    version::get_last_release().await
}

// endregion: --- Re-exports

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
/// Facade that orchestrates phase steps via `InstallStep` trait.
pub struct MinecraftDownloader<T: FileDownloader + Send> {
    requester: reqwest::Client,
    dot_minecraft_path: PathBuf,
    minecraft_instance: Root,
    download_state: MinecraftDownloadState,
    downloader: T,
}

impl<T: FileDownloader + Send + Sync> MinecraftDownloader<T> {
    pub async fn init<I: AsRef<Path>>(
        destination_path: I,
        minecraft_version: &str,
    ) -> Result<Self> {
        MinecraftDownloader::with_downloader(destination_path, minecraft_version, T::new()).await
    }

    pub async fn with_downloader<I: AsRef<Path>>(
        destination_path: I,
        minecraft_version: &str,
        downloader: T,
    ) -> Result<Self> {
        let requester = reqwest::Client::new();
        let minecraft_instance = version::fetch_root(&requester, minecraft_version).await?;
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

    pub async fn progress(&mut self) -> Result<MinecraftDownloadState> {
        match self.download_state {
            MinecraftDownloadState::GettingSources => {
                let mut ctx = InstallCtx {
                    dot_minecraft_path: &self.dot_minecraft_path,
                    instance: &self.minecraft_instance,
                    requester: &self.requester,
                    downloader: &mut self.downloader,
                };
                let mut step = AssetsStep;
                self.download_state = step.run(&mut ctx).await?;
            }

            MinecraftDownloadState::DownloadingVersion => {
                let mut ctx = InstallCtx {
                    dot_minecraft_path: &self.dot_minecraft_path,
                    instance: &self.minecraft_instance,
                    requester: &self.requester,
                    downloader: &mut self.downloader,
                };
                let mut step = ClientStep;
                self.download_state = step.run(&mut ctx).await?;
            }

            MinecraftDownloadState::DownloadingAssets => {
                let download_state = self
                    .downloader
                    .progress()
                    .await;
                match download_state {
                    Ok(DownloadState::Completed) => {
                        let libs = libraries::prepare_libraries(
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
                let runtime_res = RuntimeDownloader::<T>::new(
                    self.minecraft_instance
                        .java_version
                        .component
                        .to_string(),
                )
                .start_with(&mut self.downloader)
                .await;

                if let Err(err) = runtime_res {
                    error!("Error downloading runtime: {}", err);
                    return Err(err);
                }
                self.download_state = MinecraftDownloadState::CheckingFiles;
            }

            MinecraftDownloadState::CheckingFiles => {
                self.download_state = MinecraftDownloadState::Completed;
            }

            MinecraftDownloadState::Completed => {
                info!("Minecraft download complete!");
            }
        };

        Ok(self.download_state.clone())
    }

    pub fn requests_left(&self) -> usize {
        self.downloader
            .requests_left()
    }

    pub fn lib_chunks(&self) -> usize {
        let n = self
            .minecraft_instance
            .libraries
            .len() as f64;
        (n / num_threads() as f64).ceil() as usize
    }

    pub fn chunks(&self) -> usize {
        let n = self
            .downloader
            .requests_left() as f64;
        (n / num_threads() as f64).ceil() as usize
    }
}
