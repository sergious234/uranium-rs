use std::io::Write;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const BASE: &str = "https://resources.download.minecraft.net/";

/*

            MINECRAFT ASSETS DATA STRUCTURES

*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectData {
    pub hash: String,
    pub size: usize,
}

impl ObjectData {
    pub fn get_link(&self) -> String {
        format!("{}{}/{}", BASE, &self.hash[..2], self.hash)
    }

    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(&self.hash[..2]).join(&self.hash)
        //PathBuf::from(self.hash[..2].to_owned() + "/" + &self.hash)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadData {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resources {
    pub objects: HashMap<String, ObjectData>,
}

/*

       https://launchermeta.mojang.com/mc/game/version_manifest.json
                  MINECRAFT INSTANCES DATA STRUCTURE

*/

/// # Example:
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

impl MinecraftVersion {
    pub fn get_id_raw(&self) -> &str {
        &self.id
    }

    pub fn get_link_raw(&self) -> &str {
        &self.url
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftVersions {
    pub latest: Latest,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

impl MinecraftVersions {
    pub fn get_versions_raw(&self) -> &[MinecraftVersion] {
        &self.versions
    }
    pub fn get_instance_url(&self, instance: &str) -> Option<&str> {
        for version in &self.versions {
            if version.get_id_raw() == instance {
                return Some(version.get_link_raw());
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibData {
    artifact: Artifact,
    rules: Option<Rule>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub name: String,
    pub rules: Option<Vec<Rule>>,
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

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/*


    ASSETS INDEX DATA


*/

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: u128,
    pub url: String,
}

pub type Libraries = Vec<Library>;

/*

    Minecraft launcher_profiles.json


*/

/// A profile form `launcher_profiles.json`.
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
    /// This method returns the ID of the profile in case there is one. Also in case
    /// the profile inherits the ID from other version then it will return it.
    pub fn get_id(&self) -> Option<String> {
        let mut minecraft_path = PathBuf::new();

        // This just search for the minecraft root dir of the profile.
        if let Some(gd) = self.game_dir.as_ref() {
            for x in gd.ancestors() {
                if x.file_name()
                    .is_some_and(|f| f == ".minecraft")
                {
                    minecraft_path = x.to_path_buf();
                }
            }
        }

        let version_path = minecraft_path
            .join("versions")
            .join(&self.last_version_id)
            .join(self.last_version_id.clone() + ".json");

        let file = std::fs::File::open(version_path).ok()?;

        let r: Root = serde_json::from_reader(file).ok()?;

        Some(r.inherits_from.unwrap_or(r.id.clone()))
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

/*
 "settings" : {
  59   │     "crashAssistance" : true,
  60   │     "enableAdvanced" : false,
  61   │     "enableAnalytics" : true,
  62   │     "enableHistorical" : false,
  63   │     "enableReleases" : true,
  64   │     "enableSnapshots" : false,
  65   │     "keepLauncherOpen" : false,
  66   │     "profileSorting" : "ByLastPlayed",
  67   │     "showGameLog" : false,
  68   │     "showMenu" : false,
  69   │     "soundOn" : false
  70   │   },
  71   │   "version" : 3
*/

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

    pub fn insert(&mut self, profile_name: &str, data: Profile) {
        self.get_mut_profiles()
            .insert(profile_name.to_owned(), data);
    }

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
/// minecraft_root/versions/{version_name}/{version_name}.json
///
/// This is very similar to `Profile` but it can also have `inherits_from`
/// field which indicates that it should **inherit** the values from other
/// version. (The values must be **added** and not **overwrite** the current
/// data)
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub arguments: Arguments,

    pub asset_index: AssetIndex,

    #[serde(default = "Default::default")]
    pub assets: String,

    pub downloads: HashMap<String, DownloadData>,
    pub id: String,

    pub java_version: JavaVersion,
    pub libraries: Vec<Library>,
    pub inherits_from: Option<String>,

    #[serde(default = "Default::default")]
    pub main_class: String,

    #[serde(rename = "type")]
    pub version_type: String,
}

impl Root {
    pub fn get_index_name(&self) -> String {
        let assets_url = self.asset_index.url.as_str();
        assets_url[&assets_url
            .rfind('/')
            .unwrap_or_default()
            + 1..]
            .to_owned()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: usize,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Arguments {
    game: Vec<GameArgument>,
    //jvm: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GameArgument {
    String(String),
    Object(GameObject),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameObject {
    pub rules: Vec<Rule>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Rule {
    pub action: String,
    pub os: Option<Os>,
}

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
 *
 */

pub const RUNTIMES_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

pub type RuntimeName = String;
pub type OsName = String;
pub type Runtime = HashMap<RuntimeName, Vec<RuntimeData>>;
pub type FileRelPath = PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Runtimes {
    pub linux: Runtime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeData {
    manifest: Manifest,
}

impl RuntimeData {
    pub fn get_url(&self) -> &str {
        &self.manifest.url
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    sha1: String,
    size: usize,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeFiles {
    pub files: HashMap<FileRelPath, RuntimeFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeFile {
    #[serde(default)]
    pub downloads: HashMap<String, Manifest>,
    #[serde(default)]
    pub executable: bool,
    #[serde(rename = "type")]
    pub file_type: String,
}
