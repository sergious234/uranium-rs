use std::{fs, path::Path};

use log::error;

use crate::error::{Result, UraniumError};
use crate::variables::constants::TEMP_DIR;

pub fn overrides(destination_path: &Path, overrides_folder: &str) -> Result<()> {
    // Copy all the content of overrides into the minecraft root folder
    let options = fs_extra::dir::CopyOptions::new();
    // let mut file_options = fs_extra::file::CopyOptions::new();
    // file_options.overwrite = true;
    let overrides_folder = TEMP_DIR.to_owned() + overrides_folder;

    let entries = match fs::read_dir(&overrides_folder) {
        Ok(e) => e,
        Err(error) => {
            error!("Error reading overrides folder: {}", error);
            return Err(UraniumError::IOError(error));
        }
    };

    // Iter through the override directory and copy the content to
    // Minecraft Root (`destination_path`)
    for file in entries.flatten() {
        // There's no need to panic, ¿Is this a mess?
        // TODO! Check if file_type can actually panic here.
        if file.file_type()?.is_dir() {
            let _ = fs_extra::dir::copy(file.path(), destination_path, &options);
        } else {
            let _ = fs::copy(file.path(), destination_path.join(file.file_name()));
        }
    }

    Ok(())
}
