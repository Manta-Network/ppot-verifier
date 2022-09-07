use memmap::MmapOptions;
use ppot_verifier::calculate_hash;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;

fn main() {
    for path in ["challenge_0002_clean", "challenge_0003_clean"] {
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
