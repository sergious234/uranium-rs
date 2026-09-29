use std::path::Path;

use futures_util::future::join_all;
use mine_data_structs::{curse::*, maker};
use reqwest::Response;

use crate::common::constants::{CURSE_JSON, TEMP_DIR};
use crate::common::fs::unzip_temp_pack;
use crate::config::num_threads;
use crate::engine::{DownloadState, DownloadableObject, Downloader, FileDownloader};
use crate::error::{Result, UraniumError};
use crate::modpacks::common::ensure_pack_dirs;

/// This struct is responsible for downloading Curse modpacks.
///
/// Like RinthInstaller struct it takes a generic parameter which will be the
/// downloader used:
///
/// ```no_run
/// # use uranium_rs::engine::Downloader;
/// # use uranium_rs::modpacks::curse::CurseDownloader;
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
        ensure_pack_dirs(destination)?;

        unzip_temp_pack(modpack_path)?;

        let curse_pack = load_curse_pack(
            &TEMP_DIR
                .join(CURSE_JSON)
                .to_string_lossy(),
        )
        .ok_or(UraniumError::WrongModpackFormat)?;

        let files_ids: Vec<String> = curse_pack
            .get_files()
            .iter()
            .map(|f| {
                maker::curse_file(
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
                cf.data.get_download_url(),
                &mods_path.join(cf.data.get_file_name()),
                None,
            ));
        }

        let mut gen_downloader = T::new();
        gen_downloader.add_objects(files);

        Ok(CurseDownloader {
            gen_downloader,
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

    /// Delegates to the inner downloader's `start`.
    pub async fn start(&mut self) -> Result<()> {
        self.gen_downloader
            .start()
            .await
    }

    /// Returns the number of mods to download.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
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
        self.gen_downloader.len() / num_threads()
    }

    /// Returns how many requests chunks are left.
    #[must_use]
    pub fn requests_left(&self) -> usize {
        let left = &self
            .gen_downloader
            .requests_left();

        if left.is_multiple_of(num_threads()) {
            left / num_threads()
        } else {
            left / num_threads() + 1
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

// TODO: This is repeated in RinthInstaller, maybe put this functions in
// code_functions.rs ?
//
impl<T: FileDownloader> CurseDownloader<T> {
    async fn get_mod_responses(curse_req: &reqwest::Client, files_ids: &[String]) -> Vec<Response> {
        let mut responses: Vec<Response> = Vec::with_capacity(files_ids.len());
        let threads: usize = num_threads();

        for chunk in files_ids.chunks(threads) {
            let mut requests = Vec::with_capacity(chunk.len());
            for url in chunk {
                let task = curse_req.get(url).send();
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
        .start()
        .await?;
    Ok(())
}
