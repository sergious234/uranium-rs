use std::path::PathBuf;

use uranium_rs::downloaders::{Downloader, MinecraftDownloader};
use uranium_rs::init_logger;
use uranium_rs::version_checker::InstallationVerifier;

const VERSION: &str = "1.21.7";
const PATH: &str = "./data/minecraft_test1/";
#[tokio::test]
pub async fn test1() {
    let _ = init_logger();

    let mut md = MinecraftDownloader::<Downloader>::init(PATH, VERSION)
        .await
        .unwrap();
    let res = md.start().await;

    if res.is_ok() {
        let x =
            InstallationVerifier::new(&PathBuf::from(PATH), VERSION).await;
        let _ = x.map(|iv| {
            iv.verify();
        });
    } else {
        eprintln!("{}", res.err().unwrap());
        panic!("Could not download minecraft")
    }
}
