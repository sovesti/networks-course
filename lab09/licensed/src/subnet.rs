use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use dioxus::{logger::tracing, prelude::*};
use ip_tools::interfaces::my_subnet;
use tokio::{net::UdpSocket, runtime::Handle, task::block_in_place, time::Instant};

const MAGIC: u8 = 0xD6;

#[derive(Clone, Copy, Debug)]
pub enum Message {
    Started = 0,
    Running = 1,
    Stopped = 2,
}

impl Message {
    fn parse(buffer: &[u8]) -> Option<Self> {
        if buffer.len() < 2 || buffer[0] != MAGIC {
            return None;
        }
        match buffer[1] {
            0 => Some(Message::Started),
            1 => Some(Message::Running),
            2 => Some(Message::Stopped),
            _ => None,
        }
    }

    fn serialize(&self, buffer: &mut [u8]) {
        buffer[0] = MAGIC;
        buffer[1] = *self as u8;
    }
}

pub type KnownCopies = Signal<HashMap<SocketAddr, Instant>>;

pub struct Subnet {
    socket: UdpSocket,
    subnet: IpAddr,
    output: Vec<u8>,
    input: Vec<u8>,
    copies: KnownCopies,
}

impl Subnet {
    pub async fn setup(addr: SocketAddr, copies: KnownCopies) -> anyhow::Result<Self> {
        let mut subnet = Self::new(addr, copies).await?;
        subnet.broadcast(Message::Started).await?;
        Ok(subnet)
    }

    async fn new(addr: SocketAddr, copies: KnownCopies) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        socket.set_broadcast(true)?;
        Ok(Self {
            socket,
            subnet: my_subnet()?,
            output: vec![0; 2],
            input: vec![0; 2],
            copies,
        })
    }

    async fn recv(&mut self) -> anyhow::Result<(Message, SocketAddr)> {
        loop {
            let (bytes, addr) = self.socket.recv_from(&mut self.input).await?;
            if let Some(message) = Message::parse(&self.input[..bytes]) {
                return Ok((message, addr));
            };
        }
    }

    pub async fn handle_message(&mut self) -> anyhow::Result<()> {
        let (message, addr) = self.recv().await?;
        tracing::debug!("received {message:?} from {addr:?}");
        match message {
            Message::Started => {
                self.record_running(addr);
                self.send_to(Message::Running, addr).await?;
            }
            Message::Running => self.record_running(addr),
            Message::Stopped => {
                let _ = self.copies.remove(&addr);
            }
        }
        Ok(())
    }

    pub fn prune(&mut self, timeout: u64) {
        let stale = self
            .copies
            .read()
            .iter()
            .filter(|(_, last)| last.elapsed() > Duration::from_millis(timeout))
            .map(|(&addr, _)| addr)
            .collect::<Vec<_>>();
        stale.iter().for_each(|addr| {
            let _ = self.copies.remove(&addr);
        });
    }

    fn record_running(&mut self, addr: SocketAddr) {
        let _ = self.copies.insert(addr, Instant::now());
    }

    pub async fn broadcast(&mut self, message: Message) -> anyhow::Result<()> {
        tracing::debug!("broadcasting {message:?}");
        for port in 1..=u16::MAX {
            self.send_to(message, SocketAddr::new(self.subnet, port))
                .await?;
        }
        Ok(())
    }

    async fn send_to(&mut self, message: Message, addr: SocketAddr) -> anyhow::Result<()> {
        message.serialize(&mut self.output);
        self.socket.send_to(&mut self.output, addr).await?;
        Ok(())
    }
}

impl Drop for Subnet {
    fn drop(&mut self) {
        block_in_place(move || {
            let _ = Handle::current().block_on(self.broadcast(Message::Stopped));
        });
    }
}
