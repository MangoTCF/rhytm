#[path = "../udde.rs"]
mod udde;
use std::env;
use std::io::{Read, Write};
use std::process::exit;

use anyhow::{Context, Result};
use interprocess::os::unix::udsocket::UdStream;
use log::debug;

/**
 * Print usage information and exit.
 */
fn usage(selfpath: String) {
    eprintln!("Usage: {} <socket path> <alphanumerical id>", selfpath);
}

/**
 * TODO: implement, accepts a socket path and thread id(?) as stdin args, starts download through yt_dlp, injecting callback into hooks, which communicates with master thread to update progress bars via FIFO
 */
fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 || args.len() > 4 {
        usage(args[0].clone());
        exit(1);
    }

    let mut socket = UdStream::connect(args[1].clone())
        .with_context(|| format!("Unable to connect to socket at {}", args[1].clone()))
        .unwrap();
    socket.send(b"Hello, master thread")?;
    println!("Client write: {}", "Hello, master thread");
    let mut buf = [0; 1024];
    socket.recv(&mut buf)?;
    println!("Client read: {:?}", buf);

    let mut batch = Vec::<String>::new();
    let mut sbuf = [0 as u8; 2048];

    loop {
        socket.send(&udde::client::REQUEST_BATCH.to_ne_bytes())?;
        loop {
            let bytes = socket.recv(&mut sbuf)?;
            if bytes == 4 {
                match u32::from_ne_bytes(buf[0..4].try_into().unwrap()) {
                    udde::master::BATCH => {
                        break;
                    }
                    udde::master::REQUEST_END => {
                        exit(0);
                    }
                    _ => unimplemented!("Possible client/master version mismatch"),
                }
            }
        }
        loop {
            let bytes = socket.recv(&mut sbuf)?;

            if bytes == 1 {
                for _ in 0..buf[0] {}
            }
        }
    }

    Ok(())
}
