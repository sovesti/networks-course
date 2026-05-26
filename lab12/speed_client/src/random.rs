use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpStream, UdpSocket},
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
            buffer: Vec::with_capacity(size),
        }
    }

    pub async fn send_tcp(&mut self, target: SocketAddr, repeats: usize) -> anyhow::Result<()> {
        let mut out = BufWriter::new(TcpStream::connect(target).await?);
        self.send_all(repeats, async |bytes| out.write_all(bytes).await)
            .await?;
        Ok(out.flush().await?)
    }

    pub async fn send_udp(
        &mut self,
        port: u16,
        target: SocketAddr,
        repeats: usize,
    ) -> anyhow::Result<()> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port)).await?;
        socket.connect(target).await?;
        self.send_all(repeats, async |bytes| socket.send(bytes).await.map(|_| ()))
            .await?;
        Ok(())
    }

    async fn send_all<F>(&mut self, repeats: usize, mut send: F) -> anyhow::Result<()>
    where
        F: AsyncFnMut(&[u8]) -> io::Result<()>,
    {
        let start = millis_since_epoch();
        for _ in 0..repeats {
            self.fill(start, repeats);
            send(&self.buffer).await?;
        }
        Ok(())
    }

    fn fill(&mut self, start: u128, repeats: usize) {
        self.buffer.clear();
        self.write_header(start, repeats);
        while self.buffer.len() < self.size {
            self.buffer.push(rng().random());
        }
    }

    fn write_header(&mut self, start: u128, repeats: usize) {
        self.write(&start.to_be_bytes());
        self.write(&self.size.to_be_bytes());
        self.write(&repeats.to_be_bytes());
    }

    fn write(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }
}

fn millis_since_epoch() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
