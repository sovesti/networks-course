use std::fmt::{self, Display, Formatter};

use anyhow::anyhow;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use strum::FromRepr;

const HEADER_LENGTH: usize = 8;
const TYPE_OFFSET: usize = 0;
const CODE_OFFSET: usize = 1;
const CHECKSUM_OFFSET: usize = 2;
const ID_OFFSET: usize = 4;
const SEQ_OFFSET: usize = 6;

#[repr(u8)]
#[derive(FromRepr, Clone, Copy, PartialEq)]
pub enum MessageType {
    EchoReply = 0,
    DestinationUnreachable = 3,
    Echo = 8,
    TimeExceeded = 11,
}

impl MessageType {
    fn unreachable(&self) -> bool {
        *self == MessageType::DestinationUnreachable
    }

    fn time_exceeded(&self) -> bool {
        *self == MessageType::TimeExceeded
    }
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MessageType::EchoReply => "Echo Reply",
                MessageType::DestinationUnreachable => "Destination Unreachable",
                MessageType::Echo => "Echo",
                MessageType::TimeExceeded => "Time Exceeded",
            }
        )
    }
}

#[derive(Clone, Copy)]
pub enum MessageCode {
    DestinationUnreachable(DstUnreachableCode),
    None,
}

#[repr(u8)]
#[derive(FromRepr, Clone, Copy)]
pub enum DstUnreachableCode {
    NetworkUnreachable = 0,
    HostUnreachable = 1,
    ProtoUnreachable = 2,
    PortUnreachable = 3,
    FragmentationNeeded = 4,
    SourceRouteFailed = 5,
    DestinationNetworkUnknown = 6,
    DestinationHostUnknown = 7,
    SourceHostIsolated = 8,
    NetworkProhibited = 9,
    HostProhibited = 10,
    NetworkUnreachForTypeOfService = 11,
    HostUnreachableForTypeOfService = 12,
    CommunicationProhibited = 13,
    HostPrecedenceViolation = 14,
    PrecedenceCutoffInEffect = 15,
}

impl Display for DstUnreachableCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DstUnreachableCode::NetworkUnreachable => "Network Unreachable",
                DstUnreachableCode::HostUnreachable => "Host Unreachable",
                DstUnreachableCode::ProtoUnreachable => "Protocol Unreachable",
                DstUnreachableCode::PortUnreachable => "Port Unreachable",
                DstUnreachableCode::FragmentationNeeded =>
                    "Fragmentation Needed and Don't Fragment was Set",
                DstUnreachableCode::SourceRouteFailed => "Source Route Failed",
                DstUnreachableCode::DestinationNetworkUnknown => "Destination Network Unknown",
                DstUnreachableCode::DestinationHostUnknown => "Destination Host Unknown",
                DstUnreachableCode::SourceHostIsolated => "Source Host Isolated",
                DstUnreachableCode::NetworkProhibited =>
                    "Communication with Destination Network is Administratively Prohibited",
                DstUnreachableCode::HostProhibited =>
                    "Communication with Destination Host is Administratively Prohibited",
                DstUnreachableCode::NetworkUnreachForTypeOfService =>
                    "Destination Network Unreachable for Type of Service",
                DstUnreachableCode::HostUnreachableForTypeOfService =>
                    "Destination Host Unreachable for Type of Service",
                DstUnreachableCode::CommunicationProhibited =>
                    "Communication Administratively Prohibited",
                DstUnreachableCode::HostPrecedenceViolation => "Host Precedence Violation",
                DstUnreachableCode::PrecedenceCutoffInEffect => "Precedence cutoff in effect",
            }
        )
    }
}

fn unknown_message_type(typ: u8) -> anyhow::Error {
    anyhow!("Unknown message type: {typ}")
}

fn unknown_message_code(typ: MessageType, code: u8) -> anyhow::Error {
    anyhow!("Unknown message code for {typ}: {code}")
}

#[derive(Clone)]
pub struct Message {
    typ: MessageType,
    code: MessageCode,
    id: u16,
    seq: u16,
    data: Vec<u8>,
}

impl Message {
    pub fn new(typ: MessageType, code: MessageCode, id: u16, seq: u16, data: Vec<u8>) -> Self {
        Self {
            typ,
            code,
            id,
            seq,
            data,
        }
    }

    pub fn parse(data: &[u8]) -> anyhow::Result<Self> {
        let data = &data[parse_ip_length(data[0])..];
        let typ = parse_type(data[TYPE_OFFSET])?;
        let mut message = Self {
            typ,
            code: parse_code(typ, data[CODE_OFFSET])?,
            id: (&data[ID_OFFSET..]).read_u16::<NetworkEndian>()?,
            seq: (&data[SEQ_OFFSET..]).read_u16::<NetworkEndian>()?,
            data: data[HEADER_LENGTH..].to_vec(),
        };
        if message.failure() {
            let inner = Self::parse(&message.data)?;
            message.id = inner.id;
            message.seq = inner.seq;
        }
        Ok(message)
    }

    pub fn write(&self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        buffer.clear();
        buffer.resize(HEADER_LENGTH, 0);
        buffer[TYPE_OFFSET] = self.typ as u8;
        (&mut buffer[ID_OFFSET..]).write_u16::<NetworkEndian>(self.id)?;
        (&mut buffer[SEQ_OFFSET..]).write_u16::<NetworkEndian>(self.seq)?;
        buffer.extend_from_slice(&self.data);
        let checksum = checksum(&buffer);
        (&mut buffer[CHECKSUM_OFFSET..]).write_u16::<NetworkEndian>(checksum)?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn ours(&self, id: u16, seq: u16) -> bool {
        self.id == id && self.seq == seq
    }

    pub fn failure(&self) -> bool {
        self.typ.unreachable() || self.typ.time_exceeded()
    }

    pub fn show_error(&self) -> String {
        match (self.typ, self.code) {
            (_, MessageCode::DestinationUnreachable(code)) => format!("{code}"),
            (MessageType::TimeExceeded, _) => format!("Time to Live exceeded in Transit."),
            _ => format!("Request timed out."),
        }
    }
}

fn parse_ip_length(byte: u8) -> usize {
    (byte & 0xf) as usize * 4
}

fn parse_type(typ: u8) -> anyhow::Result<MessageType> {
    Ok(MessageType::from_repr(typ).ok_or_else(|| unknown_message_type(typ))?)
}

fn parse_code(typ: MessageType, code: u8) -> anyhow::Result<MessageCode> {
    Ok(match typ {
        MessageType::DestinationUnreachable => MessageCode::DestinationUnreachable(
            DstUnreachableCode::from_repr(code).ok_or_else(|| unknown_message_code(typ, code))?,
        ),
        _ => MessageCode::None,
    })
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
