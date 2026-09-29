//! Shared context and step trait for `.mrpack` install phases.
//!
//! Mirrors `crate::minecraft::steps`: the facade stays thin and each phase
//! lives in its own file (OCP — a new phase is a new file plus one `match`
//! arm in the installer).

use std::path::Path;

use mine_data_structs::rinth::{PackMeta, RinthMdFiles, Side};

use super::installer::RinthInstallState;
use crate::engine::FileDownloader;
use crate::error::Result;

/// Borrowed data shared across install phases.
pub struct RinthCtx<'a, T: FileDownloader> {
    pub instance_dir: &'a Path,
    pub tmp: Option<&'a Path>,
    pub meta: &'a PackMeta,
    pub files: &'a [RinthMdFiles],
    pub side: Side,
    pub downloader: &'a mut T,
    pub requester: &'a reqwest::Client,
}

/// One install phase (SRP).
#[allow(async_fn_in_trait)]
pub trait RinthStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState>;
}
