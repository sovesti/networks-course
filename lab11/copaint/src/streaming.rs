use std::io;
use std::pin::Pin;
use std::time::Duration;

use bytes::Bytes;
use dioxus::prelude::*;
use futures::{SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use rkyv::{from_bytes, rancor, to_bytes};
use tokio::{
    io::{BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::broadcast::{Receiver, Sender},
    time::{Instant, sleep_until},
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::codec::{Encoder, FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::canvas::Line;

pub async fn read_from_socket(
    mut stream: Signal<Option<OwnedReadHalf>>,
    mut error: Signal<Option<anyhow::Error>>,
    tx: Sender<Line>,
) {
    let mut tcp = stream.write();
    if let Err(err) = FramedRead::new(
        BufReader::new(tcp.as_mut().unwrap()),
        LengthDelimitedCodec::new(),
    )
    .map_err(anyhow::Error::from)
    .and_then(async |raw| from_bytes::<_, rancor::Error>(&raw).map_err(anyhow::Error::from))
    .try_for_each(async |msg| tx.send(msg).map(|_| ()).map_err(anyhow::Error::from))
    .await
    {
        error.set(Some(err.into()));
    }
}

pub async fn write_to_socket(
    mut stream: Signal<Option<OwnedWriteHalf>>,
    mut error: Signal<Option<anyhow::Error>>,
    rx: Receiver<Line>,
) {
    let mut tcp = stream.write();
    let mut out = FramedWrite::new(
        BufWriter::new(tcp.as_mut().unwrap()),
        LengthDelimitedCodec::new(),
    );
    let stream = stream_messages(rx);
    tokio::pin!(stream);
    let mut frame = Instant::now();
    loop {
        match debounced(&mut frame, &mut stream, &mut out).await {
            Ok(None) => return,
            Err(err) => error.set(Some(err)),
            _ => (),
        }
    }
}

async fn debounced<E>(
    frame: &mut Instant,
    stream: &mut Pin<&mut impl Stream<Item = anyhow::Result<Bytes>>>,
    out: &mut FramedWrite<BufWriter<&mut OwnedWriteHalf>, E>,
) -> anyhow::Result<Option<()>>
where
    E: Encoder<Bytes, Error = io::Error> + Send + Sync,
{
    tokio::select! {
        next = stream.next() => feed(out, next).await,
        _ = sleep_until(next_frame(frame)) => {
            *frame = Instant::now();
            out.flush().await.map_err(anyhow::Error::from).map(Option::Some)
        },
    }
}

fn next_frame(current: &Instant) -> Instant {
    *current + Duration::from_secs(1) / 30
}

async fn feed<E>(
    out: &mut FramedWrite<BufWriter<&mut OwnedWriteHalf>, E>,
    next: Option<anyhow::Result<Bytes>>,
) -> anyhow::Result<Option<()>>
where
    E: Encoder<Bytes, Error = io::Error> + Send + Sync,
{
    match next {
        Some(next) => async { next }
            .and_then(|msg| out.feed(msg).map_err(anyhow::Error::from))
            .await
            .map(Option::Some),
        None => Ok(None),
    }
}

fn stream_messages(rx: Receiver<Line>) -> impl Stream<Item = anyhow::Result<Bytes>> {
    BroadcastStream::new(rx)
        .map_err(anyhow::Error::from)
        .and_then(async |msg| to_bytes::<rancor::Error>(&msg).map_err(anyhow::Error::from))
        .map_ok(|raw| Bytes::from(raw.into_vec()))
        .map_err(anyhow::Error::from)
}
