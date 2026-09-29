//! Modpack sources — each platform lives in its own sub-module.
//! Adding a new source (e.g., `ftb`) means creating `src/modpacks/ftb/` and
//! registering it here — no edits to `rinth` or `curse` (OCP).

// region:    --- Modules

pub mod common;
pub mod curse;
pub mod rinth;

pub use common::ensure_pack_dirs;
// Re-exports for ergonomic access
pub use curse::CurseDownloader;
pub use rinth::{RinthInstaller, updater};

// endregion: --- Modules
