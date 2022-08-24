use memmap::MmapOptions;
use ppot_verifier::{calculate_hash};
use std::fs::OpenOptions; // TODO: Is standard okay?


fn main() {
    let path = "challenge_0011";
    let reader = OpenOptions::new()
            .read(true)
            .open(path)
            .expect("unable open file in this directory");
        // Make a memory map
        let challenge = unsafe {
            MmapOptions::new()
                .map(&reader)
                .expect("unable to create a memory map for input")
        };
        let hash = calculate_hash(&challenge);
        println!("The hash of {:?} is ", path);
        for line in hash.chunks(16) {
            print!("\t");
            for section in line.chunks(4) {
                for b in section {
                    print!("{:02x}", b);
                }
                print!(" ");
            }
        }
}
