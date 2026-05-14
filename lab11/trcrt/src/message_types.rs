use std::fmt::{self, Display, Formatter};

use anyhow::anyhow;
use strum::FromRepr;

#[repr(u8)]
#[derive(FromRepr, Clone, Copy, PartialEq)]
pub enum MessageType {
    EchoReply = 0,
    DestinationUnreachable = 3,
    Echo = 8,
    TimeExceeded = 11,
}

impl MessageType {
    pub fn parse(typ: u8) -> anyhow::Result<MessageType> {
        Ok(MessageType::from_repr(typ).ok_or_else(|| unknown_message_type(typ))?)
    }

    pub fn unreachable(&self) -> bool {
        *self == MessageType::DestinationUnreachable
    }

    pub fn time_exceeded(&self) -> bool {
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

impl MessageCode {
    pub fn parse(typ: MessageType, code: u8) -> anyhow::Result<MessageCode> {
        Ok(match typ {
            MessageType::DestinationUnreachable => MessageCode::DestinationUnreachable(
                DstUnreachableCode::from_repr(code)
                    .ok_or_else(|| unknown_message_code(typ, code))?,
            ),
            _ => MessageCode::None,
        })
    }
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
