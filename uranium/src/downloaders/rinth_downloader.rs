use super::gen_downloader::{DownloadState, Downloader};
use crate::{
    code_functions::N_THREADS,
    error::UraniumError,
    variables::constants::{RINTH_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
};
use log::info;
use mine_data_strutcs::rinth::rinth_packs::{load_rinth_pack, RinthMdFiles, RinthModpack};
use requester::requester::request_maker::RinthRequester;
use std::path::{Path, PathBuf};

/// `RinthDownloader` struct is responsable for downloading
/// the fiven modpack.
pub struct RinthDownloader {
    gen_downloader: Downloader<RinthRequester>,
    modpack: RinthModpack,
}

type Links = Vec<String>;
type Names = Vec<PathBuf>;

impl RinthDownloader {
    /// Create a new `RinthDownloader` with the given `modpack_path` and `destination`
    ///
    /// # Errors
    ///
    /// This function can returns `Err(UraniumError::WrongFileFormat)` if the given
    /// `modpack_path`is not a valid modpack file.
    pub fn new<I: AsRef<Path>>(modpack_path: I, destination: I) -> Result<Self, UraniumError> {
        let modpack = Self::load_pack(modpack_path)?;
        let (links, names) = Self::get_data(&modpack);

        let destination = destination.as_ref().to_owned();

        Self::check_mods_dir(&destination)?;
        Self::check_config_dir(&destination)?;

        Ok(RinthDownloader {
            gen_downloader: Downloader::new(
                links.into(),
                names,
                destination.into(),
                RinthRequester::new(),
            ),
            modpack,
        })
    }

    /// Returns the number of mods to download.
    #[must_use]
    pub fn len(&self) -> usize {
        self.gen_downloader.urls().len()
    }

    /// Returns `true` if there are no mods to download.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.gen_downloader.urls().is_empty()
    }

    /// Returns the number of **CHUNKS** to download.
    ///
    /// So, if `N_THREADS` is set to 2 and there are 32 mods it
    /// will return 16;
    ///
    ///
    /// 32/2 = 16
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.gen_downloader.urls().len() / N_THREADS()
    }

    /// Returns how many requests chunks are left.
    #[must_use]
    pub fn requests_left(&self) -> usize {
        let left = &self.gen_downloader.requests_left();

        if left % N_THREADS() == 0 {
            left / N_THREADS()
        } else {
            left / N_THREADS() + 1
        }
    }

    /// Simply returns the modpack name.
    #[must_use]
    pub fn get_modpack_name(&self) -> String {
        self.modpack.get_name()
    }

    fn get_data(rinth_pack: &RinthModpack) -> (Links, Names) {
        let file_links: Vec<String> = rinth_pack
            .get_files()
            .iter()
            .map(RinthMdFiles::get_download_link)
            .collect();

        info!("Downloading {} files", file_links.len());

        let file_names: Vec<PathBuf> = rinth_pack
            .get_files()
            .iter()
            .map(RinthMdFiles::get_name)
            .collect();

        for name in &file_names {
            info!("{}", name.display());
        }

        (file_links, file_names)
    }

    fn load_pack<I: AsRef<Path>>(path: I) -> Result<RinthModpack, UraniumError> {
        unzip_temp_pack(path)?;
        let Some(rinth_pack) = load_rinth_pack(&(TEMP_DIR.to_owned() + RINTH_JSON)) else {
            return Err(UraniumError::WrongFileFormat);
        };

        info!("Pack loaded {}", rinth_pack.get_name());

        Ok(rinth_pack)
    }

    /// This method will start the download and make progress until
    /// the download is completed.
    ///
    /// # Errors
    /// This function can return an `Err(UraniumError)` like `progress` can.
    pub async fn start(&mut self) -> Result<(), UraniumError> {
        self.gen_downloader.start().await
    }

    /// Make progress.
    ///
    /// If the download still in progress return
    /// the number of chunks remaining.
    ///
    /// Else return None.
    ///
    /// # Errors
    /// In case the downloader fails to download or write the chunk this method will return an
    /// error with the corresponding variant.
    pub async fn chunk(&mut self) -> Result<DownloadState, UraniumError> {
        self.gen_downloader.progress().await
    }

    fn check_mods_dir(destination: &Path) -> Result<(), UraniumError> {
        if !destination.join("mods").exists() {
            std::fs::create_dir(destination.join("mods"))
                .map_err(|_| UraniumError::CantCreateDir)?;
        }
        Ok(())
    }

    fn check_config_dir(destination: &Path) -> Result<(), UraniumError> {
        if !destination.join("config").exists() {
            std::fs::create_dir(destination.join("config"))
                .map_err(|_| UraniumError::CantCreateDir)?;
        }
        Ok(())
    }
}
