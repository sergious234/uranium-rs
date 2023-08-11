use std::path::Path;

use crate::{downloaders::updater::update_modpack, variables::constants::*};

#[allow(unused)]
#[deprecated]
pub async fn update(path: &Path) {
    update_modpack(path).await;
}

#[allow(non_snake_case)]
#[allow(unused)]
/// Returns the actual max threads allowed.
pub fn N_THREADS() -> usize {
    match NTHREADS.read() {
        Ok(e) => *e,
        Err(_) => DEFAULT_NTHREADS,
    }
}
