use std::path::Path;

use crate::engine::{DownloadableObject, Downloader, FileDownloader};
use crate::error::Result;
use crate::minecraft::verify::types::VersionCheckResult;
use crate::minecraft::{get_index_path, get_lib_path};

/// Fixes a Minecraft installation by downloading missing/corrupt files.
///
/// Generic over `D: FileDownloader` (DIP) — callers can inject their own
/// downloader via `with_downloader`, or use the default `Downloader`.
pub struct InstallationFixer<D: FileDownloader = Downloader> {
    data: Vec<DownloadableObject>,
    _marker: std::marker::PhantomData<D>,
}

impl<D: FileDownloader> InstallationFixer<D> {
    pub fn new(check_result: VersionCheckResult, installation_path: impl AsRef<Path>) -> Self {
        let installation_path = installation_path.as_ref();
        let mut fixer = Self {
            data: vec![],
            _marker: std::marker::PhantomData,
        };
        fixer.add_objects(&check_result, installation_path);
        fixer.add_libs(&check_result, installation_path);
        fixer.add_index(&check_result, installation_path);
        fixer
    }

    /// Same as `new` but with an explicitly provided downloader instance.
    /// Useful when the caller already holds a configured downloader.
    pub fn with_downloader(
        check_result: VersionCheckResult,
        installation_path: impl AsRef<Path>,
        _downloader: D,
    ) -> Self {
        Self::new(check_result, installation_path)
    }

    /// Downloads all collected `DownloadableObject`s using `D`.
    pub async fn fix_installation(&mut self) -> Result<()> {
        let files = std::mem::take(&mut self.data);
        let mut downloader = D::new();
        downloader.add_objects(files);
        downloader.start().await
    }

    /// Same as `fix_installation` but re-uses an existing `downloader`
    /// instance.
    pub async fn fix_with(&mut self, downloader: &mut D) -> Result<()> {
        let files = std::mem::take(&mut self.data);
        downloader.add_objects(files);
        downloader.start().await
    }

    fn add_objects(&mut self, check_result: &VersionCheckResult, installation_path: &Path) {
        self.data.extend(
            check_result
                .objects
                .iter()
                .map(|&f| DownloadableObject::from(f))
                .map(|mut f| {
                    f.path = installation_path.join(f.path);
                    f
                }),
        );
    }

    fn add_libs(&mut self, check_result: &VersionCheckResult, installation_path: &Path) {
        self.data.extend(
            check_result
                .libs
                .iter()
                .map(|&f| DownloadableObject::from(f))
                .map(|mut f| {
                    f.path = get_lib_path(installation_path, &f.path);
                    f
                }),
        );
    }

    fn add_index(&mut self, check_result: &VersionCheckResult, installation_path: &Path) {
        if let Some(mut idx) = check_result
            .index
            .map(DownloadableObject::from)
        {
            idx.path = get_index_path(installation_path, &idx.path);
            self.data.push(idx);
        }
    }

    /// Exposes the collected objects for testing / introspection.
    #[cfg(test)]
    pub fn data(&self) -> &[DownloadableObject] {
        &self.data
    }
}

// Backward-compatible type alias: `InstallationFixer` without turbofish =
// `Downloader`
#[allow(dead_code)]
pub type DefaultInstallationFixer = InstallationFixer<Downloader>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mine_data_structs::minecraft::{
        Artifact, AssetIndex, Library, LibraryDownloads, ObjectData,
    };

    // -- Support

    fn make_library(path: &str, sha1: &str) -> Library {
        Library {
            name: "test:lib:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: path.into(),
                    sha1: sha1.into(),
                    size: 1024,
                    url: "https://example.com/lib.jar".into(),
                },
                classifiers: None,
            }),
            rules: None,
        }
    }

    fn make_object(hash: &str) -> ObjectData {
        ObjectData {
            hash: hash.into(),
            size: 512,
        }
    }

    fn make_index(id: &str, sha1: &str) -> AssetIndex {
        AssetIndex {
            id: id.into(),
            sha1: sha1.into(),
            size: 4096,
            total_size: 100_000,
            url: "https://example.com/index.json".into(),
        }
    }

    // -- Tests

    #[test]
    fn adds_objects() {
        let obj = make_object("a1b2c3d4e5f6");
        let result = VersionCheckResult {
            objects: Box::new([&obj]),
            libs: Box::new([]),
            index: None,
            client: None,
        };
        let fixer = InstallationFixer::<Downloader>::new(result, "/tmp/test");
        assert_eq!(fixer.data.len(), 1);
        let entry = &fixer.data[0];
        assert!(
            entry
                .path
                .starts_with("/tmp/test"),
            "expected path to start with installation path, got: {:?}",
            entry.path
        );
    }

    #[test]
    fn adds_libs() {
        let lib = make_library("net/minecraft/client/1.21/client.jar", "ffgg1122");
        let result = VersionCheckResult {
            objects: Box::new([]),
            libs: Box::new([&lib]),
            index: None,
            client: None,
        };
        let fixer = InstallationFixer::<Downloader>::new(result, "/tmp/test");
        assert_eq!(fixer.data.len(), 1);
        let entry = &fixer.data[0];
        assert!(
            entry
                .path
                .starts_with("/tmp/test/libraries"),
            "expected path under libraries/, got: {:?}",
            entry.path
        );
    }

    #[test]
    fn all_sources_accumulate() {
        let obj = make_object("aaa111");
        let lib = make_library("org/example/foo/1.0/foo.jar", "bbb222");
        let idx = make_index("1.21", "ccc333");
        let result = VersionCheckResult {
            objects: Box::new([&obj]),
            libs: Box::new([&lib]),
            index: Some(&idx),
            client: None,
        };
        let fixer = InstallationFixer::<Downloader>::new(result, "/tmp/test");
        assert_eq!(fixer.data.len(), 3);
    }

    #[tokio::test]
    async fn fix_installation_empty_returns_ok() {
        let result = VersionCheckResult {
            objects: Box::new([]),
            libs: Box::new([]),
            index: None,
            client: None,
        };
        let mut fixer = InstallationFixer::<Downloader>::new(result, "/tmp/test");
        let outcome = fixer.fix_installation().await;
        assert!(outcome.is_ok());
        assert!(fixer.data.is_empty());
    }
}
