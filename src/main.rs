mod comms;
mod models;
mod schema;

use anyhow::{Context, Result};
use clap::Parser;
use comms::{DownloadStatus, Message, MessageRead, MessageWrite};
use core::result::Result::Ok;
use diesel::{query_dsl::methods::SelectDsl, sqlite, Connection, RunQueryDsl, SelectableHelper};
use diesel_migrations::MigrationHarness;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::{debug, info, log, warn};
use models::Video;
use regex::Regex;
use simplelog::{error, CombinedLogger, Config, TermLogger, TerminalMode};
use std::time::Duration;
use std::{
    env,
    fs::{self, Permissions},
    io::ErrorKind,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    process::Command,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

use crate::comms::Options;
use crate::models::NewVideo;

pub const EMBEDDED_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn ensure_dir(dir: &str) -> Result<(), std::io::Error> {
    if let Err(e) = fs::create_dir_all(dir) {
        if e.kind() != ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

fn ensure_no_file(file: &str) -> Result<(), std::io::Error> {
    if let Err(e) = fs::remove_file(file) {
        if e.kind() != ErrorKind::NotFound {
            return Err(e);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    use self::schema::videos::dsl::*;
    let options = Options::parse();

    let logs_dir = options.download_dir.clone() + &options.logs_dir_relative;

    let logger = CombinedLogger::new(vec![TermLogger::new(
        options.verbosity,
        Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )]);

    let pb_style = Arc::new(
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes:>7}/{total_bytes:7}, {bytes_per_sec} {msg:>}")
            .unwrap()
            .progress_chars("##-"),
    );
    let mp: Arc<Mutex<MultiProgress>> = Arc::new(Mutex::new(MultiProgress::new()));

    LogWrapper::new(Arc::clone(&mp).lock().unwrap().to_owned(), logger)
        .try_init()
        .expect("Failed to initialise logger");

    log::set_max_level(log::LevelFilter::Trace);

    let soup = fs::read_to_string(options.html_path.clone())?;
    debug!("Read {} bytes from {}", soup.len(), options.html_path);

    let regex = Regex::new(&options.parse_regex_str).unwrap(); //TODO?: parse this from args

    // Ensure that all directories exist
    ensure_dir(&logs_dir).unwrap();
    ensure_dir(&options.tmp_dir).unwrap();
    ensure_dir(&options.download_dir).unwrap();
    let mut connection = sqlite::SqliteConnection::establish(&(options.download_dir.to_string() + "/links.db")).unwrap();

    connection
        .run_pending_migrations(EMBEDDED_MIGRATIONS)
        .unwrap();

    let downloaded_videos: Vec<String> = videos
        .select(Video::as_select())
        .load(&mut connection)
        .unwrap()
        .iter()
        .map(|x: &Video| x.uid.clone())
        .collect();

    let links_raw = regex
        .captures_iter(&soup)
        .map(|x| {
            x.get(5)
                .expect("Unable to find video link capture")
                .as_str()
                .to_owned()
                .clone()
        })
        .collect::<Vec<String>>();

    let len_raw = links_raw.len();

    let links = Box::leak(Box::new(Vec::<String>::new()));

    for l in links_raw {
        if !downloaded_videos.contains(&l) {
            links.push(l.clone());
        }
    }

    info!(
        "Found {} links, {} found in the DB",
        len_raw,
        downloaded_videos.len()
    );
    let batches = Arc::new(Mutex::new(links.chunks(options.link_batch_size)));

    // finding client exe

    let thread_path = env::current_exe()
        .expect("Failed to get current executable")
        .with_file_name("client");

    let thread_exe = fs::File::open(thread_path.clone()).with_context(|| format!("Unable to open client executable {}", thread_path.display()))?;

    let thread_exe_metadata = thread_exe.metadata().expect("Unable to get file metadata");

    if thread_exe_metadata.permissions().mode() & 0o111 != 0o111 {
        warn!(
            "Wrong permissions on {}: {}, expected at least 0o755",
            thread_path.clone().display(),
            thread_exe_metadata.permissions().mode()
        );
        warn!("Trying to fix permissions");
        thread_exe
            .set_permissions(Permissions::from_mode(0o755))
            .expect("Unable to set permissions, exiting");
    }



    ensure_no_file(&(options.tmp_dir.clone() + "/master.sock")).unwrap();
    let listener = UnixListener::bind(options.tmp_dir.clone() + "/master.sock")?;

    let mut handles = Vec::<(JoinHandle<()>, usize)>::with_capacity(options.threads);

    for thr_id in 0..options.threads {
        let msp = options.tmp_dir.clone() + "/master.sock";

        ensure_dir(&(options.tmp_dir.clone() + "/" + &thr_id.to_string())).unwrap();

        // Spawn child
        let _client = Command::new(thread_path.clone())
            // .env_clear()
            .env("MSP", msp)
            .env("THR_ID", thr_id.to_string())
            .env("LOG_DIR", logs_dir.clone())
            .env("DOWNLOAD_DIR", options.download_dir.clone())
            .env("TMP_DIR", options.tmp_dir.clone())
            .env(
                "YT_DLP_OUTPUT_TEMPLATE",
                options.yt_dlp_output_template.clone(),
            )
            .spawn()
            .with_context(|| format!("Unable to spawn thread {}", thr_id))?;
    }
    //now we should have all our threads running and we should try to accept the conns

    let connection = Arc::new(Mutex::new(connection));

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let batches = Arc::clone(&batches);
                let connection = Arc::clone(&connection);
                let pb_style = Arc::clone(&pb_style);
                let msg = stream.read_json_msg::<Message>().unwrap();
                let mp = Arc::clone(&mp);
                let mut audio_ds: DownloadStatus = Default::default();
                let mut video_ds: DownloadStatus = Default::default();

                if std::mem::discriminant(&msg) != std::mem::discriminant(&Message::Greeting(0)) {
                    error!("Received non-greeting message from client: {:?}", msg);
                    error!(
                        "Discriminants: Client: {:?}, Greeting(0): {:?}",
                        std::mem::discriminant(&msg),
                        std::mem::discriminant(&Message::Greeting(0))
                    );
                    panic!("Client sent garbage");
                }

                let thr_id: usize;
                if let Message::Greeting(v) = msg {
                    thr_id = v;
                } else {
                    panic!("How the fuck are we getting here?");
                }

                stream.write_json_msg(&msg).unwrap();

                let pb = ProgressBar::new_spinner();
                pb.set_length(10000);
                pb.enable_steady_tick(Duration::from_millis(25));
                mp.lock().unwrap().add(pb.clone());

                let logs_dir = logs_dir.clone();

                let handle = tokio::spawn(async move {
                    debug!("Thread {:?} functional", thr_id);
                    loop {
                        let logs_dir = logs_dir.clone();
                        match stream.read_json_msg::<Message>().unwrap() {
                            // Batch request
                            Message::BatchRequest => {
                                debug!("got BatchRequest from socket {:?}", thr_id);
                                match batches.lock().unwrap().next() {
                                    Some(batch) => {
                                        let batch = &Message::Batch(batch.to_vec());

                                        debug!("Sending Batch({:?}) to thread {:?}", batch, thr_id);
                                        stream
                                            .write_json_msg(&batch)
                                            .with_context(|| format!("Unable to send Batch to thread {}", thr_id))
                                            .unwrap();
                                    }
                                    None => {
                                        debug!("No batches left, sending EndRequest to thread {:?}", thr_id);
                                        stream
                                            .write_json_msg(&Message::EndRequest)
                                            .expect(&format!("Unable to send EndRequest to thread {:?}", thr_id));
                                        return;
                                    }
                                };
                            }

                            // Log
                            Message::Log {
                                thr_id: _,
                                level,
                                target,
                                msg,
                            } => {
                                debug!("got Log message from socket {:?}", thr_id);

                                log!(target: &target, level, "{}", msg);
                            }

                            // JSON
                            Message::JSON(msg) => {
                                let json: DownloadStatus = serde_json::from_str(&msg).unwrap_or_else(|e| {
                                    std::fs::write(logs_dir.clone() + "fucked.json", &msg).unwrap();
                                    error!(
                                        "Parse failed @ {}:{}, message is {}",
                                        e.line(),
                                        e.column(),
                                        e
                                    );
                                    error!("Context: {}", &msg[e.column() - 20..e.column() + 20]);
                                    error!("                           ^");
                                    error!("                           \\-- Error is here");
                                    panic!("Fucking json")
                                });
                                pb.set_message(format!(
                                    "{} - {} [{}]",
                                    json.info_dict
                                        .creator
                                        .clone()
                                        .unwrap_or(json.info_dict.uploader.clone()),
                                    json.info_dict.title.clone(),
                                    json.info_dict.display_id.clone()
                                ));
                                pb.set_length(
                                    json.total_bytes
                                        .unwrap_or(json.total_bytes_estimate.unwrap_or(0.0) as usize) as u64,
                                );
                                pb.set_position(json.downloaded_bytes as u64);

                                if json.status == "finished" {
                                    std::fs::write(
                                        format!("{}/{}.json", logs_dir, json.filename.replace("/", "_")),
                                        &serde_json::to_string_pretty(&json).unwrap(),
                                    )
                                    .expect("Unable to write json");
                                    if json.info_dict.acodec == "none" {
                                        video_ds = json.clone();
                                    }
                                    if json.info_dict.vcodec == "none" {
                                        audio_ds = json.clone();
                                    }
                                    if json.info_dict.__real_download {
                                        let video_repr = NewVideo {
                                            title: Some(json.info_dict.title),
                                            author: json.info_dict.artist,
                                            duration: Some(json.info_dict.duration.into()),
                                            description: Some(json.info_dict.description),
                                            uid: json.info_dict.display_id.clone(),
                                            link: Some(json.info_dict.webpage_url),
                                        };
                                        debug!("Inserting video {:?}", video_repr);
                                        let inserted_rows = diesel::insert_into(videos)
                                            .values(video_repr)
                                            .execute(&mut *connection.lock().unwrap())
                                            .unwrap();
                                        pb.set_style(ProgressStyle::default_spinner());
                                    }
                                }
                                //TODO: get the json model from wherever and parse it like that instead of stupid "Value" parsing

                                //TODO! parse the fucking JSON, *insert approximately six hours of selfharm*
                            }
                            Message::DownloadStart => {
                                pb.set_style(pb_style.as_ref().clone());
                                pb.tick()
                            }
                            Message::DownloadEnd => {}
                            Message::EndRequest => {
                                unimplemented!("Unexpected EndRequest recieved from socket {:?}", thr_id)
                            }
                            Message::Batch(_) => {
                                unimplemented!("Unexpected Batch recieved from socket {:?}", thr_id)
                            }
                            Message::Greeting(_) => {
                                unimplemented!("Unexpected Greeting recieved from socket {:?}", thr_id)
                            }
                        }
                    }
                });
                handles.push((handle, thr_id));
                info!("Spawned thread {}", 0);
            }
            Err(_) => todo!(),
        }
    }
    // {
    //     /*------------------//
    //     \\Tokio handler loop\\
    //     //------------------*/
    //     debug!("Starting thread {}", thr_id);
    //     let handle =
    // }

    for handle in handles {
        match handle.0.await {
            Ok(_) => {}
            Err(e) => {
                warn!("Thread {:?} failed to finish: {}", handle.1, e)
            }
        };
    }

    Ok(())
}
