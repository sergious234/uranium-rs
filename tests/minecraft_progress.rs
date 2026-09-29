#![cfg(feature = "integration-tests")]

use uranium_rs::engine::Downloader;
use uranium_rs::minecraft::{MinecraftDownloadState, MinecraftDownloader};

/// The version step must queue the client jar, not drain the world.
///
/// Old behavior: the `DownloadingVersion` poll downloaded the client jar
/// *and* every queued asset inside `ClientStep`, leaving nothing to stream.
/// New behavior: it queues the client jar and returns, so the asset phase
/// drains per batch with a live count. Metadata fetches only.
#[tokio::test]
async fn version_step_queues_without_draining() {
    let dir = tempfile::tempdir().unwrap();
    let mut dl = MinecraftDownloader::<Downloader>::init(dir.path(), "1.21.7")
        .await
        .expect("init should succeed");

    assert!(matches!(
        dl.progress()
            .await
            .expect("sources poll should succeed"),
        MinecraftDownloadState::DownloadingVersion
    ));
    assert!(matches!(
        dl.progress()
            .await
            .expect("version poll should succeed"),
        MinecraftDownloadState::DownloadingAssets
    ));
    assert!(
        dl.requests_left() > 0,
        "client + assets should still be queued"
    );
}
