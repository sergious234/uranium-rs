#![forbid(unsafe_code)]

use std::{io::Write, path::Path};

use downloaders::RinthDownloader;
use error::{MakerError, ModpackError};
use log::info;
use modpack_maker::maker::{ModpackMaker, State};
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
pub async fn rinth_pack_download<I: AsRef<Path>>(
    file_path: I,
    destination_path: I,
) -> Result<(), ModpackError> {
    let mut rinth_downloader = RinthDownloader::new(&file_path, &destination_path)?;
    rinth_downloader.start().await;
    let total = rinth_downloader.chunks();
    let mut i = 1;

    loop {
        let _ = std::io::stdout().flush();
        if rinth_downloader.chunk().await.is_ok() {
            print!("\r{} / {}      ", i, total);
            i += 1;
        } else {
            return Ok(());
        }
    }
}

/// This function will set the max number of threads allowed to use
///
/// Use it carefully, a big number of threads may decrease the performance.
/// The default number of threads is 32.
pub fn set_threads(t: usize) {
    let mut aux = NTHREADS.write().unwrap();
    *aux = t;
}

/// Init the logger and make a log.txt file to write logs content.
///
/// If this function is not called then there will be no
/// log.txt or any kind of debug info/warn/warning message will
/// be show in console.
pub fn init_logger() {
    use chrono::prelude::Local;
    use simplelog::*;
    use std::fs::File;

    let log_file_name = format!("log_{}.txt", Local::now().format("%H-%M-%S_%d-%m-%Y"));
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
    ])
    .unwrap();
}
