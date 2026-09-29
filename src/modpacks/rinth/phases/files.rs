//! Drives the mod-file download queue to completion.

use super::super::installer::RinthInstallState;
use crate::engine::{DownloadState, FileDownloader};
use crate::error::Result;
use crate::modpacks::rinth::steps::{RinthCtx, RinthStep};

/// Polls the file downloader; reports `Verifying` once the queue drains.
pub(crate) struct FilesStep;

impl RinthStep for FilesStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState> {
        match ctx
            .downloader
            .progress()
            .await?
        {
            DownloadState::Completed => Ok(RinthInstallState::Verifying),
            _ => Ok(RinthInstallState::DownloadingFiles),
        }
    }
}
