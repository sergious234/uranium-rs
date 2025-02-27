#![feature(path_add_extension)]
#![forbid(unsafe_code)]
#![warn(clippy::all)]

//! # uranium
//!
//! The `uranium` crate provides an easy, high-level API for:
//! - Downloading Minecraft instances, mods from Rinth/Curse
//! - Making a modpack from a given directory
//! - Update a modpack from a given directory
//!
//!
//! Also, `uranium` provides high modularity level when it comes to downloaders.
//! Through the [`FileDownloader`](downloaders) trait.
//!
//! When using downloaders such as [`RinthDownloader`](RinthDownloader) it takes
//! a generic parameter `T: FileDownloader` so **YOU** the user can implement
//! your own downloader if you dislike mine :( or thinks you can do a faster
//! one.
//!
//! ``` rust no_run
//! # async fn x() {
//! use uranium_rs::downloaders::{Downloader, RinthDownloader};
//!
//! let mut rinth = RinthDownloader::<Downloader>::new("path", "destination").unwrap();
//!
//! if let Err(e) = rinth.complete().await {
//!     println!("Something went wrong: {e}")
//! } else {
//!     println!("Download complete!")
//! }
//! # }
//! ```
//!
//!
//! This crate is under development so breaking changes may occur in later
//! versions, but I'll try to avoid them.

use std::path::Path;

use downloaders::{
    CurseDownloader, Downloader, FileDownloader, MinecraftDownloader as MD, RinthDownloader, RuntimeDownloader
};
use error::{Result, UraniumError};
use log::info;
pub use mine_data_structs;
use modpack_maker::{ModpackMaker, State};
use variables::constants::*;

pub mod downloaders;
pub mod error;
pub mod modpack_maker;
pub mod searcher;

mod code_functions;
mod hashes;
mod variables;
mod zipper;

/// # Easy to go function
///
/// This function will make a Modpack from the
/// given path.
///
/// # Errors
/// This function will return a `MakeError` in case the modpack can't
/// be made for any reason.
pub async fn make_modpack<I: AsRef<Path>, J: AsRef<Path>>(
    minecraft_path: I,
    modpack_name: J,
) -> Result<()> {
    let mut maker = ModpackMaker::new(&minecraft_path, modpack_name);
    maker.start()?;
    let mut i = 0;
    loop {
        match maker.chunk().await {
            Ok(State::Finish) => return Ok(()),
            Err(e) => return Err(e),
            _ => {
                info!("{}", i);
                i += 1;
            }
        }
    }

    //ModpackMaker::make(&minecraft_path).await
}

/// # Easy to go function
///
/// This function will download the modpack specified by `file_path`
/// into `destination_path`
///
/// If there is no mods and/or config folder inside `destination_path` then they
/// will be created.
///
///
/// # Errors
/// This function will return an `UraniumError` in case the download
/// fails or when one or more paths are wrong.
pub async fn curse_pack_download<I: AsRef<Path>, J: AsRef<Path>>(
    file_path: I,
    destination_path: J,
) -> Result<()> {
    let mut curse_downloader =
        CurseDownloader::<Downloader>::new(&file_path, &destination_path).await?;
    curse_downloader
        .complete()
        .await?;
    Ok(())
}

/// # Easy to go function
///
/// This function will download the modpack specified by `file_path`
/// into `destination_path`
///
/// If there is no mods and/or config folder inside `destination_path` then they
/// will be created.
///
///
/// # Errors
/// This function will return an `UraniumError` in case the download
/// fails or when one or more paths are wrong.
pub async fn rinth_pack_download<I: AsRef<Path>, J: AsRef<Path>>(
    file_path: I,
    destination_path: J,
) -> Result<()> {
    let mut rinth_downloader = RinthDownloader::<Downloader>::new(&file_path, &destination_path)?;
    rinth_downloader
        .complete()
        .await?;
    Ok(())
}

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
pub fn set_threads(t: usize) -> Option<()> {
    let mut aux = NTHREADS.write().ok()?;
    *aux = t;
    Some(())
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
    .unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
