//! Core generic downloader.
//! Provides `FileDownloader` trait and concrete generic `Downloader`.

// region:    --- Modules

mod downloader;

pub use downloader::{DownloadState, DownloadableObject, Downloader, FileDownloader, HashType};

// endregion: --- Modules
