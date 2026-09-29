use std::path::Path;

use mine_data_structs::minecraft::Root;

use crate::engine::FileDownloader;
use crate::error::Result;
use crate::minecraft::MinecraftDownloadState;

/// Context shared across installation steps.
/// Holds borrows to the facade's owned data to avoid cloning.
pub struct InstallCtx<'a, T: FileDownloader> {
    pub dot_minecraft_path: &'a Path,
    pub instance: &'a Root,
    pub requester: &'a reqwest::Client,
    pub downloader: &'a mut T,
}

/// Trait for a single installation phase (SRP).
/// Each phase file implements this to keep `installer.rs` a thin
/// Facade/dispatcher. Adding a new phase = new file + one `match` arm in
/// `installer.rs` (OCP).
#[allow(async_fn_in_trait)]
pub trait InstallStep {
    async fn run<T: FileDownloader + Send + Sync>(
        &mut self,
        ctx: &mut InstallCtx<'_, T>,
    ) -> Result<MinecraftDownloadState>;
}
