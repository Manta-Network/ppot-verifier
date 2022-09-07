// use memmap::MmapOptions;
use ppot_verifier::{challenge_paths, response_paths};
use std::fs::OpenOptions;
use std::io::Read;

const NUM_ROUNDS: usize = 70; // TODO: Change to 71

fn main() {
    let challenge_files = challenge_paths(NUM_ROUNDS);
    let response_files = response_paths(NUM_ROUNDS);

    for (challenge, response) in challenge_files.iter().zip(response_files.iter()) {
        // Read computed hash of challenge file:
        let mut hash_path = challenge.clone().to_owned();
        hash_path.push_str("_hash");
        let mut file = OpenOptions::new()
            .read(true)
            .open(hash_path)
            .expect("unable to open file in this directory");
        let mut computed_hash = [0u8; 64];
        let _ = file.read(&mut computed_hash[..]).unwrap();
        // Read asserted hash from reponse file
        let mut file = OpenOptions::new()
            .read(true)
            .open(response)
            .expect("unable to open file in this directory");
        let mut asserted_hash = [0u8; 64];
        let _ = file.read(&mut asserted_hash[..]).unwrap();

        if computed_hash != asserted_hash {
            println!("Hashes don't match for {:?} and {:?}", challenge, response);
            println!("Computed hash");
            for line in computed_hash.chunks(16) {
                print!("\t");
                for section in line.chunks(4) {
                    for b in section {
                        print!("{:02x}", b);
                    }
                    print!(" ");
                }
            }
            println!(" ");
            println!("Asserted hash:");
            for line in asserted_hash.chunks(16) {
                print!("\t");
                for section in line.chunks(4) {
                    for b in section {
                        print!("{:02x}", b);
                    }
                    print!(" ");
                }
            }
        } else {
            // println!("The hash of {:?} is", challenge);
            // for line in computed_hash.chunks(16) {
            //     print!("\t");
            //     for section in line.chunks(4) {
            //         for b in section {
            //             print!("{:02x}", b);
            //         }
            //         print!(" ");
            //     }
            // }
            // println!(" ");
        }
    }
    // Check hashes of response files
    for (challenge, response) in challenge_files.iter().skip(1).zip(response_files.iter()) {
        // Read computed hash of response file:
        let mut hash_path = response.clone().to_owned();
        hash_path.push_str("_hash");
        let mut file = OpenOptions::new()
            .read(true)
            .open(hash_path)
            .expect("unable to open file in this directory");
        let mut computed_hash = [0u8; 64];
        let _ = file.read(&mut computed_hash[..]).unwrap();
        // Read asserted hash from challenge file
        let mut file = OpenOptions::new()
            .read(true)
            .open(challenge)
            .expect("unable to open file in this directory");
        let mut asserted_hash = [0u8; 64];
        let _ = file.read(&mut asserted_hash[..]).unwrap();
        if computed_hash != asserted_hash {
            println!("Hashes don't match for {:?} and {:?}", challenge, response);
            println!("Computed hash");
            for line in computed_hash.chunks(16) {
                print!("\t");
                for section in line.chunks(4) {
                    for b in section {
                        print!("{:02x}", b);
                    }
                    print!(" ");
                }
            }
            println!(" ");
            println!("Asserted hash:");
            for line in asserted_hash.chunks(16) {
                print!("\t");
                for section in line.chunks(4) {
                    for b in section {
                        print!("{:02x}", b);
                    }
                    print!(" ");
                }
            }
        } else {
            // println!("The hash of {:?} is", response);
            // for line in computed_hash.chunks(16) {
            //     print!("\t");
            //     for section in line.chunks(4) {
            //         for b in section {
            //             print!("{:02x}", b);
            //         }
            //         print!(" ");
            //     }
            // }
            // println!(" ");
        }
    }
}
