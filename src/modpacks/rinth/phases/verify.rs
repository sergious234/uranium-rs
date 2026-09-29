//! Checks every filtered pack file landed in the instance.

use super::super::installer::{RinthInstallState, file_to_object};
use crate::engine::FileDownloader;
use crate::error::{Result, UraniumError};
use crate::modpacks::rinth::steps::{RinthCtx, RinthStep};

/// Verifies installed files exist; hash checks already ran during download.
///
/// Files skipped at queue time (no download URL or unsafe path) are skipped
/// here too, mirroring [`file_to_object`].
pub(crate) struct VerifyStep;

impl RinthStep for VerifyStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState> {
        // -- Check presence of each expected file
        for file in ctx.files {
            let Some(obj) = file_to_object(file, ctx.instance_dir) else {
                continue;
            };
            if !obj.path.exists() {
                return Err(UraniumError::FileNotFound(obj.path.display().to_string()));
            }
        }
        Ok(RinthInstallState::Completed)
    }
}
