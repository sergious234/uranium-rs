use std::path::Path;

use crate::downloaders::{Downloader, FileDownloader, get_lib_path};
use crate::error::Result;

use crate::downloaders::get_index_path;
use crate::{downloaders::DownloadableObject, version_checker::VersionCheckResult};

pub struct InstallationFixer {
    data: Vec<DownloadableObject>,
}

impl InstallationFixer {
    pub fn new(check_result: VersionCheckResult, installation_path: impl AsRef<Path>) -> Self {
        let installation_path = installation_path.as_ref();
        let mut fixer = Self { data: vec![] };
        fixer.add_objects(&check_result, installation_path);
        fixer.add_libs(&check_result, installation_path);
        fixer.add_index(&check_result, installation_path);
        fixer
    }

    pub async fn fix_installation(&mut self) -> Result<()> {
        let files = std::mem::take(&mut self.data);

        let mut downloader = Downloader::new(files);
        downloader.complete().await
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
        if let Some(mut idx) = check_result.index.map(DownloadableObject::from) {
            idx.path = get_index_path(installation_path, &idx.path);
            self.data.push(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mine_data_structs::minecraft::{Artifact, AssetIndex, Library, LibraryDownloads, ObjectData};

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
            }),
            rules: None,
        }
    }

    fn make_object(hash: &str) -> ObjectData {
        ObjectData { hash: hash.into(), size: 512 }
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

    #[test]
    fn adds_objects() {
        let obj = make_object("a1b2c3d4e5f6");
        let result = VersionCheckResult {
            objects: Box::new([&obj]),
            libs: Box::new([]),
            index: None,
            client: None,
        };
        let fixer = InstallationFixer::new(result, "/tmp/test");

        assert_eq!(fixer.data.len(), 1);
        let entry = &fixer.data[0];
        assert!(
            entry.path.starts_with("/tmp/test"),
            "expected path to start with installation path, got: {:?}",
            entry.path,
        );
        assert!(
            entry.path.to_string_lossy().contains("a1b2c3d4e5f6"),
            "expected path to contain the object hash, got: {:?}",
            entry.path,
        );
        assert!(
            entry.url.contains("resources.download.minecraft.net"),
            "expected URL to be a Minecraft resource URL, got: {}",
            entry.url,
        );
        assert!(entry.hash.is_some());
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
        let fixer = InstallationFixer::new(result, "/tmp/test");

        assert_eq!(fixer.data.len(), 1);
        let entry = &fixer.data[0];
        assert!(
            entry.path.starts_with("/tmp/test/libraries"),
            "expected path under libraries/, got: {:?}",
            entry.path,
        );
        assert!(
            entry.path.to_string_lossy().contains("net/minecraft/client/1.21/client.jar"),
            "expected path to contain artifact path, got: {:?}",
            entry.path,
        );
        assert_eq!(entry.url, "https://example.com/lib.jar");
        assert!(entry.hash.is_some());
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
        let fixer = InstallationFixer::new(result, "/tmp/test");

        assert_eq!(fixer.data.len(), 3);
        let paths: Vec<_> = fixer.data.iter().map(|d| d.path.to_string_lossy().to_string()).collect();
        assert!(paths.iter().any(|p| p.contains("aaa111")),     "object missing");
        assert!(paths.iter().any(|p| p.contains("org/example")), "library missing");
        assert!(paths.iter().any(|p| p.contains("1.21")),       "index id missing");
    }

    #[tokio::test]
    async fn fix_installation_empty_returns_ok() {
        let result = VersionCheckResult {
            objects: Box::new([]),
            libs: Box::new([]),
            index: None,
            client: None,
        };
        let mut fixer = InstallationFixer::new(result, "/tmp/test");

        let outcome = fixer.fix_installation().await;

        assert!(outcome.is_ok(), "expected Ok(()) with no downloads, got: {:?}", outcome);
        assert!(fixer.data.is_empty());
    }
}
