use diesel::sql_types::Date;
use diesel::Queryable;
use diesel::Selectable;
use num_derive::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum client_msgs {
    Greeting,     // GREETING => \
    Log,          // LOG => log::Level as u8 => msg length as usize => log message
    BatchRequest, // BATCH_REQUEST => \
    JSON,         // JSON => json length as usize => serialized json from ytdlp
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum server_msgs {
    Greeting,   // GREETING => \
    Batch,      // BATCH => link count as u16 => links as a string, \n is the separator
    EndRequest, // REQUEST_END => \
}

// #[derive(Queryable, Selectable)]
// #[diesel(check_for_backend(diesel))]
// #[diesel(table_name = videos)]
pub struct VideoRepr {
    pub pk: String,
    pub uid: String,
    pub link: String,
    pub title: String,
    pub author: String,
    pub duration: u64,
    pub description: String,
    pub thumbnail_path: String,
    pub date: Date,
    pub other: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Fragment {
    //TODO: Perpetual update
    url: String,
    duration: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Format {
    //TODO: Perpetual update
    pub format_id: String,
    pub format_note: Option<String>,
    pub ext: String,
    pub protocol: String,
    pub acodec: Option<String>,
    pub vcodec: Option<String>,
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
    pub video_ext: String,
    pub vbr: Option<f32>,
    pub abr: Option<f32>,
    pub tbr: Option<f32>,
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Thumbnail {
    //TODO: Perpetual update
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub preference: Option<i32>,
    pub id: String,
    pub resolution: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeatPoint {
    //TODO: Perpetual update
    start_time: f32,
    end_time: f32,
    value: f32,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct InfoDict {
    //TODO: Perpetual update
    pub id: String,
    pub title: String,
    pub formats: Vec<Format>,
    pub thumbnails: Vec<Thumbnail>,
    pub description: String,
    pub channel_id: String,
    pub channel_url: String,
    pub duration: u32,
    pub view_count: u64,
    pub average_rating: Option<f32>,
    pub age_limit: u8,
    pub webpage_url: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub playable_in_embed: bool,
    pub live_status: String,
    pub release_timestamp: Option<Value>,
    pub format_sort_fields: Option<Vec<String>>,
    pub automatic_captions: Value,
    pub subtitles: Value,
    pub album: String,
    pub artist: String,
    pub track: String,
    pub release_date: Option<Value>,
    pub release_year: Option<Value>,
    pub comment_count: u32,
    pub chapters: Option<Value>,
    pub heatmap: Option<Vec<HeatPoint>>,
    pub like_count: u32,
    pub channel: String,
    pub channel_follower_count: u32,
    pub channel_is_verified: bool,
    pub uploader: String,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub upload_date: Option<String>,
    pub creator: String,
    pub alt_title: Option<String>,
    pub availability: Option<String>,
    pub original_url: Option<String>,
    pub webpage_url_basename: Option<String>,
    pub webpage_url_domain: Option<String>,
    pub extractor: String,
    pub extractor_key: Option<String>,
    pub playlist: Option<String>,
    pub playlist_index: Option<u32>,
    pub display_id: Option<String>,
    pub fulltitle: Option<String>,
    pub duration_string: Option<String>,
    pub is_live: bool,
    pub was_live: bool,
    pub requested_subtitles: Option<Value>,
    pub _has_drm: Option<Value>,
    pub epoch: u64,
    pub format: String,
    pub format_id: Option<String>,
    pub ext: String,
    pub protocol: String,
    pub language: Option<String>,
    pub format_note: Option<String>,
    pub filesize_approx: usize,
    pub tbr: f32,
    pub width: usize,
    pub height: usize,
    pub resolution: String,
    pub fps: f32,
    pub dynamic_range: String,
    pub vcodec: String,
    pub vbr: f32,
    pub streched_ratio: Option<f32>,
    pub aspect_ratio: Option<f32>,
    pub acodec: String,
    pub abr: f32,
    pub asr: Option<f32>,
    pub audio_channels: Option<String>,
    pub filename: String,
    pub _filename: String,
    pub __real_download: bool,
    pub filesize: usize,
    pub source_preference: i32,
    pub quality: f32,
    pub has_drm: bool,
    pub url: String,
    pub language_preference: i32,
    pub preference: Option<Value>,
    pub container: String,
    pub downloader_options: Option<Value>,
    pub http_headers: Option<Value>,
    pub video_ext: String,
    pub audio_ext: String,
    pub filetime: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DownloadStatus {
    //TODO: Perpetual update
    pub status: String,
    pub info_dict: InfoDict,
    pub filename: String,
    pub tmpfilename: Option<String>,
    pub downloaded_bytes: Option<usize>,
    pub total_bytes: Option<usize>,
    pub total_bytes_estimate: Option<usize>,
    pub elapsed: f32,
    pub eta: Option<f32>,
    pub speed: Option<f32>,
    pub fragment_index: Option<usize>,
    pub fragment_count: Option<usize>,
}
