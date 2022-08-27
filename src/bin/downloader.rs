//! Download all PPoT challenge and response files

use anyhow::anyhow;
use core::{cmp::min, num::ParseIntError, str::FromStr};
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{
    header::{CONTENT_RANGE, RANGE},
    Client, Method, Response, StatusCode,
};
use std::path::Path;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    task,
};

/// Result Type
pub type Result<T = (), E = anyhow::Error> = core::result::Result<T, E>;

/// Checks if the file exists by sending a [`GET`](Method::GET) request to the server at `url` and
/// checking if an [`OK`](StatusCode::OK) is returned.
#[inline]
pub async fn file_exists(client: &Client, url: &str) -> Result<bool> {
    Ok(client.request(Method::GET, url).send().await?.status() == StatusCode::OK)
}

/// Content Range
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ContentRange {
    /// Full Range Data
    ///
    /// This is the response from the server when the [`RANGE`] header `start` value is set less
    /// than the entire data stream.
    Full {
        /// Starting index
        start: u64,

        /// Ending index
        end: u64,

        /// Total size of the data stored on the server. This is not a measure of how much data is
        /// sent over in the response, that would be `end - start`.
        size: u64,
    },

    /// Size Data
    ///
    /// When the [`RANGE`] header `start` value sent to the server is exactly equal to the size of
    /// the data payload, then only that same size is returned back.
    Size(u64),
}

impl ContentRange {
    /// Parses a [`ContentRange`] from `response` returning `None` if the header did not exist or if
    /// it did exist but could not be parsed.
    #[inline]
    pub fn from_response(response: &Response) -> Option<Self> {
        response
            .headers()
            .get(CONTENT_RANGE)?
            .to_str()
            .ok()?
            .parse()
            .ok()
    }
}

impl FromStr for ContentRange {
    type Err = ContentRangeParseError;

    #[inline]
    fn from_str(range_string: &str) -> Result<Self, Self::Err> {
        let (bytes_tag, range) = range_string
            .split_once(' ')
            .ok_or(Self::Err::MissingSpace)?;
        if bytes_tag == "bytes" {
            match range.split_once('-') {
                Some((start, end_and_size)) => {
                    let (end, size) = end_and_size
                        .split_once('/')
                        .ok_or(Self::Err::MissingSlash)?;
                    Ok(Self::Full {
                        start: start.parse().map_err(Self::Err::InvalidStart)?,
                        end: end.parse().map_err(Self::Err::InvalidEnd)?,
                        size: size.parse().map_err(Self::Err::InvalidSize)?,
                    })
                }
                _ => {
                    let (star, size) = range.split_once('/').ok_or(Self::Err::MissingSlash)?;
                    if star == "*" {
                        Ok(Self::Size(size.parse().map_err(Self::Err::InvalidSize)?))
                    } else {
                        Err(Self::Err::MissingStar)
                    }
                }
            }
        } else {
            Err(Self::Err::MissingBytesTag)
        }
    }
}

/// Content Range Parse Error
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentRangeParseError {
    /// Missing Space
    MissingSpace,

    /// Missing Bytes Tag
    MissingBytesTag,

    /// Missing Slash
    MissingSlash,

    /// Missing Star
    MissingStar,

    /// Invalid Start Index
    InvalidStart(ParseIntError),

    /// Invalid End Index
    InvalidEnd(ParseIntError),

    /// Invalid Size Index
    InvalidSize(ParseIntError),
}

/// Opens the file at `path` into a [`BufWriter`] and returns its current length.
#[inline]
pub async fn open_file<P>(path: P) -> Result<(u64, BufWriter<File>)>
where
    P: AsRef<Path>,
{
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .await?;
    Ok((file.metadata().await?.len(), BufWriter::new(file)))
}

/// Sends the download request to the server at `url` with the [`RANGE`] header set to start its
/// range at `start`, returning the [`Response`] from the server and the total size of the file to
/// be downloaded. This function returns `None` if the [`RANGE`] `start` bound is equal to the size
/// of the file, meaning nothing needs to be downloaded.
#[inline]
pub async fn send_download_request(
    client: &Client,
    url: &str,
    start: u64,
) -> Result<Option<(u64, Response)>> {
    let response = client
        .request(Method::GET, url)
        .header(RANGE, format!("bytes={}-", start))
        .send()
        .await?;
    match ContentRange::from_response(&response) {
        Some(ContentRange::Full { size, .. }) => Ok(Some((size, response))),
        Some(ContentRange::Size(size)) => {
            if size == start {
                Ok(None)
            } else {
                Err(anyhow!("Size mismatch."))
            }
        }
        _ => Err(anyhow!("Failed to parse content range from '{}'", url)),
    }
}

/// Progress Bar Template
const PROGRESS_BAR_TEMPLATE: &str =
    "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}";

/// Instatiates a [`ProgressBar`] of `len` elements and style given by [`PROGRESS_BAR_TEMPLATE`]
/// and pushes it to `multibar`.
#[inline]
fn progress_bar(multibar: &MultiProgress, len: u64) -> Result<ProgressBar> {
    let progress_bar = multibar.add(ProgressBar::new(len));
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template(PROGRESS_BAR_TEMPLATE)?
            .progress_chars("#>-"),
    );
    Ok(progress_bar)
}

/// Downloads the file at `url` to `path`. If the file is not empty, we use the size of the file to
/// determine how many bytes to read from the server. This allows for restarting the download
/// process after a network or disk failure.
///
/// # Note
///
/// This function assumes that a single `path` will always be associated to a single `url` so that
/// restarting downloading makes sense.
#[inline]
pub async fn download_file<P>(
    multibar: &MultiProgress,
    client: &Client,
    url: &str,
    path: P,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let (mut amount_downloaded, file) = open_file(path).await?;
    let (total_size, mut response) =
        match send_download_request(client, url, amount_downloaded).await? {
            Some((total_size, response)) => (total_size, response),
            _ => return Ok(()),
        };
    let mut file = BufWriter::new(file);
    let progress_bar = progress_bar(multibar, total_size)?;
    progress_bar.set_message(format!("Downloading {}", url));
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        amount_downloaded = min(amount_downloaded + (chunk.len() as u64), total_size);
        progress_bar.set_position(amount_downloaded);
    }
    file.flush().await?;
    progress_bar.finish_with_message(format!("Downloaded {} to {}", url, path.display()));
    Ok(())
}

#[test]
fn print_challenge_urls_paths() {
    use ppot_verifier::{challenge_paths, challenge_urls};

    let urls = challenge_urls();
    let paths = challenge_paths(urls.len());
    let output: Vec<(&str, &str)> = urls
        .iter()
        .copied()
        .zip(paths.iter().map(|s| s.as_str()))
        .collect();
    println!("{:#?}", output);
}

#[test]
fn print_response_urls_paths() {
    use ppot_verifier::{response_paths, response_urls};

    let urls = response_urls();
    let paths = response_paths(urls.len());
    let output: Vec<(&str, &str)> = urls
        .iter()
        .copied()
        .zip(paths.iter().map(|s| s.as_str()))
        .collect();
    println!("{:#?}", output);
}

/// Spawns a multi-threaded [`tokio`] runtime and downloads a set of files in parallel.
fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(10)
        .enable_io()
        .enable_time()
        .build()?
        .block_on(async {
            let multibar = MultiProgress::new();
            let client = Client::new();
            let mut handles = vec![];
            for (url, path) in [
                (
                    "https://ppot.blob.core.windows.net/public/challenge_initial",
                    "challenge_0000",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0002_kobi",
                    "challenge_0001",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0003",
                    "challenge_0002",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0004",
                    "challenge_0003",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0005",
                    "challenge_0004",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0006",
                    "challenge_0005",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0007",
                    "challenge_0006",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0008",
                    "challenge_0007",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0009",
                    "challenge_0008",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0010",
                    "challenge_0009",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0011",
                    "challenge_0010",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0012",
                    "challenge_0011",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0013",
                    "challenge_0012",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0014",
                    "challenge_0013",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0015",
                    "challenge_0014",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0016",
                    "challenge_0015",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0017",
                    "challenge_0016",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0018",
                    "challenge_0017",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0019",
                    "challenge_0018",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0020",
                    "challenge_0019",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0021",
                    "challenge_0020",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0022",
                    "challenge_0021",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0023",
                    "challenge_0022",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0024",
                    "challenge_0023",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0025",
                    "challenge_0024",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0026",
                    "challenge_0025",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0027",
                    "challenge_0026",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0028",
                    "challenge_0027",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0029",
                    "challenge_0028",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0030",
                    "challenge_0029",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0031",
                    "challenge_0030",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0032",
                    "challenge_0031",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0033",
                    "challenge_0032",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0034",
                    "challenge_0033",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0035",
                    "challenge_0034",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0036",
                    "challenge_0035",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0037",
                    "challenge_0036",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0038",
                    "challenge_0037",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0039",
                    "challenge_0038",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0040",
                    "challenge_0039",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0041",
                    "challenge_0040",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0042",
                    "challenge_0041",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0043",
                    "challenge_0042",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0044",
                    "challenge_0043",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0045",
                    "challenge_0044",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0046",
                    "challenge_0045",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0047",
                    "challenge_0046",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0048",
                    "challenge_0047",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0049",
                    "challenge_0048",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0050",
                    "challenge_0049",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0051",
                    "challenge_0050",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0052",
                    "challenge_0051",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0053",
                    "challenge_0052",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0054",
                    "challenge_0053",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0055",
                    "challenge_0054",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0056",
                    "challenge_0055",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0057",
                    "challenge_0056",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0058",
                    "challenge_0057",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0059",
                    "challenge_0058",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0060",
                    "challenge_0059",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0061",
                    "challenge_0060",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0062",
                    "challenge_0061",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0063",
                    "challenge_0062",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0064",
                    "challenge_0063",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0065",
                    "challenge_0064",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0066",
                    "challenge_0065",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0067",
                    "challenge_0066",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0068",
                    "challenge_0067",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0069",
                    "challenge_0068",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0070",
                    "challenge_0069",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0071",
                    "challenge_0070",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0072",
                    "challenge_0071",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0001_weijie",
                    "response_0001",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0002_kobi",
                    "response_0002",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0003_poma",
                    "response_0003",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0004_pepesha",
                    "response_0004",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0005_amrullah",
                    "response_0005",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0006_zac",
                    "response_0006",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0007_youssef",
                    "response_0007",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0008_mike",
                    "response_0008",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0009_brecht",
                    "response_0009",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0010_vano",
                    "response_0010",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0011_zhiniang",
                    "response_0011",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0012_daniel",
                    "response_0012",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0013_kevin",
                    "response_0013",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0014_weijie",
                    "response_0014",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0015_anon0",
                    "response_0015",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0016_aurel",
                    "response_0016",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0017_philip",
                    "response_0017",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0018_cody",
                    "response_0018",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0019_petr",
                    "response_0019",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0020_edu",
                    "response_0020",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0021_rf",
                    "response_0021",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0022_roman",
                    "response_0022",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0023_shomari",
                    "response_0023",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0024_vb",
                    "response_0024",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0025_stefan",
                    "response_0025",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0026_geoff",
                    "response_0026",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0027_alex",
                    "response_0027",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0028_dimitris",
                    "response_0028",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0029_gustavo",
                    "response_0029",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0030_anant",
                    "response_0030",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0031_golem",
                    "response_0031",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0032_josephc",
                    "response_0032",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0033_oskar",
                    "response_0033",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0034_igor",
                    "response_0034",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0035_leonard",
                    "response_0035",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0036_stefaan",
                    "response_0036",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0037_chihcheng",
                    "response_0037",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0038_james",
                    "response_0038",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0039_wanseob",
                    "response_0039",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0040_weitang",
                    "response_0040",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0041_evan",
                    "response_0041",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0042_vaibhav",
                    "response_0042",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0043_albert",
                    "response_0043",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0044_yingtong",
                    "response_0044",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0045_ben",
                    "response_0045",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0046_tkorwin",
                    "response_0046",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0047_saravanan",
                    "response_0047",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0048_tyler",
                    "response_0048",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0049_jordi",
                    "response_0049",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0050_weijie",
                    "response_0050",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0051_joe",
                    "response_0051",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0052_zaki",
                    "response_0052",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0053_juan",
                    "response_0053",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0054_jarrad",
                    "response_0054",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0055_tyler",
                    "response_0055",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0056_auryn",
                    "response_0056",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0057_gisli",
                    "response_0057",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0058_rasikh",
                    "response_0058",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0059_pau",
                    "response_0059",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0060_weijie",
                    "response_0060",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0061_adria",
                    "response_0061",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0062_lev",
                    "response_0062",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0063_david",
                    "response_0063",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0064_ian",
                    "response_0064",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0065_adrian",
                    "response_0065",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0066_kieran",
                    "response_0066",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0067_nick",
                    "response_0067",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0068_elena",
                    "response_0068",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0069_justice",
                    "response_0069",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/response_0070_bertrand",
                    "response_0070",
                ),
            ] {
                if file_exists(&client, url).await? {
                    let multibar = multibar.clone();
                    let client = client.clone();
                    handles.push(task::spawn(async move {
                        download_file(&multibar, &client, url, path).await
                    }));
                } else {
                    multibar.println(format!("ERROR: The file at '{}' does not exist", url))?;
                }
            }
            for result in try_join_all(handles).await? {
                result?;
            }
            Ok(())
        })
}
