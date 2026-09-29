use std::{
    fs::{File, create_dir, remove_dir_all},
    path::Path,
};

use log::{error, warn};

use crate::{
    common::constants::TEMP_DIR,
    error::{Result, UraniumError},
};

pub fn unzip_temp_pack<I: AsRef<Path>>(file_path: I) -> Result<()> {
    unzip_temp_pack_at(file_path.as_ref(), &TEMP_DIR)
}

pub fn remove_temp_pack() {
    if remove_dir_all(TEMP_DIR.as_path()).is_err() {
        error!("Error at deleting temp dir");
    }
}

/// Extracts `zip_path` into `tmp` directory (DI variant — injectable `tmp` for
/// tests). The global `unzip_temp_pack` delegates to this with `TEMP_DIR`.
pub fn unzip_temp_pack_at(zip_path: &Path, tmp: &Path) -> Result<()> {
    let zip_file = File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(zip_file).map_err(|_| UraniumError::WrongFileFormat)?;

    if create_dir(tmp).is_err() {
        warn!("Could not create temporal dir");
        // try to clean stale tmp and signal retry
        if remove_dir_all(tmp).is_err() {
            error!("Error at deleting temp dir");
        }
        return Err(UraniumError::CantCreateDir("temp_dir"));
    }

    if let Err(e) = zip.extract(tmp) {
        error!("Error while extracting the modpack");
        return Err(UraniumError::ZipError(e));
    }

    Ok(())
}
