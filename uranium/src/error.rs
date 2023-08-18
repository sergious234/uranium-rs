use thiserror::Error;


#[derive(Debug, Error)]
pub enum UraniumError {
    #[error("Wrong file format")]
    WrongFileFormat,
    #[error("Wrong modpack format")]
    WrongModpackFormat,
    #[error("File not found")]
    FileNotFound,
    #[error("Cant create dir")]
    CantCreateDir,
    #[error("Error while writting the files")]
    WriteError(std::io::Error),
    #[error("Error downloading files")]
    DownloadError,
    #[error("Error making the requests")]
    RequestError,
    #[error("File hash doesnt match")]
    FileNotMatch,
}

impl std::convert::From<std::io::Error> for UraniumError {
    fn from(value: std::io::Error) -> Self {
        UraniumError::WriteError(value)
    }
}

#[derive(Debug, Error)]
pub enum MakerError {
    #[error("Cant compress the modpack")]
    CantCompress,
    #[error("Cant remove temp JSON file")]
    CantRemoveJSON,
    #[error("Cant read mods dir")]
    CantReadModsDir,
}

#[derive(Debug, Error)]
pub enum ZipError {
    #[error("Cant read dir")]
    CantReadDir,
    #[error("Zip Error")]
    ZipError(zip::result::ZipError),
    #[error("Io Error")]
    IoError(std::io::Error),
}

impl std::convert::From<std::io::Error> for ZipError {
    fn from(e: std::io::Error) -> ZipError {
        ZipError::IoError(e)
    }
}

impl std::convert::From<zip::result::ZipError> for ZipError {
    fn from(e: zip::result::ZipError) -> ZipError {
        ZipError::ZipError(e)
    }
}
