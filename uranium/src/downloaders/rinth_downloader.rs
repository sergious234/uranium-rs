use super::gen_downloader::{DownloadState, DownlodableObject, FileDownloader};
use crate::{
    code_functions::N_THREADS,
    error::UraniumError,
    variables::constants::{RINTH_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
};
use log::info;
use mine_data_strutcs::rinth::rinth_packs::{load_rinth_pack, RinthMdFiles, RinthModpack};
use std::path::{Path, PathBuf};

/// This struct is responsable for downloading
/// the given modpack.
///
/// Like CurseDownloader this struct takes a generic parameter which will be the
/// downloader to use:
///
/// ```rust
/// # use uranium::downloaders::Downloader;
/// # use uranium::downloaders::RinthDownloader;
/// # fn foo() {
/// RinthDownloader::<Downloader>::new("modpack_path", "installation path");
/// # }
pub struct RinthDownloader<T: FileDownloader> {
    gen_downloader: T,
    modpack: RinthModpack,
}

type Links = Vec<String>;
type Names = Vec<PathBuf>;

impl<T: FileDownloader> RinthDownloader<T> {
    /// Create a new `RinthDownloader` with the given `modpack_path` and `destination`
    ///
    /// # Errors
    ///
    /// This function can returns `Err(UraniumError::WrongFileFormat)` if the given
    /// `modpack_path`is not a valid modpack file.
    pub fn new<I: AsRef<Path>>(modpack_path: I, destination: I) -> Result<Self, UraniumError> {
        let modpack = Self::load_pack(modpack_path)?;
        let (links, names) = Self::get_data(&modpack);

        let destination = destination.as_ref();

        Self::check_mods_dir(destination)?;
        Self::check_rp_dir(destination)?;
        Self::check_config_dir(destination)?;

        let files = links.iter().zip(names.iter()).map(|(url, name)|
            DownlodableObject::new(url, name.to_str().unwrap_or_default(), destination, None)
        ).collect();

        Ok(RinthDownloader {
            gen_downloader: T::new(
                files
            ),
            modpack,
        })
    }

    /// Returns the number of mods to download.
    #[must_use]
    pub fn len(&self) -> usize {
        self.gen_downloader.len()
    }

    /// Returns `true` if there are no mods to download.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.gen_downloader.requests_left() == 0
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
        self.gen_downloader.len() / N_THREADS()
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
    #[must_use]
    pub async fn complete(&mut self) -> Result<(), UraniumError> {
        self.gen_downloader.complete().await
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
    #[must_use]
    pub async fn progress(&mut self) -> Result<DownloadState, UraniumError> {
        self.gen_downloader.progress().await
    }

    fn check_mods_dir(destination: &Path) -> Result<(), UraniumError> {
        if !destination.join("mods").exists() {
            std::fs::create_dir(destination.join("mods"))
                .map_err(|_| UraniumError::CantCreateDir)?;
        }
        Ok(())
    }

    fn check_rp_dir(destination: &Path) -> Result<(), UraniumError> {
        if !destination.join("resourcepacks").exists() {
            std::fs::create_dir(destination.join("resourcepacks"))
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
