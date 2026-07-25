use crate::variables::constants::{DEFAULT_NTHREADS, NTHREADS};

#[allow(non_snake_case)]
#[allow(unused)]
/// Returns the actual max threads allowed.
pub fn N_THREADS() -> usize {
    match NTHREADS.read() {
        Ok(e) => *e,
        Err(_) => DEFAULT_NTHREADS,
    }
}

use std::{
    fs::{File, create_dir, remove_dir_all},
    path::Path,
};

use log::{error, warn};

use crate::{
    error::{Result, UraniumError},
    variables::constants::TEMP_DIR,
};

#[allow(clippy::borrow_interior_mutable_const)]
pub(crate) fn unzip_temp_pack<I: AsRef<Path>>(file_path: I) -> Result<()> {
    let zip_file = File::open(file_path.as_ref())?;
    let mut zip = zip::ZipArchive::new(zip_file).map_err(|_| UraniumError::WrongFileFormat)?;

    if create_dir(TEMP_DIR.as_path()).is_err() {
        warn!("Could not create temporal dir");
        remove_temp_pack();
        return Err(UraniumError::CantCreateDir("temp_dir"));
    }

    if let Err(e) = zip.extract(TEMP_DIR.as_path()) {
        error!("Error while extracting the modpack");
        return Err(UraniumError::ZipError(e));
    }

    Ok(())
}

pub(crate) fn remove_temp_pack() {
    if remove_dir_all(TEMP_DIR.as_path()).is_err() {
        error!("Error at deleting temp dir");
    }
}
