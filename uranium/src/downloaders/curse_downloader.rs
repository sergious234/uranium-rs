use super::{gen_downloader::DownloadState, DownlodableObject};
use crate::{
    code_functions::N_THREADS,
    error::UraniumError,
    variables::constants::{CURSE_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
    FileDownloader,
};
use futures::future::join_all;
use mine_data_strutcs::{
    curse::{curse_modpacks::*, curse_mods::*},
    url_maker::maker::Curse,
};
use requester::{
    mod_searcher::Method,
    requester::request_maker::{CurseRequester, Req},
};
use reqwest::Response;
use std::path::Path;

pub struct CurseDownloader<T: FileDownloader> {
    gen_downloader: T,
    modpack: CursePack,
}

#[allow(unused)]
impl<T: FileDownloader> CurseDownloader<T> {
    pub async fn new<I: AsRef<Path>>(
        modpack_path: I,
        destination: I,
    ) -> Result<Self, UraniumError> {
        let destination = destination.as_ref();
        Self::check_mods_dir(&destination)?;
        Self::check_rp_dir(&destination)?;
        Self::check_config_dir(&destination)?;

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

        let curse_req = CurseRequester::new();

        let responses: Vec<Response> = Self::get_mod_responses(&curse_req, &files_ids).await;
        let mut files = Vec::with_capacity(responses.len());
        let mods_path = destination.join("mods/");

        for response in responses {
            let cf = response.json::<CurseResponse<CurseFile>>().await.unwrap();
            files.push(DownlodableObject::new(
                &cf.data.get_download_url(),
                cf.data.get_file_name().to_str().unwrap_or_default(),
                &mods_path,
                None,
            ));
        }

        Ok(CurseDownloader {
            gen_downloader: T::new(files),
            modpack: curse_pack,
        })
    }

    pub async fn progress(&mut self) -> Result<DownloadState, UraniumError> {
        self.gen_downloader.progress().await
    }

    pub async fn complete(&mut self) -> Result<(), UraniumError> {
        self.gen_downloader.complete().await
    }

    /// Returns the number of mods to download.
    #[must_use]
    pub fn len(&self) -> usize {
        self.gen_downloader.requests_left()
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
        self.gen_downloader.requests_left() / N_THREADS()
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
    pub fn get_modpack_name(&self) -> &str {
        &self.modpack.name
    }
}

// TODO: This is repeated in RinthDownloader, maybe put this functions in
// code_functions.rs ?
impl<T: FileDownloader> CurseDownloader<T> {
    async fn get_mod_responses(curse_req: &CurseRequester, files_ids: &[String]) -> Vec<Response> {
        let mut responses: Vec<Response> = Vec::with_capacity(files_ids.len());
        let threads: usize = N_THREADS();

        for chunk in files_ids.chunks(threads) {
            let mut requests = Vec::new();
            for url in chunk {
                let tarea = curse_req.get(url, Method::GET, "").send();
                requests.push(tarea);
            }
            //pool.push_request_vec(requests);

            let res: Vec<Response> = join_all(requests).await.into_iter().flatten().collect();

            // Wait for the current pool to end and append the results
            // to the results vector
            responses.extend(res);
        }

        responses
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
