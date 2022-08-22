use blake2::{Blake2b, Digest};
use memmap::Mmap;
use std::{fs, io};

/// Computes the hash of a potentially large file,
/// such as PPoT `challenge` or `response` files.
pub fn calculate_hash(input_map: &Mmap) -> [u8; 64] {
    let chunk_size = 1 << 30; // read by 1GB from map
    let mut hasher = Blake2b::default();

    for (counter, chunk) in input_map.chunks(chunk_size).enumerate() {
        hasher.update(&chunk);
        println!("Have hashed {:?} GB of the file", counter);
    }
    into_array_unchecked(hasher.finalize())
}

/// Error Message for the [`into_array_unchecked`] and [`into_boxed_array_unchecked`] Functions
const INTO_UNCHECKED_ERROR_MESSAGE: &str =
    "Input did not have the correct length to match the output array of length";

/// Performs the [`TryInto`] conversion into an array without checking if the conversion succeeded.
#[inline]
pub fn into_array_unchecked<T, V, const N: usize>(value: V) -> [T; N]
where
    V: TryInto<[T; N]>,
{
    match value.try_into() {
        Ok(array) => array,
        _ => unreachable!("{} {:?}.", INTO_UNCHECKED_ERROR_MESSAGE, N),
    }
}

/// Go to github repo and parse file names
pub fn get_urls() -> std::io::Result<(Vec<String>, Vec<String>)> {
    let path = "../perpetualpowersoftau";

    let mut challenge_paths = Vec::<String>::new();
    let mut response_paths = Vec::<String>::new();
    let mut counter: usize = 0;

    // Get sorted list of contributions
    let mut entries = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    for entry in entries.iter() {
        let file_name = entry.as_path().file_name().unwrap().to_str().unwrap();
        if let Ok(number) = &file_name[0..4].parse::<usize>() {
            if *number == counter {
                // Challenge paths are just numbered
                challenge_paths.push(format!(
                    "https://ppot.blob.core.windows.net/public/challenge_{:04}",
                    number + 1
                ));
                // Response paths require the participant name and there is no response to initial challenge
                if counter > 0 {
                    response_paths.push(format!(
                        "https://ppot.blob.core.windows.net/public/response_{:04}_{:}",
                        number,
                        parse_participant_name(&file_name[5..]).unwrap()
                    ));
                }

                counter += 1;
            }
        };
    }

    // Some exceptions to the naming convention
    challenge_paths[0] = "https://ppot.blob.core.windows.net/public/challenge_initial".into();
    challenge_paths[1] = "https://ppot.blob.core.windows.net/public/challenge_0002_kobi".into();
    response_paths[2] = "https://ppot.blob.core.windows.net/public/response_0003_poma".into();
    response_paths[3] = "https://ppot.blob.core.windows.net/public/response_0004_pepesha".into();
    response_paths[11] = "https://ppot.blob.core.windows.net/public/response_0012_daniel".into();
    response_paths[15] = "https://ppot.blob.core.windows.net/public/response_0016_aurel".into();
    response_paths[39] = "https://ppot.blob.core.windows.net/public/response_0040_weitang".into();

    println!("There are {:?} challenge files", challenge_paths.len());
    println!("There are {:?} response files", response_paths.len());

    Ok((challenge_paths, response_paths))
}

/// Extracts the name of a participant from a file assumed to
/// have the form "name_response". All it does is look for the
/// index where the underscore first appears and return the appropriate
/// string slice.
fn parse_participant_name(file_name: &str) -> Option<&str> {
    for (i, char) in file_name.chars().enumerate() {
        if char == '_' {
            return Some(&file_name[0..i]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_correct_urls() {
        let (challenge_paths, response_paths) = get_urls().unwrap();
        let mut all_paths_valid = true;

        // Check validity of each challenge path
        for (_i, path) in challenge_paths.iter().enumerate() {
            if !check_download_url(path) {
                println!("URL {:?} is invalid ", path);
                all_paths_valid = false;
            }
        }

        // Check validity of each response path
        for (_i, path) in response_paths.iter().enumerate() {
            if !check_download_url(path) {
                println!("URL {:?} is invalid ", path);
                all_paths_valid = false;
            }
        }
        assert_eq!(challenge_paths.len(), response_paths.len() + 1);
        assert!(all_paths_valid);
    }

    /// Checks validity of the URL by requesting a few bytes of the download.
    /// Returns `true` if the correct number were returned and `false` otherwise.
    fn check_download_url(path: &str) -> bool {
        use curl::easy::Easy;

        let mut handle = Easy::new();
        handle.url(path).unwrap();
        handle.range("0-2").unwrap();
        handle.perform().unwrap();

        let content_length = handle.content_length_download().unwrap();
        if content_length != 3f64 {
            return false;
        }
        true
    }
}
