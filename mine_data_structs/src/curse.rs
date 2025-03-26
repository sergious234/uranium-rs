use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Error;

// Modpacks

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CursePackFiles {
    #[serde(rename = "projectID")]
    project_id: usize,
    #[serde(rename = "fileID")]
    file_id: usize,
}

impl CursePackFiles {
    pub fn get_project_id(&self) -> usize {
        self.project_id
    }

    pub fn get_file_id(&self) -> usize {
        self.file_id
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CursePack {
    pub name: String,
    pub author: String,
    files: Vec<CursePackFiles>,
}

impl CursePack {
    pub fn get_files(&self) -> &Vec<CursePackFiles> {
        &self.files
    }
}

fn deserializ_pack(path: &str) -> Result<CursePack, Error> {
    let aux = read_to_string(path).unwrap();
    serde_json::from_str(&aux)
}

pub fn load_curse_pack(pack_path: &str) -> Option<CursePack> {
    match read_to_string(pack_path) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("Error reading the pack \n\n{error}");
            return None;
        }
    };

    match deserializ_pack(pack_path) {
        Ok(e) => Some(e),
        Err(error) => {
            eprintln!("Error deserializing the pack \n\n{error}");
            None
        }
    }
}

// Mods

#[derive(Deserialize, Serialize, Clone, Debug)]
/// This struct only contains data about the mod logo.
pub struct Logo {
    id: usize,
    #[serde(rename = "modId")]
    mod_id: usize,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: String,
    url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// This struct contains the data about the specific file of a mod
pub struct CurseFile {
    id: usize,
    #[serde(rename = "gameId")]
    game_id: Option<usize>,
    #[serde(rename = "modId")]
    mod_id: usize,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "fileName")]
    file_name: PathBuf,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(rename = "fileLength")]
    file_length: usize,
    #[serde(rename = "gameVersions")]
    game_versions: Vec<String>,
}

impl CurseFile {
    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_game_id(&self) -> usize {
        self.game_id
            .unwrap_or_default()
    }

    pub fn get_mod_id(&self) -> usize {
        self.mod_id
    }

    pub fn get_game_versions(&self) -> &[String] {
        &self.game_versions
    }

    pub fn get_display_name(&self) -> &str {
        &self.display_name
    }

    pub fn get_file_name(&self) -> &Path {
        &self.file_name
    }

    pub fn get_download_url(&self) -> &str {
        self.download_url
            .as_ref()
            .map_or("", |s| s)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FingerPrintInfo {
    pub id: usize,
    pub file: CurseFile,
}

/// This struct contains the data about the request of a fingerprint
/// requests are like
/// ```json
/// "data": {
///     exactMatches: [
///         CurseFile
///     ]
/// }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseFingerPrint {
    #[serde(rename = "exactMatches")]
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
    #[serde(rename = "gameId")]
    game_id: usize,
    name: String,
    slug: String,
    #[serde(rename = "downloadCount")]
    download_count: usize,
    #[serde(rename = "latestFiles")]
    latest_files: Vec<CurseFile>,
}

/// This struct contains the data about the multiple versions of a mod
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseVersions {
    data: Vec<CurseVersion>,
}

/// Because the standard response from Curse API is:
/// "data": {
///     * fields of other struct *
/// }
/// We need this struct.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurseResponse<T: Serialize> {
    pub data: T,
}
