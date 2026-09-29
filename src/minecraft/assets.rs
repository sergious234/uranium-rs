use std::path::{Path, PathBuf};

use log::error;
use mine_data_structs::minecraft::{ObjectData, Resources};
use tokio::io::AsyncWriteExt;

use crate::engine::{DownloadableObject, FileDownloader, HashType};
use crate::error::{Result, UraniumError};
use crate::minecraft::MinecraftDownloadState;
use crate::minecraft::steps::{InstallCtx, InstallStep};

pub const ASSETS_PATH: &str = "assets/";
pub const OBJECTS_PATH: &str = "objects";

// region:    --- AssetsStep (GettingSources)

pub struct AssetsStep;

#[allow(async_fn_in_trait)]
impl InstallStep for AssetsStep {
    async fn run<T: FileDownloader + Send + Sync>(
        &mut self,
        ctx: &mut InstallCtx<'_, T>,
    ) -> Result<MinecraftDownloadState> {
        let resources = get_resources(ctx.requester, ctx.instance).await?;

        create_indexes(
            ctx.dot_minecraft_path,
            &ctx.instance.get_index_name(),
            &resources,
        )
        .await?;
        create_assets_folders(ctx.dot_minecraft_path, resources.objects.values())?;

        let objects = get_assets_objects(ctx.dot_minecraft_path, resources);
        ctx.downloader
            .add_objects(objects);

        Ok(MinecraftDownloadState::DownloadingVersion)
    }
}

// endregion: --- AssetsStep

// region:    --- Helpers (moved from installer.rs:541-626)

async fn get_resources(
    requester: &reqwest::Client,
    instance: &mine_data_structs::minecraft::Root,
) -> Result<Resources> {
    let resources: Resources = requester
        .get(&instance.asset_index.url)
        .send()
        .await?
        .json::<Resources>()
        .await?;
    Ok(resources)
}

fn get_assets_objects(
    minecraft_path: &Path,
    resources: Resources,
) -> impl Iterator<Item = DownloadableObject> {
    let base = PathBuf::from(ASSETS_PATH).join(OBJECTS_PATH);
    resources
        .objects
        .into_values()
        .map(move |obj| {
            let url = obj.get_link();
            let path = base
                .join(&obj.hash[..2])
                .join(&obj.hash);
            DownloadableObject::new(
                &url,
                &minecraft_path.join(path),
                Some(HashType::Sha1(obj.hash.to_owned())),
            )
        })
}

async fn create_indexes(
    minecraft_path: &Path,
    index_name: &str,
    resources: &Resources,
) -> Result<()> {
    let indexes_path = minecraft_path
        .join(ASSETS_PATH)
        .join("indexes")
        .join(index_name);
    if let Some(parent) = indexes_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut indexes = tokio::fs::File::create(indexes_path).await?;
    indexes
        .write_all(
            serde_json::to_string(resources)
                .map_err(|err| UraniumError::OtherWithReason(err.to_string()))?
                .as_bytes(),
        )
        .await?;
    // -- Flush + fsync so the index is durable before later steps read it
    indexes.flush().await?;
    indexes.sync_all().await?;
    drop(indexes);
    Ok(())
}

fn create_assets_folders<'a>(
    minecraft_path: &Path,
    objects: impl Iterator<Item = &'a ObjectData>,
) -> Result<()> {
    std::fs::create_dir_all(minecraft_path.join("assets/indexes")).map_err(|err| {
        error!("Cant create assets/indexes: [{err}]");
        UraniumError::CantCreateDir("assets/indexes")
    })?;
    std::fs::create_dir_all(minecraft_path.join("assets/objects")).map_err(|err| {
        error!("Cant create assets/objects: [{err}]");
        UraniumError::CantCreateDir("assets/objects")
    })?;
    let obj_path = minecraft_path
        .join(ASSETS_PATH)
        .join(OBJECTS_PATH);
    for obj in objects {
        if let Some(parent) = obj.get_path().parent() {
            std::fs::create_dir_all(obj_path.join(parent))?;
        }
    }
    Ok(())
}

// endregion: --- Helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn create_indexes_on_fresh_dir() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let resources = Resources {
            objects: Default::default(),
        };
        create_indexes(dir.path(), "17.json", &resources).await?;
        let written = std::fs::read_to_string(
            dir.path()
                .join("assets/indexes/17.json"),
        )?;
        assert_eq!(written, serde_json::to_string(&resources)?);
        Ok(())
    }
}

// endregion: --- Tests
