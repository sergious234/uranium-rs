use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use futures::future::join_all;

use log::error;
use mine_data_strutcs::{
    rinth::{rinth_mods::RinthVersion, rinth_packs::RinthModpack},
    url_maker::maker,
};
use reqwest::Response;

use crate::{
    code_functions::N_THREADS, error::MakerError, hashes::rinth_hash, variables::constants,
    zipper::compress_pack,
};

type HashFilename = Vec<(String, String)>;

/// Good -> Means Uranium found the mod
/// Raw  -> Means the mod need to be added raw
enum ParseState {
    Good(RinthVersion),
    Raw(String),
}

#[derive(Clone, Copy)]
pub enum State {
    Starting,
    Searching,
    Checking,
    Writing,
    Finish,
}

/// This struct is responsable for the creation
/// of the modpacks given a minecraft path.
pub struct ModpackMaker {
    path: PathBuf,
    current_state: State,
    hash_filenames: HashFilename,
    mods_states: Vec<ParseState>,
    rinth_pack: RinthModpack,
    raw_mods: Vec<PathBuf>,
    cliente: reqwest::Client,
    threads: usize,
}

impl ModpackMaker {
    pub fn new<I: AsRef<Path>>(path: I) -> ModpackMaker {
        ModpackMaker {
            path: path.as_ref().to_path_buf(),
            current_state: State::Starting,
            hash_filenames: vec![],
            mods_states: vec![],
            rinth_pack: RinthModpack::new(),
            raw_mods: vec![],
            cliente: reqwest::Client::new(),
            threads: N_THREADS(),
        }
    }

    /// Starts the mod maker process.
    ///
    /// This method initializes the mod maker, reads the mods, and prepares internal data structures for processing.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `MakerError` in the following cases:
    ///
    /// - If there is an error while reading the mods.
    ///
    /// # Returns
    ///
    /// This method returns `Ok(())` if the mod maker was successfully started and prepared for processing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use uranium::modpack_maker::ModpackMaker;
    /// use uranium::error::MakerError;
    ///
    /// let mut mod_maker = ModpackMaker::new("path/to/your/modpack");
    ///
    /// match mod_maker.start() {
    ///     Ok(()) => println!("Mod maker started successfully!"),
    ///     Err(err) => eprintln!("Error starting mod maker: {:?}", err),
    /// }
    /// ```
    pub fn start(&mut self) -> Result<(), MakerError> {
        self.hash_filenames = self.read_mods()?;
        self.mods_states = Vec::with_capacity(self.hash_filenames.len());
        Ok(())
    }

    /// Finishes the mod maker process.
    ///
    /// This asynchronous method continues processing chunks until the mod maker has completed its work.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `MakerError` if any error occurs during the mod making process.
    ///
    /// # Returns
    ///
    /// This method returns `Ok(())` if the mod maker has successfully completed its work.
    ///
    /// # Example
    ///
    /// ```no_run
    /// async {
    ///     use uranium::modpack_maker::ModpackMaker;
    ///     use uranium::error::MakerError;
    ///
    ///     let mut mod_maker = ModpackMaker::new("your/modpack/path");
    ///
    ///     match mod_maker.finish().await {
    ///         Ok(()) => println!("Mod maker finished successfully!"),
    ///         Err(err) => eprintln!("Error finishing mod maker: {:?}", err),
    ///     }
    /// };
    /// ```
    pub async fn finish(&mut self) -> Result<(), MakerError> {
        loop {
            match self.chunk().await {
                Ok(State::Finish) => return Ok(()),
                Err(e) => return Err(e),
                _ => {}
            }
        }
    }

    /// Returns how many mods are in the minecraft
    /// directory
    #[must_use]
    pub fn len(&self) -> usize {
        self.hash_filenames.len()
    }

    /// Returns true if the are no mods in the minecraft directory
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns how many chunks the struct will download
    ///
    /// The formula is: `self.len()` / `self.threads`
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.len() / self.threads
    }

    /// This method will make progress until `Ok(State::Finish)` is returned
    /// or throw an Err.
    ///
    /// It will return the current State of the process.
    ///
    /// # Errors
    /// In case any of the steps fails this method will return `Err(MakerError)` with the
    /// cause.
    ///
    /// Can return any of the following variants: <br>
    /// - `MakerError::CantReadModsDir` <br>
    /// - `MakerError::CantCompress` <br>
    /// - `MakerError::CantRemoveJSON`
    pub async fn chunk(&mut self) -> Result<State, MakerError> {
        self.current_state = match self.current_state {
            State::Starting => {
                if self.hash_filenames.is_empty() {
                    self.hash_filenames = self.read_mods()?;
                }
                State::Searching
            }
            State::Searching => {
                if self.hash_filenames.is_empty() {
                    State::Checking
                } else {
                    self.search_mods().await;
                    State::Searching
                }
            }
            State::Checking => {
                for rinth_mod in &self.mods_states {
                    match rinth_mod {
                        ParseState::Good(m) => self.rinth_pack.add_mod(m.clone().into()),
                        ParseState::Raw(file_name) => self.raw_mods.push(PathBuf::from(file_name)),
                    }
                }
                State::Writing
            }
            State::Writing => {
                self.rinth_pack.write_mod_pack_with_name();

                if let Err(e) = compress_pack("modpack", &self.path, &self.raw_mods) {
                    error!("Error while compressing the modpack: {}", e);
                    return Err(MakerError::CantCompress);
                }

                std::fs::remove_file(constants::RINTH_JSON)
                    .map_err(|_| MakerError::CantRemoveJSON)?;

                State::Finish
            }
            State::Finish => State::Finish,
        };

        Ok(self.current_state)
    }

    async fn search_mods(&mut self) {
        let end = if self.threads > self.hash_filenames.len() {
            self.hash_filenames.len()
        } else {
            self.threads
        };

        let chunk: HashFilename = self.hash_filenames.drain(0..end).collect();

        // Get rinth_responses
        let mut rinth_responses = Vec::with_capacity(chunk.len());

        let reqs = chunk
            .iter()
            .map(|f| tokio::task::spawn(self.cliente.get(maker::ModRinth::hash(&f.0)).send()))
            .collect::<Vec<tokio::task::JoinHandle<Result<Response, reqwest::Error>>>>();

        let responses = join_all(reqs)
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<Result<Response, reqwest::Error>>>();

        rinth_responses.extend(responses);

        let rinth_parses = parse_responses(rinth_responses).await;
        for (file_name, rinth) in chunk.into_iter().zip(rinth_parses.into_iter()) {
            if let Ok(m) = rinth {
                self.mods_states.push(ParseState::Good(m));
            } else {
                self.mods_states.push(ParseState::Raw(file_name.1));
            }
        }
    }

    /// # Errors
    /// If the path dir cant be read then `Err(MakeError::CantReadModsDir)` will
    /// be returned.
    ///
    /// # Panic
    /// This function will panic when path is not a dir.
    fn read_mods(&mut self) -> Result<HashFilename, MakerError> {
        assert!(self.path.is_dir(), "{:?} is not a dir", self.path);

        let mods_path = self.path.join("mods/");

        let mods = match read_dir(&mods_path) {
            Ok(e) => e
                .into_iter()
                .map(|f| f.unwrap().path())
                .collect::<Vec<PathBuf>>(),
            Err(e) => {
                error!("Error reading the directory: {}", e);
                return Err(MakerError::CantReadModsDir);
            }
        };

        let mut hashes_names = Vec::with_capacity(mods.len());

        // Push all the (has, file_name) to the vector
        for path in mods {
            let mod_hash = rinth_hash(path.as_path());
            let file_name = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or_default()
                .to_owned();
            hashes_names.push((mod_hash, file_name));
        }

        Ok(hashes_names)
    }
}

async fn parse_responses(
    responses: Vec<Result<reqwest::Response, reqwest::Error>>,
) -> Vec<Result<RinthVersion, reqwest::Error>> {
    join_all(
        responses
            .into_iter()
            .map(|request| request.unwrap().json::<RinthVersion>()),
    )
    .await
}
