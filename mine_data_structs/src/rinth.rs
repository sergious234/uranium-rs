use core::fmt;
use std::path::Path;
use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};

pub enum Attributes {
    Loader,
    Name,
    VersionType,
}

/// `RinthMod` pretends to be the structure for the response of
/// `https://api.modrinth.com/v2/project/{id | slug}`
/// This type is also usable when requesting searches for rinth api
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RinthProject {
    // Required fields
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: String,
    pub server_side: String,
    pub body: String,
    pub status: String,
    pub project_type: String,
    pub downloads: u32,
    pub id: String,
    pub team: String,
    pub updated: String,
    #[serde(default = "Default::default")]
    pub versions: Vec<String>,
    pub icon_url: String,
    // Optional fields
    //TODO!
}

impl fmt::Display for RinthProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Mod name: {}", self.title)
    }
}

/// This struct represent the `dependencies` object from a
/// `https://api.modrinth.com/v2/project/{id|slug}/version` or
/// `https://api.modrinth.com/v2/version/{id}` request.
///
/// ```json
/// "dependencies": [
///     {
///         "version_id": null,
///         "project_id": "P7dR8mSH",
///         "file_name": null,
///         "dependency_type": "required"
///     }
/// ]
/// ```
///
/// Don't confuse this Dependency with modrinth.index.json dependencies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String,
}

impl Dependency {
    pub fn get_project_id(&self) -> &str {
        self.project_id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_default()
    }

    pub fn get_version_id(&self) -> &str {
        match self.version_id {
            Some(ref id) => id,
            None => "",
        }
    }
}

/// `RinthProject` pretends to be the response for:
/// `https://api.modrinth.com/v2/version/{version id}`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RinthVersion {
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub version_type: String,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<RinthFile>,
    pub dependencies: Vec<Dependency>,
}

impl RinthVersion {
    pub fn get_file_url(&self) -> &str {
        &self.files[0].url
    }

    pub fn get_file_name(&self) -> &str {
        &self.files[0].filename
    }

    pub fn get_hashes(&self) -> &Hashes {
        &self.files[0].hashes
    }

    pub fn get_size(&self) -> usize {
        self.files[0].size
    }

    pub fn get_loader(&self) -> &str {
        &self.loaders[0]
    }

    pub fn is_fabric(&self) -> bool {
        self.loaders
            .iter()
            .any(|l| l == "fabric")
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }
}

/// RinthVersions pretends to parse the response of:
/// `https://api.modrinth.com/v2/project/{id | slug}/version`
/// This type is commonly use.
pub type RinthVersions = Vec<RinthVersion>;

/// Simple struct for representing the "hashes" object.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hashes {
    pub sha512: String,
    pub sha1: String,
}

/// This struct represents a file from [project/{id|slug}/version](https://api.modrinth.com/v2/project/BsfnmJP5/version)
/// request to the Modrinth's API.
///
///
/// This is part of a bigger json response:
/// ```json
/// "files": [
///   {
///     "hashes": {
///       "sha1": "fcd985acd9c44830dd542e4cf2b67b79745665c2",
///       "sha512": "a4d777fcb8822ed22d0f26ec9da5ee610e7a1ca7972630555bf3718ed6de73ef36eb613a2bb9f0e6115316e6986220edb96071dea0ce5a19fd6714cd97629968"
///     },
///     "url": "https://cdn.modrinth.com/data/BsfnmJP5/versions/u2NQZcGt/autocrafting-table-mod-1.0.8.jar",
///     "filename": "autocrafting-table-mod-1.0.8.jar",
///     "primary": true,
///     "size": 420424,
///     "file_type": null
///   }
/// ]
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RinthFile {
    pub hashes: Hashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RinthHit {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub client_side: String,
    pub server_side: String,
    pub project_type: String,
    pub downloads: usize,
    pub project_id: String,
    pub author: String,
    pub versions: Vec<String>,
    pub follows: usize,
    pub license: String,
    pub icon_url: Option<String>,
}

/// This struct correspond to [**search** queries](https://api.modrinth.com/v2/search?limit=5&offset=10)
/// to the Modrinth's API
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RinthResponse {
    pub hits: Vec<RinthHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u64,
}

impl RinthResponse {
    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Display for RinthResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (index, minecraft_mod) in self.hits.iter().enumerate() {
            writeln!(f, "{:2}: {:?}", index, minecraft_mod)?;
        }
        write!(f, "")
    }
}

/// This type correspond to [**category** query](https://api.modrinth.com/v2/tag/category)
/// to the Modrinth's API
pub type RinthCategories = Vec<Category>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Category {
    pub icon: String,
    pub name: String,
    pub project_type: String,
    pub header: String,
}

/// This struct represent the modrinth.index.json inside any
/// [Modrinth](https://modrinth.com) modpack.
///
/// ```json
/// 
/// {
///     "formatVersion": 1,
///     "game": "minecraft",
///     "versionId": "6.0.0-beta.5",
///     "name": "Fabulously Optimized",
///     "files": [
///         RinthMdFiles,
///         RinthMdFiles
///         ...
///     ],
///
///     "dependencies": {
///         RinthDep,
///         RinthDep,
///         ...
///     }
/// }
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RinthModpack {
    #[serde(rename = "formatVersion")]
    pub format_version: usize,
    pub game: String,
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub name: PathBuf,
    pub files: Vec<RinthMdFiles>,
}

impl RinthModpack {
    pub fn new() -> RinthModpack {
        RinthModpack {
            format_version: 1,
            game: "minecraft".to_owned(),
            version_id: "0.0.0".to_owned(),
            name: "example".into(),
            files: Vec::new(),
        }
    }

    pub fn get_mods(&self) -> &[RinthMdFiles] {
        &self.files
    }

    pub fn get_mut_mods(&mut self) -> &mut Vec<RinthMdFiles> {
        &mut self.files
    }

    pub fn get_name(&self) -> String {
        self.name
            .display()
            .to_string()
    }

    pub fn get_files(&self) -> &Vec<RinthMdFiles> {
        &self.files
    }

    pub fn add_mod(&mut self, new_mod: RinthMdFiles) {
        self.files.push(new_mod);
    }

    pub fn write_mod_pack_with_name(&self) -> std::io::Result<()>{
        let j = serde_json::to_string_pretty(self)?;
        std::fs::write("modrinth.index.json", j)?;
        Ok(())
    }
}

/// This struct represent a mod inside the modrinth.index.json
/// file.
/// ```json
/// 
/// {
///     "path": "mods/Controlify-2.0.0-beta.14+1.21-fabric.jar",
///     "hashes": {
///          "sha1": "4643968fcdaee38ea921c0f4cc2cc1aebc21d058",
///          "sha512": "very_long_hash"
///      },
///     "env": {
///         "client": "required",
///         "server": "required"
///      },
///     "downloads": [
///         "download_link.com"
///     ],
///     "fileSize": 2791149
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinthMdFiles {
    path: PathBuf,
    hashes: Hashes,
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: usize,
}

impl From<RinthVersion> for RinthMdFiles {
    fn from(version: RinthVersion) -> RinthMdFiles {
        RinthMdFiles {
            path: ("mods/".to_owned() + version.get_file_name()).into(),
            hashes: version.get_hashes().clone(),
            downloads: vec![version
                .get_file_url()
                .to_string()],
            file_size: version.get_size(),
        }
    }
}

impl From<RinthVersionFile> for RinthMdFiles {
    fn from(version: RinthVersionFile) -> Self {
        Self {
            path: ("mods/".to_owned() + &version.name).into(),
            hashes: version.files[0]
                .hashes
                .clone(),
            downloads: vec![version.files[0]
                .url
                .to_string()],
            file_size: version.files[0].size,
        }
    }
}

impl RinthMdFiles {
    pub fn get_download_link(&self) -> &str {
        &self.downloads[0]
    }

    pub fn get_id(&self) -> Option<&str> {
        /*
            So the download link should be something like this:
            https://cdn.modrinth.com/data/DOUdJVEm/versions/QiCZiPOr/Controlify-2.0.0-beta.14%2B1.21-fabric.jar
                                          ^^^^^^^^
            And this code just extract     this     part and return it.
        */

        for download_link in &self.downloads {
            if download_link.contains("modrinth") {
                return download_link
                    .split("data/")
                    .nth(1)
                    .map(|f| &f[0..8]);
            }
        }
        None
    }

    pub fn get_name(&self) -> &str {
        // Oh god, I hate Rust strings.
        self.path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or_default()
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

/// Represents a version file in the Modrinth API.
///
/// Corresponds to [**version_file** request](https://api.modrinth.com/v2/version_file/619e250c133106bacc3e3b560839bd4b324dfda8)
/// version_file/{hash}
///
///
/// This struct models the data for a specific version of a project, including
/// metadata such as the version number, supported Minecraft versions, and
/// download information.
///
/// # Note
///
/// The following fields are deprecated or not included in this struct:
///
/// - `changelog`: Describes changes in this version, either as a string or
///   null.
/// - `status`: The current status of the version (e.g., `"listed"`,
///   `"archived"`).
/// - `requested_status`: The requested status of the version.
/// - `changelog_url`: A deprecated link to the changelog, now always null.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinthVersionFile {
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub version_type: String,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<RinthFile>,

    #[serde(default = "Default::default")]
    pub dependency: Vec<Dependency>,
}

pub fn load_rinth_pack<I: AsRef<Path>>(pack_path: I) -> Option<RinthModpack> {
    read_to_string(&pack_path)
        .map(|s| serde_json::from_str(&s).ok())
        .ok()
        .flatten()
}
