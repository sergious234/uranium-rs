//! So I had the necesity of something that based on a Curse `mod_id`
//! and `file_id` would return me the `URL` for the CurseForge API.

const BASE_CUR_URL: &str = "https://api.curseforge.com";

pub fn curse_file(mod_id: &str, file_id: &str) -> String {
    format!("{}/v1/mods/{}/files/{}", BASE_CUR_URL, mod_id, file_id)
}

pub fn curse_hash() -> String {
    format!("{}/v1/fingerprints", BASE_CUR_URL)
}
