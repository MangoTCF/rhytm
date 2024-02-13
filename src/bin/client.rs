use std::env;
use std::io::{Read, Write};
use std::process::exit;

use anyhow::{Context, Result};
use interprocess::local_socket::LocalSocketStream;

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

    let mut socket = LocalSocketStream::connect(args[1].clone())
        .with_context(|| format!("Unable to connect to socket at {}", args[1].clone()))
        .unwrap();
    socket.write_all(b"Hello, master thread")?;
    let mut buf = Vec::<u8>::new();
    socket.read(&mut buf)?;
    println!(
        "{}",
        std::str::from_utf8(&buf).expect("Client sent invalid UTF-8")
    );
    Ok(())
}
