use std::path::Path;

use futures::future::join_all;
use mine_data_structs::{
    curse::{curse_modpacks::*, curse_mods::*},
    url_maker::maker::Curse,
};
use reqwest::Response;

use super::{gen_downloader::DownloadState, DownloadableObject};
use crate::{
    code_functions::N_THREADS,
    error::{Result, UraniumError},
    variables::constants::{CURSE_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
    FileDownloader,
};

/// This struct is responsible for downloading Curse modpacks.
///
/// Like RinthDownloader struct it takes a generic parameter which will be the
/// downloader used:
///
/// ```no_run
/// # use uranium::downloaders::Downloader;
/// # use uranium::downloaders::CurseDownloader;
/// # async fn foo() {
/// CurseDownloader::<Downloader>::new("modpack_path", "installation_path").await;
/// # }
pub struct CurseDownloader<T: FileDownloader> {
    gen_downloader: T,
    modpack: CursePack,
}

impl<T: FileDownloader> CurseDownloader<T> {
    pub async fn new<I: AsRef<Path>, J: AsRef<Path>>(
        modpack_path: I,
        destination: J,
    ) -> Result<Self> {
        let destination = destination.as_ref();
        Self::check_mods_dir(destination)?;
        Self::check_rp_dir(destination)?;
        Self::check_config_dir(destination)?;

        unzip_temp_pack(modpack_path)?;

        let curse_pack = load_curse_pack((TEMP_DIR.to_owned() + CURSE_JSON).as_ref())
            .expect("Couldnt load the pack");

        let files_ids: Vec<String> = curse_pack
            .get_files()
            .iter()
            .map(|f| {
                Curse::file(
                    &f.get_project_id().to_string(),
                    &f.get_file_id().to_string(),
                )
            })
            .collect();

        let mut header_map = reqwest::header::HeaderMap::new();
        let (_, curse_api_key) = std::env::vars()
            .find(|(v, _)| v == "CURSE_API_KEY")
            .unwrap_or_default();

        /* TODO!: This should be other Error kind since the problem isn't coming from
           reqwest but from http InvalidHeaderValue error kind
        */
        header_map.insert("x-api-key", curse_api_key.parse()?);
        header_map.insert("Content-Type", "application/json".parse()?);
        header_map.insert("Accept", "application/json".parse()?);

        let client = reqwest::ClientBuilder::new()
            .default_headers(header_map)
            .build()?;

        let responses: Vec<Response> = Self::get_mod_responses(&client, &files_ids).await;
        let mut files = Vec::with_capacity(responses.len());
        let mods_path = destination.join("mods/");

        for response in responses {
            let cf = response
                .json::<CurseResponse<CurseFile>>()
                .await?;
            files.push(DownloadableObject::new(
                &cf.data.get_download_url(),
                cf.data
                    .get_file_name()
                    .to_str()
                    .unwrap_or_default(),
                &mods_path,
                None,
            ));
        }

        Ok(CurseDownloader {
            gen_downloader: T::new(files),
            modpack: curse_pack,
        })
    }

    /// This function will call `FileDownloader::progress()` and returns it's
    /// output.
    pub async fn progress(&mut self) -> Result<DownloadState> {
        self.gen_downloader
            .progress()
            .await
    }

    /// This function will call `FileDownloader::complete' and returns it's
    /// output.
    pub async fn complete(&mut self) -> Result<()> {
        self.gen_downloader
            .complete()
            .await
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
    pub fn get_modpack_name(&self) -> &str {
        &self.modpack.name
    }

    /// Returns a reference to the modpack
    #[must_use]
    pub fn get_curse_pack(&self) -> &CursePack {
        &self.modpack
    }
}

// TODO: This is repeated in RinthDownloader, maybe put this functions in
// code_functions.rs ?
//
// Also how requests are done should look like Downloader where tasks are
// spawned.
impl<T: FileDownloader> CurseDownloader<T> {
    async fn get_mod_responses(curse_req: &reqwest::Client, files_ids: &[String]) -> Vec<Response> {
        let mut responses: Vec<Response> = Vec::with_capacity(files_ids.len());
        let threads: usize = N_THREADS();

        for chunk in files_ids.chunks(threads) {
            let mut requests = Vec::with_capacity(chunk.len());
            for url in chunk {
                let task = async move { curse_req.get(url).send() }.await;
                requests.push(task);
            }
            let res: Vec<Response> = join_all(requests)
                .await
                .into_iter()
                .flatten()
                .collect();
            responses.extend(res);
        }

        responses
    }

    // Duplicate code ? Maybe
    // DRY ? Nope
    // Wet ? ;)
    fn check_mods_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("mods")
            .exists()
        {
            std::fs::create_dir(destination.join("mods"))
                .map_err(|_| UraniumError::CantCreateDir("mods"))?;
        }
        Ok(())
    }

    fn check_rp_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("resourcepacks")
            .exists()
        {
            std::fs::create_dir(destination.join("resourcepacks"))
                .map_err(|_| UraniumError::CantCreateDir("resourcepacks"))?;
        }
        Ok(())
    }

    fn check_config_dir(destination: &Path) -> Result<()> {
        if !destination
            .join("config")
            .exists()
        {
            std::fs::create_dir(destination.join("config"))
                .map_err(|_| UraniumError::CantCreateDir("config"))?;
        }
        Ok(())
    }
}
