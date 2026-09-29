//! Copies `overrides/` plus the side-matching override folder into the
//! instance.

use std::path::Path;

use mine_data_structs::rinth::Side;
use walkdir::WalkDir;

use super::super::installer::RinthInstallState;
use crate::common::constants::{
    CLIENT_OVERRIDES_FOLDER, OVERRIDES_FOLDER, SERVER_OVERRIDES_FOLDER,
};
use crate::engine::FileDownloader;
use crate::error::Result;
use crate::modpacks::rinth::steps::{RinthCtx, RinthStep};

/// Copies overrides for the installer's `Side`.
pub(crate) struct OverridesStep;

impl RinthStep for OverridesStep {
    async fn run<T: FileDownloader>(
        &mut self,
        ctx: &mut RinthCtx<'_, T>,
    ) -> Result<RinthInstallState> {
        copy_overrides(ctx.tmp, ctx.instance_dir, ctx.side)?;
        Ok(RinthInstallState::DownloadingFiles)
    }
}

/// Copies applicable override folders from the extracted pack (internal).
#[doc(hidden)]
pub(crate) fn copy_overrides(tmp: Option<&Path>, dest: &Path, side: Side) -> Result<()> {
    // -- Nothing to copy without an extracted pack
    let Some(tmp) = tmp else { return Ok(()) };

    // -- Select folders for the side
    let mut folders = vec![OVERRIDES_FOLDER];
    match side {
        Side::Client => folders.push(CLIENT_OVERRIDES_FOLDER),
        Side::Server => folders.push(SERVER_OVERRIDES_FOLDER),
    }

    // -- Copy each present folder, preserving relative layout
    for folder in folders {
        let src = tmp.join(folder);
        if !src.exists() {
            continue;
        }
        for entry in WalkDir::new(&src)
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(&src)
                .map_err(|_| {
                    crate::error::UraniumError::OtherWithReason(
                        "override file outside pack dir".to_string(),
                    )
                })?;
            if rel.as_os_str().is_empty() {
                continue;
            }
            let target = dest.join(rel);
            if path.is_dir() {
                std::fs::create_dir_all(&target)?;
            } else if path.is_file() {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(path, &target)?;
            }
        }
    }
    Ok(())
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    fn seed_pack(tmp: &Path) -> TestResult<()> {
        for (folder, file) in [
            ("overrides", "config/common.cfg"),
            ("client-overrides", "config/client.cfg"),
            ("server-overrides", "config/server.cfg"),
        ] {
            let p = tmp.join(folder).join(file);
            std::fs::create_dir_all(
                p.parent()
                    .ok_or("should have parent")?,
            )?;
            std::fs::write(&p, folder)?;
        }
        Ok(())
    }

    #[test]
    fn client_side_copies_client_overrides() -> TestResult<()> {
        let tmp = tempfile::tempdir()?;
        let dest = tempfile::tempdir()?;
        seed_pack(tmp.path())?;
        copy_overrides(Some(tmp.path()), dest.path(), Side::Client)?;

        let read = |f: &str| std::fs::read_to_string(dest.path().join(f));
        assert_eq!(read("config/common.cfg")?.as_str(), "overrides");
        assert_eq!(read("config/client.cfg")?.as_str(), "client-overrides");
        assert!(read("config/server.cfg").is_err());
        Ok(())
    }

    #[test]
    fn server_side_copies_server_overrides() -> TestResult<()> {
        let tmp = tempfile::tempdir()?;
        let dest = tempfile::tempdir()?;
        seed_pack(tmp.path())?;
        copy_overrides(Some(tmp.path()), dest.path(), Side::Server)?;

        let read = |f: &str| std::fs::read_to_string(dest.path().join(f));
        assert_eq!(read("config/common.cfg")?.as_str(), "overrides");
        assert_eq!(read("config/server.cfg")?.as_str(), "server-overrides");
        assert!(read("config/client.cfg").is_err());
        Ok(())
    }

    #[test]
    fn missing_tmp_is_noop() -> TestResult<()> {
        let dest = tempfile::tempdir()?;
        copy_overrides(None, dest.path(), Side::Client)?;
        Ok(())
    }
}

// endregion: --- Tests
