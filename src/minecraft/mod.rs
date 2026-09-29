//! Minecraft-specific download and verification.

// region:    --- Modules

pub mod assets;
pub mod client;
pub mod installer;
pub mod libraries;
pub mod profile;
pub mod runtime;
pub mod steps;
pub mod verify;
pub mod version;

pub(crate) use client::{get_index_path, get_lib_path};
pub use installer::{MinecraftDownloadState, MinecraftDownloader};
pub use version::{get_last_release, get_last_snapshot, list_instances};

// endregion: --- Modules
