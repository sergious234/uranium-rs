//! Modrinth (`.mrpack`) installer.
//!
//! Installs a Modrinth modpack, a zip containing `modrinth.index.json` plus
//! optional overrides, into a Minecraft instance. The installer resolves
//! `dependencies` (MC + loader versions), installs vanilla Minecraft, installs
//! the loader when declared (Fabric/Quilt), copies the side-matching overrides
//! (`overrides/` plus `client-overrides/` or `server-overrides/`), then
//! downloads each `files[]` entry passing the `env` filter, with SHA-1
//! verification. Read `installed_profile_id` for the launch target.
//!
//! # Quick start — one-shot
//!
//! ```rust no_run
//! # async fn run() -> uranium_rs::error::Result<()> {
//! use uranium_rs::rinth_pack_download;
//! rinth_pack_download("Fabulously-Optimized.mrpack", "/home/user/.minecraft").await?;
//! # Ok(()) }
//! ```
//!
//! # Incremental progress
//!
//! Poll `progress` to update a progress bar or UI:
//!
//! ```rust no_run
//! # async fn run() -> uranium_rs::error::Result<()> {
//! use uranium_rs::engine::Downloader;
//! use uranium_rs::modpacks::rinth::{RinthInstaller, RinthInstallState};
//! let mut dl = RinthInstaller::<Downloader>::new("pack.mrpack", "/home/user/.minecraft")?;
//! loop {
//!     match dl.progress().await? {
//!         RinthInstallState::Completed => break,
//!         state => println!("{state:?}: {} batches left", dl.requests_left()),
//!     }
//! }
//! # Ok(()) }
//! ```

// region:    --- Modules

mod installer;
pub mod loaders;
pub mod maker;
mod phases;
mod steps;
pub mod updater;

use std::path::Path;

pub use installer::{RinthInstallState, RinthInstaller, rinth_pack_download};
pub use loaders::{
    FabricStep, ForgeStep, LoaderCtx, LoaderInstallStep, LoaderKind, LoaderRequirement,
    NeoForgeStep, QuiltStep,
};
use log::info;
pub use maker::{ModpackMaker, State};
pub use mine_data_structs::rinth::{
    Env, PackMeta, RinthMdFiles, RinthModpack, Side, SideRequirement,
};

use crate::error::Result;

/// Convenience helper to create a mrpack from a directory.
pub async fn make_modpack<I: AsRef<Path>, J: AsRef<Path>>(
    minecraft_path: I,
    modpack_name: J,
) -> Result<()> {
    let mut maker = ModpackMaker::new(&minecraft_path, modpack_name);
    let mut i = 0;
    loop {
        match maker.progress().await {
            Ok(State::Finish) => return Ok(()),
            Err(e) => return Err(e),
            _ => {
                info!("{}", i);
                i += 1;
            }
        }
    }
}

// endregion: --- Modules
