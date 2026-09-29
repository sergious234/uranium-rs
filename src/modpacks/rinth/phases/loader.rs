//! Runs the pack's declared mod loader install.

use mine_data_structs::rinth::LoaderKind;

use super::super::installer::RinthInstallState;
use super::super::loaders::{FabricStep, QuiltStep};
use super::super::loaders::{ForgeStep, NeoForgeStep};
use super::super::loaders::{LoaderCtx, LoaderInstallStep};
use crate::engine::FileDownloader;
use crate::error::{Result, UraniumError};
use crate::modpacks::rinth::steps::{RinthCtx, RinthStep};

/// Installs `dependencies`' loader and reports its profile id.
pub(crate) struct LoaderStep {
    profile_id: Option<String>,
}

impl LoaderStep {
    pub(crate) fn new() -> Self {
        Self { profile_id: None }
    }

    pub(crate) fn profile_id(&self) -> Option<&str> {
        self.profile_id.as_deref()
    }
}

impl RinthStep for LoaderStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState> {
        // -- Resolve requirement and MC version
        let req = ctx
            .meta
            .loader
            .as_ref()
            .ok_or_else(|| UraniumError::other("loader phase entered without a loader"))?;
        let mc = ctx
            .meta
            .minecraft_version
            .as_deref()
            .ok_or_else(|| {
                UraniumError::other("pack declares a loader but no minecraft version")
            })?;

        // -- Dispatch to the matching loader
        let mut lctx = LoaderCtx {
            instance_dir: ctx.instance_dir,
            mc_version: mc,
            loader_version: &req.version,
            downloader: &mut *ctx.downloader,
            requester: ctx.requester,
        };
        let id = match req.loader {
            LoaderKind::Fabric => {
                FabricStep
                    .install(&mut lctx)
                    .await?
            }
            LoaderKind::Quilt => {
                QuiltStep
                    .install(&mut lctx)
                    .await?
            }
            LoaderKind::Forge => {
                ForgeStep
                    .install(&mut lctx)
                    .await?
            }
            LoaderKind::NeoForge => {
                NeoForgeStep
                    .install(&mut lctx)
                    .await?
            }
        };
        self.profile_id = Some(id);
        Ok(RinthInstallState::CopyingOverrides)
    }
}
