use serde::{de::DeserializeOwned, Serialize};

use mine_data_strutcs::{
    rinth::rinth_mods::{RinthProject, RinthResponse, RinthVersion},
    url_maker::maker,
};

#[derive(Clone, Debug)]
pub enum SearchType {
    QUERY(String),
    FOR(u32, u32),
    MOD(String),
    PROJECT(String),
    VERSION(String),
    VERSIONS(String),
    RESOURCEPACKS(u32, u32),
    MODPACKS(u32, u32),
}

pub async fn search(search: SearchType) {
    match search {
        SearchType::QUERY(q) => query(&q).await,
        SearchType::FOR(limit, offset) => search_for(limit, offset).await,
        SearchType::MOD(_) => {
            todo!()
        }
        SearchType::PROJECT(id) => search_project(&id).await,
        SearchType::VERSION(id) => search_version(&id).await,
        SearchType::VERSIONS(id) => search_versions(&id).await,
        SearchType::RESOURCEPACKS(limit, offset) => search_resourcepacks(limit, offset).await,
        SearchType::MODPACKS(limit, offset) => search_modpacks(limit, offset).await,
    }
}

#[allow(unused)]
async fn query(q: &str) {
    let url = maker::ModRinth::querry(q);
    let data = get_data::<RinthResponse>(&url).await;
    write_data(data).await;
}

#[allow(unused)]
async fn get(id: &str) {
    let url = maker::ModRinth::mod_version_by_id(id);
    let version = get_data::<RinthVersion>(&url).await;
    let data = get_data::<Vec<u8>>(&version.get_file_url()).await;
    write_file(&version.get_file_name(), data).await;
}

async fn search_project(id: &str) {
    let url = maker::ModRinth::get_project_by_id(id);
    let data = get_data::<RinthProject>(&url).await;
    write_data(data).await;
}

async fn search_versions(id: &str) {
    let url = maker::ModRinth::mod_version_by_id(id);
    let data = get_data::<RinthVersion>(&url).await;
    write_data(data).await;
}

async fn search_resourcepacks(limit: u32, offset: u32) {
    let url = maker::ModRinth::resourcepacks(limit, offset);
    let data = get_data::<RinthResponse>(&url).await;
    write_data(data).await;
}

async fn search_modpacks(limit: u32, offset: u32) {
    let url = maker::ModRinth::modpacks(limit, offset);
    let data = get_data::<RinthResponse>(&url).await;
    write_data(data).await;
}

async fn search_version(id: &str) {
    let url = maker::ModRinth::mod_version_by_id(id);
    let data = get_data::<RinthVersion>(&url).await;
    write_data(data).await;
}

async fn search_for(limit: u32, offset: u32) {
    let url = maker::ModRinth::search_for(limit, offset);
    let data = get_data::<RinthResponse>(&url).await;
    write_data(data).await;
}

async fn get_data<T: DeserializeOwned>(url: &str) -> T {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();
    response.json::<T>().await.unwrap()
}

async fn write_data<T: Serialize>(data: T) {
    let bytes = serde_json::to_vec(&data).unwrap();
    tokio::fs::write("response.json", bytes).await.unwrap();
}

async fn write_file(file_name: &str, data: Vec<u8>) {
    tokio::fs::write(file_name, data).await.unwrap();
}
