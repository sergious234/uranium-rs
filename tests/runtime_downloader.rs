use uranium_rs::downloaders::RuntimeDownloader;

#[tokio::test]
async fn download_runtime() {
    let mut runtime_downloader = RuntimeDownloader::new("java-runtime-beta".to_owned());

    let x = runtime_downloader
        .download()
        .await;
    if let Err(e) = &x {
        println!("{e}");
    }

    assert!(x.is_ok())
}
