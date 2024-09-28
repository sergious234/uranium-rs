use std::path::PathBuf;
use uranium::downloaders::{Downloader, RinthDownloader};
use uranium::{make_modpack, rinth_pack_download};

#[tokio::test]
async fn make_and_download() {
    let pack_name = PathBuf::from("../tests/test1.mrpack");

    assert!(make_modpack("../tests/data/minecraft_test1/", &pack_name)
        .await
        .is_ok());
    assert!(std::fs::exists(&pack_name).unwrap());

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
    std::fs::remove_file(&pack_name).unwrap();
}

#[tokio::test]
async fn make_and_download_without_ext() {
    let pack_name = PathBuf::from("../tests/test2");
    let pack_name_ext = PathBuf::from("../tests/test2.mrpack");

    assert!(make_modpack("../tests/data/minecraft_test1/", &pack_name)
        .await
        .is_ok());
    assert!(std::fs::exists(&pack_name_ext).unwrap());

    std::fs::remove_file(&pack_name_ext).unwrap();
}
