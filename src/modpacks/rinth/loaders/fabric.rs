//! Fabric loader install (fetches the ready profile, queues its libraries).

use mine_data_structs::rinth::LoaderKind;

use super::profile::install_profile;
use super::{LoaderCtx, LoaderInstallStep};
use crate::engine::FileDownloader;
use crate::error::Result;

const FABRIC_META: &str = "https://meta.fabricmc.net";

/// Installs a Fabric loader into an instance.
///
/// Fetches `/profile/json` for the MC + loader versions, converts its
/// libraries, queues their downloads, and writes
/// `versions/<profile-id>/<profile-id>.json` for the launcher.
///
/// # Errors
///
/// This function will return an error if the install fails.
///
/// # Example
///
/// ```rust no_run
/// use uranium_rs::engine::Downloader;
/// use uranium_rs::modpacks::rinth::{FabricStep, LoaderCtx, LoaderInstallStep};
/// # async fn run() -> uranium_rs::error::Result<()> {
/// # let tmp = tempfile::tempdir()?;
/// # let requester = reqwest::Client::new();
/// # let mut dl = Downloader::new();
/// let mut ctx = LoaderCtx {
///     instance_dir: tmp.path(),
///     mc_version: "1.21",
///     loader_version: "0.16.9",
///     downloader: &mut dl,
///     requester: &requester,
/// };
/// let profile = FabricStep.install(&mut ctx).await?;
/// # Ok(()) }
/// ```
pub struct FabricStep;

impl LoaderInstallStep for FabricStep {
    fn loader(&self) -> LoaderKind {
        LoaderKind::Fabric
    }

    async fn install<T: FileDownloader>(&mut self, ctx: &mut LoaderCtx<'_, T>) -> Result<String> {
        let url = format!(
            "{FABRIC_META}/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, ctx.loader_version
        );
        install_profile(ctx, &url).await
    }
}
