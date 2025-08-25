//! # Downloaders
//!
//! This module contains the core logic for all file downloading operations.
//! It is designed to be highly modular and extensible, allowing for the
//! integration of various download sources, such as official Minecraft,
//! CurseForge, and Modrinth.
//!
//! The central component is the `FileDownloader` trait, which defines the
//! contract for any object that can handle a download. This allows different
//! downloader implementations to be used interchangeably.
//!
//! The module provides the following specific downloader implementations:
//!
//! * [`CurseDownloader`]: For downloading modpacks from CurseForge.
//! * [`RinthDownloader`]: For downloading modpacks from Modrinth.
//! * [`RuntimeDownloader`]: Specifically for downloading Java runtimes.
//! * [`MinecraftDownloader`]: For handling the complex process of
//!   downloading Minecraft versions, assets, and libraries.
//!
//! ## Examples
//!
//! The following example demonstrates how to use the `MinecraftDownloader`
//! to download a specific version of Minecraft.
//!
//! ```no_run
//! use uranium_rs::downloaders::{FileDownloader, Downloader, MinecraftDownloader, MinecraftDownloadState};
//! use uranium_rs::error::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut minecraft_down = MinecraftDownloader::<Downloader>::init(
//!         "path/to/my/minecraft_instance",
//!         "1.20.1"
//!     ).await?;
//!
//!     loop {
//!         let state = minecraft_down.progress().await?;
//!
//!         match state {
//!             MinecraftDownloadState::Completed => {
//!                 println!("Installation completed!");
//!                 break;
//!             },
//!             _ => {
//!                 println!("Current state: {:?}", state);
//!             }
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub use curse_downloader::CurseDownloader;
pub use gen_downloader::{DownloadState, DownloadableObject, Downloader, FileDownloader, HashType};
pub use minecraft_downloader::{
    MinecraftDownloadState, MinecraftDownloader, get_last_release, get_last_snapshot,
    list_instances,
};
pub use rinth_downloader::RinthDownloader;
pub use runtime_downloader::RuntimeDownloader;
pub use updater::update_modpack;

mod curse_downloader;
mod gen_downloader;
mod minecraft_downloader;
mod rinth_downloader;
mod runtime_downloader;
mod updater;
