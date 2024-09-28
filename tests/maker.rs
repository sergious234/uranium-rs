use std::path::PathBuf;

use uranium::make_modpack;

#[tokio::test]
async fn make() {
    let pack_name = PathBuf::from("tests/test1.mrpack");

    assert!(make_modpack("tests/data/minecraft_test1/", &pack_name)
        .await
        .is_ok());
    assert!(std::fs::exists(&pack_name).unwrap());
    std::fs::remove_file(&pack_name).unwrap();
}

#[tokio::test]
async fn make_and_download_without_ext() {
    let pack_name = PathBuf::from("tests/test2");
    let pack_name_ext = PathBuf::from("tests/test2.mrpack");

    if let Err(e) = make_modpack("tests/data/minecraft_test1/", &pack_name).await {
        panic!("Error happened while making the modpack {e}")
    }
    assert!(std::fs::exists(&pack_name_ext).unwrap());

    std::fs::remove_file(&pack_name_ext).unwrap();
}
