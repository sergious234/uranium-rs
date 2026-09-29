//! Shared loader-profile pipeline (internal).
//!
//! Fabric and Quilt both expose `/profile/json` with the same shape: a Mojang
//! profile (`id`, `inheritsFrom`, `mainClass`, `arguments`) whose libraries
//! are vendor Maven entries (`name`, repo `url`, optional `sha1`/`size`).
//! This module fetches that profile, converts the libraries, queues their
//! downloads, and writes the profile JSON for the launcher.

use std::path::PathBuf;

use log::warn;
use mine_data_structs::minecraft::{Artifact, Library, LibraryDownloads};
use serde::Deserialize;

use super::LoaderCtx;
use crate::engine::{DownloadableObject, FileDownloader};
use crate::error::{Result, UraniumError};
use crate::minecraft::libraries::prepare_libraries;

// region:    --- Vendor shape

/// One vendor library entry (`group:artifact:version` + repo base URL).
#[derive(Debug, Deserialize)]
pub(crate) struct MavenLib {
    pub name: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Vendor profile response (`/profile/json`).
#[derive(Debug, Deserialize)]
pub(crate) struct LoaderProfile {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    pub libraries: Vec<MavenLib>,
}

/// Maven coordinates resolved to a download URL path and an instance path.
pub(crate) struct MavenTarget {
    pub url_path: String,
    pub file_path: PathBuf,
}

// endregion: --- Vendor shape

// region:    --- Pipeline

/// Fetches, converts, queues and writes a loader profile (internal).
#[doc(hidden)]
pub(crate) async fn install_profile<T: FileDownloader>(
    ctx: &mut LoaderCtx<'_, T>,
    profile_url: &str,
) -> Result<String> {
    // -- Fetch vendor profile
    let profile: LoaderProfile = ctx
        .requester
        .get(profile_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // -- Convert libraries; hash-less entries ride along for the JSON
    let mut libs = Vec::with_capacity(profile.libraries.len());
    for lib in &profile.libraries {
        let target = maven_target(&lib.name)
            .ok_or_else(|| UraniumError::other(format!("bad maven coordinates: {}", lib.name)))?;
        let jar_url = format!("{}{}", repo_url(&lib.url), target.url_path);
        let sha1 = resolve_sha1(ctx.requester, &jar_url, lib.sha1.as_deref()).await;
        let downloads = sha1.map(|s| LibraryDownloads {
            artifact: Artifact {
                path: target.file_path.clone(),
                sha1: s,
                size: lib.size.unwrap_or(0),
                url: jar_url.clone(),
            },
            classifiers: None,
        });
        if downloads.is_none() {
            warn!("{jar_url} has no hash, downloading unverified");
            ctx.downloader
                .add_object(DownloadableObject::new(
                    &jar_url,
                    &ctx.instance_dir
                        .join("libraries")
                        .join(&target.file_path),
                    None,
                ));
        }
        libs.push(Library {
            name: lib.name.clone(),
            downloads,
            rules: None,
        });
    }

    // -- Queue hashed entries (prepare_libraries skips downloads-less ones)
    let queued: Vec<DownloadableObject> = prepare_libraries(&libs, ctx.instance_dir)?.collect();
    ctx.downloader
        .add_objects(queued);

    // -- Write the profile JSON for the launcher
    let dir = ctx
        .instance_dir
        .join("versions")
        .join(&profile.id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{}.json", profile.id)),
        serde_json::to_string(&serde_json::json!({
            "id": profile.id,
            "inheritsFrom": profile.inherits_from,
            "mainClass": profile.main_class,
            "arguments": profile.arguments,
            "libraries": libs,
        }))?,
    )?;

    // -- Hand the launch target back
    Ok(profile.id)
}

/// Resolves `group:artifact:version[:classifier]` to URL and file paths
/// (internal).
#[doc(hidden)]
pub(crate) fn maven_target(name: &str) -> Option<MavenTarget> {
    // -- Split coordinates
    let parts: Vec<&str> = name.split(':').collect();
    let (group, artifact, version, classifier) = match parts.as_slice() {
        [g, a, v] => (*g, *a, *v, None),
        [g, a, v, c] => (*g, *a, *v, Some(*c)),
        _ => return None,
    };
    if group.is_empty() || artifact.is_empty() || version.is_empty() {
        return None;
    }

    // -- Build file name and paths (URLs always use '/')
    let filename = match classifier {
        Some(c) if !c.is_empty() => format!("{artifact}-{version}-{c}.jar"),
        _ => format!("{artifact}-{version}.jar"),
    };
    let group_path = group.replace('.', "/");
    Some(MavenTarget {
        url_path: format!("{group_path}/{artifact}/{version}/{filename}"),
        file_path: PathBuf::from(group_path)
            .join(artifact)
            .join(version)
            .join(filename),
    })
}

/// Prefers the profile sha1, else tries the Maven `.sha1` sidecar (internal).
#[doc(hidden)]
pub(crate) async fn resolve_sha1(
    requester: &reqwest::Client,
    jar_url: &str,
    known: Option<&str>,
) -> Option<String> {
    // -- Inline hash wins
    if let Some(s) = known {
        return Some(s.to_owned());
    }

    // -- Try the sidecar, tolerating `<hash>  <filename>` format
    let text = requester
        .get(format!("{jar_url}.sha1"))
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let hash = text
        .split_whitespace()
        .next()?;
    (hash.len() == 40
        && hash
            .chars()
            .all(|c| c.is_ascii_hexdigit()))
    .then(|| hash.to_owned())
}

fn repo_url(base: &str) -> String {
    if base.ends_with('/') {
        base.to_owned()
    } else {
        format!("{base}/")
    }
}

// endregion: --- Pipeline

// region:    --- Tests

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    const FABRIC_PROFILE: &str = r#"{
        "id": "fabric-loader-0.16.9-1.21",
        "inheritsFrom": "1.21",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
        "libraries": [
            {"name": "org.ow2.asm:asm:9.7.1", "url": "https://maven.fabricmc.net/",
             "sha1": "f0ed132a49244b042cd0e15702ab9f2ce3cc8436", "size": 126093},
            {"name": "net.fabricmc:intermediary:1.21", "url": "https://maven.fabricmc.net/"}
        ]
    }"#;

    const QUILT_PROFILE: &str = r#"{
        "id": "quilt-loader-0.26.3-1.21",
        "inheritsFrom": "1.21",
        "type": "release",
        "mainClass": "org.quiltmc.loader.impl.launch.knot.KnotClient",
        "arguments": {"game": []},
        "libraries": [
            {"name": "org.quiltmc:quilt-loader:0.26.3",
             "url": "https://maven.quiltmc.org/repository/release/"}
        ]
    }"#;

    #[test]
    fn parses_fabric_profile() -> TestResult<()> {
        let p: LoaderProfile = serde_json::from_str(FABRIC_PROFILE)?;
        assert_eq!(p.id, "fabric-loader-0.16.9-1.21");
        assert_eq!(p.inherits_from, "1.21");
        assert_eq!(p.libraries.len(), 2);
        assert!(p.libraries[0].sha1.is_some());
        assert!(p.libraries[1].sha1.is_none());
        Ok(())
    }

    #[test]
    fn parses_quilt_profile() -> TestResult<()> {
        let p: LoaderProfile = serde_json::from_str(QUILT_PROFILE)?;
        assert_eq!(p.id, "quilt-loader-0.26.3-1.21");
        assert_eq!(
            p.main_class,
            "org.quiltmc.loader.impl.launch.knot.KnotClient"
        );
        Ok(())
    }

    #[test]
    fn maven_target_layout() -> TestResult<()> {
        let t = maven_target("org.ow2.asm:asm:9.7.1").ok_or("should parse")?;
        assert_eq!(t.url_path, "org/ow2/asm/asm/9.7.1/asm-9.7.1.jar");
        assert_eq!(
            t.file_path,
            Path::new("org/ow2/asm/asm/9.7.1/asm-9.7.1.jar")
        );
        Ok(())
    }

    #[test]
    fn maven_target_classifier_and_rejects_junk() -> TestResult<()> {
        let t = maven_target("a.b:c:1.0:linux").ok_or("should parse")?;
        assert_eq!(t.url_path, "a/b/c/1.0/c-1.0-linux.jar");
        assert!(maven_target("noversion").is_none());
        assert!(maven_target("a:b:c:d:e").is_none());
        assert!(maven_target(":b:1.0").is_none());
        Ok(())
    }

    #[test]
    fn repo_url_slash() {
        assert_eq!(repo_url("https://x/"), "https://x/");
        assert_eq!(repo_url("https://x"), "https://x/");
    }
}

// endregion: --- Tests
