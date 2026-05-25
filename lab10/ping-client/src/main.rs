mod icmp;
mod message;
mod stats;

use std::{
    env::args,
    net::{SocketAddr, ToSocketAddrs},
    thread,
    time::{Duration, Instant},
};

use chrono::Local;

use crate::{
    icmp::IcmpConnection,
    message::MessageType,
    stats::{Session, Stats},
};

fn host() -> String {
    args().nth(1).unwrap()
}

fn addr() -> SocketAddr {
    (host(), 0).to_socket_addrs().unwrap().next().unwrap()
}

const BYTES: usize = 64;
const ATTEMPTS: u16 = 4;

fn message(attempt: u16) -> String {
    let message = format!("Ping {attempt} {}", Local::now().format("%H:%M:%S%.3f"));
    let len = message.len();
    message + &"*".repeat(BYTES - len)
}

fn ping(
    attempt: u16,
    addr: SocketAddr,
    socket: &mut IcmpConnection,
    stats: &mut Stats,
) -> anyhow::Result<()> {
    let start = Instant::now();
    socket.send(MessageType::Echo, attempt, message(attempt).as_bytes())?;
    match socket.recv(attempt) {
        Ok(None) => stats.lost("Request timed out."),
        Ok(Some(message)) if message.failure() => stats.lost(&message.show_error()),
        Ok(Some(message)) => stats.received(message.len(), addr, start.elapsed()),
        Err(err) => stats.lost(&format!("Error: {err:?}")),
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let addr = addr();
    let mut socket = IcmpConnection::new(addr, 122)?;
    let session = Session::new(host(), addr, BYTES);
    println!("{session}");
    let mut stats = Stats::new(session);
    for attempt in 0..ATTEMPTS {
        ping(attempt, addr, &mut socket, &mut stats)?;
        thread::sleep(Duration::from_secs(1));
    }
    print!("{stats}");
    Ok(())
}
