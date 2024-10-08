use std::fs::read_to_string;

use serde::{Deserialize, Serialize};
use serde_json::Error;

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
