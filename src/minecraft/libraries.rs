use std::env::consts::OS;
use std::path::Path;

use mine_data_structs::minecraft::{Artifact, Library};

use crate::engine::{DownloadableObject, HashType};
use crate::error::Result;

// region:    --- Libraries helpers (moved from installer.rs:637-696 +
// lib_chunks)

pub fn prepare_libraries(
    libs: &[Library],
    minecraft_path: &Path,
) -> Result<impl Iterator<Item = DownloadableObject>> {
    let lib_path = minecraft_path.join("libraries");

    fn extract_native(lib: &Library) -> Option<&Artifact> {
        if let Some(downloads) = &lib.downloads
            && let Some(classifiers) = &downloads.classifiers
        {
            return match OS {
                "linux"
                    if classifiers
                        .natives_linux
                        .is_some() =>
                {
                    classifiers
                        .natives_linux
                        .as_ref()
                }
                "windows"
                    if classifiers
                        .natives_windows
                        .is_some() =>
                {
                    classifiers
                        .natives_windows
                        .as_ref()
                }
                _ => None,
            };
        }
        None
    }

    let value = lib_path.clone();
    let natives = libs
        .iter()
        .flat_map(|l| extract_native(l))
        .map(move |l| {
            DownloadableObject::new(
                &l.url,
                &value.join(&l.path),
                Some(HashType::Sha1(l.sha1.clone())),
            )
        });

    Ok(libs
        .iter()
        .filter(|l| l.applies() && l.downloads.is_some())
        .flat_map(|l| l.downloads.as_ref())
        .map(|d| &d.artifact)
        .map(move |a| {
            DownloadableObject::new(
                &a.url,
                &lib_path.join(&a.path),
                Some(HashType::Sha1(a.sha1.clone())),
            )
        })
        .chain(natives))
}

// endregion: --- Libraries helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use mine_data_structs::minecraft::{
        Arguments, Artifact, AssetIndex, Classifiers, JavaVersion, Library, LibraryDownloads, Os,
        OsName, Root, Rule,
    };

    use super::*;

    // -- Support

    fn dummy_root(libraries: Box<[Library]>) -> Root {
        Root {
            arguments: Arguments {
                game: Box::new([]),
                jvm: Box::new([]),
            },
            asset_index: AssetIndex {
                id: "test".into(),
                sha1: "000".into(),
                size: 0,
                total_size: 0,
                url: "https://example.com/index.json".into(),
            },
            assets: "test".into(),
            downloads: HashMap::new(),
            id: "test".into(),
            java_version: JavaVersion {
                component: "java-runtime-test".into(),
                major_version: 21,
            },
            libraries,
            inherits_from: None,
            main_class: "net.minecraft.client.main.Main".into(),
            version_type: "release".into(),
        }
    }

    fn lib_with_artifact(name: &str, path: &str, sha1: &str) -> Library {
        Library {
            name: name.into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from(path),
                    sha1: sha1.into(),
                    size: 100,
                    url: "https://example.com/lib.jar".into(),
                },
                classifiers: None,
            }),
            rules: None,
        }
    }

    // -- Tests

    #[test]
    fn prepare_libraries_filters_by_os_rules() {
        let lib_always = lib_with_artifact("always:lib:1.0", "always/lib/1.0/lib.jar", "aaa111");
        let lib_disallowed = Library {
            name: "never:lib:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("never/lib/1.0/lib.jar"),
                    sha1: "bbb222".into(),
                    size: 100,
                    url: "https://example.com/lib.jar".into(),
                },
                classifiers: None,
            }),
            rules: Some(Box::new([Rule {
                action: "disallow".into(),
                os: Some(Os {
                    name: Some(OsName::Linux),
                    arch: None,
                }),
            }])),
        };

        let instance = dummy_root(Box::new([lib_always, lib_disallowed]));

        let objects: Vec<DownloadableObject> =
            prepare_libraries(&instance.libraries, &PathBuf::from("/test"))
                .unwrap()
                .collect();

        let names: Vec<_> = objects
            .iter()
            .map(|o| o.path.to_string_lossy())
            .collect();
        assert!(
            names
                .iter()
                .any(|p| p.contains("always")),
            "expected always, got: {names:?}"
        );
        assert!(
            !names
                .iter()
                .any(|p| p.contains("never")),
            "expected never excluded, got: {names:?}"
        );
    }

    #[test]
    fn prepare_libraries_includes_natives() {
        let lib_with_natives = Library {
            name: "natives:test:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: Artifact {
                    path: PathBuf::from("natives/main.jar"),
                    sha1: "ccc333".into(),
                    size: 100,
                    url: "https://example.com/main.jar".into(),
                },
                classifiers: Some(Classifiers {
                    natives_linux: Some(Artifact {
                        path: PathBuf::from("natives/linux/native.so"),
                        sha1: "ddd444".into(),
                        size: 50,
                        url: "https://example.com/native.so".into(),
                    }),
                    natives_windows: None,
                    natives_macos: None,
                }),
            }),
            rules: None,
        };

        let instance = dummy_root(Box::new([lib_with_natives]));

        let objects: Vec<DownloadableObject> =
            prepare_libraries(&instance.libraries, &PathBuf::from("/test"))
                .unwrap()
                .collect();

        let paths: Vec<_> = objects
            .iter()
            .map(|o| {
                o.path
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(
            paths
                .iter()
                .any(|p| p.contains("main.jar")),
            "expected main, got: {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| p.contains("native.so")),
            "expected native, got: {paths:?}"
        );
    }

    #[test]
    fn prepare_libraries_skips_library_without_downloads() {
        let lib_no_downloads = Library {
            name: "empty:lib:1.0".into(),
            downloads: None,
            rules: None,
        };
        let lib_with = lib_with_artifact("has:downloads:1.0", "has/downloads.jar", "eee555");

        let instance = dummy_root(Box::new([lib_no_downloads, lib_with]));

        let objects: Vec<DownloadableObject> =
            prepare_libraries(&instance.libraries, &PathBuf::from("/test"))
                .unwrap()
                .collect();

        let paths: Vec<_> = objects
            .iter()
            .map(|o| {
                o.path
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(
            !paths
                .iter()
                .any(|p| p.contains("empty")),
            "expected empty skipped, got: {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| p.contains("has")),
            "expected has, got: {paths:?}"
        );
    }
}

// endregion: --- Tests
