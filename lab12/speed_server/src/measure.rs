use std::{
    io::{Cursor, Read},
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use byteorder::{BigEndian, ReadBytesExt};
use dioxus::logger::tracing;
use dioxus_signals::{ReadableExt, Signal, WritableExt};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};

#[derive(Debug)]
struct MeasureConfig {
    start: SystemTime,
    size: usize,
    repeats: usize,
}

impl MeasureConfig {
    fn parse(raw: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(raw);
        Ok(Self {
            start: start(&mut cursor)?,
            size: read_usize(&mut cursor)?,
            repeats: read_usize(&mut cursor)?,
        })
    }

    fn millis_elapsed(&self) -> anyhow::Result<usize> {
        Ok(SystemTime::now().duration_since(self.start)?.as_millis() as usize)
    }

    fn size() -> usize {
        size_of::<u128>() + 2 * size_of::<usize>()
    }
}

fn start(cursor: &mut impl Read) -> anyhow::Result<SystemTime> {
    let epoch = ReadBytesExt::read_u128::<BigEndian>(cursor)?;
    Ok(UNIX_EPOCH + Duration::from_millis(epoch as u64))
}

fn read_usize(cursor: &mut impl Read) -> anyhow::Result<usize> {
    Ok(ReadBytesExt::read_u64::<BigEndian>(cursor)? as usize)
}

#[derive(Default, Clone, Copy)]
pub struct MeasuringState {
    bps: Signal<usize>,
    received: Signal<usize>,
    packets: Signal<usize>,
    total: Signal<usize>,
}

impl MeasuringState {
    pub fn new(
        bps: Signal<usize>,
        received: Signal<usize>,
        packets: Signal<usize>,
        total: Signal<usize>,
    ) -> Self {
        Self {
            bps,
            received,
            packets,
            total,
        }
    }

    fn reset(&mut self) {
        self.bps.set(0);
        self.packets.set(0);
        self.received.set(0);
    }

    fn received(&mut self, config: &MeasureConfig, bytes: usize) -> anyhow::Result<()> {
        self.received.add_assign(bytes);
        self.packets.add_assign(1);
        if config.millis_elapsed()? > 0 {
            self.bps
                .set(*self.received.read() * 1000 / config.millis_elapsed()?);
        }
        Ok(())
    }
}

pub struct MeasureTraffic {
    state: MeasuringState,
    buffer: Vec<u8>,
}

impl MeasureTraffic {
    pub fn new(state: MeasuringState) -> Self {
        Self {
            state,
            buffer: vec![0; 64 * 1024],
        }
    }

    pub async fn measure_udp(&mut self, addr: SocketAddr) -> anyhow::Result<()> {
        self.state.reset();
        let mut udp = UdpSocket::bind(addr).await?;
        let config = self.parse_config_udp(&mut udp).await?;
        self.state.total.set(config.repeats);
        loop {
            tokio::select! {
                bytes = udp.recv(&mut self.buffer) => self.state.received(&config, bytes?)?,
                _ = tokio::time::sleep(Duration::from_secs(1)) => return Ok(())
            }
        }
    }

    pub async fn measure_tcp(&mut self, addr: SocketAddr) -> anyhow::Result<()> {
        self.state.reset();
        let mut tcp = accept_tcp(addr).await?;
        let config = self.parse_config_tcp(&mut tcp).await?;
        self.state.total.set(config.repeats * config.size);
        while let bytes = tcp.read(&mut self.buffer).await?
            && bytes > 0
        {
            self.state.received(&config, bytes)?;
        }
        Ok(())
    }

    async fn parse_config_udp(&mut self, udp: &mut UdpSocket) -> anyhow::Result<MeasureConfig> {
        let mut config = self.buffer.as_mut_slice();
        let mut total = 0;
        while total < size_of::<MeasureConfig>() {
            let bytes = udp.recv(config).await?;
            total += bytes;
            config = &mut config[bytes..];
        }
        let config = MeasureConfig::parse(&self.buffer)?;
        tracing::debug!("{config:?}");
        self.state.received(&config, total)?;
        Ok(config)
    }

    async fn parse_config_tcp(&mut self, tcp: &mut TcpStream) -> anyhow::Result<MeasureConfig> {
        let mut config = vec![0; MeasureConfig::size()];
        tcp.read_exact(&mut config).await?;
        let parse = MeasureConfig::parse(&config);
        let config = parse?;
        self.state.received(&config, MeasureConfig::size())?;
        Ok(config)
    }
}

async fn accept_tcp(addr: SocketAddr) -> anyhow::Result<TcpStream> {
    let tcp = TcpListener::bind(addr).await?;
    let (tcp, _) = tcp.accept().await?;
    Ok(tcp)
}

trait SignalExt {
    fn add_assign(&mut self, other: usize);
}

impl SignalExt for Signal<usize> {
    fn add_assign(&mut self, other: usize) {
        let old = *self.read();
        self.set(old + other);
    }
}
