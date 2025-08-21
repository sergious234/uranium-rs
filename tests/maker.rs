use std::path::PathBuf;

use uranium_rs::{init_logger, make_modpack};

const MODS_PATHS: &str = "tests/data/minecraft_test1/";

async fn make() {
    println!("{:?}", std::env::current_dir());
    let pack_name = PathBuf::from("tests/test1.mrpack");

    if let Err(e) = make_modpack("tests/data/minecraft_test1/", &pack_name).await {
        panic!("Something went wrong when making the modpack {e}");
    }
    assert!(std::fs::exists(&pack_name).unwrap());
    std::fs::remove_file(&pack_name).unwrap();
}


#[tokio::test]
async fn make_and_download_without_ext() {
    use uranium_rs::modpack_maker::ModpackMaker2;

    let pack_name = PathBuf::from("tests/test2");
    let pack_name_ext = PathBuf::from("tests/test2.mrpack");

    let _ = init_logger();
    let maker = ModpackMaker2::new(MODS_PATHS, &pack_name); 
    if let Err(e) = maker.finish().await {
        panic!("Error happened while making the modpack {e}");
    }

    //if let Err(e) = make_modpack("tests/data/minecraft_test1/", &pack_name).await {
    //    eprintln!("Error happened while making the modpack {e}");
    //    return;
    //}
    assert!(std::fs::exists(&pack_name_ext).unwrap());

    std::fs::remove_file(&pack_name_ext).unwrap();
}
