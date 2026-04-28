use tokio::net::UdpSocket;

use std::{io::Write, net::SocketAddr};

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

pub struct UdtConfig {
    port: u16,
    loss: f64,
}

impl UdtConfig {
    pub fn new(port: u16, loss: f64) -> Self {
        Self { port, loss }
    }

    async fn open_socket(&self) -> anyhow::Result<UdpSocket> {
        Ok(UdpSocket::bind(format!("0.0.0.0:{}", self.port)).await?)
    }

    fn loss(&self) -> PacketLoss {
        PacketLoss::new(self.loss)
    }
}

pub struct UdtStream {
    loss: PacketLoss,
    socket: UdpSocket,
}

impl UdtStream {
    async fn recv(&mut self, buffer: &mut [u8]) -> anyhow::Result<(usize, SocketAddr)> {
        loop {
            let response = self.socket.recv_from(buffer).await?;
            if self.lost() {
                continue;
            }
            return Ok(response);
        }
    }

    fn lost(&mut self) -> bool {
        if self.loss.sample() {
            log::debug!("packet lost =(");
            true
        } else {
            false
        }
    }

    async fn open(config: UdtConfig) -> anyhow::Result<Self> {
        Ok(Self {
            socket: config.open_socket().await?,
            loss: config.loss(),
        })
    }
}

pub async fn connect(config: UdtConfig, target: SocketAddr) -> anyhow::Result<UdtConnection> {
    UdtConnection::open(UdtStream::open(config).await?, None, target).await
}

pub async fn listen(config: UdtConfig) -> anyhow::Result<UdtConnection> {
    let mut stream = UdtStream::open(config).await?;
    let mut buffer = vec![0; 32 * 1024];
    let (bytes, addr) = stream.recv(&mut buffer).await?;
    UdtConnection::open(stream, Some(Vec::from(&buffer[..bytes])), addr).await
}

pub struct UdtConnection {
    stream: UdtStream,
    kept: Option<Vec<u8>>,
}

impl UdtConnection {
    async fn open(
        stream: UdtStream,
        kept: Option<Vec<u8>>,
        target: SocketAddr,
    ) -> anyhow::Result<Self> {
        stream.socket.connect(target).await?;
        Ok(Self { stream, kept })
    }

    pub async fn recv(&mut self, mut buffer: &mut [u8]) -> anyhow::Result<usize> {
        if let Some(kept) = self.kept.take() {
            buffer.write(&kept)?;
            return Ok(kept.len());
        }
        loop {
            let bytes = self.stream.socket.recv(buffer).await?;
            if self.stream.lost() {
                continue;
            }
            return Ok(bytes);
        }
    }

    pub async fn send(&mut self, buffer: &[u8]) -> anyhow::Result<usize> {
        Ok(self.stream.socket.send(buffer).await?)
    }
}
