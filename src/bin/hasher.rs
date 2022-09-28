use memmap::MmapOptions;
use ppot_verifier::{calculate_hash, challenge_paths, response_paths};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;

const NUM_ROUNDS: usize = 72;

fn main() {
    let challenge_files = challenge_paths(NUM_ROUNDS);
    let response_files = response_paths(NUM_ROUNDS);

    for path in response_files.iter() {
        // Saves hash to `response_xxxx_hash`
        let mut hash_path = path.to_owned();
        hash_path.push_str("_hash");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&hash_path)
        {
            Ok(mut file) => {
                let now = Instant::now();
                hash_to(&mut file, path).unwrap();
                println!("File {:?} has been hashed in \n {:?}", path, now.elapsed());
            }
            // std::io::ErrorKind(AlreadyExists) => { todo!() },
            _ => println!("File {:?} has been hashed", path),
        }
    }

    for path in challenge_files.iter() {
        // Saves hash to `challenge_xxxx_hash`
        let mut hash_path = path.to_owned();
        hash_path.push_str("_hash");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&hash_path)
        {
            Ok(mut file) => {
                let now = Instant::now();
                hash_to(&mut file, path).unwrap();
                println!("File {:?} has been hashed in \n {:?}", path, now.elapsed());
            }
            // std::io::ErrorKind(AlreadyExists) => { todo!() },
            _ => println!("File {:?} has already been hashed", path),
        }
    }
}

/// Hashes the file at `path` and saves the hash to `file`.
fn hash_to(file: &mut File, path: &str) -> Result<(), std::io::Error> {
    // Make memory map from `path`
    let reader = OpenOptions::new()
        .read(true)
        .open(path)
        .expect("unable open file in this directory");
    // Make a memory map
    let reader = unsafe {
        MmapOptions::new()
            .map(&reader)
            .expect("unable to create a memory map for input")
    };
    let hash = calculate_hash(&reader);
    file.write_all(&hash)?;
    Ok(())
}

/// Computes Blake2 hash of all files specified by a list
/// of paths, returning all hashes.
fn _hash_all(files: Vec<String>) -> Vec<[u8; 64]> {
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
