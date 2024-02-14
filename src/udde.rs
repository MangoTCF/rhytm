use std::{io::{Read, Write}, mem::size_of, thread::sleep, time::Duration};

use anyhow::{Context, Result};
use interprocess::os::unix::udsocket::UdStream;
use log::{info, trace};

pub mod client {
    pub const REQUEST_BATCH: u32 = 0xBA; // format: 0xBA => number of links
    pub const JSON: u32 = 0xBB; // format: 0xBB => json length as u64 => stringified json
}
pub mod master {
    pub const BATCH: u32 = 0xAA; // format: 0xAA => number of links as u8 => links, separated by newlines
    pub const REQUEST_END: u32 = 0xAB; // format: 0xAA => number of links as u8 => links, separated by newlines
}

pub fn send_datagram(conn: &mut UdStream, datagram: &[u8]) {
    let mut sent = 0;

    conn.send(&datagram.len().to_ne_bytes())
        .expect("Unable to send datagram length");
    info!("sent datagram length: {}", datagram.len());

    conn.write_all(datagram).with_context(|| {
        format!(
            "Unable to send datagram: {}",
            std::str::from_utf8(&datagram[sent..]).unwrap()
        )
    }).unwrap();
    info!("sent datagram data: {:?}", datagram);

}

pub fn recv_datagram(conn: &UdStream, mut buf: &mut [u8]) {
    let mut to_recv = 0;
    let mut lenbuf = [0_u8; size_of::<u32>()];
    conn.bytes();
    loop {
        let bytes = conn
            .recv(&mut lenbuf)
            .expect("Unable to receive datagram length");
        info!("recved {}b as len", bytes);
        if bytes >= size_of::<u32>() {
            to_recv = u32::from_ne_bytes(lenbuf[..4].try_into().unwrap());
            break;
        }
    }

    let mut tbuf = Vec::<u8>::new();
    while to_recv > 0 {
        to_recv -= conn
            .recv(&mut tbuf)
            .expect("Unable to receive datagram data") as u32;
        info!(
            "lenbuf is {:?}, tbuf is {:?}, {}b remaining",
            lenbuf, tbuf, to_recv
        );
        sleep(Duration::from_millis(200));
    }
    buf.write_all(&tbuf)
        .expect("Unable to write datagram data to buf");
}
