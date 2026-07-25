use std::fmt::Write;
use std::io::Read;
use std::{fs, path::Path};

use murmurhash32::murmurhash2;
use sha1::{Digest, Sha1};

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex_string = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut hex_string, "{:02x}", byte).unwrap();
    }
    hex_string
}

fn get_sha1_from_file<I: AsRef<Path>>(file_path: I) -> String {
    let mut hasher = Sha1::new();
    let mut file = fs::File::open(&file_path).expect("get_sha1_from_file: failed to open file");

    let mut buffer = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buffer)
            .expect("get_sha1_from_file: failed to read file");
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let temp = hasher.finalize().to_vec();
    bytes_to_hex(&temp)
}

// This function is coded like shit, remember to check if file exists before
// using it or it will panic.
//
// For sure I need to fix it.
pub fn rinth_hash(path: &Path) -> String {
    get_sha1_from_file(path)
}

// TODO! Remove curse
pub fn _curse_hash(path: &String) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .unwrap();
    buffer.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
    murmurhash2(&buffer).to_string()
}
