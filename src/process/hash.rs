use macros_rs::crashln;
use merkle_hash::{bytes_to_hex, Algorithm, MerkleTree};
use std::path::PathBuf;

pub fn create(path: PathBuf) -> String {
    let tree = match MerkleTree::builder(&path.to_str().unwrap()).algorithm(Algorithm::Blake3).hash_names(false).build() {
        Ok(v) => v,
        Err(e) => crashln!("Invalid UTF-8 sequence: {}", e),
    };

    bytes_to_hex(tree.root.item.hash)
}
