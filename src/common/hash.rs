use std::io::Read;
use std::{fs, path::Path};

use sha1::{Digest, Sha1};

use crate::error::Result;

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn get_sha1_from_file<I: AsRef<Path>>(path: I) -> Result<String> {
    let mut hasher = Sha1::new();
    let mut file = fs::File::open(&path)?;
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(bytes_to_hex(&hasher.finalize()))
}

pub fn rinth_hash(path: &Path) -> Result<String> {
    get_sha1_from_file(path)
}
