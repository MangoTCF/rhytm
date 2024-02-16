use std::os::unix::net::{UnixDatagram, UnixListener};
#[path = "../udde.rs"]
mod udde;

use anyhow::Result;
use log::{info, LevelFilter};
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};

fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])?;

    let _ = std::fs::remove_file("/tmp/fuckyous");
    let _ = std::fs::remove_file("/tmp/fuckyouc");

    let server = UnixListener::bind("/tmp/fuckyous")?;
    let socket = UnixDatagram::bind("/tmp/fuckyous")?;
    let client = UnixDatagram::unbound()?;
    client.connect_addr(&socket.local_addr()?)?;
    socket.connect_addr(&client.local_addr()?)?;

    info!("sent {}b as s1", socket.send(b"cock")?);
    info!("sent {}b as s2", socket.send(b"balls")?);
    info!("sent {}b as c1", client.send(b"cock")?);
    info!("sent {}b as c2", client.send(b"balls")?);

    let mut buf = [0; 1024];
    let mut b = 0;
    b = client.recv(&mut buf)?;
    info!("recvd c1: {}({}b)", std::str::from_utf8(&buf)?, b);
    b = client.recv(&mut buf)?;
    info!("recvd c2: {}({}b)", std::str::from_utf8(&buf)?, b);
    b = socket.recv(&mut buf)?;
    info!("recvd s1: {}({}b)", std::str::from_utf8(&buf)?, b);
    b = socket.recv(&mut buf)?;
    info!("recvd s2: {}({}b)", std::str::from_utf8(&buf)?, b);

    Ok(())
}
