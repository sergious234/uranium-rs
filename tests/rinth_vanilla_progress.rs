#![cfg(feature = "integration-tests")]

use uranium_rs::engine::{Downloader, FileDownloader};
use uranium_rs::modpacks::rinth::{RinthInstallState, RinthInstaller, RinthModpack, Side};

/// The vanilla phase must stream per-batch progress, not one-shot.
///
/// Old behavior: the second poll downloaded all of vanilla and jumped to
/// `CopyingOverrides`. New behavior: each poll runs one vanilla batch and
/// re-emits `InstallingMinecraft` with a live queue count. Stops after two
/// polls (no full download needed to prove streaming).
#[tokio::test]
async fn vanilla_phase_streams_progress() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = RinthModpack::new_with("1.0.0".to_owned(), "progress-pack".into(), vec![]);
    manifest
        .dependencies
        .insert("minecraft".to_owned(), "1.21.7".to_owned());
    let mut dl = RinthInstaller::from_pack(manifest, dir.path(), Downloader::new(), Side::Client);

    for i in 0..2 {
        assert!(matches!(
            dl.progress()
                .await
                .expect("vanilla poll should succeed"),
            RinthInstallState::InstallingMinecraft
        ));
        if i == 0 {
            assert!(
                dl.requests_left() > 0,
                "vanilla queue should still be draining"
            );
        }
    }
}
