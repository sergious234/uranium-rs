//! Installs the vanilla Minecraft version the pack depends on.
//!
//! Polls one vanilla batch per call so the installer re-emits
//! `InstallingMinecraft` until vanilla completes. Uses the engine-default
//! downloader; the installer's custom `T` governs mod-file downloads only.

use super::super::installer::RinthInstallState;
use crate::engine::{Downloader, FileDownloader};
use crate::error::Result;
use crate::minecraft::{MinecraftDownloadState, MinecraftDownloader};
use crate::modpacks::rinth::steps::{RinthCtx, RinthStep};

/// Polls the pack's `dependencies.minecraft` install, one batch per call.
pub(crate) struct VanillaStep {
    vanilla: Option<MinecraftDownloader<Downloader>>,
}

impl VanillaStep {
    pub(crate) fn new() -> Self {
        Self { vanilla: None }
    }

    /// Batches still pending in the vanilla queue (0 before init or done).
    pub(crate) fn requests_left(&self) -> usize {
        self.vanilla
            .as_ref()
            .map_or(0, MinecraftDownloader::requests_left)
    }
}

impl RinthStep for VanillaStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState> {
        // -- Skip packs with no MC version
        let Some(version) = ctx
            .meta
            .minecraft_version
            .clone()
        else {
            return Ok(RinthInstallState::CopyingOverrides);
        };

        // -- Init on first poll, then one vanilla batch per call
        let dl = match self.vanilla.as_mut() {
            Some(dl) => dl,
            None => {
                let dl =
                    MinecraftDownloader::<Downloader>::init(ctx.instance_dir, &version).await?;
                self.vanilla.insert(dl)
            }
        };
        match dl.progress().await? {
            MinecraftDownloadState::Completed => Ok(RinthInstallState::CopyingOverrides),
            _ => Ok(RinthInstallState::InstallingMinecraft),
        }
    }
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    use mine_data_structs::rinth::{PackMeta, Side};

    use super::*;
    use crate::config::DownloaderConfig;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn skips_pack_without_minecraft_version() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let requester = reqwest::Client::new();
        let mut dl = Downloader::from_config(DownloaderConfig::default());
        let meta = PackMeta {
            minecraft_version: None,
            loader: None,
        };
        let mut ctx = RinthCtx {
            instance_dir: dir.path(),
            tmp: None,
            meta: &meta,
            files: &[],
            side: Side::Client,
            downloader: &mut dl,
            requester: &requester,
        };
        let mut step = VanillaStep::new();
        assert!(matches!(
            step.run(&mut ctx).await?,
            RinthInstallState::CopyingOverrides
        ));
        assert_eq!(step.requests_left(), 0);
        Ok(())
    }
}

// endregion: --- Tests
