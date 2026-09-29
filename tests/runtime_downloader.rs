#![cfg(feature = "integration-tests")]

use uranium_rs::engine::Downloader;
use uranium_rs::minecraft::runtime::RuntimeDownloader;

#[tokio::test]
async fn download_runtime() {
    let mut runtime_downloader =
        RuntimeDownloader::<Downloader>::new("java-runtime-beta".to_owned());

    let x = runtime_downloader
        .start()
        .await;
    if let Err(e) = &x {
        println!("{e}");
    }

    assert!(x.is_ok())
}
