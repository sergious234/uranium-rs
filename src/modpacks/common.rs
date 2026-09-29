use std::path::Path;

use crate::error::{Result, UraniumError};

/// Ensures `mods`, `resourcepacks`, and `config` directories exist under
/// `destination`.
pub fn ensure_pack_dirs(destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination).map_err(|_| UraniumError::CantCreateDir("instance"))?;
    for dir in ["mods", "resourcepacks", "config"] {
        let p = destination.join(dir);
        if !p.exists() {
            std::fs::create_dir_all(&p).map_err(|_| match dir {
                "mods" => UraniumError::CantCreateDir("mods"),
                "resourcepacks" => UraniumError::CantCreateDir("resourcepacks"),
                "config" => UraniumError::CantCreateDir("config"),
                _ => UraniumError::CantCreateDir("unknown"),
            })?;
        }
    }
    Ok(())
}

/// Trait for pack sources — enables OCP extension without modifying existing
/// code.
///
/// Future sources (e.g., FTB) implement this trait in their own module.
pub trait PackSource {
    fn pack_name(&self) -> &str;
}
