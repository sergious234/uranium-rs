use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use zip::{write::FileOptions, ZipWriter};

use crate::{
    error::ZipError,
    variables::constants::{self, CONFIG_DIR, OVERRIDES_FOLDER},
};

use super::uranium_structs::UraniumFile;
use crate::zipper::uranium_structs::FileType;
use log::{error, info, warn};

/// Compresses a Minecraft modpack into a ZIP archive.
///
/// This function takes the name of the output ZIP archive, the path to the 
/// modpack files, and a list of raw mods as input.
///
/// It creates a ZIP archive with the specified name and adds the modpack's 
/// files and configurations to it. Additionally, it can include raw mods in the archive.
///
/// # Arguments
///
/// * `name` - A string representing the name of the output ZIP archive.
/// It should include the file extension.
///
/// * `path` - A [`Path`](std::path::Path) representing the path to the modpack 
/// files to be compressed.
///
/// * `raw_mods` - A slice of types that implement [`AsRef<Path>`](std::path::AsRef) 
/// representing the filenames of raw mods to include in the archive.
///
/// # Errors
///
/// This function can return an error of type `ZipError` in the following cases:
///
/// - If there is an error while creating or writing to the ZIP archive.
///
pub fn compress_pack<P: AsRef<Path>>(
    name: &str,
    path: &Path,
    raw_mods: &[P],
) -> Result<(), ZipError> {
    let zip_file = File::create(name.to_owned() + constants::EXTENSION)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.add_directory(OVERRIDES_FOLDER, options)?;

    zip.add_directory(
        PathBuf::from(OVERRIDES_FOLDER)
            .join(CONFIG_DIR)
            .as_os_str()
            .to_str()
            .unwrap_or_default(),
        options,
    )?;

    let mut config_files: Vec<UraniumFile> = Vec::new();

    // Iter through all the files and sub-directories in "config/" and set the file type.
    search_files(path, &PathBuf::from(CONFIG_DIR), &mut config_files)?;

    add_files_to_zip(path, &mut config_files, &mut zip, options)?;

    // Add the modpack_temp.json file
    let modpack_json = File::open(constants::RINTH_JSON).unwrap();
    let modpack_bytes = modpack_json.bytes().flatten().collect::<Vec<u8>>();

    // Add the hardcoded .jar mods
    add_raw_mods(path, &mut zip, raw_mods, options)?;

    // Finally add the modpack.json file
    zip.start_file(constants::RINTH_JSON, options)?;
    zip.write_all(&modpack_bytes)?;
    zip.finish()?;

    Ok(())
}

fn search_files(
    minecraft_path: &Path,
    relative_path: &Path,
    config_files: &mut Vec<UraniumFile>,
) -> Result<(), ZipError> {
    // Get this directory files
    let sub_config_files = get_new_files(
        minecraft_path.to_owned().join(relative_path).as_path(),
        relative_path,
    )?;

    // Go through the sub_config_files vector and set the right tipe to each file. Then add them to config_files
    for mut config_file in sub_config_files {
        let path: PathBuf = minecraft_path
            .to_owned()
            .join(&config_file.get_absolute_path());

        if Path::is_file(&path) {
            config_file.set_type(FileType::Data);
            config_files.push(config_file.clone());
        } else {
            config_file.set_type(FileType::Dir);
            config_files.push(config_file.clone());
            let new_path = relative_path.join(config_file.get_name());
            search_files(minecraft_path, &new_path, config_files)?;
        }
    }

    Ok(())
}

fn get_new_files(path: &Path, relative_path: &Path) -> Result<Vec<UraniumFile>, ZipError> {
    let sub_directory = match std::fs::read_dir(path) {
        Ok(dir) => dir,
        Err(e) => {
            error!("Error al leer {:?}: {}", path, e);
            return Err(ZipError::CantReadDir);
        }
    };

    let sub_config_files: Vec<UraniumFile> = sub_directory
        .map(|file| {
            UraniumFile::new(
                relative_path,
                file.unwrap().file_name().to_str().unwrap(),
                FileType::Other,
            )
        })
        .collect();
    Ok(sub_config_files)
}

fn add_files_to_zip(
    minecraft_path: &Path,
    config_files: &mut Vec<UraniumFile>,
    zip: &mut ZipWriter<File>,
    options: FileOptions,
) -> Result<(), ZipError> {
    for file in config_files {
        match_file(minecraft_path, zip, options, file)?;
    }
    Ok(())
}

fn match_file(
    root_path: &Path,
    zip: &mut ZipWriter<File>,
    options: FileOptions,
    file: &mut UraniumFile,
) -> Result<(), ZipError> {
    let overrides: PathBuf = PathBuf::from("overrides/");
    match file.get_type() {
        FileType::Data => {
            let absolute_path = root_path.to_owned().join(file.get_absolute_path());
            let rel_path = overrides.join(file.get_absolute_path());
            append_config_file(&absolute_path, &rel_path, zip, options)?;
        }

        FileType::Dir => {
            zip.add_directory(
                "overrides/".to_owned() + &file.get_path() + &file.get_name(),
                options,
            )?;
        }

        FileType::Other => {}
    };

    Ok(())
}

fn append_config_file(
    absolute_path: &PathBuf,
    rel_path: &Path,
    zip: &mut ZipWriter<File>,
    option: FileOptions,
) -> Result<(), ZipError> {
    // Read the file
    let file = match File::open(absolute_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Unable to open {:?}: {}", absolute_path, e);
            return Err(ZipError::IoError(e));
        }
    };

    let buffer = file.bytes().flatten().collect::<Vec<u8>>();

    // Is a recoverable error reading 0 bytes from file ?
    // In this case Uranium will just send a warning about it
    // and dont add the file
    if buffer.is_empty() {
        warn!("No bytes readed from the pack");
        return Ok(());
    }

    // Add the file to the zip
    let _ = zip.start_file(rel_path.as_os_str().to_str().unwrap_or_default(), option);
    let _ = zip.write_all(&buffer);
    Ok(())
}

fn add_raw_mods<P: AsRef<Path>>(
    path: &Path,
    zip: &mut ZipWriter<File>,
    raw_mods: &[P],
    options: FileOptions,
) -> Result<(), ZipError> {
    zip.add_directory("overrides/mods", options)?;

    for jar_file in raw_mods {
        let file_name = PathBuf::from("overrides/mods/").join(jar_file);

        info!("Adding {:?}", &file_name);

        println!(
            "{}",
            (path.join("mods/").join(jar_file))
                .as_os_str()
                .to_str()
                .unwrap_or_default()
        );

        let jar_path = path.join("mods/").join(jar_file);
        let buffer = match std::fs::read(&jar_path) {
            Ok(data) => data,
            Err(e) => {
                error!("Error reading {:?}: {}", jar_path, e);
                panic!();
            }
        };

        let _ = zip.start_file(file_name.as_os_str().to_str().unwrap_or_default(), options);
        let _ = zip.write_all(&buffer);
    }
    Ok(())
}
