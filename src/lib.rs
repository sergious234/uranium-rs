#![forbid(unsafe_code)]

//! # uranium
//!
//! The `uranium` crate provides an easy, high-level API for:
//! - Downloading Minecraft instances, mods from Rinth/Curse
//! - Making a modpack from a given directory
//! - Update a modpack from a given directory
//!
//!
//! Also, `uranium` provides high modularity level when it comes to downloaders.
//! Through the [`FileDownloader`](engine) trait.
//!
//! When using downloaders such as
//! [`RinthInstaller`](modpacks::rinth::RinthInstaller) it takes
//! a generic parameter `T: FileDownloader`, so **YOU**, the user, can implement
//! your own downloader if you dislike mine :( or thinks you can do a faster
//! one.
//!
//! ``` rust no_run
//! # async fn x() -> uranium_rs::error::Result<()> {
//! use uranium_rs::engine::{Downloader, FileDownloader};
//! use uranium_rs::modpacks::rinth::RinthInstaller;
//!
//! let mut rinth = RinthInstaller::<Downloader>::new("path", "destination")?;
//!
//! if let Err(e) = rinth.start().await {
//!     println!("Something went wrong: {e}")
//! } else {
//!     println!("Download complete!")
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # New modular layout (v2.0)
//!
//! - `crate::engine` — generic `FileDownloader` trait + `Downloader` +
//!   `DownloadableObject`
//! - `crate::common` — shared helpers (`hash`, `fs`, `constants`)
//! - `crate::config` — `DownloaderConfig` and global thread helpers
//! - `crate::minecraft` — `MinecraftDownloader` (installer), `verify`
//!   (InstallationVerifier/Fixer), `runtime`
//! - `crate::modpacks` — `rinth` (downloader, maker, updater) and `curse`
//!   (downloader)
//!
//! This crate is under development so breaking changes may occur in later
//! versions, but I'll try to avoid them.

// region:    --- Modules

pub mod common;
pub mod config;
pub mod engine;
pub mod error;
pub mod minecraft;
pub mod modpacks;

// endregion: --- Modules
use std::path::Path;

use config::{DownloaderConfig, NTHREADS};
use engine::Downloader;
use error::{Result, UraniumError};
pub use mine_data_structs;
use minecraft::MinecraftDownloader as MD;

/// # Easy to go function
///
/// This function still work in progress
///
/// # Errors
/// This function will return an `Err(UraniumError)` in case the
/// `MinecraftDownloader` has an error during the download.
pub async fn download_minecraft<I: AsRef<Path>>(instance: &str, destination_path: I) -> Result<()> {
    let mut minecraft_downloader = MD::<Downloader>::init(destination_path, instance).await?;
    minecraft_downloader
        .start()
        .await?;
    Ok(())
}

/// This function will set the max number of threads allowed to use.
///
/// Use it carefully, a big number of threads may decrease the performance.
/// The default number of threads is 32.
///
/// In case the number of threads can't be updated this function will return
/// None, in case of success Some(()) is returned.
///
/// Deprecated — prefer `DownloaderConfig::with_max_concurrent` or
/// `Downloader::from_config`.
pub fn set_threads(t: usize) -> Option<()> {
    let mut aux = NTHREADS.write().ok()?;
    *aux = t;
    Some(())
}

/// Returns current global thread limit.
pub fn threads() -> usize {
    crate::config::num_threads()
}

/// Creates a `DownloaderConfig` with the given concurrency.
pub fn downloader_config(max_concurrent: usize) -> DownloaderConfig {
    DownloaderConfig::new(max_concurrent)
}

/// Init the logger and make a log.txt file to write logs content.
///
/// If this function is not called then there will be no
/// log.txt or any kind of debug info/warn/warning message will
/// be show in console.
///
/// # Panics
/// Will panic in case log files or `CombinedLogger` cant be created.
pub fn init_logger() -> Result<()> {
    use std::fs::File;

    use chrono::prelude::Local;
    use simplelog::{
        ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
    };

    let home_dir = dirs::home_dir().ok_or(UraniumError::OtherWithReason(
        "Cant get user home directory".to_string(),
    ))?;

    let log_file_name = home_dir
        .join(".uranium")
        .join(format!("log_{}", Local::now().format("%H-%M-%S_%d-%m-%Y")));

    let latest_log_file = home_dir
        .join(".uranium")
        .join("latest_log_file.txt");

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(log_file_name)?,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(latest_log_file)?,
        ),
    ])
    .map_err(|e| UraniumError::OtherWithReason(e.to_string()))?;
    Ok(())
}

// region:    --- Re-exports for ergonomics

pub use engine::{DownloadState, DownloadableObject, HashType};
pub use modpacks::curse::curse_pack_download;
pub use modpacks::rinth::updater::update_modpack;
pub use modpacks::rinth::{make_modpack, rinth_pack_download};

// endregion: --- Re-exports

#[cfg(test)]
mod tests {}
