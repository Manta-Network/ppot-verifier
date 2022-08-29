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

/// Temporary hack to get the challenge file URLs.
pub fn challenge_urls() -> [&'static str; 72] {
    [
        "https://ppot.blob.core.windows.net/public/challenge_initial",
        "https://ppot.blob.core.windows.net/public/challenge_0002_kobi",
        "https://ppot.blob.core.windows.net/public/challenge_0003",
        "https://ppot.blob.core.windows.net/public/challenge_0004",
        "https://ppot.blob.core.windows.net/public/challenge_0005",
        "https://ppot.blob.core.windows.net/public/challenge_0006",
        "https://ppot.blob.core.windows.net/public/challenge_0007",
        "https://ppot.blob.core.windows.net/public/challenge_0008",
        "https://ppot.blob.core.windows.net/public/challenge_0009",
        "https://ppot.blob.core.windows.net/public/challenge_0010",
        "https://ppot.blob.core.windows.net/public/challenge_0011",
        "https://ppot.blob.core.windows.net/public/challenge_0012",
        "https://ppot.blob.core.windows.net/public/challenge_0013",
        "https://ppot.blob.core.windows.net/public/challenge_0014",
        "https://ppot.blob.core.windows.net/public/challenge_0015",
        "https://ppot.blob.core.windows.net/public/challenge_0016",
        "https://ppot.blob.core.windows.net/public/challenge_0017",
        "https://ppot.blob.core.windows.net/public/challenge_0018",
        "https://ppot.blob.core.windows.net/public/challenge_0019",
        "https://ppot.blob.core.windows.net/public/challenge_0020",
        "https://ppot.blob.core.windows.net/public/challenge_0021",
        "https://ppot.blob.core.windows.net/public/challenge_0022",
        "https://ppot.blob.core.windows.net/public/challenge_0023",
        "https://ppot.blob.core.windows.net/public/challenge_0024",
        "https://ppot.blob.core.windows.net/public/challenge_0025",
        "https://ppot.blob.core.windows.net/public/challenge_0026",
        "https://ppot.blob.core.windows.net/public/challenge_0027",
        "https://ppot.blob.core.windows.net/public/challenge_0028",
        "https://ppot.blob.core.windows.net/public/challenge_0029",
        "https://ppot.blob.core.windows.net/public/challenge_0030",
        "https://ppot.blob.core.windows.net/public/challenge_0031",
        "https://ppot.blob.core.windows.net/public/challenge_0032",
        "https://ppot.blob.core.windows.net/public/challenge_0033",
        "https://ppot.blob.core.windows.net/public/challenge_0034",
        "https://ppot.blob.core.windows.net/public/challenge_0035",
        "https://ppot.blob.core.windows.net/public/challenge_0036",
        "https://ppot.blob.core.windows.net/public/challenge_0037",
        "https://ppot.blob.core.windows.net/public/challenge_0038",
        "https://ppot.blob.core.windows.net/public/challenge_0039",
        "https://ppot.blob.core.windows.net/public/challenge_0040",
        "https://ppot.blob.core.windows.net/public/challenge_0041",
        "https://ppot.blob.core.windows.net/public/challenge_0042",
        "https://ppot.blob.core.windows.net/public/challenge_0043",
        "https://ppot.blob.core.windows.net/public/challenge_0044",
        "https://ppot.blob.core.windows.net/public/challenge_0045",
        "https://ppot.blob.core.windows.net/public/challenge_0046",
        "https://ppot.blob.core.windows.net/public/challenge_0047",
        "https://ppot.blob.core.windows.net/public/challenge_0048",
        "https://ppot.blob.core.windows.net/public/challenge_0049",
        "https://ppot.blob.core.windows.net/public/challenge_0050",
        "https://ppot.blob.core.windows.net/public/challenge_0051",
        "https://ppot.blob.core.windows.net/public/challenge_0052",
        "https://ppot.blob.core.windows.net/public/challenge_0053",
        "https://ppot.blob.core.windows.net/public/challenge_0054",
        "https://ppot.blob.core.windows.net/public/challenge_0055",
        "https://ppot.blob.core.windows.net/public/challenge_0056",
        "https://ppot.blob.core.windows.net/public/challenge_0057",
        "https://ppot.blob.core.windows.net/public/challenge_0058",
        "https://ppot.blob.core.windows.net/public/challenge_0059",
        "https://ppot.blob.core.windows.net/public/challenge_0060",
        "https://ppot.blob.core.windows.net/public/challenge_0061",
        "https://ppot.blob.core.windows.net/public/challenge_0062",
        "https://ppot.blob.core.windows.net/public/challenge_0063",
        "https://ppot.blob.core.windows.net/public/challenge_0064",
        "https://ppot.blob.core.windows.net/public/challenge_0065",
        "https://ppot.blob.core.windows.net/public/challenge_0066",
        "https://ppot.blob.core.windows.net/public/challenge_0067",
        "https://ppot.blob.core.windows.net/public/challenge_0068",
        "https://ppot.blob.core.windows.net/public/challenge_0069",
        "https://ppot.blob.core.windows.net/public/challenge_0070",
        "https://ppot.blob.core.windows.net/public/challenge_0071",
        "https://ppot.blob.core.windows.net/public/challenge_0072",
    ]
}

/// Challenge path names numbered from 0 to n
pub fn challenge_paths(n: usize) -> Vec<String> {
    (0..n + 1).map(|i| format!("challenge_{:04}", i)).collect()
}

/// Temporary hack to get the response file URLs.
pub fn response_urls() -> [&'static str; 71] {
    [
        "https://ppot.blob.core.windows.net/public/response_0001_weijie",
        "https://ppot.blob.core.windows.net/public/response_0002_kobi",
        "https://ppot.blob.core.windows.net/public/response_0003_poma",
        "https://ppot.blob.core.windows.net/public/response_0004_pepesha",
        "https://ppot.blob.core.windows.net/public/response_0005_amrullah",
        "https://ppot.blob.core.windows.net/public/response_0006_zac",
        "https://ppot.blob.core.windows.net/public/response_0007_youssef",
        "https://ppot.blob.core.windows.net/public/response_0008_mike",
        "https://ppot.blob.core.windows.net/public/response_0009_brecht",
        "https://ppot.blob.core.windows.net/public/response_0010_vano",
        "https://ppot.blob.core.windows.net/public/response_0011_zhiniang",
        "https://ppot.blob.core.windows.net/public/response_0012_daniel",
        "https://ppot.blob.core.windows.net/public/response_0013_kevin",
        "https://ppot.blob.core.windows.net/public/response_0014_weijie",
        "https://ppot.blob.core.windows.net/public/response_0015_anon0",
        "https://ppot.blob.core.windows.net/public/response_0016_aurel",
        "https://ppot.blob.core.windows.net/public/response_0017_philip",
        "https://ppot.blob.core.windows.net/public/response_0018_cody",
        "https://ppot.blob.core.windows.net/public/response_0019_petr",
        "https://ppot.blob.core.windows.net/public/response_0020_edu",
        "https://ppot.blob.core.windows.net/public/response_0021_rf",
        "https://ppot.blob.core.windows.net/public/response_0022_roman",
        "https://ppot.blob.core.windows.net/public/response_0023_shomari",
        "https://ppot.blob.core.windows.net/public/response_0024_vb",
        "https://ppot.blob.core.windows.net/public/response_0025_stefan",
        "https://ppot.blob.core.windows.net/public/response_0026_geoff",
        "https://ppot.blob.core.windows.net/public/response_0027_alex",
        "https://ppot.blob.core.windows.net/public/response_0028_dimitris",
        "https://ppot.blob.core.windows.net/public/response_0029_gustavo",
        "https://ppot.blob.core.windows.net/public/response_0030_anant",
        "https://ppot.blob.core.windows.net/public/response_0031_golem",
        "https://ppot.blob.core.windows.net/public/response_0032_josephc",
        "https://ppot.blob.core.windows.net/public/response_0033_oskar",
        "https://ppot.blob.core.windows.net/public/response_0034_igor",
        "https://ppot.blob.core.windows.net/public/response_0035_leonard",
        "https://ppot.blob.core.windows.net/public/response_0036_stefaan",
        "https://ppot.blob.core.windows.net/public/response_0037_chihcheng",
        "https://ppot.blob.core.windows.net/public/response_0038_james",
        "https://ppot.blob.core.windows.net/public/response_0039_wanseob",
        "https://ppot.blob.core.windows.net/public/response_0040_weitang",
        "https://ppot.blob.core.windows.net/public/response_0041_evan",
        "https://ppot.blob.core.windows.net/public/response_0042_vaibhav",
        "https://ppot.blob.core.windows.net/public/response_0043_albert",
        "https://ppot.blob.core.windows.net/public/response_0044_yingtong",
        "https://ppot.blob.core.windows.net/public/response_0045_ben",
        "https://ppot.blob.core.windows.net/public/response_0046_tkorwin",
        "https://ppot.blob.core.windows.net/public/response_0047_saravanan",
        "https://ppot.blob.core.windows.net/public/response_0048_tyler",
        "https://ppot.blob.core.windows.net/public/response_0049_jordi",
        "https://ppot.blob.core.windows.net/public/response_0050_weijie",
        "https://ppot.blob.core.windows.net/public/response_0051_joe",
        "https://ppot.blob.core.windows.net/public/response_0052_zaki",
        "https://ppot.blob.core.windows.net/public/response_0053_juan",
        "https://ppot.blob.core.windows.net/public/response_0054_jarrad",
        "https://ppot.blob.core.windows.net/public/response_0055_tyler",
        "https://ppot.blob.core.windows.net/public/response_0056_auryn",
        "https://ppot.blob.core.windows.net/public/response_0057_gisli",
        "https://ppot.blob.core.windows.net/public/response_0058_rasikh",
        "https://ppot.blob.core.windows.net/public/response_0059_pau",
        "https://ppot.blob.core.windows.net/public/response_0060_weijie",
        "https://ppot.blob.core.windows.net/public/response_0061_adria",
        "https://ppot.blob.core.windows.net/public/response_0062_lev",
        "https://ppot.blob.core.windows.net/public/response_0063_david",
        "https://ppot.blob.core.windows.net/public/response_0064_ian",
        "https://ppot.blob.core.windows.net/public/response_0065_adrian",
        "https://ppot.blob.core.windows.net/public/response_0066_kieran",
        "https://ppot.blob.core.windows.net/public/response_0067_nick",
        "https://ppot.blob.core.windows.net/public/response_0068_elena",
        "https://ppot.blob.core.windows.net/public/response_0069_justice",
        "https://ppot.blob.core.windows.net/public/response_0070_bertrand",
        "https://ppot.blob.core.windows.net/public/response_0071_edward",
    ]
}

/// Response path names numbered from 1 to n
pub fn response_paths(n: usize) -> Vec<String> {
    (1..n + 1).map(|i| format!("response_{:04}", i)).collect()
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
