use std::{fs::read_to_string, path::PathBuf};
use std::path::Path;
use serde::{Deserialize, Serialize};

use super::rinth_mods::{Hashes, RinthVersion};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RinthModpack {
    #[serde(rename="formatVersion")]
    format_version: usize,
    game: String,
    #[serde(rename="versionId")]
    version_id: String,
    name: PathBuf,
    files: Vec<RinthMdFiles>,
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
        self.name.display().to_string()
    }

    pub fn get_files(&self) -> &Vec<RinthMdFiles> {
        &self.files
    }

    pub fn add_mod(&mut self, new_mod: RinthMdFiles) {
        self.files.push(new_mod);
    }

    pub fn write_mod_pack_with_name(&self) {
        let j = serde_json::to_string_pretty(self).unwrap();
        std::fs::write("modrinth.index.json", j).unwrap();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinthMdFiles {
    path: PathBuf,
    hashes: Hashes,
    downloads: Vec<String>,
    #[serde(rename="fileSize")]
    file_size: usize,
}

impl From<RinthVersion> for RinthMdFiles {
    fn from(version: RinthVersion) -> RinthMdFiles {
        RinthMdFiles {
            path: ("mods/".to_owned() + version.get_file_name()).into(),
            hashes: version.get_hashes().clone(),
            downloads: vec![version.get_file_url().to_string()],
            file_size: version.get_size(),
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
                return download_link.split("data/").nth(1).map(|f| &f[0..8]);
            }
        }
        None
    }

    pub fn get_name(&self) -> String {
        // Oh god, I hate Rust strings.
        self.path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

pub fn load_rinth_pack<I: AsRef<Path>>(pack_path: I) -> Option<RinthModpack> {
     read_to_string(&pack_path)
         .map(|s| serde_json::from_str(&s).ok())
         .ok()
         .flatten()
}
