use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use log::info;

use crate::common::constants::EXECUTABLE_MODE;
use crate::engine::{DownloadableObject, FileDownloader, HashType};
use crate::error::{Result, UraniumError};
use crate::minecraft::MinecraftDownloadState;
use crate::minecraft::steps::{InstallCtx, InstallStep};

// region:    --- ClientStep (DownloadingVersion)

pub struct ClientStep;

#[allow(async_fn_in_trait)]
impl InstallStep for ClientStep {
    async fn run<T: FileDownloader + Send + Sync>(
        &mut self,
        ctx: &mut InstallCtx<'_, T>,
    ) -> Result<MinecraftDownloadState> {
        create_version_folder(ctx).await?;
        Ok(MinecraftDownloadState::DownloadingAssets)
    }
}

// endregion: --- ClientStep

// region:    --- Version filesystem helpers
//
async fn create_version_folder<T: FileDownloader + Send + Sync>(
    ctx: &mut InstallCtx<'_, T>,
) -> Result<()> {
    let instance_folder = ctx
        .dot_minecraft_path
        .join("versions")
        .join(&ctx.instance.id);

    info!("Instance folder: {instance_folder:?}");

    if !instance_folder.exists() {
        std::fs::create_dir_all(&instance_folder)?;
    }

    check_client(ctx, &instance_folder).await?;
    check_instance(ctx, &instance_folder)?;
    Ok(())
}

async fn check_client<T: FileDownloader + Send + Sync>(
    ctx: &mut InstallCtx<'_, T>,
    instance_folder: &Path,
) -> Result<()> {
    let client_path = instance_folder.join(ctx.instance.id.clone() + ".jar");
    if !client_path.exists() {
        download_client(ctx, client_path).await?;
    } else {
        std::fs::set_permissions(
            &client_path,
            std::fs::Permissions::from_mode(EXECUTABLE_MODE),
        )?;
    }
    Ok(())
}

async fn download_client<T: FileDownloader + Send + Sync>(
    ctx: &mut InstallCtx<'_, T>,
    client_path: PathBuf,
) -> Result<()> {
    info!("Downloading client!");
    const CLIENT: &str = "client";
    let (url, hash) = ctx
        .instance
        .downloads
        .get(CLIENT)
        .map(|i| (&i.url, i.sha1.to_string()))
        .ok_or(UraniumError::OtherWithReason(
            "Client .jar not found in the minecraft instance".to_owned(),
        ))?;
    let obj = DownloadableObject::new(url, &client_path, Some(HashType::Sha1(hash)));
    ctx.downloader.add_object(obj);
    // Queue only: the DownloadingAssets phase drains per batch.
    // Pre-create the jar so the exec bit survives the later
    // truncate-in-place write (empty files never hash-match, so the
    // entry stays queued).
    std::fs::File::create(&client_path)?;
    Ok(std::fs::set_permissions(
        &client_path,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )?)
}

fn check_instance<T: FileDownloader>(
    ctx: &InstallCtx<'_, T>,
    instance_folder: &Path,
) -> Result<()> {
    let instance_path = instance_folder.join(ctx.instance.id.clone() + ".json");
    if !instance_path.exists() {
        info!("Writing client json!");
        let mut instance_file = std::fs::File::create(instance_path)?;
        let content = serde_json::to_string(ctx.instance)
            .map_err(|e| UraniumError::OtherWithReason(e.to_string()))?;
        instance_file.write_all(content.as_bytes())?;
    }
    Ok(())
}

// endregion: --- Version filesystem helpers

// region:    --- Path helpers

pub fn get_index_path(installation_path: &Path, index_name: &Path) -> PathBuf {
    installation_path
        .join("assets")
        .join("indexes")
        .join(index_name)
}

pub fn get_lib_path(installation_path: &Path, lib_path: &Path) -> PathBuf {
    installation_path
        .join("libraries")
        .join(lib_path)
}

// endregion: --- Path helpers
