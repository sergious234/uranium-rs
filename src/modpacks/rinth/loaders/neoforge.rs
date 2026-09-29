//! NeoForge loader install (not supported yet).
//!
//! Same long pole as Forge: Maven resolution plus installer processors that
//! execute under a JVM at install time. Stub keeps dispatch total until then.

use mine_data_structs::rinth::LoaderKind;

use super::{LoaderCtx, LoaderInstallStep};
use crate::engine::FileDownloader;
use crate::error::{Result, UraniumError};

/// Placeholder for a future NeoForge installer.
pub struct NeoForgeStep;

impl LoaderInstallStep for NeoForgeStep {
    fn loader(&self) -> LoaderKind {
        LoaderKind::NeoForge
    }

    async fn install<T: FileDownloader>(&mut self, _ctx: &mut LoaderCtx<'_, T>) -> Result<String> {
        Err(UraniumError::other(
            "neoforge loader installation is not supported yet",
        ))
    }
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T> = core::result::Result<T, Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn neoforge_is_unsupported() -> TestResult<()> {
        let tmp = tempfile::tempdir()?;
        let requester = reqwest::Client::new();
        let mut dl = crate::engine::Downloader::new();
        let mut ctx = LoaderCtx {
            instance_dir: tmp.path(),
            mc_version: "1.21",
            loader_version: "21.0.0",
            downloader: &mut dl,
            requester: &requester,
        };
        let err = match NeoForgeStep
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
