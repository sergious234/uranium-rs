pub mod minecraft_downloader;

mod updater;
mod curse_downloader;
mod functions;
mod gen_downloader;
mod rinth_downloader;

// Re-export the structs/functions so the user dont have to:
// use uranium::downloaders::rinth_downloader::RinthDownloader
//
// instead can:
// use uranium::downloaders::RinthDownloader

pub use self::curse_downloader::curse_modpack_downloader;
pub use self::rinth_downloader::RinthDownloader;
pub use self::updater::update_modpack;
