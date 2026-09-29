//! Quilt loader install (fetches the ready profile, queues its libraries).

use mine_data_structs::rinth::LoaderKind;

use super::profile::install_profile;
use super::{LoaderCtx, LoaderInstallStep};
use crate::engine::FileDownloader;
use crate::error::Result;

const QUILT_META: &str = "https://meta.quiltmc.org";

/// Installs a Quilt loader into an instance.
///
/// Same pipeline as Fabric: fetches `/profile/json`, converts its libraries,
/// queues their downloads, and writes the profile JSON for the launcher.
///
/// # Errors
///
/// This function will return an error if the install fails.
///
/// # Example
///
/// ```rust no_run
/// use uranium_rs::engine::Downloader;
/// use uranium_rs::modpacks::rinth::{LoaderCtx, LoaderInstallStep, QuiltStep};
/// # async fn run() -> uranium_rs::error::Result<()> {
/// # let tmp = tempfile::tempdir()?;
/// # let requester = reqwest::Client::new();
/// # let mut dl = Downloader::new();
/// let mut ctx = LoaderCtx {
///     instance_dir: tmp.path(),
///     mc_version: "1.21",
///     loader_version: "0.26.3",
///     downloader: &mut dl,
///     requester: &requester,
/// };
/// let profile = QuiltStep.install(&mut ctx).await?;
/// # Ok(()) }
/// ```
pub struct QuiltStep;

impl LoaderInstallStep for QuiltStep {
    fn loader(&self) -> LoaderKind {
        LoaderKind::Quilt
    }

    async fn install<T: FileDownloader>(&mut self, ctx: &mut LoaderCtx<'_, T>) -> Result<String> {
        let url = format!(
            "{QUILT_META}/v3/versions/loader/{}/{}/profile/json",
            ctx.mc_version, ctx.loader_version
        );
        install_profile(ctx, &url).await
    }
}
