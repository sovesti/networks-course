mod stats;

use std::{
    env::args,
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    thread,
    time::{Duration, Instant},
};

use anyhow::bail;
use chrono::{DateTime, Local, Utc};
use serde::Serialize;

use crate::stats::{Session, Stats};

fn port() -> u16 {
    args().nth(1).unwrap_or("3038".to_string()).parse().unwrap()
}

fn addr() -> SocketAddr {
    args()
        .nth(2)
        .unwrap_or("127.0.0.1:3037".to_string())
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
}

#[derive(Serialize)]
struct Report {
    attempt: usize,
    time: DateTime<Utc>,
}

fn message(attempt: usize) -> anyhow::Result<String> {
    Ok(toml::to_string(&Report {
        attempt,
        time: Utc::now(),
    })?)
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
    println!("Ping {attempt} {}", Local::now().format("%H:%M:%S%.3f"));
    socket.send_to(message(attempt)?.as_bytes(), addr)?;
    match socket.recv_from(&mut buf) {
        Ok((bytes, addr)) => stats.received(buf[0..bytes].len(), addr, start.elapsed()),
        Err(err) if err.kind() == ErrorKind::TimedOut => stats.lost(),
        Err(err) => bail!(err),
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let addr = addr();
    let mut socket = UdpSocket::bind(format!("0.0.0.0:{}", port()))?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut stats = Stats::new(Session::new(addr));
    for attempt in 0..usize::MAX {
        ping(attempt, addr, &mut socket, &mut stats)?;
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
