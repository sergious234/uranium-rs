//! Full `.mrpack` pipeline: vanilla Minecraft, loader, overrides, mod files.
//!
//! A `.mrpack` is a zip with `modrinth.index.json` plus `overrides/` and
//! optional `client-overrides/` / `server-overrides/`. The installer resolves
//! `dependencies` (MC + loader versions), installs vanilla Minecraft, installs
//! the loader when declared (Fabric/Quilt; Forge/NeoForge report unsupported),
//! copies the side-matching overrides, then downloads each `files[]` entry
//! passing the `env` filter, with SHA-1 verification.
//!
//! Poll `progress` for continuous state updates; `start` runs to completion.
//! The terminal state is always `Completed` — read `installed_profile_id`
//! for the launch target (`None` for vanilla-only packs).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use log::{error, info};
use mine_data_structs::rinth::{
    PackMeta, RinthMdFiles, RinthModpack, Side, join_inside, load_rinth_pack,
};

use super::loaders::LoaderRequirement;
use super::phases::{FilesStep, LoaderStep, OverridesStep, VanillaStep, VerifyStep};
use super::steps::{RinthCtx, RinthStep};
use crate::HashType;
use crate::common::constants::{RINTH_JSON, TEMP_DIR};
use crate::common::fs::unzip_temp_pack_at;
use crate::config::num_threads;
use crate::engine::{DownloadableObject, FileDownloader};
use crate::error::{Result, UraniumError};
use crate::modpacks::common::{PackSource, ensure_pack_dirs};

// region:    --- State

/// Observable install phase for progress reporting (e.g. a REST API).
///
/// Use `as_str` for stable plain-string forwarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RinthInstallState {
    ResolvingManifest,
    InstallingMinecraft,
    InstallingLoader,
    CopyingOverrides,
    DownloadingFiles,
    Verifying,
    Completed,
}

impl RinthInstallState {
    /// Stable phase name for string-based progress forwarding.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ResolvingManifest => "ResolvingManifest",
            Self::InstallingMinecraft => "InstallingMinecraft",
            Self::InstallingLoader => "InstallingLoader",
            Self::CopyingOverrides => "CopyingOverrides",
            Self::DownloadingFiles => "DownloadingFiles",
            Self::Verifying => "Verifying",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Debug)]
enum InnerState {
    Resolving,
    InstallingMinecraft,
    InstallingLoader,
    CopyingOverrides,
    DownloadingFiles,
    Verifying,
    Done,
}

impl From<&InnerState> for RinthInstallState {
    fn from(value: &InnerState) -> Self {
        use InnerState as IS;
        match value {
            IS::Resolving => Self::ResolvingManifest,
            IS::InstallingMinecraft => Self::InstallingMinecraft,
            IS::InstallingLoader => Self::InstallingLoader,
            IS::CopyingOverrides => Self::CopyingOverrides,
            IS::DownloadingFiles => Self::DownloadingFiles,
            IS::Verifying => Self::Verifying,
            IS::Done => Self::Completed,
        }
    }
}

// endregion: --- State

// region:    --- Installer

/// Installs a Modrinth pack (`.mrpack`) into a Minecraft instance.
///
/// The install `Side` is fixed at construction (C1): it selects the override
/// folder and filters `files[].env`, so it cannot change mid-install.
///
/// Use `new` for a client install with default concurrency, `with_side` to
/// pick a side, or `with_downloader` to supply a custom `T: FileDownloader`.
/// The custom downloader governs mod-file downloads; the vanilla Minecraft
/// phase uses the engine-default downloader.
pub struct RinthInstaller<T: FileDownloader> {
    downloader: T,
    manifest: RinthModpack,
    meta: PackMeta,
    files: Vec<RinthMdFiles>,
    side: Side,
    instance_dir: PathBuf,
    tmp: Option<PathBuf>,
    state: InnerState,
    requester: reqwest::Client,
    installed_profile_id: Option<String>,
    vanilla: VanillaStep,
}

impl<T: FileDownloader> PackSource for RinthInstaller<T> {
    fn pack_name(&self) -> &str {
        self.manifest.pack_name()
    }
}

impl<T: FileDownloader> RinthInstaller<T> {
    /// Prepares a client-side install of the pack at `modpack_path` into
    /// `destination`.
    ///
    /// `modpack_path` is the local `.mrpack` zip; `destination` is the
    /// Minecraft instance folder (e.g. `~/.minecraft`).
    ///
    /// # Errors
    ///
    /// This function will return an error if the pack cannot be read.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::RinthInstaller;
    /// # async fn run() -> uranium_rs::error::Result<()> {
    /// let mut dl = RinthInstaller::<Downloader>::new("pack.mrpack", "/home/user/.minecraft")?;
    /// dl.start().await?;
    /// # Ok(()) }
    /// ```
    pub fn new<I: AsRef<Path>, J: AsRef<Path>>(modpack_path: I, destination: J) -> Result<Self> {
        Self::with_side(modpack_path, destination, Side::default())
    }

    /// Prepares an install for the given `side` with default concurrency.
    ///
    /// # Errors
    ///
    /// This function will return an error if the pack cannot be read.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::{RinthInstaller, Side};
    /// # async fn run() -> uranium_rs::error::Result<()> {
    /// let mut dl = RinthInstaller::<Downloader>::with_side("pack.mrpack", "/srv/mc", Side::Server)?;
    /// dl.start().await?;
    /// # Ok(()) }
    /// ```
    pub fn with_side<I: AsRef<Path>, J: AsRef<Path>>(
        modpack_path: I,
        destination: J,
        side: Side,
    ) -> Result<Self> {
        Self::with_downloader(modpack_path, destination, T::new(), side)
    }

    /// Prepares an install for the given `side` using your own `downloader`.
    ///
    /// Use this to control concurrency, retries or to inject a mock in tests.
    ///
    /// # Errors
    ///
    /// This function will return an error if the pack cannot be read.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium_rs::config::DownloaderConfig;
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::{RinthInstaller, Side};
    /// # async fn run() -> uranium_rs::error::Result<()> {
    /// let downloader = Downloader::from_config(DownloaderConfig::new(16));
    /// let mut dl = RinthInstaller::with_downloader(
    ///     "pack.mrpack",
    ///     "/home/user/.minecraft",
    ///     downloader,
    ///     Side::Client,
    /// )?;
    /// dl.start().await?;
    /// # Ok(()) }
    /// ```
    pub fn with_downloader<I: AsRef<Path>, J: AsRef<Path>>(
        modpack_path: I,
        destination: J,
        downloader: T,
        side: Side,
    ) -> Result<Self> {
        let tmp = unique_tmp()?;
        Self::with_downloader_at(
            modpack_path.as_ref(),
            destination.as_ref(),
            downloader,
            side,
            &tmp,
        )
    }

    /// Builds an installer from an already-parsed manifest (no zip is read).
    ///
    /// No directories are created and there are no overrides to copy. Useful
    /// when the manifest is already in memory or in tests.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use std::path::PathBuf;
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::{RinthInstaller, RinthModpack, Side};
    /// # fn run() -> uranium_rs::error::Result<()> {
    /// let manifest = RinthModpack::default();
    /// let dl = RinthInstaller::<Downloader>::from_pack(
    ///     manifest,
    ///     PathBuf::from("/tmp/.minecraft"),
    ///     Downloader::new(),
    ///     Side::Client,
    /// );
    /// # Ok(()) }
    /// ```
    pub fn from_pack(
        manifest: RinthModpack,
        destination: impl AsRef<Path>,
        downloader: T,
        side: Side,
    ) -> Self {
        Self::build(manifest, destination.as_ref(), downloader, side, None)
    }

    /// Reads `modrinth.index.json` from a zip via a temporary directory
    /// (internal).
    #[doc(hidden)]
    pub(crate) fn load_pack_at(zip_path: &Path, tmp: &Path) -> Result<RinthModpack> {
        match unzip_temp_pack_at(zip_path, tmp) {
            Err(UraniumError::CantCreateDir("temp_dir")) => {
                // retry with cleaned tmp (unzip_temp_pack_at already removed
                // stale dir)
                unzip_temp_pack_at(zip_path, tmp)?
            }
            Err(e) => return Err(e),
            Ok(_) => {}
        }

        match load_rinth_pack(tmp.join(RINTH_JSON)) {
            Some(manifest) => {
                info!("Pack loaded {}", manifest.pack_name());
                Ok(manifest)
            }
            None => Err(UraniumError::WrongFileFormat),
        }
    }

    /// Injectable-tmp variant of `with_downloader` for tests (internal).
    #[doc(hidden)]
    pub(crate) fn with_downloader_at(
        modpack_path: &Path,
        destination: &Path,
        downloader: T,
        side: Side,
        tmp: &Path,
    ) -> Result<Self> {
        ensure_pack_dirs(destination)?;
        let manifest = Self::load_pack_at(modpack_path, tmp)?;
        Ok(Self::build(
            manifest,
            destination,
            downloader,
            side,
            Some(tmp.to_path_buf()),
        ))
    }

    fn build(
        manifest: RinthModpack,
        destination: &Path,
        mut downloader: T,
        side: Side,
        tmp: Option<PathBuf>,
    ) -> Self {
        let files: Vec<RinthMdFiles> = manifest
            .files_for_side(side)
            .cloned()
            .collect();
        let mut queued = 0;
        for file in &files {
            match file_to_object(file, destination) {
                Some(obj) => {
                    downloader.add_object(obj);
                    queued += 1;
                }
                None => info!(
                    "Skipping {}: no URL or unsafe path",
                    file.get_path().display()
                ),
            }
        }
        info!("Queued {queued} files for {side:?}");
        let meta = manifest.meta();
        let state = match tmp {
            Some(_) => InnerState::Resolving,
            None => InnerState::InstallingMinecraft,
        };
        RinthInstaller {
            downloader,
            manifest,
            meta,
            files,
            side,
            instance_dir: destination.to_path_buf(),
            tmp,
            state,
            requester: reqwest::Client::new(),
            installed_profile_id: None,
            vanilla: VanillaStep::new(),
        }
    }

    fn ctx(&mut self) -> RinthCtx<'_, T> {
        RinthCtx {
            instance_dir: &self.instance_dir,
            tmp: self.tmp.as_deref(),
            meta: &self.meta,
            files: &self.files,
            side: self.side,
            downloader: &mut self.downloader,
            requester: &self.requester,
        }
    }

    fn cleanup(&self) {
        if let Some(tmp) = self.tmp.as_ref()
            && std::fs::remove_dir_all(tmp).is_err()
        {
            error!("Error at deleting temp dir");
        }
    }

    /// Runs the install to completion, returning the terminal state.
    ///
    /// Cleans the temporary directory when finished.
    ///
    /// # Errors
    ///
    /// This function will return an error if the install fails.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::RinthInstaller;
    /// # async fn run() -> uranium_rs::error::Result<()> {
    /// let mut dl = RinthInstaller::<Downloader>::new("pack.mrpack", "/tmp/.minecraft")?;
    /// dl.start().await?;
    /// # Ok(()) }
    /// ```
    pub async fn start(&mut self) -> Result<RinthInstallState> {
        loop {
            match self.progress().await {
                Ok(RinthInstallState::Completed) => return Ok(RinthInstallState::Completed),
                Ok(_) => {}
                Err(e) => {
                    self.cleanup();
                    return Err(e);
                }
            }
        }
    }

    /// Advances the install one step and returns the new state.
    ///
    /// Poll repeatedly until `RinthInstallState::Completed`. On error the
    /// scratch dir is removed and the installer must not be reused.
    ///
    /// # Errors
    ///
    /// This function will return an error if the install fails.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium_rs::engine::Downloader;
    /// use uranium_rs::modpacks::rinth::{RinthInstaller, RinthInstallState};
    /// # async fn run() -> uranium_rs::error::Result<()> {
    /// let mut dl = RinthInstaller::<Downloader>::new("pack.mrpack", "/tmp/.minecraft")?;
    /// loop {
    ///     match dl.progress().await? {
    ///         RinthInstallState::Completed => break,
    ///         state => println!("{state:?}: {} batches left", dl.requests_left()),
    ///     }
    /// }
    /// # Ok(()) }
    /// ```
    pub async fn progress(&mut self) -> Result<RinthInstallState> {
        let result = self.advance().await;
        if result.is_err() {
            self.cleanup();
        }
        result
    }

    async fn advance(&mut self) -> Result<RinthInstallState> {
        use InnerState as IS;

        // -- Resolve is done in the ctor; first poll just moves on
        if matches!(self.state, IS::Resolving) {
            self.state = IS::InstallingMinecraft;
            return Ok(RinthInstallState::InstallingMinecraft);
        }

        // -- Run the current phase step
        let next = match self.state {
            IS::Resolving => IS::InstallingMinecraft,
            IS::InstallingMinecraft => {
                // -- VanillaStep is stateful: disjoint field borrows let the
                // -- persisted step run against a ctx built inline
                let mut ctx = RinthCtx {
                    instance_dir: &self.instance_dir,
                    tmp: self.tmp.as_deref(),
                    meta: &self.meta,
                    files: &self.files,
                    side: self.side,
                    downloader: &mut self.downloader,
                    requester: &self.requester,
                };
                match self
                    .vanilla
                    .run(&mut ctx)
                    .await?
                {
                    RinthInstallState::CopyingOverrides => {
                        if self.meta.loader.is_some() {
                            IS::InstallingLoader
                        } else {
                            IS::CopyingOverrides
                        }
                    }
                    _ => return Ok(RinthInstallState::InstallingMinecraft),
                }
            }
            IS::InstallingLoader => {
                let mut step = LoaderStep::new();
                step.run(&mut self.ctx())
                    .await?;
                self.installed_profile_id = step
                    .profile_id()
                    .map(str::to_owned);
                IS::CopyingOverrides
            }
            IS::CopyingOverrides => {
                let mut step = OverridesStep;
                step.run(&mut self.ctx())
                    .await?;
                IS::DownloadingFiles
            }
            IS::DownloadingFiles => {
                let mut step = FilesStep;
                match step
                    .run(&mut self.ctx())
                    .await?
                {
                    RinthInstallState::Verifying => IS::Verifying,
                    _ => return Ok(RinthInstallState::DownloadingFiles),
                }
            }
            IS::Verifying => {
                let mut step = VerifyStep;
                step.run(&mut self.ctx())
                    .await?;
                if let Some(id) = self
                    .installed_profile_id
                    .clone()
                    && !self
                        .instance_dir
                        .join("versions")
                        .join(&id)
                        .join(format!("{id}.json"))
                        .exists()
                {
                    return Err(UraniumError::FileNotFound(format!("loader profile {id}")));
                }
                IS::Done
            }
            IS::Done => return Ok(RinthInstallState::Completed),
        };

        self.state = next;
        if matches!(self.state, IS::Done) {
            self.cleanup();
        }
        Ok((&self.state).into())
    }

    /// Current state without advancing.
    #[must_use]
    pub fn state(&self) -> RinthInstallState {
        (&self.state).into()
    }

    /// Install side chosen at construction.
    #[must_use]
    pub fn side(&self) -> Side {
        self.side
    }

    /// The parsed manifest (read-only).
    #[must_use]
    pub fn manifest(&self) -> &RinthModpack {
        &self.manifest
    }

    /// Resolved MC version and loader requirement (read-only).
    #[must_use]
    pub fn pack_meta(&self) -> &PackMeta {
        &self.meta
    }

    /// Loader needed to launch, if the pack declares one.
    ///
    /// `Some` means the install completes on disk but needs Phase 2 loader
    /// support before launch; `None` means vanilla-launchable.
    #[must_use]
    pub fn loader_requirement(&self) -> Option<&LoaderRequirement> {
        self.meta.loader.as_ref()
    }

    /// Installed loader profile id (e.g. `fabric-loader-0.16.9-1.21`).
    ///
    /// This is what the launcher must launch. `None` until the
    /// `InstallingLoader` phase succeeds; always `None` for vanilla-only
    /// packs. The profile JSON uses `inheritsFrom`, which the launcher
    /// resolves engine-side.
    #[must_use]
    pub fn installed_profile_id(&self) -> Option<&str> {
        self.installed_profile_id
            .as_deref()
    }

    /// Whether the download queue is empty.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.downloader
            .requests_left()
            == 0
    }

    /// Total number of batches for progress reporting.
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.downloader
            .len()
            .div_ceil(num_threads())
    }

    /// Number of batches still pending.
    ///
    /// Reports the vanilla queue while `InstallingMinecraft`, the mod-file
    /// queue otherwise.
    #[must_use]
    pub fn requests_left(&self) -> usize {
        if matches!(self.state, InnerState::InstallingMinecraft) {
            return self
                .vanilla
                .requests_left()
                .div_ceil(num_threads());
        }
        self.downloader
            .requests_left()
            .div_ceil(num_threads())
    }

    /// Pack name for display (from `modrinth.index.json`).
    #[must_use]
    pub fn get_modpack_name(&self) -> String {
        self.pack_name().to_owned()
    }
}

// endregion: --- Installer

// region:    --- Support

/// Disambiguates scratch dirs of concurrent installs in one process.
static INSTALL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Creates a unique scratch dir for one install (internal).
///
/// Every public constructor used to share the global [`TEMP_DIR`], so two
/// concurrent installs extracted their packs into the same directory. Each
/// installer now gets `TEMP_DIR/mrpack-<pid>-<n>` instead.
fn unique_tmp() -> Result<PathBuf> {
    std::fs::create_dir_all(&*TEMP_DIR).map_err(|_| UraniumError::CantCreateDir("temp_dir"))?;
    Ok(TEMP_DIR.join(format!(
        "mrpack-{}-{}",
        std::process::id(),
        INSTALL_COUNTER.fetch_add(1, Ordering::Relaxed)
    )))
}

/// Builds the download task for a pack file (internal).
#[doc(hidden)]
pub(crate) fn file_to_object(file: &RinthMdFiles, dest: &Path) -> Option<DownloadableObject> {
    let url = file.download_link()?;
    let path = join_inside(dest, file.get_path())?;
    Some(DownloadableObject::new(
        url,
        &path,
        Some(HashType::Sha1(file.get_sha1().to_owned())),
    ))
}

// endregion: --- Support

// region:    --- One-shot

/// Installs a Modrinth pack in one call (client side).
///
/// Convenience wrapper around `RinthInstaller::new(...).start().await`.
///
/// # Errors
///
/// This function will return an error if the pack cannot be read.
///
/// # Example
///
/// ```rust no_run
/// # async fn run() -> uranium_rs::error::Result<()> {
/// use uranium_rs::rinth_pack_download;
/// rinth_pack_download("pack.mrpack", "/home/user/.minecraft").await?;
/// # Ok(()) }
/// ```
pub async fn rinth_pack_download<I: AsRef<Path>, J: AsRef<Path>>(
    file_path: I,
    destination_path: J,
) -> Result<()> {
    use crate::engine::Downloader;

    let mut installer = RinthInstaller::<Downloader>::new(&file_path, &destination_path)?;
    installer.start().await?;
    Ok(())
}

// endregion: --- One-shot

// region:    --- Tests

#[cfg(test)]
mod tests {
    use mine_data_structs::rinth::{Env, Hashes, RinthMdFiles, RinthModpack};

    use super::*;
    use crate::engine::Downloader;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    fn file(path: &str, downloads: Vec<String>) -> RinthMdFiles {
        RinthMdFiles {
            path: path.into(),
            hashes: Hashes {
                sha1: "aaa".to_owned(),
                sha512: "aaa512".to_owned(),
            },
            downloads,
            file_size: 10,
            env: Env::default(),
        }
    }

    #[test]
    fn file_to_object_builds_task() -> TestResult<()> {
        let obj = file_to_object(
            &file("mods/a.jar", vec!["https://example.com/a.jar".to_owned()]),
            Path::new("/instance"),
        )
        .ok_or("should build an object")?;
        assert_eq!(obj.path, PathBuf::from("/instance/mods/a.jar"));
        Ok(())
    }

    #[test]
    fn file_to_object_skips_url_less_and_escaping_files() -> TestResult<()> {
        let dest = Path::new("/instance");
        assert!(file_to_object(&file("mods/a.jar", vec![]), dest).is_none());
        assert!(
            file_to_object(
                &file("../evil.jar", vec!["https://example.com/e.jar".to_owned()]),
                dest
            )
            .is_none()
        );
        Ok(())
    }

    #[tokio::test]
    async fn no_loader_skips_loader_phase() -> TestResult<()> {
        let dest = tempfile::tempdir()?;
        let mut dl = RinthInstaller::<Downloader>::from_pack(
            RinthModpack::default(),
            dest.path(),
            Downloader::new(),
            Side::Client,
        );
        let mut seen = Vec::new();
        loop {
            match dl.progress().await? {
                RinthInstallState::Completed => break,
                s => seen.push(s),
            }
        }
        assert!(!seen.contains(&RinthInstallState::InstallingLoader));
        assert!(
            dl.installed_profile_id()
                .is_none()
        );
        Ok(())
    }
}

// endregion: --- Tests
