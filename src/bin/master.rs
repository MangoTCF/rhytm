#[path = "../udde.rs"]
mod udde;

use std::{
    env,
    fs::{self, Permissions},
    io::ErrorKind,
    os::unix::fs::PermissionsExt,
    process::Command,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use core::mem::size_of;

use diesel::{sqlite, Connection};
use diesel_migrations::embed_migrations;
use num_traits::FromPrimitive;
use serde_json::Value;

use std::os::unix::net::UnixDatagram;

use anyhow::{Context, Result};
use core::result::Result::Ok;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use tokio::task::JoinHandle;

use log::{debug, info, log, warn, LevelFilter};
use regex::Regex;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};

use crate::udde::{client_msgs, server_msgs, DownloadStatus};

const THREAD_COUNT: usize = 1;
const LINK_BATCH_SIZE: usize = 5;
const TMP_DIR: &str = "/tmp/rhytm"; // TODO: parse from args
const DOWNLOAD_DIR: &str = "/home/mango/Radio/"; // TODO: parse from args
const LOGS_DIR_RELATIVE: &str = "/logs/";
const PARSE_REGEX_STR: &str = r"(https://(music)|(www)\.youtube\.com/)?(watch\?v=)([a-zA-Z0-9/\.\?=\-_]+)";

#[tokio::main]
async fn main() -> Result<()> {
    // std::panic::set_hook(Box::new(panic_hook));

    let logger = CombinedLogger::new(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )]);

    let mp: Arc<Mutex<MultiProgress>> = Arc::new(Mutex::new(MultiProgress::new()));

    LogWrapper::new(Arc::clone(&mp).lock().unwrap().to_owned(), logger)
        .try_init()
        .expect("Failed to initialise logger");

    log::set_max_level(log::LevelFilter::Trace);

    let path = "/home/mango/pt/rhytm/test_data/ytm.html"; //TODO: parse this from args

    let soup = fs::read_to_string(path)?;
    debug!("Read {} bytes from {}", soup.len(), path);

    let regex = Regex::new(PARSE_REGEX_STR).unwrap(); //TODO?: parse this from args

    let migrations = embed_migrations!();
    let db = sqlite::SqliteConnection::establish(&(DOWNLOAD_DIR.to_string() + "links.db"));

    let links = Box::leak(Box::new(
        regex
            .captures_iter(&soup)
            .map(|x| {
                x.get(5)
                    .expect("Unable to find video ID capture")
                    .as_str()
                    .to_owned()
                    .clone()
            })
            .collect::<Vec<String>>(),
    ));

    info!("Found {} links", links.len());
    let batches = Arc::new(Mutex::new(links.chunks(LINK_BATCH_SIZE)));

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

    // Ensure that all directories exist
    match std::fs::create_dir(DOWNLOAD_DIR).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("Unable to create {}: {}", DOWNLOAD_DIR, err);
            }
        }
        None => {}
    }
    match std::fs::create_dir(DOWNLOAD_DIR.to_owned() + LOGS_DIR_RELATIVE).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!(
                    "Unable to create {}{}: {}",
                    DOWNLOAD_DIR, LOGS_DIR_RELATIVE, err
                );
            }
        }
        None => {}
    };
    match std::fs::create_dir(TMP_DIR).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("Unable to create {}: {}", TMP_DIR, err);
            }
        }
        None => {}
    };

    let mut handles = Vec::<(JoinHandle<_>, usize)>::with_capacity(THREAD_COUNT);

    for thr_id in 0..THREAD_COUNT {
        let mut hbuf = [0 as u8];
        let mut sbuf = Vec::<u8>::new();
        let batches = Arc::clone(&batches);
        let msp = &format!("{}/{}/master.sock", TMP_DIR, thr_id);
        let csp = &format!("{}/{}/client.sock", TMP_DIR, thr_id);

        match std::fs::create_dir(TMP_DIR.to_string() + "/" + &thr_id.to_string()).err() {
            Some(err) => {
                if err.kind() != ErrorKind::AlreadyExists {
                    panic!("Unable to create {}: {}", TMP_DIR, err)
                }
                match std::fs::remove_file(msp).err() {
                    Some(err) => {
                        if err.kind() != ErrorKind::NotFound {
                            panic!("Unable to remove old socket @ {}: {}", msp, err);
                        }
                    }
                    None => {}
                };

                match std::fs::remove_file(csp).err() {
                    Some(err) => {
                        if err.kind() != ErrorKind::NotFound {
                            panic!("Unable to remove old socket @ {}: {}", csp, err);
                        }
                    }
                    None => {}
                };
            }
            None => {}
        };

        let master_socket = UnixDatagram::bind(msp).expect(&format!("Unable to create socket @ {}", msp));

        // Spawn child
        let _client = Command::new(thread_path.clone())
            .arg(csp)
            .arg(msp)
            .arg(thr_id.to_string())
            .spawn()
            .with_context(|| format!("Unable to spawn thread {}", thr_id))?;

        let to = master_socket.read_timeout()?;
        master_socket.set_read_timeout(Some(Duration::from_millis(5000)))?;
        master_socket
            .recv(&mut hbuf)
            .expect("Unable to recieve greeting from client");
        master_socket.set_read_timeout(to)?;
        if <client_msgs as FromPrimitive>::from_u8(hbuf[0]).unwrap() == client_msgs::Greeting {
            debug!("Recved greeting from client");
            master_socket
                .connect(csp)
                .expect("Unable to connect to client");
            master_socket
                .send(&[server_msgs::Greeting as u8])
                .expect("Unable to send greeting to client");
        }

        /*------------------//
        \\Tokio handler loop\\
        //------------------*/
        debug!("Starting thread {}", thr_id);
        let handle = tokio::spawn(async move {
            debug!("Thread {} functional", thr_id);
            let thr_target = &format!("thread {}", thr_id);
            loop {
                let (_, addr) = master_socket
                    .recv_from(&mut hbuf)
                    .expect("Unable to receive data from master socket");

                match FromPrimitive::from_u8(hbuf[0]).unwrap_or_else(|| {
                    let bytes = master_socket.recv(&mut sbuf).unwrap_or(0);
                    panic!(
                        "Wrong client packet header: {}, possible server/client version mismatch. {}b of pending socket contents are:\n\
                        {:#?}\n\
                        -----EOM-----",
                        hbuf[0],
                        bytes,
                        &sbuf[0..bytes]
                    )
                }) {
                    client_msgs::BatchRequest => {
                        debug!(
                            "got BatchRequest from socket {}",
                            addr.as_pathname().unwrap().to_str().unwrap()
                        );
                        match batches.lock().unwrap().next() {
                            Some(x) => {
                                master_socket
                                    .send(&[server_msgs::Batch as u8])
                                    .expect(&format!("Unable to send batch header to thread {}", thr_id));

                                let batch_ser = x.join("\n");
                                debug!("Sending to client: \n{}\n---EOM---", batch_ser);
                                let batch_ser = batch_ser.as_bytes();
                                master_socket
                                    .send(batch_ser)
                                    .expect(&format!("Unable to send batch to thread {}", thr_id));
                            }
                            None => {
                                debug!("No batches left, sending EndRequest to thread {}", thr_id);
                                master_socket
                                    .send(&[server_msgs::EndRequest as u8])
                                    .expect(&format!(
                                        "Unable to send EndRequest header to thread {}",
                                        thr_id
                                    ));
                                return;
                            }
                        };
                    }
                    client_msgs::Greeting => {
                        unimplemented!(
                            "Unexpected greeting recieved from socket {}",
                            addr.as_pathname().unwrap().to_str().unwrap()
                        )
                    }
                    client_msgs::Log => {
                        debug!(
                            "got Log message from socket {}",
                            addr.as_pathname().unwrap().to_str().unwrap()
                        );

                        let mut lbuf = [0; 5];
                        let b = master_socket.recv(&mut lbuf).expect(&format!(
                            "Unable to recieve log message from thread {}",
                            thr_id
                        ));

                        let l = log::Level::from_str(std::str::from_utf8(&lbuf[..b]).expect(&format!("Invalid UTF-8 in thread {}", thr_id))).expect(
                            &format!(
                                "Unable to parse \"{}\" as log level from thread {}",
                                std::str::from_utf8(&lbuf).unwrap(),
                                thr_id
                            ),
                        );

                        let mut len = [0; size_of::<usize>()];

                        master_socket
                            .recv(&mut len)
                            .expect("Unable to receive log message length from thread");

                        let mut msg = vec![0; usize::from_ne_bytes(len)];

                        let b = master_socket
                            .recv(&mut msg)
                            .expect("Unable to receive log message from thread");
                        log!(target: thr_target, l, "{}", std::str::from_utf8(&msg[..b]).expect("Unable to parse log message from thread"));
                    }
                    client_msgs::JSON => {
                        debug!(
                            "got JSON from socket {}",
                            addr.as_pathname().unwrap().to_str().unwrap()
                        );
                        let mut len = [0; size_of::<usize>()];
                        master_socket
                            .recv(&mut len)
                            .expect("Unable to receive log message length from thread");

                        let mut msg = vec![0; usize::from_ne_bytes(len)];
                        let b = master_socket
                            .recv(&mut msg)
                            .expect("Unable to receive log message from thread");
                        let json: DownloadStatus = serde_json::from_slice(&msg[..b]).unwrap_or_else(|err| {
                            let path = "/home/mango/pt/rhytm/test_output/ohno.json";
                            std::fs::write(path, &msg[..b]).expect("Unable to write json");
                            panic!(
                                "Unable to parse JSON from thread {}: {}\n!!JSON Written to {}!!",
                                thr_id, err, path
                            )
                        });

                        if json.status == "finished" {
                            std::fs::write(
                                format!(
                                    "/home/mango/pt/rhytm/test_output/{}.json",
                                    json.filename.replace("/", "_")
                                ),
                                &msg[..b],
                            )
                            .expect("Unable to write json");
                        }
                        //TODO: get the json model from wherever and parse it like that instead of stupid "Value" parsing

                        //TODO! parse the fucking JSON, *insert approximately six hours of selfharm*
                    }
                }
            }
        });
        handles.push((handle, thr_id));
        info!("Spawned thread {}", 0);
    }

    for handle in handles {
        match handle.0.await {
            Ok(_) => {}
            Err(e) => {
                warn!("Thread {} failed to finish: {}", handle.1, e)
            }
        };
    }

    Ok(())
}
