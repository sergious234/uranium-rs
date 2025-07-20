use std::{
    fs::{File, create_dir, remove_dir_all},
    path::Path,
};

use log::{error, warn};

use crate::{
    error::{Result, UraniumError},
    variables::constants::TEMP_DIR,
};

pub fn unzip_temp_pack<I: AsRef<Path>>(file_path: I) -> Result<()> {
    let zip_file = match File::open(file_path.as_ref()) {
        Ok(file) => file,
        Err(e) => {
            let path = file_path
                .as_ref()
                .as_os_str()
                .to_str()
                .unwrap();
            warn!("Error trying to open the zip file!: {}", e);
            return Err(UraniumError::FileNotFound(path.to_string()));
        }
    };

    let mut zip = zip::ZipArchive::new(zip_file).map_err(|_| UraniumError::WrongFileFormat)?;

    if create_dir(TEMP_DIR).is_err() {
        error!("Could not create temporal dir");
        remove_temp_pack();
        return Err(UraniumError::CantCreateDir("temp_dir"));
    }

    if let Err(e) = zip.extract(TEMP_DIR) {
        error!("Error while extracting the modpack");
        return Err(UraniumError::ZipError(e));
    }

    Ok(())
}

pub(crate) fn remove_temp_pack() {
    if remove_dir_all(TEMP_DIR).is_err() {
        error!("Error at deleting temp dir");
    }
}
