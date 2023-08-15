use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
/// This struct only contains data about the mod logo.
struct Logo {
    id: usize,
    #[serde(rename="modId")]
    mod_id: usize,
    #[serde(rename="thumbnailUrl")]
    thumbnail_url: String,
    url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// This struct contains the data about the specific file of a mod
pub struct CurseFile {
    id: usize,
    #[serde(rename="gameId")]
    game_id: Option<usize>,
    #[serde(rename="modId")]
    mod_id: usize,
    #[serde(rename="displayName")]
    display_name: String,
    #[serde(rename="fileName")]
    file_name: PathBuf,
    #[serde(rename="downloadUrl")]
    download_url: Option<String>,
    #[serde(rename="fileLength")]
    file_length: usize,
    #[serde(rename="gameVersions")]
    game_versions: Vec<String>,
}

impl CurseFile {
    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_game_id(&self) -> usize {
        self.game_id.unwrap_or_default()
    }

    pub fn get_mod_id(&self) -> usize {
        self.mod_id
    }

    pub fn get_game_versions(&self) -> &[String] {
        &self.game_versions
    }

    pub fn get_display_name(&self) -> String {
        self.display_name.clone()
    }

    pub fn get_file_name(&self) -> PathBuf {
        self.file_name.clone()
    }

    pub fn get_download_url(&self) -> String {
        self.download_url.clone().unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FingerPrintInfo {
    id: usize,
    pub file: CurseFile,
}

/// This struct contains the data about the request of a fingerprint
/// Fingerprint requets are like 
/// ```json
/// "data": {
///     exactMatches: [
///         CurseFile
///     ]
/// }
/// ```
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseFingerPrint {
    #[serde(rename="exactMatches")]
    exact_matches: Vec<FingerPrintInfo>,
}

impl CurseFingerPrint {
    pub fn get_file(&self) -> &CurseFile {
        &self.exact_matches[0].file
    }
}

/// This struct contains the data about a single version of a mod
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseVersion {
    id: usize,
    #[serde(rename="gameId")]
    game_id: usize,
    name: String,
    slug: String,
    #[serde(rename="downloadCount")]
    download_counr: usize,
    #[serde(rename="latestFiles")]
    latest_files: Vec<CurseFile>,
}

/// This struct contains the data about the multiple versions of a mod
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseVersions {
    data: Vec<CurseVersion>,
}

/// Because the standar response from Curse API is:
/// "data": {
///     * fields of other struct *
/// }
/// We need this struct.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseResponse<T: Serialize> {
    pub data: T,
}
