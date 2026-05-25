use std::{
    cmp::Ordering,
    collections::HashMap,
    env::args,
    net::{SocketAddr, UdpSocket},
    thread,
    time::Duration,
};

use chrono::{DateTime, Utc};
use rand::{
    distr::{Bernoulli, Distribution},
    rngs::ThreadRng,
};
use serde::Deserialize;

struct PacketLoss {
    distr: Bernoulli,
    rng: ThreadRng,
}

impl PacketLoss {
    fn new(p: f64) -> Self {
        Self {
            distr: Bernoulli::new(p).unwrap(),
            rng: rand::rng(),
        }
    }

    fn sample(&mut self) -> bool {
        self.distr.sample(&mut self.rng)
    }
}

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Debug)]
struct Report {
    attempt: usize,
    time: DateTime<Utc>,
}

impl TryFrom<&[u8]> for Report {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        Ok(toml::from_slice(value)?)
    }
}

impl Ord for Report {
    fn cmp(&self, other: &Self) -> Ordering {
        self.attempt.cmp(&other.attempt)
    }
}

#[derive(Default)]
struct Clients {
    known: HashMap<SocketAddr, Report>,
}

impl Clients {
    fn reported(&mut self, addr: SocketAddr, report: Report) {
        if !self.known.contains_key(&addr) {
            log::info!("New client {addr}");
            let _ = self.known.insert(addr, report);
            return;
        }
        self.replace_previous(addr, report);
    }

    fn replace_previous(&mut self, addr: SocketAddr, report: Report) {
        let previous = self.known.get(&addr).unwrap();
        if previous < &report {
            log_lost(addr, previous, &report);
            let _ = self.known.insert(addr, report);
        }
    }

    fn check_abnormals(&mut self) {
        let absent = self.absent(Utc::now());
        absent.iter().for_each(|addr| {
            log::warn!(
                "Lost connection with {addr} after {}",
                self.known.remove(&addr).unwrap().attempt
            )
        });
    }

    fn absent(&mut self, now: DateTime<Utc>) -> Vec<SocketAddr> {
        self.known
            .iter()
            .filter(|(_, last)| (now - last.time).num_seconds() >= abnormal_absence())
            .map(|(&addr, _)| addr)
            .collect()
    }
}

fn log_lost(addr: SocketAddr, previous: &Report, report: &Report) {
    let lost: Vec<String> = ((previous.attempt + 1)..report.attempt)
        .map(|id| id.to_string())
        .collect();
    if !lost.is_empty() {
        log::warn!("{addr}: lost packets {}", lost.join(", "))
    }
}

fn port() -> u16 {
    args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(3037)
}

fn abnormal_absence() -> i64 {
    args().nth(2).and_then(|p| p.parse().ok()).unwrap_or(10)
}

const BUFFER_SIZE: usize = 32 * 1024;

fn reported(clients: &mut Clients, addr: SocketAddr, buf: &[u8]) -> anyhow::Result<()> {
    let report = buf.try_into()?;
    clients.reported(addr, report);
    clients.check_abnormals();
    Ok(())
}

fn reply(
    socket: &mut UdpSocket,
    clients: &mut Clients,
    addr: SocketAddr,
    buf: &[u8],
) -> anyhow::Result<()> {
    reported(clients, addr, buf)?;
    socket.send_to("OK".as_bytes(), addr)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut listener = UdpSocket::bind(format!("0.0.0.0:{}", port())).unwrap();
    let mut loss = PacketLoss::new(0.2);
    let mut clients = Clients::default();
    let mut buf = [0; BUFFER_SIZE];
    while let Ok((bytes, addr)) = listener.recv_from(&mut buf) {
        if loss.sample() {
            continue;
        }
        thread::sleep(Duration::from_millis(100));
        if let Err(err) = reply(&mut listener, &mut clients, addr, &buf[0..bytes]) {
            log::error!("{addr}: {err:?}");
        }
    }
    Ok(())
}
