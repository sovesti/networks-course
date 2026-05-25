use std::{collections::HashMap, path::PathBuf};

use bytes::{Bytes, BytesMut};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody, combinators::BoxBody};
use hyper::{HeaderMap, body::Frame, header::HeaderValue};
use tokio::{fs::File, sync::Mutex};
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Clone, Debug)]
pub struct CachedResource {
    pub etag: String,
    pub time: String,
    pub headers: HeaderMap<HeaderValue>,
}

impl CachedResource {
    fn new(tag: &str, time: &str, headers: &HeaderMap<HeaderValue>) -> Self {
        Self {
            etag: tag.to_owned(),
            time: time.to_owned(),
            headers: headers.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Cache {
    folder: PathBuf,
    tags: Mutex<HashMap<String, CachedResource>>,
}

impl Cache {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder,
            tags: Mutex::new(HashMap::new()),
        }
    }

    pub async fn stream_file(
        &self,
        uri: &hyper::Uri,
    ) -> anyhow::Result<BoxBody<Bytes, anyhow::Error>> {
        let file = self.open(uri).await?;
        Ok(StreamBody::new(
            FramedRead::new(file, BytesCodec::new())
                .map_ok(BytesMut::freeze)
                .map_ok(Frame::data)
                .map_err(anyhow::Error::from),
        )
        .boxed())
    }

    pub async fn open(&self, uri: &hyper::Uri) -> anyhow::Result<File> {
        Ok(File::open(self.path(&self.tag(uri).await?.etag)).await?)
    }

    pub async fn tag(&self, uri: &hyper::Uri) -> anyhow::Result<CachedResource> {
        Ok(self
            .tags
            .lock()
            .await
            .get(&uri.to_string())
            .ok_or_else(|| anyhow::anyhow!("Cache miss"))?
            .clone())
    }

    pub async fn create_entry(
        &self,
        uri: &hyper::Uri,
        tag: &str,
        time: &str,
        headers: &HeaderMap<HeaderValue>,
    ) -> anyhow::Result<File> {
        self.tags
            .lock()
            .await
            .insert(uri.to_string(), CachedResource::new(tag, time, headers));
        Ok(File::create(self.path(tag)).await?)
    }

    fn path(&self, tag: &str) -> PathBuf {
        let mut path = self.folder.clone();
        path.push(&tag[1..(tag.len() - 1)]);
        path
    }

    pub async fn remove_entry(&self, uri: &hyper::Uri) {
        self.tags.lock().await.remove(&uri.to_string());
    }
}
