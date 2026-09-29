use mine_data_structs::minecraft::{MinecraftVersions, Root};
use reqwest::Client;

use crate::error::{Result, UraniumError};

const INSTANCES_LIST: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

/// Fetches the full version manifest.
pub async fn list_instances() -> Result<MinecraftVersions> {
    let requester = Client::new();
    let instances = requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?;
    Ok(instances)
}

pub async fn get_last_snapshot() -> Result<String> {
    let requester = Client::new();
    Ok(requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?
        .latest
        .snapshot)
}

pub async fn get_last_release() -> Result<String> {
    let requester = Client::new();
    Ok(requester
        .get(INSTANCES_LIST)
        .send()
        .await?
        .json::<MinecraftVersions>()
        .await?
        .latest
        .release)
}

/// Fetches the `Root` for a specific version id. Used by
/// `MinecraftDownloader::with_downloader`.
pub async fn fetch_root(requester: &Client, minecraft_version: &str) -> Result<Root> {
    let instances = list_instances().await?;
    let instance_url = instances
        .get_instance_url(minecraft_version)
        .ok_or(UraniumError::OtherWithReason(format!(
            "Version {minecraft_version} doesn't exist"
        )))?;
    let root: Root = requester
        .get(instance_url)
        .send()
        .await?
        .json()
        .await?;
    Ok(root)
}
