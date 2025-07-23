use std::path::PathBuf;

use uranium_rs::init_logger;
use uranium_rs::version_checker::InstallationVerifier;

#[tokio::test]
pub async fn test1() {
    let _ = init_logger();
    let x = InstallationVerifier::new(&PathBuf::from("/home/sergio/.minecraft/"), "1.21.1").await;
    let _ = x.map(|iv| {
        iv.verify();
    });
}
