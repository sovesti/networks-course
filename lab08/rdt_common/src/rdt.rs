use std::pin::pin;
use std::{cell::RefCell, io::Write, rc::Rc, time::Duration};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Instant, sleep_until};

use crate::{
    segment::{
        META_LENGTH, ParsedRdtSegment, PreparedRdtSegment, RdtHeader, SegmentType, checksum_correct,
    },
    udt::UdtConnection,
};

pub struct RdtConfig {
    timeout: Duration,
}

impl RdtConfig {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

const BUFFER_SIZE: usize = 32 * 1024;

pub struct RdtConnection {
    config: RdtConfig,
    seq: u8,
    prev: Vec<u8>,
    transport: Rc<RefCell<UdtConnection>>,
}

impl RdtConnection {
    pub fn new(transport: UdtConnection, config: RdtConfig) -> RdtConnection {
        Self {
            config,
            seq: 0,
            prev: vec![],
            transport: Rc::new(RefCell::new(transport)),
        }
    }

    pub async fn send<R: AsyncRead + Unpin>(
        &mut self,
        mut data: R,
        buffer: &mut [u8],
    ) -> anyhow::Result<()> {
        let mut head = vec![0; BUFFER_SIZE - META_LENGTH];
        let mut out = vec![0; BUFFER_SIZE];
        while let Ok(bytes) = pin!(data.read(&mut head)).await
            && bytes > 0
        {
            self.send_segment(head[..bytes].to_vec(), buffer, &mut out)
                .await?;
        }
        self.send_segment(vec![], buffer, &mut out).await?;
        Ok(())
    }

    async fn send_segment(
        &mut self,
        data: Vec<u8>,
        buffer: &mut [u8],
        out: &mut [u8],
    ) -> anyhow::Result<()> {
        let header = RdtHeader::new(SegmentType::Pkt, self.seq);
        log::debug!("Sending {header:?}");
        let bytes = PreparedRdtSegment::new(header, data).write(out)?;
        let mut started = Instant::now();
        while self
            .try_sending(buffer, &out[..bytes], &mut started)
            .await?
            .is_none()
        {}
        self.prev.resize(bytes, 0);
        self.prev.copy_from_slice(&out[..bytes]);
        self.seq = 1 - self.seq;
        log::debug!("Sent {bytes} bytes");
        return Ok(());
    }

    async fn try_sending(
        &mut self,
        buffer: &mut [u8],
        out: &[u8],
        started: &mut Instant,
    ) -> anyhow::Result<Option<()>> {
        let mut transport = self.transport.borrow_mut();
        transport.send(out).await?;
        tokio::select! {
            _ = sleep_until(*started + self.config.timeout) => {
                log::debug!("ACK timed out");
                *started = Instant::now();
                return Ok(None);
            },
            Ok(bytes) = transport.recv(buffer) => {
                match ParsedRdtSegment::try_from(&buffer[..bytes]) {
                    Ok(segment) if checksum_correct(&buffer[..bytes]) && segment.is_ack(self.seq) => return Ok(Some(())),
                    other => self.report(other, RdtHeader::new(SegmentType::Ack, self.seq)),
                }
            }
        }
        drop(transport);
        self.retry().await?;
        Ok(None)
    }

    pub async fn recv<W: AsyncWrite + Unpin>(
        &mut self,
        file: &mut W,
        mut buffer: &mut [u8],
    ) -> anyhow::Result<usize> {
        let mut out = vec![0; BUFFER_SIZE];
        let mut total = 0;
        while let bytes = self.recv_segment(&mut buffer, &mut out).await?
            && (bytes > 0 || total == 0)
        {
            file.write_all(&out[..bytes]).await?;
            total += bytes;
        }
        Ok(total)
    }

    async fn recv_segment(&mut self, buffer: &mut [u8], out: &mut [u8]) -> anyhow::Result<usize> {
        let written = loop {
            if let Some(written) = self.try_receiving(buffer, out).await? {
                break written;
            }
        };
        self.prev.resize(META_LENGTH, 0);
        PreparedRdtSegment::new(RdtHeader::new(SegmentType::Ack, self.seq), vec![])
            .write(&mut self.prev)?;
        self.seq = 1 - self.seq;
        log::debug!("Received {written} bytes");
        return Ok(written);
    }

    async fn try_receiving(
        &mut self,
        buffer: &mut [u8],
        mut out: &mut [u8],
    ) -> anyhow::Result<Option<usize>> {
        let bytes = self.transport.borrow_mut().recv(buffer).await?;
        match ParsedRdtSegment::try_from(&buffer[..bytes]) {
            Ok(segment) if checksum_correct(&buffer[..bytes]) && segment.is_pkt(self.seq) => {
                let mut ack = vec![0; META_LENGTH];
                let header = RdtHeader::new(SegmentType::Ack, self.seq);
                log::debug!("Sending {header:?}");
                PreparedRdtSegment::new(header, vec![]).write(&mut ack)?;
                self.transport.borrow_mut().send(&ack).await?;
                return Ok(Some(out.write(segment.data(buffer))?));
            }
            other => self.report(other, RdtHeader::new(SegmentType::Pkt, self.seq)),
        }
        self.retry().await?;
        Ok(None)
    }

    async fn retry(&mut self) -> anyhow::Result<()> {
        log::debug!("Retrying last segment");
        self.transport
            .borrow_mut()
            .send(&self.prev)
            .await
            .map(|_| ())
    }

    fn report(&self, result: anyhow::Result<ParsedRdtSegment>, expected: RdtHeader) {
        match result {
            Ok(segment) => log::debug!("Unexpected segment: {segment:?}, expected: {expected:?}"),
            Err(err) => log::error!("{err:?}"),
        }
    }
}
