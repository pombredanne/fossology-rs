use std::{fs::File, io, path::Path};

use sha2::{Digest, Sha256};

pub fn hash256_for_path<P: AsRef<Path>>(path: P) -> String {
    let mut file = File::open(path).unwrap();
    let mut sha256 = Sha256::new();
    io::copy(&mut file, &mut sha256).unwrap();
    let hash: sha2::digest::generic_array::GenericArray<u8, <Sha256 as Digest>::OutputSize> =
        sha256.finalize();

    hex::encode_upper(hash)
}
