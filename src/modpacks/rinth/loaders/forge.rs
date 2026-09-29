//! Forge loader install (not supported yet).
//!
//! Forge needs Maven version-list resolution plus installer jars whose
//! processors execute under a JVM at install time — deferred long-pole work.
//! This stub keeps dispatch total until then.

use mine_data_structs::rinth::LoaderKind;

use super::{LoaderCtx, LoaderInstallStep};
use crate::engine::FileDownloader;
use crate::error::{Result, UraniumError};

/// Placeholder for a future Forge installer.
pub struct ForgeStep;

impl LoaderInstallStep for ForgeStep {
    fn loader(&self) -> LoaderKind {
        LoaderKind::Forge
    }

    async fn install<T: FileDownloader>(&mut self, _ctx: &mut LoaderCtx<'_, T>) -> Result<String> {
        Err(UraniumError::other(
            "forge loader installation is not supported yet",
        ))
    }
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn forge_is_unsupported() -> TestResult<()> {
        let tmp = tempfile::tempdir()?;
        let requester = reqwest::Client::new();
        let mut dl = crate::engine::Downloader::new();
        let mut ctx = LoaderCtx {
            instance_dir: tmp.path(),
            mc_version: "1.21",
            loader_version: "47.0.0",
            downloader: &mut dl,
            requester: &requester,
        };
        let err = match ForgeStep
            .install(&mut ctx)
            .await
        {
            Err(e) => e,
            Ok(_) => return Err("should fail".into()),
        };
        assert!(
            err.to_string()
                .contains("not supported yet")
        );
        Ok(())
    }
}

// endregion: --- Tests
