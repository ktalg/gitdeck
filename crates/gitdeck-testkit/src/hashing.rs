use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn sha256_hex_of_file(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("cannot read {} for hashing: {error}", path.display()));
    sha256_hex(&bytes)
}

#[cfg(test)]
mod hashing_tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_the_empty_and_known_digests() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }
}
