use std::path::PathBuf;

use uranium_rs::downloaders::{Downloader, MinecraftDownloader};
use uranium_rs::init_logger;
use uranium_rs::version_checker::InstallationVerifier;

const VERSION: &str = "1.21.7";
#[tokio::test]
pub async fn test1() {
    let _ = init_logger();

    let mut md = MinecraftDownloader::<Downloader>::init("/home/sergio/.minecraft/", VERSION)
        .await
        .unwrap();
    let res = md.start().await;

    if res.is_ok() {
        let x =
            InstallationVerifier::new(&PathBuf::from("/home/sergio/.minecraft/"), VERSION).await;
        let _ = x.map(|iv| {
            iv.verify();
        });
    } else {
        panic!("Could not download minecraft")
    }
}
