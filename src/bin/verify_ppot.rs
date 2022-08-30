use manta_trusted_setup::groth16::kzg::Accumulator;
use manta_trusted_setup::groth16::ppot::kzg::PerpetualPowersOfTauCeremony;
use manta_trusted_setup::groth16::ppot::serialization::{
    read_kzg_proof, read_subaccumulator, Compressed, PpotSerializer,
};
use manta_util::into_array_unchecked;
use memmap::{Mmap, MmapOptions};
use ppot_verifier::{challenge_paths, response_paths};
use std::fs::OpenOptions;

/// Size of subaccumulator we are verifying
const NUM_POWERS: usize = 1 << 5;
/// Subaccumulator type
type SmallCeremony = PerpetualPowersOfTauCeremony<PpotSerializer, NUM_POWERS>;

/// Number of rounds of ceremony to verify
const NUM_ROUNDS: usize = 71;

/// Given a path, produces a read-only MemMap to that path
unsafe fn try_into_mmap(path: &str) -> Option<Mmap> {
    match OpenOptions::new().read(true).open(path) {
        Ok(file) => Some(
            MmapOptions::new()
                .map(&file)
                .expect("unable to create a memory map for input"),
        ),
        _ => {
            println!("Unable to open file at {:?}", path);
            None
        }
    }
}

fn main() {
    unsafe {
        println!("Hello Todd");
        let challenges = challenge_paths(NUM_ROUNDS);
        let responses = response_paths(NUM_ROUNDS);

        let mut prev = Accumulator::<SmallCeremony>::default();
        for i in 1..NUM_ROUNDS {
            // read next accumulator from challenge file
            let next = read_subaccumulator::<SmallCeremony>(
                &try_into_mmap(&challenges[i-1]).unwrap(),
                Compressed::No,
            )
            .unwrap();
            // read next challenge hash from response file
            let response = try_into_mmap(&responses[i]).unwrap();
            let challenge_hash: [u8; 64] = into_array_unchecked(
                response
                    .get(0..64)
                    .expect("Response file header is 64 bit hash of challenge file"),
            );
            // read proof from response file
            let proof = read_kzg_proof(&response).unwrap();
            // verify
            prev = match Accumulator::<SmallCeremony>::verify_transform(
                prev,
                next,
                challenge_hash,
                proof.cast_to_subceremony(),
            ) {
                Ok(accumulator) => accumulator,
                Err(e) => {
                    println!("Verification error {:?} occurred checking round {:?}", e, i);
                    // To continue with verification, try just using the next subaccumulator to continue
                    // TODO: Remove that
                    read_subaccumulator::<SmallCeremony>(
                        &try_into_mmap(&challenges[i + 1]).unwrap(),
                        Compressed::No,
                    )
                    .unwrap()
                }
            };
        }
    }
}
