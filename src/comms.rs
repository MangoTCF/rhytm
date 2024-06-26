use std::{io::Read, io::Write, os::unix::net::UnixStream};

use anyhow::{Context, Error, Ok};
use clap::Parser;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const THREAD_COUNT: usize = 1;
const LINK_BATCH_SIZE: usize = 5;
const TMP_DIR: &str = "/tmp/rhytm"; // TODO: parse from args
const DOWNLOAD_DIR: &str = "."; // TODO: parse from args
const LOGS_DIR_RELATIVE: &str = "/logs/";
const PARSE_REGEX_STR: &str = r"(https://(music)|(www)\.youtube\.com/)?(watch\?v=)([a-zA-Z0-9/\.\?=\-_]+)";
const YT_DLP_OUTPUT_TEMPLATE: &str = "%(title,fulltitle)s - %(uploader)s - [%(id)s]";

pub trait MessageRead: std::io::Read {
    fn read_json_msg<T: for<'a> Deserialize<'a>>(&mut self) -> Result<T, Error>;
}

impl MessageRead for UnixStream {
    fn read_json_msg<T: for<'a> Deserialize<'a>>(&mut self) -> Result<T, Error> {
        let mut lbuf = [0 as u8; std::mem::size_of::<usize>()];
        self.read_exact(&mut lbuf)
            .context("Unable to read msg length")?;

        let size = usize::from_ne_bytes(lbuf.try_into().unwrap());

        let mut buf = Vec::with_capacity(size);
        buf.resize(size, 0);
        self.read_exact(&mut buf)
            .context("Unable to read message")?;
        serde_json::from_slice(&buf).context("Unable to deserialize")
    }
}

pub trait MessageWrite: std::io::Write {
    fn write_json_msg<T: Serialize>(&mut self, msg: &T) -> Result<usize, Error>;
}

impl MessageWrite for UnixStream {
    fn write_json_msg<T: Serialize>(&mut self, msg: &T) -> Result<usize, Error> {
        let msg = serde_json::to_vec(msg).context("Unable to serialize")?;
        let size = msg.len();

        self.write_all(&size.to_ne_bytes())
            .context("Unable to write msg length")?;
        self.write_all(&msg).context("Unable to write message")?;
        Ok(size)
    }
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(version, author, about, long_about = None)]
pub struct Options {
    #[arg(short, long, default_value = "info")]
    pub verbosity: LevelFilter,

    #[arg(short='j', long, default_value_t = THREAD_COUNT)]
    pub threads: usize,

    #[arg(short='b', long, default_value_t = LINK_BATCH_SIZE)]
    pub link_batch_size: usize,

    #[arg(short, long, default_value = TMP_DIR)]
    pub tmp_dir: String,

    #[arg(short, long, default_value = DOWNLOAD_DIR)]
    pub download_dir: String,

    #[arg(short, long, default_value = LOGS_DIR_RELATIVE)]
    pub logs_dir_relative: String,

    #[arg(short, long, default_value = PARSE_REGEX_STR)]
    pub parse_regex_str: String,

    #[arg(short, long, default_value = YT_DLP_OUTPUT_TEMPLATE)]
    pub yt_dlp_output_template: String,

    #[arg(required(true))]
    pub html_path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Message {
    Greeting(usize),
    Log {
        thr_id: usize,
        level: log::Level,
        target: String,
        msg: String,
    },
    BatchRequest,
    Batch(Vec<String>),
    JSON(String),
    DownloadStart,
    DownloadEnd,
    EndRequest,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    //TODO: Perpetual update
    url: String,
    duration: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Format {
    //TODO: Perpetual update
    pub format_id: String,
    pub format_index: Option<String>,
    pub format_note: Option<String>,
    pub ext: String,
    pub protocol: String,
    #[serde(default)]
    pub acodec: String,
    #[serde(default)]
    pub vcodec: String,
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub rows: Option<u32>,
    pub columns: Option<u32>,
    pub fragments: Option<Vec<Fragment>>,
    pub resolution: String,
    pub aspect_ratio: Option<f32>,
    pub http_headers: Map<String, Value>,
    pub audio_ext: String,
    pub audio_channels: Option<i16>,
    pub video_ext: String,
    pub vbr: Option<f32>,
    pub abr: Option<f32>,
    pub asr: Option<i32>,
    pub tbr: Option<f32>,
    pub format: String,
    pub filesize_approx: Option<usize>,
    pub filesize: Option<usize>,
    pub manifest_url: Option<String>,
    pub language: Option<String>,
    pub preference: Option<i32>,
    pub quality: Option<f32>,
    #[serde(default)]
    pub has_drm: bool,
    pub source_preference: Option<i32>,
    pub language_preference: Option<i32>,
    pub dynamic_range: Option<String>,
    pub container: Option<String>,
    pub downloader_options: Option<Map<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Thumbnail {
    //TODO: Perpetual update
    pub url: String,
    pub height: Option<usize>,
    pub width: Option<usize>,
    pub preference: Option<i32>,
    pub id: String,
    pub resolution: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct HeatPoint {
    //TODO: Perpetual update
    start_time: f32,
    end_time: f32,
    value: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    pub title: String,
    pub start_time: f32,
    pub end_time: f32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct InfoDict {
    //TODO: Perpetual update
    pub age_limit: u8,
    pub abr: f32,
    pub asr: Option<f32>,
    pub aspect_ratio: Option<f32>,
    pub average_rating: Option<f32>,
    pub fps: Option<f32>,
    pub quality: f32,
    pub stretched_ratio: Option<f32>,
    pub vbr: Option<f32>,
    pub language_preference: Option<i32>,
    pub preference: Option<i32>,
    pub source_preference: i32,
    pub comment_count: Option<u32>,
    #[serde(default)]
    pub channel_follower_count: u32,
    pub duration: u32,
    pub like_count: Option<u32>,
    pub playlist_index: Option<u32>,
    pub epoch: u64,
    pub view_count: u64,
    pub _has_drm: Option<bool>,
    pub __real_download: bool,
    #[serde(default)]
    pub channel_is_verified: bool,
    pub has_drm: Value, //Either boolean or a string "maybe" fsfr
    pub is_live: bool,
    pub playable_in_embed: bool,
    pub was_live: bool,
    pub automatic_captions: Value,
    pub release_timestamp: Value,
    pub subtitles: Value,
    pub audio_channels: Option<usize>,
    pub filesize: Option<usize>,
    pub filetime: Option<usize>,
    pub filesize_approx: usize,
    pub height: Option<usize>,
    pub width: Option<usize>,
    pub _filename: String,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub acodec: String,
    pub alt_title: Option<String>,
    pub audio_ext: String,
    pub availability: String,
    pub channel: String,
    pub creator: Option<String>,
    pub chapters: Option<Vec<Chapter>>,
    pub container: Option<String>,
    pub channel_id: String,
    pub channel_url: String,
    pub display_id: String,
    pub description: String,
    pub dynamic_range: Option<String>,
    pub duration_string: String,
    pub ext: String,
    pub extractor: String,
    pub extractor_key: String,
    pub format: String,
    pub format_index: Option<String>,
    pub filename: String,
    pub fulltitle: String,
    pub format_id: String,
    pub format_note: String,
    pub id: String,
    pub language: Option<String>,
    pub live_status: String,
    pub original_url: String,
    pub playlist: Option<String>,
    pub protocol: String,
    pub resolution: String,
    pub release_date: Option<String>,
    pub release_year: Option<u32>,
    pub requested_subtitles: Option<String>,
    pub title: String,
    pub track: Option<String>,
    pub url: String,
    pub uploader: String,
    pub uploader_id: Option<String>,
    pub upload_date: String,
    pub uploader_url: Option<String>,
    pub vcodec: String,
    pub video_ext: String,
    pub webpage_url: String,
    pub webpage_url_domain: String,
    pub webpage_url_basename: String,
    pub tbr: Option<f32>,
    pub formats: Vec<Format>,
    pub categories: Vec<String>,
    pub format_sort_fields: Option<Vec<String>>,
    pub http_headers: Map<String, Value>,
    pub tags: Vec<String>,
    pub heatmap: Option<Vec<HeatPoint>>,
    pub thumbnails: Vec<Thumbnail>,
    pub thumbnail: Option<String>,
    pub downloader_options: Option<Map<String, Value>>,
    pub _format_sort_fields: Option<Vec<String>>,
    pub artists: Option<Vec<String>>,
    pub creators: Option<Vec<String>>,
    pub manifest_url: Option<String>,
    pub license: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct DownloadStatus {
    //TODO: Perpetual update
    pub status: String,
    pub info_dict: InfoDict,
    pub filename: String,
    pub tmpfilename: Option<String>,
    #[serde(default)]
    pub downloaded_bytes: usize,
    pub total_bytes: Option<usize>,
    pub total_bytes_estimate: Option<f64>,
    pub elapsed: Option<f32>,
    pub eta: Option<f32>,
    pub _eta_str: Option<String>,
    pub _speed_str: Option<String>,
    pub _percent_str: Option<String>,
    pub _total_bytes_str: Option<String>,
    pub _downloaded_bytes_str: Option<String>,
    pub _total_bytes_estimate_str: Option<String>,
    pub _elapsed_str: Option<String>,
    pub _default_template: Option<String>,
    pub speed: Option<f32>,
    #[serde(default)]
    pub fragment_index: usize,
    #[serde(default)]
    pub fragment_count: usize,
    pub ctx_id: Option<usize>,
    pub max_progress: Option<f32>,
    pub progress_idx: Option<usize>,
}
