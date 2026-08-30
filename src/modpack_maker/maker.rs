use std::{
    collections::HashMap,
    fs::read_dir,
    path::{Path, PathBuf},
};

use log::error;
use mine_data_structs::rinth::{RinthModpack, RinthVersion};
use rrhodium::{SearchBuilder, SearchType};

use crate::variables::constants::{MRPACK, OVERRIDES_FOLDER};
use crate::{
    error::Result, error::UraniumError, hashes::rinth_hash, variables::constants::RINTH_JSON,
};

#[derive(Clone, Copy)]
pub enum State {
    Starting,
    Searching,
    Checking,
    Writing,
    Finish,
}

impl From<&InnerState> for State {
    fn from(value: &InnerState) -> Self {
        use InnerState as IS;
        match value {
            IS::Reading => Self::Starting,
            IS::Searching { mods: _ } => Self::Searching,
            IS::Writing { data: _ } => Self::Writing,
            _ => Self::Finish,
        }
    }
}

#[derive(Debug)]
struct HashPath {
    pub hash: String,
    pub path: PathBuf,
}

#[derive(Debug)]
enum SearchResult {
    Found(Box<RinthVersion>),
    NotFound(HashPath),
}

enum InnerState {
    Reading,
    Searching {
        mods: Box<dyn Iterator<Item = HashPath>>,
    },
    Writing {
        data: HashMap<String, SearchResult>,
    },
    Finish,
}

pub struct ModpackMaker {
    /// Where the mods are
    path: PathBuf,
    /// Where to save the modpack
    modpack_path: PathBuf,
    state: InnerState,
    client: reqwest::Client,
}

impl ModpackMaker {
    pub fn new<I: AsRef<Path>, J: AsRef<Path>>(path: I, modpack_name: J) -> Self {
        let mut modpack_name = modpack_name
            .as_ref()
            .to_path_buf();
        if !modpack_name
            .extension()
            .is_some_and(|e| {
                e == MRPACK
            })
        {
            modpack_name.set_extension(MRPACK);
        }
        Self {
            path: path.as_ref().to_path_buf(),
            state: InnerState::Reading,
            client: reqwest::ClientBuilder::new()
                .user_agent("uranium-rs/modpack maker contact: sergious234@gmail.com")
                .build()
                .unwrap(),
            modpack_path: modpack_name,
        }
    }

    pub async fn finish(mut self) -> Result<()> {
        loop {
            match self.progress().await {
                Ok(State::Finish) => return Ok(()),
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn progress(&mut self) -> Result<State> {
        use InnerState as IS;

        match &mut self.state {
            IS::Reading => {
                self.state = IS::Searching {
                    mods: Box::new(self.read_mods()?),
                };
            }
            IS::Searching { mods } => {
                let url = SearchBuilder::new()
                    .search_type(SearchType::VersionFiles)
                    .build_url();

                let mods_data = mods.collect::<Vec<_>>();
                let mut results: HashMap<String, SearchResult> = self
                    .client
                    .post(url)
                    .json(&serde_json::json!({
                        "hashes": mods_data.iter().map(|m| &m.hash).collect::<Vec<_>>(),
                        "algorithm": "sha1",
                    }))
                    .send()
                    .await?
                    .json::<HashMap<String, RinthVersion>>()
                    .await?
                    .into_iter()
                    .map(|(k, v)| (k, SearchResult::Found(Box::new(v))))
                    .collect();

                for m in mods_data {
                    results
                        .entry(m.hash.clone())
                        .or_insert(SearchResult::NotFound(m));
                }
                log::info!("{results:#?}");
                self.state = IS::Writing { data: results };
            }
            IS::Writing { data: ref_data } => {
                // Little trick here so the borrow checker is happy.
                let data = std::mem::take(ref_data);
                self.write_modpack(data)?;
                self.state = IS::Finish;
            }
            IS::Finish => {}
        };
        Ok((&self.state).into())
    }

    /// # Errors
    /// If the path dir cant be read then `Err(MakeError::CantReadModsDir)` will
    /// be returned.
    ///
    /// # Panic
    /// This function will panic when path is not a dir.
    fn read_mods(&mut self) -> Result<impl Iterator<Item = HashPath> + 'static> {
        if !self.path.is_dir() {
            return Err(UraniumError::CantReadModsDir);
        }

        let mods_path = self.path.join("mods/");

        let entries = match read_dir(&mods_path) {
            Ok(e) => e,
            Err(e) => {
                error!("Error reading the directory: {}", e);
                return Err(UraniumError::IOError(e));
            }
        };

        let mut hashes = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let hash = rinth_hash(&path)?;
            hashes.push(HashPath {
                hash,
                path: path.to_owned(),
            });
        }

        Ok(hashes.into_iter())
    }

    fn write_modpack(&self, data: HashMap<String, SearchResult>) -> Result<()> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&self.modpack_path)?);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        for entry in walkdir::WalkDir::new(self.path.join("config"))
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            let name = path
                .strip_prefix(&self.path)
                .unwrap();

            if path.is_file() {
                Self::add_file_to_zip(&mut zip, path, options)?;
            } else {
                zip.add_directory(name.to_string_lossy(), options)?;
            }
        }

        let mut rinth_pack =
            RinthModpack::new_with("1.0".to_string(), self.modpack_path.clone(), vec![]);

        zip.add_directory(OVERRIDES_FOLDER, options)?;
        for value in data.into_values() {
            match value {
                SearchResult::Found(rv) => rinth_pack.add_mod((*rv).into()),
                SearchResult::NotFound(m) => {
                    let path = PathBuf::new()
                        .join(OVERRIDES_FOLDER)
                        .join(m.path.file_name().unwrap());
                    Self::add_file_to_zip_path(&mut zip, path, &m.path, options)?;
                }
            }
        }

        zip.start_file(RINTH_JSON, options)?;
        serde_json::to_writer(&mut zip, &rinth_pack).expect("Pack can't be serialized");

        zip.finish()?;
        log::info!("Modpack written successfully!");
        Ok(())
    }

    fn add_file_to_zip<P: AsRef<std::path::Path>>(
        zip: &mut zip::ZipWriter<std::fs::File>,
        path: P,
        options: zip::write::SimpleFileOptions,
    ) -> Result<()> {
        let mut f = std::fs::File::open(&path)?;
        zip.start_file(
            path.as_ref()
                .to_string_lossy(),
            options,
        )?;
        std::io::copy(&mut f, zip)?;
        Ok(())
    }

    fn add_file_to_zip_path<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
        zip: &mut zip::ZipWriter<std::fs::File>,
        name: P,
        file_path: Q,
        options: zip::write::SimpleFileOptions,
    ) -> Result<()> {
        let mut f = std::fs::File::open(file_path)?;
        zip.start_file(
            name.as_ref()
                .to_string_lossy(),
            options,
        )?;
        std::io::copy(&mut f, zip)?;
        Ok(())
    }
}
