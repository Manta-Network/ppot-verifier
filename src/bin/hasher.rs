use memmap::MmapOptions;
use ppot_verifier::{calculate_hash, challenge_paths, response_paths};
use std::fs::OpenOptions; // TODO: Is standard okay?


const NUM_ROUNDS: usize = 71;

fn main() {
    let challenge_files = challenge_paths(NUM_ROUNDS);
    let response_files = response_paths(NUM_ROUNDS);
    let _challenge_hashes = hash_all(challenge_files);
    let _response_hashes = hash_all(response_files);

    // Write the subaccumulator to file
    // TODO: Can't figure out the best way to do this
}

/// Computes Blake2 hash of all files specified by a list
/// of paths, returning all hashes.
fn hash_all(files: Vec<String>) -> Vec<[u8; 64]> {
    let mut hashes = vec![[0u8; 64]; files.len()];
    // TODO: This can be parallelized
    for (i, file) in files.iter().enumerate() {
        let reader = OpenOptions::new()
            .read(true)
            .open(file)
            .expect("unable open file in this directory");
        // Make a memory map
        let challenge = unsafe {
            MmapOptions::new()
                .map(&reader)
                .expect("unable to create a memory map for input")
        };
        hashes[i] = calculate_hash(&challenge);
    }
    hashes
}
