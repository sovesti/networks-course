use std::{
    env::args,
    net::{SocketAddr, UdpSocket},
    thread,
    time::Duration,
};

use rand::{
    distr::{Bernoulli, Distribution},
    rngs::ThreadRng,
};

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

fn port() -> u16 {
    args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(3037)
}

const BUFFER_SIZE: usize = 32 * 1024;

fn reply(socket: &mut UdpSocket, buf: &[u8], addr: SocketAddr) -> anyhow::Result<()> {
    let response = str::from_utf8(buf)?.to_uppercase();
    socket.send_to(response.as_bytes(), addr)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut listener = UdpSocket::bind(format!("0.0.0.0:{}", port())).unwrap();
    let mut loss = PacketLoss::new(0.2);
    let mut buf = [0; BUFFER_SIZE];
    while let Ok((bytes, addr)) = listener.recv_from(&mut buf) {
        if loss.sample() {
            log::info!("Packet lost");
            continue;
        }
        thread::sleep(Duration::from_millis(100));
        match reply(&mut listener, &buf[0..bytes], addr) {
            Ok(_) => log::info!("Exchanged {bytes} bytes with {addr}"),
            Err(err) => log::error!("{err:?}"),
        }
    }
    Ok(())
}
