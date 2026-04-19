mod stats;

use std::{
    env::args,
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::{Duration, Instant},
};

use anyhow::bail;
use chrono::Local;

use crate::stats::{Session, Stats};

fn addr() -> SocketAddr {
    args()
        .nth(1)
        .unwrap_or("127.0.0.1:3037".to_string())
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
}

fn message() -> String {
    "Lorem ipsum dolor sit amet orci.".to_string()
}

const BUFFER_SIZE: usize = 32 * 1024;

fn ping(
    attempt: usize,
    addr: SocketAddr,
    socket: &mut UdpSocket,
    stats: &mut Stats,
) -> anyhow::Result<()> {
    let mut buf = [0; BUFFER_SIZE];
    let start = Instant::now();
    // println!(
    //     "=== Ping {attempt} {} ===",
    //     Local::now().format("%H:%M:%S%.3f")
    // );
    // println!("Sending: {}", message());
    socket.send_to(message().as_bytes(), addr)?;
    match socket.recv_from(&mut buf) {
        Ok((bytes, addr)) => received(stats, &buf[0..bytes], addr, start.elapsed()),
        Err(err) if err.kind() == ErrorKind::TimedOut => stats.lost(),
        Err(err) => bail!(err),
    }
    Ok(())
}

fn received(stats: &mut Stats, bytes: &[u8], addr: SocketAddr, time: Duration) {
    // str::from_utf8(bytes)
    //     .iter()
    //     .for_each(|msg| println!("Response: {msg}"));
    stats.received(bytes.len(), addr, time);
}

fn main() -> anyhow::Result<()> {
    let addr = addr();
    let mut socket = UdpSocket::bind("0.0.0.0:3038")?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.connect(addr)?;
    let session = Session::new(addr, message().as_bytes().len());
    println!("{session}");
    let mut stats = Stats::new(session);
    for attempt in 1..=10 {
        ping(attempt, addr, &mut socket, &mut stats)?;
    }
    println!("{stats}");
    Ok(())
}
