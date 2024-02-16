#[path = "../udde.rs"]
mod udde;

use std::{
    env,
    fs::{self, Permissions},
    io::{ErrorKind, Read},
    os::unix::fs::PermissionsExt,
    process::Command,
    str::FromStr,
    sync::{Arc, Mutex},
};

use num_traits::FromPrimitive;

use std::os::unix::net::UnixDatagram;

use anyhow::{Context, Result};
use core::result::Result::Ok;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use tokio::task::JoinHandle;

use log::{debug, info, warn, Level, LevelFilter};
use regex::Regex;
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};

use crate::udde::{client_msgs, server_msgs};

const THREAD_COUNT: usize = 1;
const LINK_BATCH_SIZE: usize = 5;
const TMP_DIR: &str = "/tmp/rhytm"; // TODO: parse from args
const DOWNLOAD_DIR: &str = "/home/mango/Radio/"; // TODO: parse from args
const LOGS_DIR_RELATIVE: &str = "/logs/";
const PARSE_REGEX_STR: &str =
    r"(https://(music)|(www)\.youtube\.com/)?(watch\?v=)([a-zA-Z0-9/\.\?=\-_]+)";

/**
 * TODO: implement initalize, then create NUM_THREADS sockets in /tmp and start the threads, passing the FIFOs and pre-spliced link list to them, add callback(-s?) to update progress
 */
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

    let path = "/home/mango/cleanhome/programming/rhytm/test_data/ytm.html"; //TODO: parse this from args

    let mut soup = Vec::<u8>::new();
    let bytes = fs::File::open(path)
        .expect("Unable to open file")
        .read_to_end(&mut soup)
        .expect("Unable to read file");
    debug!("Read {} bytes from {}", bytes, path);

    let regex = Regex::new(PARSE_REGEX_STR).unwrap(); //TODO?: parse this from args

    let links = Box::leak(Box::new(
        regex
            .captures_iter(std::str::from_utf8(&soup)?)
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

    let mut handles = Vec::<JoinHandle<_>>::with_capacity(THREAD_COUNT);

    // Ensure that all directories exist
    match std::fs::create_dir(DOWNLOAD_DIR).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("Unable to create {}", DOWNLOAD_DIR)
            }
        }
        None => {}
    }
    match std::fs::create_dir(DOWNLOAD_DIR.to_owned() + LOGS_DIR_RELATIVE).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("Unable to create {}{}", DOWNLOAD_DIR, LOGS_DIR_RELATIVE)
            }
        }
        None => {}
    };
    match std::fs::create_dir(TMP_DIR).err() {
        Some(err) => {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("Unable to create {}", TMP_DIR)
            }
        }
        None => {}
    };

    for thr_id in 0..THREAD_COUNT {
        let mut hbuf = [0 as u8];
        let mut sbuf = [0 as u8; 2048];
        let batches = Arc::clone(&batches);
        let msp = &format!("/tmp/rhytm/master-{}.sock", thr_id);
        let csp = &format!("/tmp/rhytm/thread-{}.sock", thr_id);

        match std::fs::remove_file(msp).err() {
            Some(err) => {
                if err.kind() != ErrorKind::NotFound {
                    panic!("Unable to remove old socket @ {}", msp)
                }
            }
            None => {}
        };
        match std::fs::remove_file(csp).err() {
            Some(err) => {
                if err.kind() != ErrorKind::NotFound {
                    panic!("Unable to remove old socket @ {}", msp)
                }
            }
            None => {}
        };

        let master_socket =
            UnixDatagram::bind(msp).expect(&format!("Unable to create socket @ {}", msp));

        // Spawn child
        let client = Command::new(thread_path.clone())
            .arg(csp)
            .arg(msp)
            .arg(stringify!(thr_id))
            .spawn()
            .with_context(|| format!("Unable to spawn thread {}", thr_id))?;

        master_socket
            .recv(&mut hbuf)
            .expect("Unable to recieve greeting from client");
        if <client_msgs as FromPrimitive>::from_u8(hbuf[0]).unwrap() == client_msgs::Greeting {
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
        let handle = tokio::spawn(async move {
            loop {
                let (_, addr) = master_socket
                    .recv_from(&mut hbuf)
                    .expect("Unable to receive data from master socket");

                match FromPrimitive::from_u8(hbuf[0]).unwrap_or_else(|| {
                    let mut dbuf = [0; 4096];
                    let bytes = master_socket.recv(&mut dbuf).unwrap_or(0);
                    ;
                    panic!(
                        "Wrong client packet header: {}, possible server/client version mismatch. Up to {}b of pending socket contents are:\n{}\n-----EOM-----",
                        hbuf[0],
                        bytes,
                        std::str::from_utf8(&dbuf[0..bytes]).unwrap_or(&format!("Unable to convert raw buffer contents into UTF-8, printing u8 values:\n{:#?}", &dbuf[0..bytes]))
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
                                    .expect(&format!(
                                        "Unable to send batch header to thread {}",
                                        thr_id
                                    ));

                                let batch_ser = x.join("\n");
                                debug!("Sending to client: \n{}\n---EOM---", batch_ser);
                                let batch_ser = batch_ser.as_bytes();
                                master_socket
                                    .send(batch_ser)
                                    .expect(&format!("Unable to send batch to thread {}", thr_id));
                            }
                            None => {
                                debug!("No batches left, sending EndRequest to thread {}", thr_id);
                                master_socket.send(&[server_msgs::EndRequest as u8]).expect(
                                    &format!(
                                        "Unable to send EndRequest header to thread {}",
                                        thr_id
                                    ),
                                );
                            }
                        };
                    }
                    client_msgs::Greeting => {
                        unimplemented!(
                            "Unexpected greeting recieved from thread {}",
                            addr.as_pathname()
                                .unwrap()
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()[7..]
                                .to_string()
                        )
                    }
                    client_msgs::Log => {
                        debug!(
                            "got log message from thread {}",
                            addr.as_pathname()
                                .unwrap()
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()[7..]
                                .to_string()
                        );

                        let mut lbuf = [0; 5];
                        master_socket.recv(&mut lbuf).expect(&format!(
                            "Unable to recieve log message from thread {}",
                            thr_id
                        ));

                        match log::Level::from_str(
                            std::str::from_utf8(&lbuf)
                                .expect(&format!("Invalid UTF-8 in thread {}", thr_id)),
                        )
                        .expect(&format!("Unable to parse log level from thread {}", thr_id))
                        {
                            Level::Error => todo!(),
                            Level::Warn => todo!(),
                            Level::Info => todo!(),
                            Level::Debug => todo!(),
                            Level::Trace => {}
                        };
                    }
                    client_msgs::JSON => {
                        debug!(
                            "got BatchRequest from thread {}",
                            addr.as_pathname()
                                .unwrap()
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()
                        )
                    }
                }
            }
        });
        handles.push(handle);
        info!("Spawned thread {}", 0);
    }

    //spawn child

    //TODO!: Create a "master" socket for each thread, spawn THREAD_COUNT tokio routines and handle the appropriate sockets in each one
    //ffs this shit is hard to figure out

    // loop {
    //     let (b, addr) = master_socket.recv_from(&mut hbuf)?;
    // }

    Ok(())
}
