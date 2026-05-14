use std::io;
use std::time::Instant;
use std::{
    io::{ErrorKind, Read},
    net::SocketAddr,
    time::Duration,
};

use anyhow::bail;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::message::Message;
use crate::message_types::{MessageCode, MessageType};

const BUFFER_SIZE: usize = 1024;

struct Buffers {
    input: Vec<u8>,
    output: Vec<u8>,
}

impl Buffers {
    fn new() -> Self {
        Self {
            input: vec![0; BUFFER_SIZE],
            output: Vec::with_capacity(BUFFER_SIZE),
        }
    }
}

pub struct IcmpConnection {
    socket: Socket,
    remote: SockAddr,
    buffers: Buffers,
    id: u16,
}

impl IcmpConnection {
    pub fn new(remote: SocketAddr, id: u16, timeout: Duration) -> anyhow::Result<Self> {
        Ok(Self {
            socket: setup_socket(format!("0.0.0.0:0").parse()?, timeout)?,
            remote: remote.into(),
            buffers: Buffers::new(),
            id,
        })
    }

    pub fn send(&mut self, typ: MessageType, seq: u16, data: &[u8]) -> anyhow::Result<()> {
        Message::new(typ, MessageCode::None, self.id, seq, data.to_vec())
            .write(&mut self.buffers.output)?;
        self.socket.send_to(&self.buffers.output, &self.remote)?;
        Ok(())
    }

    pub fn recv(&mut self, seq: u16) -> anyhow::Result<Option<Message>> {
        let started = Instant::now();
        let timeout = self.socket.read_timeout()?.unwrap();
        while started.elapsed() < timeout {
            match self.recv_once() {
                Ok(Some(msg)) if msg.ours(self.id, seq) => return Ok(Some(msg)),
                Ok(Some(_)) => (),
                Ok(None) => return Ok(None),
                Err(err) if err.is::<io::Error>() => return Err(err),
                Err(err) => println!("{err:?}"),
            }
        }
        Ok(None)
    }

    pub fn set_ttl(&mut self, ttl: u32) -> anyhow::Result<()> {
        self.socket.set_ttl_v4(ttl)?;
        Ok(())
    }

    fn recv_once(&mut self) -> anyhow::Result<Option<Message>> {
        match self.socket.read(&mut self.buffers.input) {
            Ok(bytes) => Message::parse(&self.buffers.input[..bytes]).map(Option::Some),
            Err(err) if err.kind() == ErrorKind::TimedOut => Ok(None),
            Err(err) => bail!(err),
        }
    }
}

fn setup_socket(local: SocketAddr, timeout: Duration) -> anyhow::Result<Socket> {
    let socket = Socket::new_raw(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    socket.bind(&local.into())?;
    socket.set_read_timeout(Some(timeout))?;
    Ok(socket)
}
