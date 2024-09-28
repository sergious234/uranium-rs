use reqwest::header::InvalidHeaderValue;
use thiserror::Error;
use tokio::task::JoinError;

use crate::downloaders::DownloadableObject;

pub type Result<T> = std::result::Result<T, UraniumError>;

#[derive(Debug, Error)]
pub enum UraniumError {
    #[error("Wrong file format")]
    WrongFileFormat,
    #[error("Wrong modpack format")]
    WrongModpackFormat,
    #[error("File `{0}` not found")]
    FileNotFound(String),
    #[error("Can't create dir: `{0}`")]
    CantCreateDir(&'static str),
    #[error("Error while writing the files: `{0}`")]
    WriteError(std::io::Error),
    #[error("IO Error: `{0}`")]
    IOError(std::io::Error),
    #[error("Error downloading files")]
    DownloadError,
    #[error("Error making the requests: `{0}`")]
    RequestError(reqwest::Error),
    #[error("File hash doesnt match")]
    FileNotMatch(DownloadableObject),
    #[error("Files hashes doesnt match")]
    FilesDontMatch(Vec<DownloadableObject>),
    #[error("Zip Error: `{0}`")]
    ZipError(zip::result::ZipError),
    #[error("Can't compress the modpack")]
    CantCompress,
    #[error("Can't remove temp JSON file")]
    CantRemoveJSON,
    #[error("Can't read mods dir")]
    CantReadModsDir,
    #[error("Error in async task")]
    AsyncRuntimeError,
    #[error("Error")]
    Other,
    #[error("Error: `{0}`")]
    OtherWithReason(String),
}

impl From<reqwest::Error> for UraniumError {
    fn from(value: reqwest::Error) -> Self {
        UraniumError::RequestError(value)
    }
}

impl From<InvalidHeaderValue> for UraniumError {
    fn from(_value: InvalidHeaderValue) -> Self {
        UraniumError::Other
    }
}

impl From<std::io::Error> for UraniumError {
    fn from(value: std::io::Error) -> Self {
        type Ioe = std::io::ErrorKind;
        match value.kind() {
            Ioe::PermissionDenied | Ioe::NotFound => UraniumError::WriteError(value),
            _ => UraniumError::IOError(value),
        }
    }
}

impl From<zip::result::ZipError> for UraniumError {
    fn from(value: zip::result::ZipError) -> Self {
        UraniumError::ZipError(value)
    }
}

impl From<JoinError> for UraniumError {
    // TODO!: Add more variants ?.? ?
    fn from(_value: JoinError) -> Self {
        Self::AsyncRuntimeError
    }
}
