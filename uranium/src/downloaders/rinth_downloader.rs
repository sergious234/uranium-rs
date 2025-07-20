use std::path::{Path, PathBuf};

use log::info;
use mine_data_structs::rinth::{RinthMdFiles, RinthModpack, load_rinth_pack};

use super::gen_downloader::{DownloadState, DownloadableObject, FileDownloader};
use crate::zipper::pack_unzipper::remove_temp_pack;
use crate::{
    code_functions::N_THREADS,
    error::{Result, UraniumError},
    variables::constants::{RINTH_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
};

/// This struct is responsible for downloading
/// the given modpack.
///
/// Like CurseDownloader this struct takes a generic parameter which will be the
/// downloader to use:
///
/// ```rust no_run
/// # use uranium_rs::downloaders::Downloader;
/// # use uranium_rs::downloaders::RinthDownloader;
/// # use uranium_rs::error::Result;
/// # fn foo() -> Result<()> {
/// RinthDownloader::<Downloader>::new("modpack_path", "installation path")?;
/// # Ok(())
/// # }
/// ```
pub struct RinthDownloader<T: FileDownloader> {
    gen_downloader: T,
    modpack: RinthModpack,
}

type Links = Vec<String>;
type Names = Vec<PathBuf>;

impl<T: FileDownloader> RinthDownloader<T> {
    /// Create a new `RinthDownloader` with the given `modpack_path` and
    /// `destination`.
    ///
    /// # Example
    /// ```no_run
    /// 
    /// use uranium_rs::downloaders::{RinthDownloader, Downloader};
    /// use uranium_rs::error::Result;
    ///
    /// # async fn foo() -> Result<()> {
    /// //                                  FileDownloader to use (mandatory)
    /// //                                           vvvvvvvvvv
    /// let mut rinth_downloader = RinthDownloader::<Downloader>::new(
    ///     "/my_modpack/path",
    ///     "/installation/path"
    /// )?;
    ///
    /// rinth_downloader.complete().await?;
    /// Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function can return `Err(UraniumError::WrongFileFormat)` if the
    /// given `modpack_path` is not a valid modpack file. Also, can fail if the
    /// mods dir, resourcepacks dir or config dir are missing and can't be
    /// created.
    pub fn new<I: AsRef<Path>, J: AsRef<Path>>(modpack_path: I, destination: J) -> Result<Self> {
        let modpack = Self::load_pack(modpack_path)?;
        let (links, names) = Self::get_data(&modpack);

        let destination = destination.as_ref();

        Self::check_mods_dir(destination)?;
        Self::check_rp_dir(destination)?;
        Self::check_config_dir(destination)?;

        let files = links
            .iter()
            .zip(names.iter())
            .map(|(url, name)| DownloadableObject::new(url, &destination.join(name), None))
            .collect();

        Ok(RinthDownloader {
            gen_downloader: T::new(files),
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
        self.gen_downloader
            .requests_left()
            == 0
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
        let left = &self
            .gen_downloader
            .requests_left();

        if left % N_THREADS() == 0 {
            left / N_THREADS()
        } else {
            left / N_THREADS() + 1
        }
    }

    /// Simply returns the modpack name.
    #[must_use]
    pub fn get_modpack_name(&self) -> String {
        self.modpack
            .name
            .to_str()
            .unwrap_or_default()
            .to_string()
    }

    fn get_data(rinth_pack: &RinthModpack) -> (Links, Names) {
        let file_links: Vec<String> = rinth_pack
            .get_files()
            .iter()
            .map(RinthMdFiles::get_download_link)
            .map(str::to_owned)
            .collect();

        info!("Downloading {} files", file_links.len());

        let file_names: Vec<PathBuf> = rinth_pack
            .get_files()
            .iter()
            .map(RinthMdFiles::get_path)
            .map(Path::to_owned)
            .collect();

        for name in &file_names {
            info!("{}", name.display());
        }

        (file_links, file_names)
    }

    fn load_pack<I: AsRef<Path>>(path: I) -> Result<RinthModpack> {
        match unzip_temp_pack(&path) {
            Err(UraniumError::CantCreateDir("temp_dir")) => {
                // retry
                unzip_temp_pack(path)?
            }
            Err(e) => Err(e)?,
            Ok(_) => {}
        }

        if let Some(rinth_pack) = load_rinth_pack(&(TEMP_DIR.to_owned() + RINTH_JSON)) {
            info!("Pack loaded {}", rinth_pack.get_name());
            Ok(rinth_pack)
        } else {
            Err(UraniumError::WrongFileFormat)
        }
    }

    /// This method will start the download and make progress until
    /// the download is completed.
    ///
    /// # Errors
    /// This function can return an `Err(UraniumError)` like `progress` can.
    pub async fn complete(&mut self) -> Result<()> {
        let r = self
            .gen_downloader
            .complete()
            .await;
        remove_temp_pack();
        r
    }

    /// Make progress.
    ///
    /// If the download still in progress return
    /// the number of chunks remaining.
    ///
    /// Else return None.
    ///
    /// # Errors
    /// In case the downloader fails to download or write the chunk this method
    /// will return an error with the corresponding variant.
    pub async fn progress(&mut self) -> Result<DownloadState> {
        let r = self
            .gen_downloader
            .progress()
            .await;
        if let Ok(DownloadState::Completed) = r {
            remove_temp_pack();
        }
        r
    }

    pub fn get_modpack(&self) -> &RinthModpack {
        &self.modpack
    }

    fn check_mods_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("mods")
            .exists()
        {
            info!("Creating mods dir");
            std::fs::create_dir(destination.join("mods"))?;
        }
        Ok(())
    }

    fn check_rp_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("resourcepacks")
            .exists()
        {
            info!("Creating resourcepacks dir");
            std::fs::create_dir(destination.join("resourcepacks"))?;
        }
        Ok(())
    }

    fn check_config_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("config")
            .exists()
        {
            info!("Creating config dir");
            std::fs::create_dir(destination.join("config"))?;
        }
        Ok(())
    }
}
