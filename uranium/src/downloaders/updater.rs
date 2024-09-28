use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use mine_data_structs::rinth::RinthVersion;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hashes::rinth_hash;
use crate::searcher::rinth::{SearchBuilder, SearchType};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Content {
    hashes: Vec<String>,
    algorithm: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
}

impl Content {
    pub fn new(hashes: Vec<String>, game_versions: Vec<String>) -> Content {
        Content {
            hashes,
            algorithm: "sha1".to_owned(),
            loaders: vec!["fabric".to_owned()],
            game_versions,
        }
    }
}

pub async fn update_modpack<I: AsRef<Path>>(minecraft_path: I) -> Result<()> {
    let mods_path = PathBuf::from(minecraft_path.as_ref()).join("mods/");
    let mods_names = std::fs::read_dir(&mods_path)?;
    let mods_hashes = mods_names
        .map(|f| rinth_hash(f.unwrap().path().as_path()))
        .collect::<Vec<String>>();

    let updates = get_updates(&mods_hashes).await?;

    for hash in mods_hashes {
        match updates.get(&hash) {
            Some(v) if v.get_hashes().sha1 != hash => {
                println!("Update available for {}", v.name);
            }
            Some(v) => {
                println!("{} is up to date!", v.name);
            }
            None => {}
        }
    }

    Ok(())
    // TODO update!
}

async fn get_updates(mods_hashes: &[String]) -> Result<HashMap<String, RinthVersion>> {
    let client = reqwest::Client::new();
    let post_content = Content::new(mods_hashes.to_owned(), vec!["1.19.2".to_owned()]);
    let url = SearchBuilder::new()
        .search_type(SearchType::VersionFile { hash: "".into() })
        .build_url();
    let response = client
        .post(&url)
        .json(&post_content)
        .send()
        .await?;

    Ok(response
        .json::<HashMap<String, RinthVersion>>()
        .await?)
}
