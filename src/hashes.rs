use std::io::Read;
use std::{fs, path::Path};

use sha1::{Digest, Sha1};

use crate::error::Result;

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex_string = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex_string.push_str(&format!("{byte:02x}"));
    }
    hex_string
}

fn get_sha1_from_file<I: AsRef<Path>>(file_path: I) -> Result<String> {
    let mut hasher = Sha1::new();
    let mut file = fs::File::open(&file_path)?;

    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(bytes_to_hex(&hasher.finalize()))
}

pub fn rinth_hash(path: &Path) -> Result<String> {
    get_sha1_from_file(path)
}
