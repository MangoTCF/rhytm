use std::{f32::consts::SQRT_2, io::Read};
#[path = "../udde.rs"]
mod udde;

use anyhow::Result;
use interprocess::os::unix::udsocket::{UdStream, UdStreamListener};
use log::{info, LevelFilter};
use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode};

fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])?;


    let listener = UdStreamListener::bind("/tmp/fuckyou")?;
    let mut client = UdStream::connect("/tmp/fuckyou")?;
    let mut server = listener.incoming().next().unwrap().expect("conn fup");

    info!("sending s1");
    udde::send_datagram(&mut server, b"datagram");
    info!("sending s2");
    udde::send_datagram(&mut server, b"datagram");
    info!("sending c1");
    udde::send_datagram(&mut client, b"datagram");
    info!("sending c2");
    udde::send_datagram(&mut client, b"datagram");

    let mut buf = [0; 1024];
    udde::recv_datagram(&client, &mut buf);
    info!("recvd c1: {}", std::str::from_utf8(&buf)?);
    udde::recv_datagram(&client, &mut buf);
    info!("recvd c2: {}", std::str::from_utf8(&buf)?);
    udde::recv_datagram(&server, &mut buf);
    info!("recvd s1: {}", std::str::from_utf8(&buf)?);
    udde::recv_datagram(&server, &mut buf);
    info!("recvd s2: {}", std::str::from_utf8(&buf)?);

    Ok(())
}
