use std::net::Ipv4Addr;

use anyhow::anyhow;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

use crate::message_types::{MessageCode, MessageType};

const IP_LENGTH_OFFSET: usize = 0;
const IP_SOURCE_OFFSET: usize = 12;

const HEADER_LENGTH: usize = 8;
const TYPE_OFFSET: usize = 0;
const CODE_OFFSET: usize = 1;
const CHECKSUM_OFFSET: usize = 2;
const ID_OFFSET: usize = 4;
const SEQ_OFFSET: usize = 6;

struct IpHeader {
    length: usize,
    source: Ipv4Addr,
}

impl IpHeader {
    fn new() -> Self {
        Self {
            length: 0,
            source: Ipv4Addr::UNSPECIFIED,
        }
    }

    fn parse(data: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            length: parse_ip_length(data[IP_LENGTH_OFFSET]),
            source: parse_ip_source(&data[IP_SOURCE_OFFSET..])?,
        })
    }
}

fn parse_ip_length(byte: u8) -> usize {
    (byte & 0xf) as usize * 4
}

fn parse_ip_source(data: &[u8]) -> anyhow::Result<Ipv4Addr> {
    Ok(Ipv4Addr::from_octets(
        data.first_chunk()
            .ok_or_else(|| anyhow!("IP header too short {data:?}"))?
            .clone(),
    ))
}

pub struct IcmpHeader {
    typ: MessageType,
    code: MessageCode,
    id: u16,
    seq: u16,
}

impl IcmpHeader {
    fn parse(data: &[u8]) -> anyhow::Result<Self> {
        let typ = MessageType::parse(data[TYPE_OFFSET])?;
        Ok(Self {
            typ,
            code: MessageCode::parse(typ, data[CODE_OFFSET])?,
            id: (&data[ID_OFFSET..]).read_u16::<NetworkEndian>()?,
            seq: (&data[SEQ_OFFSET..]).read_u16::<NetworkEndian>()?,
        })
    }

    fn write(&self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        buffer.clear();
        buffer.resize(HEADER_LENGTH, 0);
        buffer[TYPE_OFFSET] = self.typ as u8;
        (&mut buffer[ID_OFFSET..]).write_u16::<NetworkEndian>(self.id)?;
        (&mut buffer[SEQ_OFFSET..]).write_u16::<NetworkEndian>(self.seq)?;
        Ok(())
    }
}

pub struct Message {
    ip: IpHeader,
    header: IcmpHeader,
    data: Vec<u8>,
}

impl Message {
    pub fn new(typ: MessageType, code: MessageCode, id: u16, seq: u16, data: Vec<u8>) -> Self {
        Self {
            ip: IpHeader::new(),
            header: IcmpHeader { typ, code, id, seq },
            data,
        }
    }

    pub fn parse(data: &[u8]) -> anyhow::Result<Self> {
        let ip = IpHeader::parse(data)?;
        let data = &data[ip.length..];
        let mut message = Self {
            ip,
            header: IcmpHeader::parse(data)?,
            data: data[HEADER_LENGTH..].to_vec(),
        };
        if message.failure() {
            let inner = Self::parse(&message.data)?;
            message.header.id = inner.header.id;
            message.header.seq = inner.header.seq;
        }
        Ok(message)
    }

    pub fn write(&self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        self.header.write(buffer)?;
        buffer.extend_from_slice(&self.data);
        let checksum = checksum(&buffer);
        (&mut buffer[CHECKSUM_OFFSET..]).write_u16::<NetworkEndian>(checksum)?;
        Ok(())
    }

    pub fn ours(&self, id: u16, seq: u16) -> bool {
        self.header.id == id && self.header.seq == seq
    }

    pub fn failure(&self) -> bool {
        self.rejected() || self.header.typ.time_exceeded()
    }

    pub fn rejected(&self) -> bool {
        self.header.typ.unreachable()
    }

    pub fn show_error(&self) -> String {
        match (self.header.typ, self.header.code) {
            (_, MessageCode::DestinationUnreachable(code)) => format!("{code}"),
            (MessageType::TimeExceeded, _) => format!("Time to Live exceeded in Transit."),
            _ => format!("Request timed out."),
        }
    }

    pub fn source(&self) -> Ipv4Addr {
        self.ip.source
    }
}

fn checksum(slice: &[u8]) -> u16 {
    let (chunks, remainder) = slice.as_chunks::<2>();
    let remainder = remainder.iter().map(|&tail| tail as u32).sum::<u32>();
    let mut sum = chunks
        .iter()
        .map(|&chunk| u16::from_be_bytes(chunk) as u32)
        .fold(remainder, |acc, next| acc.overflowing_add(next).0);
    while sum > 0xFFFF {
        sum = (sum >> 16) + (sum & 0xFFFF)
    }
    0xFFFF - sum as u16
}
