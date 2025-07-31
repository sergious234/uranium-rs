//! This module contains all relevant data structs for downloading/launching
//! minecraft.
//!
//! Also some QoL getters are provided for `strings` which you may get
//! by modifying fields of the desired struct.
//!
//!
//! Data structures for parsing launcher_profiles.json are also provided
//! with a variaty of methods such as: `ProfilesJson::read_json_from`,
//! `ProfilesJson::insert`, etc.
//!
//! A funcion for getting minecraft path is also included
//! [`get_minecraft_path`], it supports windows and linux systems for now.
//!
//! For some info about java-runtimes go to [Runtimes]. What a surprise eh ?
//!
//! For info about downloading a new minecraft version go look [Root], also
//! for launching minecraft.
//!
//! For info about minecraft versions (snapshots, releases, lastest versions...)
//! go look [MinecraftVersions].

use std::io::Write;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

const BASE: &str = "https://resources.download.minecraft.net/";

/*

            MINECRAFT ASSETS DATA STRUCTURES

*/

/// Very simple struct, no need for explanation.
///
/// This is used in [Resources].
///
/// From piston-meta.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectData {
    pub hash: String,
    pub size: usize,
}

impl ObjectData {
    pub fn get_link(&self) -> String {
        format!("{BASE}{}/{}", &self.hash[..2], &self.hash)
    }

    /// Returns the actual path:
    /// PathBuf::from(&self.hash[..2]).join(&self.hash)
    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(&self.hash[..2]).join(&self.hash)
    }
}

/// Struct which represent data from `Root::downloads` field.
///
/// Look [Root] for more information.
///
/// From piston-meta.
///
/// Yep, this is the SAME EXACT STRUCT AS [Manifest]. But for semantic
/// reasons I'll keep both structs.
///
/// Maybe I should just make an alias right... ? PR !!!
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadData {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

/// Represent the JSON from piston-meta which contains the assets.
///
/// If you come from [AssetIndex] congrats! Otherwise go look at it
/// for a better understanding of this struct.
///
/// This is the fecthed answer from the link in [AssetIndex]
///
/// This looks like:
///
/// ```json
/// {
///  "objects": {
///    "icons/icon_128x128.png": {
///      "hash": "b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356",
///      "size": 9101
///    },
///    "icons/icon_16x16.png": {
///      "hash": "5ff04807c356f1beed0b86ccf659b44b9983e3fa",
///      "size": 781
///    }
/// }
/// ```

#[derive(Serialize, Deserialize, Debug)]
pub struct Resources {
    pub objects: IndexMap<String, ObjectData>,
}

/*

       https://launchermeta.mojang.com/mc/game/version_manifest.json
                  MINECRAFT INSTANCES DATA STRUCTURE

*/

/// A Minecraft version from launchermeta.mojang.com
///
/// Example:
/// ```json
/// {
///  "id": "24w39a",
///  "type": "snapshot",
///  "url": "https://piston-meta.mojang.com/v1/packages/2b6f8eddb01877162fd33b3bbb25f569b7582ee9/24w39a.json",
///  "time": "2024-09-25T13:19:16+00:00",
///  "releaseTime": "2024-09-25T13:08:41+00:00"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub instance_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// The whole JSON from launchermeta.mojang.com with `latest` and `versions`.
///
/// This struct represents the whole `JSON` found in:
///
/// "<https://launchermeta.mojang.com/mc/game/version_manifest.json>"
#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftVersions {
    pub latest: Latest,
    pub versions: Box<[MinecraftVersion]>,
}

impl MinecraftVersions {
    pub fn get_latest_release_id(&self) -> &str {
        &self.latest.release
    }

    pub fn get_latest_snapshot_id(&self) -> &str {
        &self.latest.snapshot
    }

    pub fn get_latest_release_url(&self) -> &str {
        // This unwrap is safe since the version we are looking for
        // will always exists.
        &self
            .versions
            .iter()
            .find(|v| v.id == self.get_latest_release_id())
            .unwrap()
            .url
    }

    pub fn get_latest_snapshot_url(&self) -> &str {
        // This unwrap is safe since the version we are looking for
        // will always exists.
        &self
            .versions
            .iter()
            .find(|v| v.id == self.get_latest_snapshot_id())
            .unwrap()
            .url
    }

    /// Returns the url which correspond to the given version. In case the
    /// version doesn't exist `None` will be returned.
    ///
    /// <https://piston-meta.mojang.com/v1/packages/f0025b5b37c7efcf50807fc24b5fc7ef7ab18ea5/1.21.4.json>
    pub fn get_instance_url(&self, instance: &str) -> Option<&str> {
        for version in &self.versions {
            if version.id == instance {
                return Some(&version.url);
            }
        }
        None
    }
}

/// Both: release and snapshot latest versions of minecraft.
///
/// From launchermeta.mojang.com
#[derive(Serialize, Deserialize, Debug)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

/// A Library from a piston-meta version.
///
/// This struct represents the folowing `JSON` from piston-meta:
///
///```json
/// {
/// "downloads": {
///   "artifact": {
///     "path": "org/lwjgl/lwjgl-jemalloc/3.3.3/lwjgl-jemalloc-3.3.3-natives-macos.jar",
///     "sha1": "2906637657a57579847238c9c72d2c4bde7083f8",
///     "size": 153131,
///     "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl-jemalloc/3.3.3/lwjgl-jemalloc-3.3.3-natives-macos.jar"
///   }
/// },
/// "name": "org.lwjgl:lwjgl-jemalloc:3.3.3:natives-macos",
/// "rules": [
///   {
///     "action": "allow",
///     "os": {
///       "name": "osx"
///     }
///   }
/// ]
/// }
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub name: String,
    pub rules: Option<Box<[Rule]>>,
}

impl Library {
    pub fn get_os(&self) -> Option<Os> {
        self.rules
            .as_ref()
            .and_then(|r| {
                r.iter()
                    .find(|x| x.os.is_some())
                    .unwrap()
                    .os
            })
    }

    pub fn get_url(&self) -> &str {
        self.downloads
            .as_ref()
            .unwrap()
            .artifact
            .url
            .as_str()
    }

    pub fn get_rel_path(&self) -> Option<&Path> {
        self.downloads
            .as_ref()
            .map(|ld| ld.artifact.path.as_path())
    }

    pub fn get_hash(&self) -> Option<&str> {
        self.downloads
            .as_ref()
            .map(|ld| ld.artifact.sha1.as_str())
    }
}

/*


    ASSETS INDEX DATA


*/

/// This struct represent *AssetIndex* field from piston-meta versions.
///
/// It has an URL to a JSON which contains the assets of the version.
///
/// Look [Resources]
#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: u128,
    pub url: String,
}

/*

    Minecraft launcher_profiles.json


*/

/// A profile form `launcher_profiles.json` in minecraft root dir.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default = "Default::default")]
    pub icon: String,
    pub last_version_id: String,
    pub name: String,

    pub game_dir: Option<PathBuf>,

    #[serde(rename = "type")]
    pub profile_type: String,

    #[serde(default = "Default::default")]
    pub java_args: String,
}

impl Profile {
    pub fn new(
        icon: &str,
        last_version_id: &str,
        name: &str,
        profile_type: &str,
        path: Option<&Path>,
    ) -> Self {
        let path = path.map(std::borrow::ToOwned::to_owned);

        Self {
            icon: icon.to_string(),
            last_version_id: last_version_id.to_string(),
            name: name.to_string(),
            game_dir: path,
            profile_type: profile_type.to_string(),
            java_args: "".to_string(),
        }
    }

    //TODO!: Docs
    /// This method returns the ID of the profile in case there is one. Also in
    /// case the profile inherits the ID from other version then it will
    /// return it.
    pub fn get_id(&self) -> Option<String> {
        let minecraft_path = self
            .game_dir
            .as_ref()
            .map(|x| {
                x.ancestors().find(|e| {
                    e.file_name()
                        .is_some_and(|f| f == ".minecraft")
                })
            })??
            .to_path_buf();

        let version_path = minecraft_path
            .join("versions")
            .join(&self.last_version_id)
            .join(&self.last_version_id)
            .with_extension("json");

        let file = std::fs::File::open(version_path).ok()?;

        let r: Root = serde_json::from_reader(file).ok()?;

        Some(
            r.inherits_from
                .unwrap_or(r.id.clone()),
        )
    }
}

/// Settings from `launcher_profiles.json`
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub crash_assistance: bool,
    pub enable_advanced: bool,
    pub enable_analytics: bool,
    pub enable_historical: bool,
    pub enable_releases: bool,
    pub enable_snapshots: bool,
    pub keep_launcher_open: bool,
    pub profile_sorting: String,
    pub show_game_log: bool,
    pub show_menu: bool,
    pub sound_on: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            crash_assistance: false,
            enable_advanced: false,
            enable_analytics: false,
            enable_historical: true,
            enable_releases: true,
            enable_snapshots: false,
            keep_launcher_open: true,
            profile_sorting: "ByLastPlayed".to_owned(),
            show_game_log: true,
            show_menu: true,
            sound_on: true,
        }
    }
}

/// Represents the contents of the `launcher_profiles.json` file used by the
/// Minecraft launcher.
///
/// This struct holds the various profiles configured in the Minecraft launcher,
/// along with launcher settings and the configuration version. It is designed
/// to be serialized and deserialized, allowing it to be easily read from or
/// written to a file in JSON format.
///
/// # JSON Format
///
/// The expected structure of the `launcher_profiles.json` file is:
///
/// ```json
/// {
///   "profiles": {
///     "Profile1": {
///       // Profile-specific fields
///     },
///     "Profile2": {
///       // Profile-specific fields
///     }
///   },
///   "settings": {
///     // Launcher settings fields
///   },
///   "version": 2
/// }
/// ```
///
/// # See Also
///
/// * [`Profile`] - Represents the individual profile data.
/// * [`Settings`] - Represents the global launcher settings.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ProfilesJson {
    pub profiles: HashMap<String, Profile>,
    pub settings: Settings,
    pub version: usize,
}

impl ProfilesJson {
    /// Reads the content of `launcher_profiles.json` from `path`.
    ///
    /// In case there is an error parsing the json then a default
    /// `Ok(ProfilesJson)` will be returned.
    ///
    /// # Errors
    ///
    /// In case there is no `launcher_profiles.json` in `path` then this
    /// function will return an `io::Error`.
    ///
    /// # Panic
    ///
    /// This function won't panic.
    pub fn read_json_from<I: AsRef<Path>>(path: I) -> Result<ProfilesJson, std::io::Error> {
        let content = std::io::read_to_string(std::fs::File::open(path)?)?;
        let parsed =
            serde_json::from_str::<ProfilesJson>(&content).map_err(std::io::Error::from)?;

        Ok(parsed)
    }

    /// This method **removes** a profile. Nah I'm joking it will insert it.
    pub fn insert(&mut self, profile_name: &str, data: Profile) {
        self.get_mut_profiles()
            .insert(profile_name.to_owned(), data);
    }

    /// This method **inserts** a profile. Nah I'm joking it will remove it.
    pub fn remove(&mut self, name: &str) {
        self.profiles.remove(name);
    }

    fn get_mut_profiles(&mut self) -> &mut HashMap<String, Profile> {
        &mut self.profiles
    }

    /// This function returns a reference to a `HashMap<String, ProfileData>`
    /// where the `String` is the identifier of the profile and
    /// `ProfileData` is obviously the data.
    pub fn get_profiles(&self) -> &HashMap<String, Profile> {
        &self.profiles
    }

    /// Saves the profiles into a file.
    pub fn save(&self) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(
                get_minecraft_path()
                    .ok_or(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        ".minecraft not found",
                    ))?
                    .join("launcher_profiles.json"),
            )?;

        file.write_all(serde_json::to_string_pretty(&self)?.as_bytes())?;
        Ok(())
    }
}

/// This struct represent a .json file inside
/// _minecraft_root/versions/{version_name}/{version_name}.json_
///
/// **Careful**: `inherits_from` indicates that it should **inherit** the values
/// from other version. (The values must be **added** and not **overwrite** the
/// current data)
///
/// This struct is also the repr of minecraft version from piston-meta.
///
/// This struct will be used when launching minecraft from the command line
/// and/or when downloading a new minecraft profile from piston-meta.
///
/// # JSON EXAMPLE
/// An example of the outter JSON might looks like this:
/// ```json
/// {
///     arguments: {…},
///     assetIndex: {…},
///     assets: "19",
///     complianceLevel: 1,
///     downloads: {…},
///     id: "1.21.4",
///     javaVersion: {…},
///     libraries: […],
///     logging: {…},
///     mainClass: "net.minecraft.client.main.Main",
///     minimumLauncherVersion: 21,
///     type: "release"
/// }
/// ```
/// The fields are renamed to snake_case and type -> version_type since  `type`
/// is a keyword.
///
/// # Downloading minecraft yay !
///
/// When using this struct to download a new minecraft version fields of
/// insterest are:
/// - `asset_index`
/// - `assets`
/// - `libraries`
/// - `downloads`
/// - `java_version` (In case Java is not installed)
///
/// You will find urls to file/files you need in order to run minecraft.
///
/// # Launching minecraft !
///
/// So in order to launch minecraft you will need to read a file from
/// _minecraft_root/versions/{version_name}/{version_name}.json_
///
/// This file contains a JSON with this struct. You will need the following
/// fields in order to launch minecraft:
/// - arguments
/// - assets
/// - java_version (In case Java is not installed or you want the custom
///   runtime)
/// - libraries (you must add them to java path -cp)
/// - main_class
/// - logging (Not mandatory but usefull)
///
///
/// I KNOW TIME AND RELEASETIME FIELDS ARE MISSING, NEED THEM ? PR !!!!
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub arguments: Arguments,

    pub asset_index: AssetIndex,

    #[serde(default = "Default::default")]
    pub assets: String,

    /// .minecraft/versions/version/version.jar
    pub downloads: HashMap<String, DownloadData>,
    /// Actual version example: 1.21.7
    pub id: String,

    pub java_version: JavaVersion,
    pub libraries: Box<[Library]>,
    pub inherits_from: Option<String>,

    #[serde(default = "Default::default")]
    pub main_class: String,

    #[serde(rename = "type")]
    pub version_type: String,
}

impl Root {
    pub fn get_index_name(&self) -> String {
        self.assets.clone() + ".json"
        /*
        let assets_url = self.asset_index.url.as_str();
        assets_url[&assets_url
            .rfind('/')
            .unwrap_or_default()
            + 1..]
            .to_owned()
        */
    }
}

/// This may surprise you but this structs represent the *JAVA VERSION*
///
/// component is the runtime, i.e: "java-runtime-delta", "java-runtime-alpha"...
#[derive(Serialize, Deserialize, Debug)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: usize,
}

/// Arguments which must be pass to java when launching minecraft.
///
/// *IMPORTANT*: jvm args are not supported yet!
///
/// Want them right now ? PR !!!
#[derive(Serialize, Deserialize, Debug)]
pub struct Arguments {
    pub game: Box<[GameArgument]>,
    //jvm: HashMap<String, String>,
}

/// This enum represent the 2 kinds of arguments that appears in piston-meta.
///
/// There are 2 kinds of arguments:
///  + `String`: basic string argument
///  + `Object`: an argument which has rules/actions associed to it.
///
///  Examples of `Object` arguments are "--demo", "--width" or "--height".
///  Examples of `String` arguments are "--gameDir", "--version" or
/// "--username".
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GameArgument {
    String(String),
    Object {
        rules: Box<[Rule]>,
        value: ValueType,
    },
}

/// GO LOOK [GameArgument] !!!
///
/// Two value types:
///  - Single (A single String)
///  - Multiple (Multiple String)
///
/// From piston-meta
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ValueType {
    // Just like me
    Single(String),
    Multiple(Box<[String]>),
}

/// A Rule for whatever Mojang/Microsft thinks its neccesary.
///
/// Used in libraries or args from piston-meta.
#[derive(Serialize, Deserialize, Debug)]
pub struct Rule {
    pub action: String,
    pub os: Option<Os>,
}

/// Enum which contains the differents Osssssssssss.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(tag = "name")]
pub enum Os {
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
    #[serde(other)]
    Other,
}

/// This struct represents the following `JSON`:
/// ```json
///  "artifact": {
///    "path": "org/lwjgl/lwjgl-jemalloc/3.3.3/lwjgl-jemalloc-3.3.3-natives-macos.jar",
///    "sha1": "2906637657a57579847238c9c72d2c4bde7083f8",
///    "size": 153131,
///    "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl-jemalloc/3.3.3/lwjgl-jemalloc-3.3.3-natives-macos.jar"
///  }
/// ```
///
///This is the `downloads` field of [`Library`] struct
#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryDownloads {
    pub artifact: Artifact,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Artifact {
    pub path: PathBuf,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// Returns `Some(.minecraft path)` on success, otherwise `None`.
///
/// MacOS not supported.
pub fn get_minecraft_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let minecraft_path = PathBuf::from(appdata).join(".minecraft");
            Some(minecraft_path)
        } else {
            None
        }
    } else if cfg!(target_os = "linux") {
        if let Some(home_dir) = dirs::home_dir() {
            let minecraft_path = home_dir.join(".minecraft");
            Some(minecraft_path)
        } else {
            None
        }
    } else {
        None
    }
}

/*
 *
 *
 *
 *  MINECRAFT JAVA RUNTIMES
 *
 *
 */

pub const RUNTIMES_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Fancy name for String
pub type RuntimeName = String;

/// Fancy name for String
pub type OsName = String;

/// Fancy name for `HashMap<RuntimeName, Box<[RuntimeData]>>`
///
/// So in this tipe RuntimeName would be something like "java-runtime-delta"
///
/// And each Runtime has its array (99.99% only one item) of Data.
pub type Runtime = HashMap<RuntimeName, Box<[RuntimeData]>>;

/// Fancy name for PathBuf
pub type FileRelPath = PathBuf;

/// So here we are gonna need at least 2 request to get the actual files from
/// the runtimes. First a request to [RUNTIMES_URL], then here we must chose
/// the prefered runtime.
///
/// Once selected one more request to he Manifest's URL and then parse
/// the response into [RuntimeFiles] struct.
///
/// Runtimes fetched from [RUNTIMES_URL]
///
/// Some archs are missing, I dont care, open a pull request if you need them.
///
/// The response looks like this:
///
/// ```json
/// {
///    gamecore {…}
///    linux {…}
///    linux-i386 {…}
///    mac-os {…}
///    mac-os-arm64 {…}
///    windows-arm64 {…}
///    windows-x64 {…}
///    windows-x86 {…}
/// }```
///
///
/// Right now only linux, mac-os and windows-x64 are supported and the field
/// gamecore is ignored/missing.
#[derive(Debug, Serialize, Deserialize)]
pub struct Runtimes {
    pub linux: Runtime,
    #[serde(rename = "windows-x64")]
    pub windowsx64: Runtime,
    #[serde(rename = "mac-os")]
    pub macos: Runtime,
    #[serde(rename = "mac-os-arm64")]
    pub macosarm: Runtime,
}

/// Data of each Runtime.
///
/// Missing fields:
///  - availability
///  - version
///
/// Need them ? PR !!
#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeData {
    manifest: Manifest,
}

impl RuntimeData {
    pub fn get_url(&self) -> &str {
        &self.manifest.url
    }

    pub fn get_size(&self) -> usize {
        self.manifest.size
    }

    pub fn get_sha1(&self) -> &str {
        &self.manifest.sha1
    }
}

/// Hash, size and url of the runtime files.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

// https://piston-meta.mojang.com/v1/packages/3bfc5fdcc28d8897aa12f372ea98a9afeb11a813/manifest.json
/// This is the response fetched from piston-meta when asking
/// for runtimes. This response correspond to the url
/// of the [Manifest] inside [RuntimeData]
#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeFiles {
    pub files: HashMap<FileRelPath, RuntimeFile>,
}

/// This struct represent each file from the runtime.
///
/// It reuses [Manifest] struct so be careful.
///
/// The `downloads` String should be:
/// - raw
/// - lmza
#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeFile {
    #[serde(default)]
    pub downloads: HashMap<String, Manifest>,
    #[serde(default)]
    pub executable: bool,
    #[serde(rename = "type")]
    pub file_type: String,
}
