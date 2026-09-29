use std::path::Path;

use uranium_rs::{make_modpack, modpacks::rinth::ModpackMaker};

const MODS_PATHS: &str = "tests/data/minecraft_test1/";

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// -- Helpers

fn assert_mrpack_valid(path: &Path) -> Result<()> {
    assert!(path.exists(), "expected mrpack at {path:?} to exist");
    let file = std::fs::File::open(path).map_err(|e| format!("open {path:?}: {e}"))?;
    let zip = zip::ZipArchive::new(file).map_err(|e| format!("invalid zip {path:?}: {e}"))?;
    let names: Vec<String> = zip
        .file_names()
        .map(|s| s.to_string())
        .collect();
    assert!(
        names
            .iter()
            .any(|n| n == "modrinth.index.json"),
        "missing modrinth.index.json in {names:?}"
    );
    // overrides/ should exist (even if empty, writer adds directory)
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("overrides/")),
        "missing overrides/ in {names:?}"
    );
    Ok(())
}

// -- Tests

#[tokio::test]
async fn make_with_extension() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let pack = dir
        .path()
        .join("test1.mrpack");

    make_modpack(MODS_PATHS, &pack)
        .await
        .map_err(|e| format!("make_modpack failed: {e}"))?;

    assert_mrpack_valid(&pack)?;
    // TempDir auto-cleans on drop
    Ok(())
}

#[tokio::test]
async fn make_without_extension_appends_mrpack() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let pack_without_ext = dir.path().join("test2");
    let pack_expected = dir
        .path()
        .join("test2.mrpack");

    // -- ModpackMaker should append .mrpack when missing
    let maker = ModpackMaker::new(MODS_PATHS, &pack_without_ext);
    maker
        .finish()
        .await
        .map_err(|e| format!("ModpackMaker::finish failed: {e}"))?;

    assert_mrpack_valid(&pack_expected)?;
    assert!(
        !pack_without_ext.exists() || pack_without_ext == pack_expected,
        "should not create file without extension"
    );
    Ok(())
}
