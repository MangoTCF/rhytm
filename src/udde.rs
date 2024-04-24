use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientMsgs {
    Greeting,
    Log {
        thr_id: usize,
        level: log::Level,
        target: String,
        msg: String,
    },
    BatchRequest,
    Batch(Vec<String>),
    JSON(String),
    EndRequest,
}

// #[derive(Queryable, Selectable)]
// #[diesel(check_for_backend(diesel))]
// #[diesel(table_name = videos)]
// pub struct VideoRepr {
//     pub pk: String,
//     pub uid: String,
//     pub link: String,
//     pub title: String,
//     pub author: String,
//     pub duration: u64,
//     pub description: String,
//     pub thumbnail_path: String,
//     pub date: Date,
//     pub other: String,
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DownloaderOptions {
    pub http_chunk_size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    //TODO: Perpetual update
    url: String,
    duration: f32,
}

#[derive(Serialize, Deserialize, Debug)]
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
    pub downloader_options: Option<DownloaderOptions>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct HeatPoint {
    //TODO: Perpetual update
    start_time: f32,
    end_time: f32,
    value: f32,
}
#[derive(Serialize, Deserialize, Debug)]
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
    pub vbr: f32,
    pub language_preference: i32,
    pub preference: Option<i32>,
    pub source_preference: i32,
    pub comment_count: Option<u32>,
    pub channel_follower_count: u32,
    pub duration: u32,
    pub like_count: u32,
    pub playlist_index: Option<u32>,
    pub epoch: u64,
    pub view_count: u64,
    pub _has_drm: Option<bool>,
    pub __real_download: bool,
    #[serde(default)]
    pub channel_is_verified: bool,
    pub has_drm: bool,
    pub is_live: bool,
    pub playable_in_embed: bool,
    pub was_live: bool,
    pub automatic_captions: Value,
    pub release_timestamp: Value,
    pub subtitles: Value,
    pub audio_channels: Option<usize>,
    pub filesize: usize,
    pub filetime: Option<usize>,
    pub filesize_approx: usize,
    pub height: Option<usize>,
    pub width: Option<usize>,
    pub _filename: String,
    pub album: String,
    pub artist: String,
    pub acodec: String,
    pub alt_title: String,
    pub audio_ext: String,
    pub availability: String,
    pub channel: String,
    pub creator: String,
    pub chapters: Option<String>,
    pub container: String,
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
    pub track: String,
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
    pub downloader_options: Map<String, Value>,
    pub _format_sort_fields: Option<Vec<String>>,
    pub artists: Vec<String>,
    pub creators: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DownloadStatus {
    //TODO: Perpetual update
    pub status: String,
    pub info_dict: InfoDict,
    pub filename: String,
    pub tmpfilename: Option<String>,
    #[serde(default)]
    pub downloaded_bytes: usize,
    pub total_bytes: usize,
    pub total_bytes_estimate: Option<usize>,
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
}
