#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]


use std::path::Path;

use downloaders::{RinthDownloader, minecraft_downloader::MinecraftDownloader};
use error::{MakerError, UraniumError};
use log::info;
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
pub async fn make_modpack<I: AsRef<Path>>(minecraft_path: I) -> Result<(), MakerError> {
    let mut maker = ModpackMaker::new(&minecraft_path);
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
pub async fn rinth_pack_download<I: AsRef<Path>>(
    file_path: I,
    destination_path: I,
) -> Result<(), UraniumError> {
    let mut rinth_downloader = RinthDownloader::new(&file_path, &destination_path)?;
    rinth_downloader.start().await?;
    Ok(())
}

/// # Easy to go function
///
/// This function still work in progress
///
/// # Errors
/// This function will return an `Err(UraniumError)` in case the `MinecraftDownloader` has an error
/// during the download.
pub async fn download_minecraft<I: AsRef<Path>>(
    instance: &str,
    destination_path: I,
) -> Result<(), UraniumError> {
    let mut minecraft_downloader = MinecraftDownloader::init(destination_path, instance).await;
    let _ = minecraft_downloader.start().await?;
    Ok(())
}

/// This function will set the max number of threads allowed to use
///
/// Use it carefully, a big number of threads may decrease the performance.
/// The default number of threads is 32.
///
/// # Panics
/// Will panic in case `RwLockWriteGuard` cant be acquired
pub fn set_threads(t: usize) {
    let mut aux = NTHREADS.write().unwrap();
    *aux = t;
}

/// Init the logger and make a log.txt file to write logs content.
///
/// If this function is not called then there will be no
/// log.txt or any kind of debug info/warn/warning message will
/// be show in console.
///
/// # Panics
/// Will panic in case log files or `CombinedLogger` cant be created.
pub fn init_logger() {
    use chrono::prelude::Local;
    use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger};
    use std::fs::File;

    let log_file_name = format!("log_{}.txt", Local::now().format("%H-%M-%S_%d-%m-%Y"));
    let lastest_log_file = "lastest_log_file";
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(log_file_name).unwrap(),
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(lastest_log_file).unwrap(),
        ),
    ])
    .unwrap();
}
