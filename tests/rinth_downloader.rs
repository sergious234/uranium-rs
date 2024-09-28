use std::path::PathBuf;

use uranium::downloaders::{Downloader, RinthDownloader};
use uranium::rinth_pack_download;

#[tokio::test]
async fn download_pack() {
    let pack_name = PathBuf::from("../tests/data/test1.mrpack");

    match RinthDownloader::<Downloader>::new(&pack_name, "../tests/data/unzipper_test/") {
        Ok(downloader) => {
            let contains_fabric = downloader
                .get_modpack()
                .get_mods()
                .iter()
                .any(|e| e.get_name() == "fabric-api-0.100.7+1.21.jar");
            let contains_sodium = downloader
                .get_modpack()
                .get_mods()
                .iter()
                .any(|e| e.get_name() == "sodium-fabric-0.5.11+mc1.21.jar");
            assert!(contains_sodium && contains_fabric);
        }
        Err(e) => {
            panic!("No downloader: {e}")
        }
    }

    assert!(
        rinth_pack_download(&pack_name, "../tests/data/unzipper_test")
            .await
            .is_ok()
    );

    assert!(
        std::fs::exists("../tests/data/minecraft_test1/mods/fabric-api-0.100.7+1.21.jar")
            .is_ok_and(|r| r)
    );
    assert!(
        std::fs::exists("../tests/data/unzipper_test/mods/fabric-api-0.100.7+1.21.jar")
            .is_ok_and(|r| r)
    );

    // Clear the mess we just made.
    std::fs::remove_dir_all("../tests/data/unzipper_test").unwrap();
    std::fs::create_dir("../tests/data/unzipper_test").unwrap();
}
