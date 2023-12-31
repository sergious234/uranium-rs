use crate::{
    code_functions::N_THREADS,
    error::UraniumError,
    variables::constants::{CURSE_JSON, TEMP_DIR},
    zipper::pack_unzipper::unzip_temp_pack,
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
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{functions::overrides, gen_downloader::Downloader};

pub async fn curse_modpack_downloader<I: AsRef<Path>>(
    path: I,
    destination_path: I,
) -> Result<(), UraniumError> {
    unzip_temp_pack(path)?;

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

    // Get the info of each mod to get the url and download it
    let responses: Vec<Response> = get_mod_responses(&curse_req, &files_ids).await;
    let mods_path = destination_path.as_ref().to_path_buf().join("mods/");

    let mut names = Vec::with_capacity(files_ids.len());
    let download_urls = get_download_urls(&curse_req, responses, &mut names).await;

    // All the above code was just for obtaining the download urls
    // and the names.

    let _ = Downloader::new(
        Arc::new(download_urls),
        names,
        Arc::new(mods_path),
        curse_req,
    )
    .start()
    .await;

    overrides(destination_path.as_ref(), "overrides");
    Ok(())
}

async fn get_mod_responses(curse_req: &CurseRequester, files_ids: &[String]) -> Vec<Response> {
    let mut responses: Vec<Response> = Vec::with_capacity(files_ids.len());
    let threads: usize = N_THREADS();

    for chunk in files_ids.chunks(threads) {
        let mut requests = Vec::new();
        for url in chunk {
            let tarea = curse_req.get(url, Method::GET, "");
            requests.push(tarea);
        }
        //pool.push_request_vec(requests);

        let res: Vec<Response> = join_all(requests)
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect();

        // Wait for the current pool to end and append the results
        // to the results vector
        responses.extend(res);
    }

    responses
}

#[allow(unused)]
async fn get_download_urls(
    curse_req: &CurseRequester,
    responses: Vec<Response>,
    names: &mut Vec<PathBuf>,
) -> Vec<String> {
    // In order to get rid of reallocations pre allocate the vector with
    // responses capacity.
    // The vector rarelly will get full beacause of empty links.
    let mut download_urls = Vec::with_capacity(responses.len());

    for response in responses {
        // Parse the response into a CurseResponse<CurseFile>
        let curse_file = response.json::<CurseResponse<CurseFile>>().await;
        if let Ok(file) = curse_file {
            let download_url = file.data.get_download_url();

            // In case the download link its empty, because CurseApi could give
            // a right response but with empty download link... -.-
            if download_url.is_empty() {
                println!(
                    "There is no download link for {}",
                    file.data.get_file_name().display()
                );
            } else {
                names.push(file.data.get_file_name());
                download_urls.push(download_url);
            }
        }
    }
    download_urls
}
