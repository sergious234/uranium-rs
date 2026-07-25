use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;

pub static TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::state_dir()
        .map(|d| d.join("uranium"))
        .unwrap()
});

/// In case NTHREADS cant be read this value will be returned
pub const DEFAULT_NTHREADS: usize = 8;
pub const RINTH_JSON: &str = "modrinth.index.json";
pub const CURSE_JSON: &str = "manifest.json";
pub const OVERRIDES_FOLDER: &str = "overrides/";
pub const PROFILES_FILE: &str = "launcher_profiles.json";
pub const MRPACK: &str = "mrpack";

pub static NTHREADS: RwLock<usize> = RwLock::new(8);

pub const EXECUTABLE_MODE: u32 = 0o766;
