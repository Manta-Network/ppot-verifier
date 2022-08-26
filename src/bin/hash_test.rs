use memmap::MmapOptions;
use ppot_verifier::{calculate_hash};
use std::fs::OpenOptions; // TODO: Is standard okay?
use std::io::{Read, Write};

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
    // make writer (open file in write mode)
    // writer.write_all()
    // see `std::io` traits
    let mut hash_path = path.to_owned();
    hash_path.push_str("_hash");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(hash_path.clone())
        .expect("unable to open file in this directory");
    file.write_all(&hash).unwrap();
    drop(file);

    // Check that it worked
    println!("Opening hash file");
    let mut file = OpenOptions::new()
        .read(true)
        .open(hash_path)
        .expect("unable to open file in this directory");
    let mut contents = Vec::<u8>::new();
    let bytes_read = file.read(&mut contents[..]).unwrap();
    println!("The contents of the file are {:?}", contents);
    assert_eq!(bytes_read, 64);
}
