#![allow(unused)]
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use derive_more::Display;
pub use maker::ModpackMaker;
pub use maker::State;
use mine_data_structs::minecraft::Profile;
use mine_data_structs::rinth::{RinthModpack, RinthVersion, RinthVersionFile, RinthVersions};
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::{Body, ClientBuilder};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use zip::ZipWriter;

use crate::error::{Result, UraniumError};
use crate::hashes::rinth_hash;
use crate::searcher::rinth::{SearchBuilder, SearchType};

mod maker;

#[derive(Clone, Copy, Debug)]
enum MakingProgress {
    ReadingProfile,
    RetrievingMods,
    WritingModpack,
    Finished,
}

enum InnerState {
    ReadingMods {
        data: HashMap<String, PathBuf>,
        dir: std::fs::ReadDir,
    },
    SendingRequests {
        data: HashMap<String, PathBuf>,
    },
    WritingModpack,
    End,
}

#[derive(Display)]
pub enum ModLoaders {
    #[display("forge")]
    Forge,
    #[display("fabric")]
    Fabric,
    #[display("quilt")]
    Quilt,
}

struct ModpackMaker2 {
    mods: Vec<RinthVersionFile>,
    client: reqwest::Client,
    overrides: Vec<PathBuf>,
    path: PathBuf,
    state: MakingProgress,
    inner: InnerState,
    modpack: RinthModpack,
}

impl ModpackMaker2 {
    pub fn new(profile: Profile) -> Result<Self> {
        let path = profile
            .game_dir
            .clone()
            .unwrap_or_else(|| PathBuf::new());
        if !Path::exists(&path) {
            log::error!("Path: {:?}, not found", &path);
            return Err(UraniumError::FileNotFound(path.display().to_string()));
        }

        let client = ClientBuilder::new()
            .user_agent("uranium-rs/ModpackMaker contact: sergious234@gmail.com")
            .build()?;

        let dir = read_dir(path.join("mods"))?;

        Ok(Self {
            mods: vec![],
            path: path.to_path_buf(),
            overrides: vec![],
            client,
            state: MakingProgress::ReadingProfile,
            inner: InnerState::ReadingMods {
                data: HashMap::new(),
                dir: dir,
            },
            modpack: RinthModpack::new(),
        })
    }

    pub async fn progress(&mut self) -> Result<MakingProgress> {
        use InnerState as IS;
        use MakingProgress as MP;

        let mut next_state = None;
        let current_state = match self.inner {
            IS::ReadingMods {
                ref mut data,
                ref mut dir,
            } => {
                let mut i = 0;
                for minecraft_mod in dir.take(16) {
                    i += 1;
                    let minecraft_mod = minecraft_mod?;
                    let path = minecraft_mod.path();
                    let hash = rinth_hash(&path);
                    data.insert(hash, path);
                }

                // Go to the next state when there is no more files left.
                if i != 16 {
                    next_state = Some(IS::SendingRequests {
                        data: std::mem::take(data),
                    });
                }
                MP::ReadingProfile
            }

            IS::SendingRequests { ref mut data } => {
                #[derive(Serialize, Debug)]
                struct RequestBody<'a> {
                    hashes: &'a [String],
                    algorithm: String,
                }

                let url = "https://api.modrinth.com/v2/version_files";

                let hashes: Vec<String> = data.keys().cloned().collect();

                let x = self
                    .client
                    .post(url)
                    .json(&RequestBody {
                        hashes: &hashes,
                        algorithm: "sha1".to_string(),
                    })
                    .send()
                    .await?
                    .json::<HashMap<String, RinthVersionFile>>()
                    .await?;

                for hash in &hashes {
                    if !x.contains_key(hash) {
                        let x = data.remove(hash).unwrap();
                        self.overrides.push(x);
                    }
                }

                self.mods
                    .extend(x.into_values());

                self.modpack.name = "New modpack".into();
                self.modpack.files.extend(
                    self.mods
                        .drain(..)
                        .map(Into::into),
                );

                // for x in self.mods {
                //     self.modpack.files.push(x.into());
                // }
                // self.modpack.files = self.mods.iter().cloned().map(|m|
                // m.into()).collect();

                next_state = Some(IS::End);
                MP::Finished
            }

            IS::WritingModpack { .. } => {
                const OVERRIDES_FOLDERS: [&str; 2] = ["resourcepacks", "config"];
                let mut zip = ZipWriter::new(std::fs::File::open("test")?);

                for or_folder in OVERRIDES_FOLDERS {
                    let or_path = self.path.join(or_folder);
                    if or_path.exists() {
                        println!("{:?} exists", or_path)
                    }
                }

                MP::WritingModpack
            }

            IS::End => MP::Finished,
        };

        if let Some(next_state) = next_state {
            self.inner = next_state;
        }

        Ok(current_state)
    }

    pub fn get_modpack(&mut self) -> Option<&mut RinthModpack> {
        match self.inner {
            InnerState::End => Some(&mut self.modpack),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use mine_data_structs::minecraft::Profile;

    use crate::modpack_maker::MakingProgress;
    use crate::modpack_maker::ModpackMaker2;

    #[tokio::test]
    async fn make_test() {
        let path = "/home/sergio/.minecraft/Quilt1.19.2";
        if !std::fs::exists(&path).unwrap() {
            println!("F");
            return;
        }

        let fake_profile = Profile::new(
            "",
            "quilt-loader-0.26.3-1.19.2",
            "",
            "",
            Some(&std::path::PathBuf::from(
                "/home/sergio/.minecraft/Quilt1.19.2",
            )),
        );
        let mut mm3 = ModpackMaker2::new(fake_profile).unwrap();
        //init_logger().unwrap();
        loop {
            let p = mm3.progress().await;

            if let Ok(MakingProgress::Finished) = p {
                println!("Done!");

                let modpack = mm3.get_modpack().unwrap();

                println!("{}", serde_json::to_string_pretty(modpack).unwrap());

                return;
            } else if let Err(e) = p {
                println!("{}", e);
                break;
            } else {
                println!("[{:?}] Requests left: 0", p);
            }
        }
    }
}

/*

    TODO:
        - Estructura para analizar un profile (&Profile) y crear un modpack a partir
        de ese profile.
        - La estructura tiene que ser capaz de:
            · Saber los mods del perfil
            · Tener los mods cargados con la estructura de version_file (RinthVersionFile)
              para saber datos de la versión especifica actual.
                (https://api.modrinth.com/v2/version_file/619e250c133106bacc3e3b560839bd4b324dfda8)
            · Tener los mods cargados con la estructura de project/{slug}/version (RinthVersions)
                para saber los datos de las versiones mas nuevas del mod que sigan usando la version
                de minecraft actual.
                (https://api.modrinth.com/v2/project/Jw3Wx1KR/version)
            · Poder mostrar la version mas actualizada del mod para la versión de minecraft.
            · Usar la misma filosofia de progress() para facilitar la asincronicidad.
            · enum MakingProgress {
            ·   ReadingMods
            ·   RetrievingMods
            ·   LookingForUpdates
            ·   Finished
            · }

        Ejemplo:

            mods
              | sodium.jar
              | crate.jar
              | fabric-api.jar
              | minimap.jar

           https://api.modrinth.com/v2/project/Jw3Wx1KR/version?game_versions=["1.19"]


           {
            "property1": {
                "name": "Version 1.0.0",
                "version_number": "1.0.0",
            },

            "property2": {
                "name": "Version 1.0.0",
                "version_number": "1.0.0",
            }
           }

*/
