#![cfg(feature = "integration-tests")]

use uranium_rs::engine::{Downloader, FileDownloader};
use uranium_rs::modpacks::rinth::{FabricStep, LoaderCtx, LoaderInstallStep};

#[tokio::test]
async fn installs_fabric_loader() {
    let tmp = tempfile::tempdir().unwrap();
    let requester = reqwest::Client::new();
    let mut dl = Downloader::new();
    let mut ctx = LoaderCtx {
        instance_dir: tmp.path(),
        mc_version: "1.21",
        loader_version: "0.16.9",
        downloader: &mut dl,
        requester: &requester,
    };

    let id = FabricStep
        .install(&mut ctx)
        .await
        .expect("fabric install should succeed");
    assert_eq!(id, "fabric-loader-0.16.9-1.21");
    drop(ctx);

    dl.start()
        .await
        .expect("loader libraries should download");

    assert!(
        tmp.path()
            .join("versions/fabric-loader-0.16.9-1.21/fabric-loader-0.16.9-1.21.json")
            .exists()
    );
    assert!(
        tmp.path()
            .join("libraries/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar")
            .exists()
    );
}
