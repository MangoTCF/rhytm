use std::{
    env,
    fs::{self, Permissions},
    io::{self, ErrorKind, Read, Write},
    net::Shutdown,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::Command,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use core::result::Result::Ok;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use log::{error, info, trace, warn, LevelFilter};
use regex::Regex;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};

const THREAD_COUNT: usize = 6;
const LINK_BATCH_SIZE: usize = 5;
const VIDEO_FOLDER: &str = "/home/mango/Radio/";
const LOGS_FOLDER: &str = "/home/mango/Radio/logs/";
const PARSE_REGEX_STR: &str =
    r"(https://((music\.)|(www\.))youtube\.com/)(watch)[a-zA-Z0-9/\.\?=\-_]+";

fn conn_handle_error(result: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    match result {
        Ok(val) => Some(val),
        Err(error) => {
            eprintln!("There was an error with an incoming connection: {}", error);
            None
        }
    }
}

fn bind_socket_with_cleanup(path: &str) -> LocalSocketListener {
    match fs::remove_file(path) {
        Ok(_) => {
            warn!("Removed leftover socket file: {}", path);
        }
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => (),
                _ => {
                    error!("Unable to remove leftover socket file: {}", e);
                    panic!();
                }
            };
        }
    };

    LocalSocketListener::bind(path).expect(&format!("Failed to bind to socket {}", path))
}

/**
 * TODO: implement initalize, then create NUM_THREADS sockets in /tmp and start the threads, passing the FIFOs and pre-spliced link list to them, add callback(-s?) to update progress
 */
fn main() -> Result<()> {
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

    let path = "/home/mango/cleanhome/programming/rhytm/test_data/ytm.html"; //TODO: parse this from args

    let regex = Regex::new(PARSE_REGEX_STR).unwrap(); //TODO?: parse this from args

    let mut soup = String::new();

    let bytes = fs::File::open(path)
        .expect("Unable to open file")
        .read_to_string(&mut soup)
        .expect("Unable to read file");

    let links: Vec<&str> = regex.captures_iter(&soup).map(|x| x.extract::<6>().1.last().unwrap().to_owned()).collect();

    trace!("Read {} bytes from {}", bytes, path);
    info!("Found {} links", links.len());

    let thread_path = env::current_exe()
        .expect("Failed to get current executable")
        .with_file_name("client");

    let thread_exe = fs::File::open(thread_path.clone())
        .with_context(|| format!("Unable to open client executable {}", thread_path.display()))?;
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

    let mut handles = Vec::<LocalSocketStream>::with_capacity(THREAD_COUNT);

    let sockpath = &format!("/tmp/rhytm-thread-{}.sock", 0);
    let socket = bind_socket_with_cleanup(sockpath);
    let child = Command::new(thread_path.clone())
        .arg(sockpath)
        .arg("0")
        .spawn()
        .with_context(|| format!("Unable to spawn thread {}", "0"))?;

    let conn_handle = socket.incoming().next();
    if !conn_handle.is_none() {
        handles.push(conn_handle.unwrap().expect("Handle is wrong"));
    }

    info!("Spawned thread {}", 0);

    Ok(())
}
