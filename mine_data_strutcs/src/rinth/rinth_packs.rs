use super::rinth_mods::{Hashes, RinthVersion};
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::PathBuf};

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

    pub fn get_mods(&self) -> &Vec<RinthMdFiles> {
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

impl std::convert::From<RinthVersion> for RinthMdFiles {
    fn from(version: RinthVersion) -> RinthMdFiles {
        RinthMdFiles {
            path: ("mods/".to_owned() + &version.get_file_name()).into(),
            hashes: version.get_hashes().clone(),
            downloads: vec![version.get_file_url()],
            file_size: version.get_size(),
        }
    }
}

impl RinthMdFiles {
    pub fn get_download_link(&self) -> String {
        self.downloads[0].clone()
    }

    pub fn get_download_link_raw(&self) -> &str {
        &self.downloads[0]
    }

    pub fn get_id(&self) -> Option<&str> {
        for download_link in &self.downloads {
            if download_link.contains("modrinth") {
                return download_link.split("data/").nth(1).map(|f| &f[0..8]);
            }
        }
        None
    }

    pub fn get_name(&self) -> PathBuf {
        self.path.clone()
        // self.path.strip_prefix("mods/").unwrap().to_owned()
    }

    pub fn get_raw_name(&self) -> &PathBuf {
        &self.path
        // self.path.file_name().unwrap().to_os_string().into_string().unwrap()
        /*
        self.path.strip_prefix("mods/").expect(
            &format!("ERROR: Cant get raw name of {}", self.path)
        )
            */
    }
}

fn deserializ_pack(path: &str) -> Option<RinthModpack> {
    serde_json::from_str(&read_to_string(path).unwrap()).ok()
}

pub fn load_rinth_pack(pack_path: &str) -> Option<RinthModpack> {
    match read_to_string(pack_path) {
        Ok(_) => {}
        Err(_) => {
            return None;
        }
    };

    deserializ_pack(pack_path)
}
