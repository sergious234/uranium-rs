//! Mod-loader requirements and installation.
//!
//! Detection types live in `mine_data_structs`; each loader owns its vendor
//! API shape in its own file (OCP). Fabric and Quilt install from their
//! `/profile/json` endpoints; Forge/NeoForge are stubbed (installer
//! processors need a JVM at install time — a different problem class).

// region:    --- Modules

mod fabric;
mod forge;
mod neoforge;
mod profile;
mod quilt;

use std::path::Path;

pub use fabric::FabricStep;
pub use forge::ForgeStep;
pub use mine_data_structs::rinth::{LoaderKind, LoaderRequirement};
pub use neoforge::NeoForgeStep;
pub use quilt::QuiltStep;

use crate::engine::FileDownloader;
use crate::error::Result;

// endregion: --- Modules

// region:    --- Install seam

/// Borrowed data for a loader install.
pub struct LoaderCtx<'a, T: FileDownloader> {
    pub instance_dir: &'a Path,
    pub mc_version: &'a str,
    pub loader_version: &'a str,
    pub downloader: &'a mut T,
    pub requester: &'a reqwest::Client,
}

/// Installs one mod loader into an instance.
///
/// Returns the installed profile id (e.g. `fabric-loader-0.16.9-1.21`),
/// which is what the launcher must launch.
#[allow(async_fn_in_trait)]
pub trait LoaderInstallStep {
    fn loader(&self) -> LoaderKind;
    async fn install<T: FileDownloader>(&mut self, ctx: &mut LoaderCtx<'_, T>) -> Result<String>;
}

// endregion: --- Install seam
