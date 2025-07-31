#![allow(rustdoc::bare_urls)]
//! Module containing data structs to operate with Rinth API <3.
//!
//! Even tho this crate only contains de data structures (and some QoL
//! functions) of the API the requests and parameters are also explained in each
//! struct.
//!
//! Be aware that Rinth API is in constant change, right now v2 is the
//! production server but at this time they are already working in v3. In any
//! case if something get deprecated or new structs are available/needed I'm
//! sure you already know... PR !!!!!!

// TODO:
// Project type allowed values are: mod, modpack, resourcepack, shader.
// This looks like an enum right ?

use std::collections::HashMap;
use std::path::Path;
use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};

// ===================
// |Projects section |
// ===================

/// Struct which contains all the "hits" for a given search.
///
/// This struct is the one Modrinth will return when asked for
/// projects.
///
/// # API Request
/// ## Search projects
///
/// GET https://api.modrinth.com/v2/search
///
/// ### Parameters
/// - Query parameters
///   - query:  string
///   - facets: string
///  
///   - index:  string  *Allowed values: relevance downloads follows newest
///     updated*
///   - offset: integer
///   - limit:  integer
///
/// # More info
/// `https://docs.modrinth.com/api/operations/searchprojects/`
///
/// # Compatibility
///
/// "Deprecated values below. WILL BE REMOVED V3!"
/// <https://github.com/modrinth/code/blob/b9d90aa6356c88c8d661c04ab84194cf08ea0198/apps/labrinth/src/models/v3/projects.rs>
///
/// Facets, filters and versions seems to be "deprecated" in V3 ??
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchProjects {
    pub hits: Box<[Hit]>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u64,
}

impl SearchProjects {
    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Every hit of [`SearchProjects`] struct.
///
/// This struct contains the information of each hit.
///
/// # Missing fields
///
/// Want them ? PR !!
/// - `thread_id`
/// - `monetization_status`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hit {
    /// The slug of a project, used for vanity URLs.
    /// Regex: ^[\w!@$()`.+,"\-']{3,64}$
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Box<String>,
    /// Allowed values: required optional unsupported unknown
    pub client_side: String,
    /// Allowed values: required optional unsupported unknown
    pub server_side: String,
    /// Allowed values: mod modpack resourcepack shader
    pub project_type: String,
    pub downloads: usize,
    pub icon_url: Option<String>,
    pub color: Option<usize>,
    pub project_id: String,
    pub author: String,
    pub display_categories: Box<[String]>,
    pub versions: Box<[String]>,
    pub follows: usize,
    /// format: ISO-8601
    pub date_created: String,
    /// format: ISO-8601
    pub date_modified: String,
    /// The latest version of minecraft that this project supports.
    pub latest_version: Option<String>,
    pub license: String,
    /// All gallery images attached to the project (urls)
    pub gallery: Box<[String]>,
}

pub enum Attributes {
    Loader,
    Name,
    VersionType,
}

/// A project returned from the API
///
/// # API Request
/// ## Get a project
///
/// GET https://api.modrinth.com/v2/project/{id | slug}
///
/// ### Parameters
/// - id|slug String (Required)
///
/// This type is also usable when requesting searches for rinth api
///
/// # Missing fields
/// - `issues_url`
/// - `source_url`
/// - `wiki_url`
/// - `discord_url`
/// - `donation_url`
/// - `color`
/// - `thread_id`
/// - `monetization_status`
/// - `moderator message`
/// - `approved`
/// - `queued`
/// - `license`
/// - `gallery`
///
/// Want any of them ? PR !!!
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RinthProject {
    /// The slug of a project, used for vanity URLs.
    /// Regex: ^[\w!@$()`.+,"\-']{3,64}$
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Box<[String]>,
    /// Allowed values: required optional unsupported unknown
    pub client_side: String,
    /// Allowed values: required optional unsupported unknown
    pub server_side: String,
    pub body: String,
    /// Allowed values: approved archived rejected draft unlisted processing
    /// withheld scheduled private unknown
    pub status: String,
    /// Allowed values: approved archived unlisted private draft
    pub requested_status: Option<String>,
    /// A list of categories which are searchable but non-primary.
    pub additional_categories: Box<[String]>,
    /// Allowed values: mod modpack resourcepack shader
    pub project_type: String,
    pub downloads: u32,
    pub icon_url: Option<String>,
    pub id: String,
    pub team: String,
    /// The link to the long description of the project. Always null, only kept
    /// for legacy compatibility.
    pub body_url: Option<String>,
    /// format: ISO-8601
    pub published: String,
    /// format: ISO-8601
    pub updated: String,
    pub followers: u32,
    /// A list of the version IDs of the project (will never be empty unless
    /// draft status)
    pub versions: Box<[String]>,
    /// A list of all of the game versions supported by the project
    pub game_versions: Box<[String]>,
    pub loaders: Box<[String]>,
}

/// Fancy name for array of [`RinthProject`]
///
/// This type will be used when asking for multiple projects.
///
/// # API Request
///
/// ## Get multiple projects
///
/// GET https://api.modrinth.com/v2/projects
///
/// ### Parameters
/// - Query parameters
///   - ids String array (Required)
///
/// ## Get a list of random projects
///
/// GET https://api.modrinth.com/v2/projects_random
///
/// ### Parameters
/// - count int (Required)
///
/// # More info:
///
/// <https://docs.modrinth.com/api/operations/getprojects/>
pub type RinthProjects = Box<[RinthProject]>;

/// Struct used when getting all of a project's dependencies.
///
/// This is from v2 API so maybe its removed in the future.
///
/// # API Requests
/// ## Get all of a project's dependencies
///
/// GET https://api.modrinth.com/v2/project/{id|slug}/dependencies
///
/// ### Parameters
/// - id|slug String (Required)
///
/// # More info
/// <https://github.com/modrinth/code/blob/main/apps/labrinth/src/routes/v2/projects.rs>
pub struct Dependencies {
    pub projects: Box<[RinthProject]>,
    pub versions: Box<[DependencyInfo]>,
}

/// A specific version of a project. This is from v2 API so maybe its removed in
/// a future.
///
/// Used in [`Dependencies`] [`ProjectVersions`]
///
/// # API Requests
/// ## Get a version
///
/// GET https://api.modrinth.com/v2/version/{id}
///
/// ### Parameters
/// - id String (Required)
///
/// # Latest version of a project from a hash, loader(s), and game version(s)
///
/// POST https://api.modrinth.com/v2/version_file/{hash}/update
///
/// ### Parameters
/// - Path Parameters
///   - hash String (Required)
///
/// - Query Parameters
///   - algorithm String (Required) Allowed values: sha1 sha512
///
/// ### Request Body
/// - loaders String array (Required)
/// - game_versions String array (Required) example: ["1.18", "1.18.1", ...]
///
/// # Missing fields
/// - `requested_status`
///
/// # Used in
/// <https://docs.modrinth.com/api/operations/getlatestversionfromhash/>
/// <https://docs.modrinth.com/api/operations/getversion/>
///
/// # More info
/// <https://github.com/modrinth/code/blob/main/apps/labrinth/src/models/v2/projects.rs>
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DependencyInfo {
    pub name: String,
    pub version_number: String,

    pub dependencies: Box<[Dependency]>,

    pub game_versions: Box<[String]>,
    // Allowed values: release beta alpha
    pub version_type: String,

    /// A list of loaders this project supports (has a newtype struct)
    pub loaders: Box<[String]>,
    pub featured: bool,
    // Allowed values: listed archived draft unlisted scheduled unknown
    pub status: String,

    pub id: String,
    pub project_id: String,
    pub author_id: String,
    /// format: ISO-8601
    pub date_published: String,
    pub downloads: u32,
    pub changelog: String,
    pub changelog_url: Option<String>,
    /// A list of files available for download for this version.
    pub files: Box<[DependencyFiles]>,
}

/// A dendency which describes what versions are required, break support, or are
/// optional to the version's functionality
///
/// Go look [`DependencyInfo`] for more information, this struct is used there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

/// A single project file, with a url for the file and the file's hash.
///
/// Do you think the description isn't good enough ? I copied it from
/// the original repo: <https://github.com/modrinth/code/blob/main/apps/labrinth/src/models/v3/projects.rs>
///
/// Go look [`DependencyInfo`] for more information, this struct is used there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DependencyFiles {
    pub hashes: std::collections::HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u32,
    /// Allowed values: required-resource-pack optional-resource-pack
    pub file_type: Option<String>,
}

// ===================
// |Versions section |
// ===================

/// Struct which is used when retrieving mods.
///
/// This is from v2 API so maybe its removed in the future.
///
/// This struct is also compatible with the following requests:
///
/// # API Requests
/// ## 1. List project's versions
///
/// GET https://api.modrinth.com/v2/project/{id|slug}/version
///
/// ### Parameters
/// - Path Parameters
///   - id|slug String (Required)
///
/// - Query Parameters
///   - loaders: String array
///   - game_versions: String array
///   - featured: bool
///
///
/// ## 2. Get multiple versions
///
/// GET https://api.modrinth.com/v2/versions
///
/// ### Parameters
/// - ids String array.
///
/// ## 3. Get version from hash
///
/// GET https://api.modrinth.com/v2/version_file/{hash}
///
/// ### Parameters
/// - hash string (Required)
///
/// # More info
/// <https://docs.modrinth.com/api/operations/getprojectversions/>
/// <https://docs.modrinth.com/api/operations/getversions/>
pub struct ProjectVersions {
    pub versions: Box<[DependencyInfo]>,
}

/// Fancy name for `HashMap<String,DependencyInfo>`.
///
/// # API Requests
/// ## Get versions from hashses
///
/// GET https://api.modrinth.com/v2/version_files
///
/// ### Parameters
/// - hashes String array (Required)
/// - algorithm String *Allowed values: sha1 sha512*
///
/// # Used in
/// <https://docs.modrinth.com/api/operations/versionsfromhashes/>
pub type DependencyInfosH = HashMap<String, DependencyInfo>;

// TODO: https://docs.modrinth.com/api/operations/getlatestversionsfromhashes/

// DEPRECATING THIS !!
// VVVVVVVVVVVVVVVVV

// This struct represent the `dependencies` object from a
// `https://api.modrinth.com/v2/project/{id|slug}/version` or
// `https://api.modrinth.com/v2/version/{id}` request.
//
// ```json
// "dependencies": [
//     {
//         "version_id": null,
//         "project_id": "P7dR8mSH",
//         "file_name": null,
//         "dependency_type": "required"
//     }
// ]
// ```
//
// Don't confuse this Dependency with modrinth.index.json dependencies.

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
///       "sha512": "hjdkashk139hdfksajsakjcxjsefwjo283..."
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
        RinthModpack::default()
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

    pub fn write_mod_pack_with_name(&self) -> std::io::Result<()> {
        let j = serde_json::to_string_pretty(self)?;
        std::fs::write("modrinth.index.json", j)?;
        Ok(())
    }
}

impl std::default::Default for RinthModpack {
    fn default() -> Self {
        RinthModpack {
            format_version: 1,
            game: "minecraft".to_owned(),
            version_id: "0.0.0".to_owned(),
            name: "example".into(),
            files: Vec::new(),
        }
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
            downloads: vec![
                version
                    .get_file_url()
                    .to_string(),
            ],
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
            downloads: vec![
                version.files[0]
                    .url
                    .to_string(),
            ],
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

    pub fn get_sha1(&self) -> &str {
        &self.hashes.sha1
    }

    pub fn get_sha512(&self) -> &str {
        &self.hashes.sha512
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
