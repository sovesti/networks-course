use std::{
    io::{self, BufWriter, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket},
    time::{SystemTime, UNIX_EPOCH},
};

use rand::{RngExt, rng};

pub struct RandomTraffic {
    size: usize,
    buffer: Vec<u8>,
}

impl RandomTraffic {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            buffer: vec![0; size],
        }
    }

    pub fn send_tcp(&mut self, target: SocketAddr, repeats: usize) -> anyhow::Result<()> {
        let mut out = BufWriter::new(TcpStream::connect(target)?);
        self.send_all(repeats, |bytes| out.write_all(bytes))?;
        Ok(out.flush()?)
    }

    pub fn send_udp(
        &mut self,
        port: u16,
        target: SocketAddr,
        repeats: usize,
    ) -> anyhow::Result<()> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port))?;
        socket.connect(target)?;
        self.send_all(repeats, |bytes| socket.send(bytes).map(|_| ()))?;
        Ok(())
    }

    fn send_all<F>(&mut self, repeats: usize, mut send: F) -> anyhow::Result<()>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        self.fill_first();
        send(&self.buffer)?;
        (1..repeats).try_for_each(|_| {
            self.fill_other();
            send(&self.buffer).map(|_| ())
        })?;
        Ok(())
    }

    fn fill_first(&mut self) {
        self.buffer.clear();
        self.buffer
            .extend_from_slice(&millis_since_epoch().to_be_bytes());
        self.fill()
    }

    fn fill_other(&mut self) {
        self.buffer.clear();
        self.fill()
    }

    fn fill(&mut self) {
        while self.buffer.len() < self.size {
            self.buffer.push(rng().random());
        }
    }
}

fn millis_since_epoch() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
