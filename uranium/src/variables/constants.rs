#![allow(unused)]
use std::sync::RwLock;

pub const EXTENSION: &str = ".mrpack";
pub const TEMP_DIR: &str = "./temp_dir/";
pub const DEFAULT_NTHREADS: usize = 32;
pub const RINTH_JSON: &str = "modrinth.index.json";
pub const CURSE_JSON: &str = "manifest.json";
pub const CONFIG_DIR: &str = "config/";
pub const OVERRIDES_FOLDER: &str = "overrides/";

pub static NTHREADS: RwLock<usize> = RwLock::new(32);


// ERROR MESSAGES
pub const DOWNLOAD_ERROR_MSG: &str = "Error with the download request";
pub const CANT_CREATE_DIR: &str = "Cant create the directory";
