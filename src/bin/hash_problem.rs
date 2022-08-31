// Temporary bin to resolve a weird issue with the hash of certain files

// The files with invalid hashes are `challenge_0002` and `challenge_0003`.
// My numbering differs from ppot by 1, so the corresponding URLs are 
// "https://ppot.blob.core.windows.net/public/challenge_0003"
// "https://ppot.blob.core.windows.net/public/challenge_0004"

// This function is an abridged version of the `downloader`


use anyhow::anyhow;
use core::{cmp::min, num::ParseIntError, str::FromStr};
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use manta_util::http::reqwest::{
    header::{CONTENT_RANGE, RANGE},
    Client, Method, Response, StatusCode,
};
use std::path::Path;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    task,
};

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
                    "https://ppot.blob.core.windows.net/public/challenge_0003",
                    "challenge_0002_clean",
                ),
                (
                    "https://ppot.blob.core.windows.net/public/challenge_0004",
                    "challenge_0003_clean",
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