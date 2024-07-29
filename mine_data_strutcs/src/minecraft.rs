use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const BASE: &str = "https://resources.download.minecraft.net/";

/*

            MINECRAFT ASSETS DATA STRUCTURES

*/

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
    sha1: String,
    size: usize,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resources {
    pub objects: HashMap<String, ObjectData>,
}

/*
#[derive(Serialize, Deserialize, Debug)]
pub struct Instancee<'a> {
    id: &'a str,
    downloads: HashMap<&'a str, DownloadData<'a>>,
}
*/

/*

       https://launchermeta.mojang.com/mc/game/version_manifest.json
                  MINECRAFT INSTANCES DATA STRUCTURE

*/
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MinecraftVersion {
    id: String,
    #[serde(rename = "type")]
    instance_type: String,
    url: String,
    time: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

impl MinecraftVersion {
    pub fn get_id_raw(&self) -> &str {
        &self.id
    }

    pub fn get_link_raw(&self) -> &str {
        &self.url
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MinecraftInstances {
    //latest: (String, String),
    versions: Vec<MinecraftVersion>,
}

impl MinecraftInstances {
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

/*


       MINECRAFT INSTANCE DATA STRUCTURE


*/

/*


       LIBRARY DATA


*/

#[derive(Debug, Serialize, Deserialize, Default)]
struct Artifact {
    path: PathBuf,
    sha1: String,
    size: usize,
    url: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    #[default]
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Rule {
    os: Os,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LibData {
    artifact: Artifact,
    rules: Option<Rule>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Library {
    downloads: LibData,
    name: String,
}

impl Library {
    pub fn get_os(&self) -> Option<Os> {
        if let Some(rule) = self.downloads.rules.as_ref() {
            return Some(rule.os);
        }
        None
    }

    pub fn get_url(&self) -> &str {
        self.downloads.artifact.url.as_str()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/*


    ASSETS INDEX DATA


*/

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AssestIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: u128,
    pub url: String,
}

pub type Libraries = Vec<Library>;

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftInstance {
    #[serde(rename = "assetIndex")]
    pub assest_index: AssestIndex,
    pub id: String,
    pub downloads: HashMap<String, DownloadData>,
    libraries: Libraries,
}

impl MinecraftInstance {
    pub fn get_libs(&self) -> &Libraries {
        &self.libraries
    }

    pub fn get_assests_url(&self) -> &str {
        &self.assest_index.url
    }

    pub fn get_index_name(&self) -> String {
        let assests_url = self.assest_index.url.as_str();
        assests_url[&assests_url.rfind('/').unwrap_or_default() + 1..].to_owned()
    }
}

pub trait Lib {
    fn get_paths(&self) -> Vec<PathBuf>;
    fn get_urls(&self) -> Vec<&str>;
}

impl Lib for Libraries {
    fn get_paths(&self) -> Vec<PathBuf> {
        self.iter()
            .map(|l| l.downloads.artifact.path.clone())
            .collect()
    }

    fn get_urls(&self) -> Vec<&str> {
        self.iter().map(Library::get_url).collect()
    }
}

/*

    Minecraft launcher_profiles.json


*/

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProfileData {
    icon: String,
    #[serde(rename = "lastVersionId")]
    last_version_id: String,
    name: String,

    #[serde(rename = "gameDir")]
    game_dir: Option<PathBuf>,

    #[serde(rename = "type")]
    profile_type: String,
}

impl ProfileData {
    pub fn new(
        icon: &str,
        last_version_id: &str,
        name: &str,
        profile_type: &str,
        path: Option<&Path>,
    ) -> Self {
        let path = path.map(std::borrow::ToOwned::to_owned);

        ProfileData {
            icon: icon.to_string(),
            last_version_id: last_version_id.to_string(),
            name: name.to_string(),
            game_dir: path,
            profile_type: profile_type.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    crash_assistance: bool,
    enable_advanced: bool,
    enable_analytics: bool,
    enable_historical: bool,
    enable_releases: bool,
    enable_snapshots: bool,
    keep_launcher_open: bool,
    profile_sorting: String,
    show_game_log: bool,
    show_menu: bool,
    sound_on: bool,
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
            profile_sorting: "ByLasPlayed".to_owned(),
            show_game_log: true,
            show_menu: true,
            sound_on: true,
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct ProfilesJson {
    profiles: HashMap<String, ProfileData>,
    settings: Settings,
    version: usize,
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
    /// In case there is an error parsing the json then a default `Ok(ProfilesJson)`
    /// will be returned.
    ///
    /// # Errors
    ///
    /// In case there is no `launcher_profiles.json` in `path` then this function will
    /// return an `io::Error`.
    ///
    /// # Panic
    ///
    /// This function won't panic.
    pub fn read_json_from<I: AsRef<std::path::Path>>(
        path: I,
    ) -> Result<ProfilesJson, std::io::Error> {
        let content = std::io::read_to_string(std::fs::File::open(path)?)?;
        let parsed = serde_json::from_str::<ProfilesJson>(&content);

        Ok(parsed.unwrap_or_default())
    }

    pub fn add_profile(&mut self, profile_name: &str, data: ProfileData) {
        self.get_mut_profiles()
            .insert(profile_name.to_owned(), data);
    }

    fn get_mut_profiles(&mut self) -> &mut HashMap<String, ProfileData> {
        &mut self.profiles
    }

    /// This function returns a reference to a `HashMap<String, ProfileData>` where
    /// the `String` is the identifier of the profile and `ProfileData` is obviously the
    /// data.
    pub fn get_profiles(&self) -> &HashMap<String, ProfileData> {
        &self.profiles
    }
}
