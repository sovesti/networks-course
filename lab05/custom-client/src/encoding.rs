use std::fs;
use std::io::{BufWriter, ErrorKind, Read, Write};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;

const LINE_LEN: usize = 76;
const BUF_LEN: usize = 64 * 1024;
const CRLF: &'static str = "\r\n";

pub struct EncodedFile<'a, W: Write> {
    to: BufWriter<&'a mut W>,
    buf: Vec<u8>,
    encoded: Vec<u8>,
    file: fs::File,
}

impl<'a, W: Write> EncodedFile<'a, W> {
    pub fn new(to: &'a mut W, file: &str) -> anyhow::Result<Self> {
        Ok(Self {
            to: BufWriter::new(to),
            buf: vec![0; BUF_LEN],
            encoded: vec![0; BUF_LEN / 3 * 4 + 4],
            file: fs::File::open(file)?,
        })
    }

    pub fn read_from_file(&mut self, remainder: usize) -> Option<usize> {
        match self.file.read(&mut self.buf[remainder..]) {
            Ok(0) => None,
            Ok(len) => Some(remainder + len),
            Err(e) if e.kind() == ErrorKind::Interrupted => Some(remainder),
            Err(_) => None,
        }
    }

    pub fn chunked_encode(&mut self, len: usize) -> anyhow::Result<usize> {
        let remainder = self.encode_full_chunks(len)?;
        let remainder: Vec<u8> = self.buf[(len - remainder)..len].iter().cloned().collect();
        self.buf[..remainder.len()].clone_from_slice(&remainder);
        Ok(remainder.len())
    }

    fn encode_full_chunks(&mut self, len: usize) -> anyhow::Result<usize> {
        let mut chunks = self.buf[..len].chunks_exact(LINE_LEN / 4 * 3);
        while let Some(chunk) = chunks.next() {
            write!(&mut self.to, "{CRLF}")?;
            let len = BASE64_STANDARD.encode_slice(&chunk, &mut self.encoded)?;
            self.to.write_all(&self.encoded[..len])?;
        }
        Ok(chunks.remainder().len())
    }

    pub fn encode_remainder(&mut self, len: usize) -> anyhow::Result<()> {
        write!(&mut self.to, "{CRLF}")?;
        let len = BASE64_STANDARD.encode_slice(&self.buf[..len], &mut self.encoded)?;
        self.to.write_all(&self.encoded[..len])?;
        write!(&mut self.to, "{CRLF}")?;
        Ok(())
    }
}
