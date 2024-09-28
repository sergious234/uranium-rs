const BASE_CUR_URL: &str = "https://api.curseforge.com";

pub struct Curse;

impl Curse {
    pub fn file(mod_id: &str, file_id: &str) -> String {
        format!("{}/v1/mods/{}/files/{}", BASE_CUR_URL, mod_id, file_id)
    }

    pub fn hash() -> String {
        format!("{}/v1/fingerprints", BASE_CUR_URL)
    }
}
