use std::{fs, io::Read, path::Path};

use hex::ToHex;
use murmurhash32::murmurhash2;
use sha1::{Digest, Sha1};

// TODO: 
// Remove unwraps

fn get_sha1_from_file<I: AsRef<Path>>(file_path: I) -> String {
    let mut hasher = Sha1::new();
    let mut file = fs::File::open(&file_path).unwrap();

    let metadata = fs::metadata(&file_path).unwrap();

    // let mut buffer = Vec::with_capacity(
    //     metadata
    //         .len()
    //         .try_into()
    //         .unwrap_or_default(),
    // ); //vec![0; metadata.len() as usize];

    let mut buffer = vec![0; metadata.len() as usize];
    buffer.clear();

    let _ = file.read_to_end(&mut buffer);

    hasher.update(buffer);
    let temp = hasher.finalize().to_vec();
    temp.encode_hex::<String>()
}

pub fn rinth_hash(path: &Path) -> String {
    get_sha1_from_file(path)
}

// TODO! Remove curse
pub fn _curse_hash(path: &String) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .unwrap();
    buffer.retain(|&x| (x != 9 && x != 10 && x != 13 && x != 32));
    murmurhash2(&buffer).to_string()
}
