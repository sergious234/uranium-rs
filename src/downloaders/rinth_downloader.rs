use std::path::Path;

use log::info;
use mine_data_structs::rinth::{RinthModpack, load_rinth_pack};

use super::gen_downloader::{DownloadState, DownloadableObject, FileDownloader};
use crate::{
    code_functions::{num_threads, remove_temp_pack, unzip_temp_pack},
    downloaders::Downloader,
    error::{Result, UraniumError},
    variables::constants::{RINTH_JSON, TEMP_DIR},
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
    downloader: T,
    modpack: RinthModpack,
}

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
    /// //                                      Downloader to use (mandatory)
    /// //                                           vvvvvvvvvv
    /// let mut rinth_downloader = RinthDownloader::<Downloader>::new(
    ///     "/my_modpack/path",
    ///     "/installation/path"
    /// )?;
    ///
    /// rinth_downloader.start().await?;
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

        let destination = destination.as_ref();

        Self::check_mods_dir(destination)?;
        Self::check_rp_dir(destination)?;
        Self::check_config_dir(destination)?;

        let objs = modpack
            .files
            .iter()
            .map(|file| {
                DownloadableObject::new(
                    file.get_download_link(),
                    &destination.join(file.get_path()),
                    None,
                )
            });

        let mut downloader = T::new();
        downloader.add_objects(objs);

        Ok(RinthDownloader {
            downloader,
            modpack,
        })
    }


    /// Returns `true` if there are no mods to download.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.downloader
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
        self.downloader.len() / num_threads()
    }

    /// Returns how many requests chunks are left.
    #[must_use]
    pub fn requests_left(&self) -> usize {
        let left = &self
            .downloader
            .requests_left();

        if left.is_multiple_of(num_threads()) {
            left / num_threads()
        } else {
            left / num_threads() + 1
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

    fn load_pack<I: AsRef<Path>>(path: I) -> Result<RinthModpack> {
        match unzip_temp_pack(&path) {
            Err(UraniumError::CantCreateDir("temp_dir")) => {
                // retry
                unzip_temp_pack(path)?
            }
            Err(e) => Err(e)?,
            Ok(_) => {}
        }

        if let Some(rinth_pack) = load_rinth_pack(TEMP_DIR.join(RINTH_JSON)) {
            info!("Pack loaded {}", rinth_pack.get_name());
            Ok(rinth_pack)
        } else {
            Err(UraniumError::WrongFileFormat)
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let r = self.downloader.start().await;
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
            .downloader
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
        .start()
        .await?;
    Ok(())
}
