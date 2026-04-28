use std::io::{BufReader, Cursor, Seek, SeekFrom, Write};

use anyhow::{Context, bail};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

const CHECKSUM_LENGTH: usize = size_of::<u16>();
pub const META_LENGTH: usize = CHECKSUM_LENGTH + size_of::<RdtHeader>();

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum SegmentType {
    Ack = 0,
    Pkt = 1,
}

impl TryFrom<u8> for SegmentType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self> {
        match value {
            0 => Ok(Self::Ack),
            1 => Ok(Self::Pkt),
            _ => bail!("Unknown segment type {value}"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RdtHeader {
    typ: SegmentType,
    seq: u8,
}

impl RdtHeader {
    pub fn new(typ: SegmentType, seq: u8) -> Self {
        Self { typ, seq }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParsedRdtSegment {
    header: RdtHeader,
    offset: usize,
    length: usize,
}

impl TryFrom<&[u8]> for ParsedRdtSegment {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        Ok(ParsedRdtSegment {
            header: read_meta(value, BufReader::new(value))?,
            offset: META_LENGTH,
            length: value.len(),
        })
    }
}

fn read_meta(value: &[u8], mut reader: BufReader<&[u8]>) -> anyhow::Result<RdtHeader> {
    let _checksum = reader
        .read_u16::<BigEndian>()
        .with_context(|| incorrect_header(value))?;
    let typ = reader
        .read_u8()
        .with_context(|| incorrect_header(value))?
        .try_into()?;
    let seq = reader.read_u8().with_context(|| incorrect_header(value))?;
    Ok(RdtHeader { typ, seq })
}

impl ParsedRdtSegment {
    pub fn is_ack(&self, seq: u8) -> bool {
        self.header.typ == SegmentType::Ack && self.header.seq == seq
    }

    pub fn is_pkt(&self, seq: u8) -> bool {
        self.header.typ == SegmentType::Pkt && self.header.seq == seq
    }

    pub fn data<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        &buffer[self.offset..self.length]
    }
}

#[derive(Clone)]
pub struct PreparedRdtSegment {
    header: RdtHeader,
    data: Vec<u8>,
}

impl PreparedRdtSegment {
    pub fn new(header: RdtHeader, data: Vec<u8>) -> Self {
        Self { header, data }
    }

    pub fn write(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        let length = META_LENGTH + self.data.len();
        let mut cursor = Cursor::new(buffer);
        cursor.write(&[0; CHECKSUM_LENGTH])?;
        cursor.write_u8(self.header.typ as u8)?;
        cursor.write_u8(self.header.seq)?;
        cursor.write(&self.data)?;
        cursor.seek(SeekFrom::Start(0))?;
        cursor.write_u16::<BigEndian>(checksum(&cursor.get_ref()[..length]))?;
        Ok(length)
    }
}

fn incorrect_header(header: &[u8]) -> String {
    format!("Incorrect segment header:\n{header:?}")
}

fn checksum(slice: &[u8]) -> u16 {
    let (chunks, remainder) = slice.as_chunks::<CHECKSUM_LENGTH>();
    let remainder = remainder.iter().map(|&tail| tail as u16).sum::<u16>();
    0xFFFFu16
        - chunks
            .iter()
            .map(|&chunk| u16::from_be_bytes(chunk))
            .fold(remainder, |acc, next| acc.overflowing_add(next).0)
}

pub fn checksum_correct(buffer: &[u8]) -> bool {
    checksum_matches(u16::from_be_bytes([buffer[0], buffer[1]]), &buffer[2..])
}

fn checksum_matches(stored: u16, buffer: &[u8]) -> bool {
    stored == checksum(buffer)
}

#[cfg(test)]
mod test {
    use crate::segment::{checksum, checksum_matches};

    const EVEN: [u8; 4] = [10, 20, 30, 40];
    const EVEN_DAMAGED: [u8; 4] = [10, 21, 30, 40];
    const ODD: [u8; 3] = [10, 20, 30];
    const ODD_DAMAGED: [u8; 3] = [10, 20, 31];

    #[test]
    fn untouched_checksum_correct_even() {
        assert!(checksum_matches(checksum(&EVEN), &EVEN));
    }

    #[test]
    fn touched_checksum_incorrect_even() {
        assert!(!checksum_matches(checksum(&EVEN), &EVEN_DAMAGED));
    }

    #[test]
    fn untouched_checksum_correct_odd() {
        assert!(checksum_matches(checksum(&ODD), &ODD));
    }

    #[test]
    fn touched_checksum_incorrect_odd() {
        assert!(!checksum_matches(checksum(&ODD), &ODD_DAMAGED));
    }
}
