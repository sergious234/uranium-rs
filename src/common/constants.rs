use std::path::PathBuf;
use std::sync::LazyLock;

pub static TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::state_dir()
        .map(|d| d.join("uranium"))
        .unwrap_or_else(|| std::env::temp_dir().join("uranium"))
});

pub const RINTH_JSON: &str = "modrinth.index.json";
pub const CURSE_JSON: &str = "manifest.json";
pub const OVERRIDES_FOLDER: &str = "overrides/";
pub const CLIENT_OVERRIDES_FOLDER: &str = "client-overrides/";
pub const SERVER_OVERRIDES_FOLDER: &str = "server-overrides/";
pub const PROFILES_FILE: &str = "launcher_profiles.json";
pub const MRPACK: &str = "mrpack";
pub const EXECUTABLE_MODE: u32 = 0o766;
