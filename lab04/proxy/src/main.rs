mod cache;

use std::env::args;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, combinators::BoxBody};
use http_body_util::{Empty, StreamBody};
use hyper::body::Incoming;
use hyper::header::{ETAG, HOST, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use hyper::service::service_fn;
use hyper::{HeaderMap, Method, Request, Response, StatusCode, server};
use hyper_util::rt::TokioIo;
use reqwest::redirect::Policy;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::spawn;

use crate::cache::Cache;

type BoxedBody = BoxBody<Bytes, anyhow::Error>;
type Headers = HeaderMap<HeaderValue>;
type BoxedResponse = anyhow::Result<Response<BoxedBody>>;

#[derive(Deserialize, Debug)]
struct Config {
    blacklist: Vec<String>,
}

impl Config {
    fn blocked(&self, request: &hyper::Uri) -> bool {
        request
            .host()
            .is_some_and(|host| self.host_blocked(request, host.to_owned()))
    }

    fn host_blocked(&self, request: &hyper::Uri, host: String) -> bool {
        self.blacklist_contains(&host)
            || request
                .path_and_query()
                .is_some_and(|path| self.blacklist_contains(&(host + path.as_str())))
    }

    fn blacklist_contains(&self, entry: &str) -> bool {
        self.blacklist.iter().any(|blocked| blocked == entry)
    }
}

#[derive(Clone, Debug)]
struct Context {
    cache: Arc<Cache>,
    client: Arc<reqwest::Client>,
    config: Arc<Config>,
}

#[derive(Debug)]
struct IncomingRequest {
    context: Context,
    request: Request<Incoming>,
}

impl IncomingRequest {
    fn new(context: Context, request: Request<Incoming>) -> Self {
        Self { context, request }
    }

    fn uri(&self) -> anyhow::Result<hyper::Uri> {
        Ok(hyper::Uri::from_str(&format!(
            "https:/{}",
            self.request.uri()
        ))?)
    }

    fn method(&self) -> Method {
        self.request.method().clone()
    }
}

fn response(code: StatusCode, body: BoxedBody, headers: &Headers) -> BoxedResponse {
    let mut builder = Response::builder();
    headers.iter().for_each(|(k, v)| {
        builder.headers_mut().unwrap().append(k, v.clone());
    });
    Ok(builder.status(code).body(body)?)
}

fn response_ok(body: BoxedBody, headers: &Headers) -> BoxedResponse {
    response(StatusCode::OK, body, headers)
}

async fn if_modified_status(
    uri: &hyper::Uri,
    client: Arc<reqwest::Client>,
    cached: &cache::CachedResource,
) -> anyhow::Result<StatusCode> {
    Ok(client
        .get(uri.to_string())
        .header(IF_MODIFIED_SINCE, &cached.time)
        .header(IF_NONE_MATCH, &cached.etag)
        .send()
        .await?
        .status())
}

async fn create_cache_file(
    uri: &hyper::Uri,
    received: &reqwest::Response,
    context: Context,
) -> anyhow::Result<Option<File>> {
    let etag = received.headers().get(&ETAG);
    let time = received.headers().get(&LAST_MODIFIED);
    if etag.is_none() || time.is_none() {
        return Ok(None);
    }
    let etag = etag.unwrap().to_str()?;
    let time = time.unwrap().to_str()?;
    Ok(Some(
        context
            .cache
            .create_entry(uri, etag, time, received.headers())
            .await?,
    ))
}

async fn write_to_cache(received: &mut reqwest::Response, mut file: File) -> anyhow::Result<()> {
    while let Some(next) = received.chunk().await? {
        file.write_all(&next).await?;
    }
    Ok(())
}

fn boxed_response(headers: Headers, received: reqwest::Response) -> BoxedResponse {
    let code = received.status();
    let body = received
        .bytes_stream()
        .map_ok(hyper::body::Frame::data)
        .map_err(anyhow::Error::from);
    response(code, StreamBody::new(body).boxed(), &headers)
}

async fn maybe_through_cache(
    uri: &hyper::Uri,
    context: Context,
    mut received: reqwest::Response,
    file: Option<File>,
) -> BoxedResponse {
    match file {
        Some(file) => {
            write_to_cache(&mut received, file).await?;
            response_ok(context.cache.stream_file(uri).await?, received.headers())
        }
        None => boxed_response(received.headers().clone(), received),
    }
}

fn fix_uri(incoming: &mut IncomingRequest, uri: &hyper::Uri) -> anyhow::Result<()> {
    *incoming.request.uri_mut() = uri
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/")
        .parse()?;
    Ok(())
}

fn fix_host(incoming: &mut IncomingRequest, uri: &hyper::Uri) -> anyhow::Result<()> {
    incoming.request.headers_mut().insert(
        HOST,
        HeaderValue::from_str(uri.host().ok_or_else(|| anyhow!("Incorrect URL: {uri}"))?)?,
    );
    Ok(())
}

async fn cache_miss(method: Method, mut incoming: IncomingRequest) -> BoxedResponse {
    let uri = incoming.uri()?;
    fix_uri(&mut incoming, &uri)?;
    fix_host(&mut incoming, &uri)?;
    let response = incoming
        .context
        .client
        .request(incoming.request.method().clone(), uri.to_string())
        .headers(incoming.request.headers().clone())
        .body(reqwest::Body::wrap(incoming.request.into_body()))
        .send()
        .await?;
    let file = match method {
        Method::GET => create_cache_file(&uri, &response, incoming.context.clone()).await?,
        _ => None,
    };
    maybe_through_cache(&uri, incoming.context, response, file).await
}

async fn send_to(incoming: IncomingRequest, uri: hyper::Uri) -> BoxedResponse {
    if let Ok(cached) = incoming.context.cache.tag(&uri).await {
        match if_modified_status(&uri, incoming.context.client.clone(), &cached).await? {
            StatusCode::NOT_MODIFIED => {
                log::info!("Cache hit: {uri}");
                return response_ok(
                    incoming.context.cache.stream_file(&uri).await?,
                    &cached.headers,
                );
            }
            _ => incoming.context.cache.remove_entry(&uri).await,
        }
    }
    log::info!("Cache miss: {uri}");
    cache_miss(incoming.method(), incoming).await
}

fn resource_blocked() -> BoxedResponse {
    log::info!("Tried to access blocked resource");
    let body = "Resource is blocked"
        .to_owned()
        .map_err(anyhow::Error::from)
        .boxed();
    response(StatusCode::EXPECTATION_FAILED, body, &HeaderMap::new())
}

async fn maybe_redirect(request: IncomingRequest) -> BoxedResponse {
    let uri = request.uri()?;
    log::info!("Requested resource: {uri}");
    if request.context.config.blocked(&uri) {
        return resource_blocked();
    }
    let res = send_to(request, uri).await?;
    log::info!("Response: {}", res.status());
    Ok(res)
}

fn bad_request(err: anyhow::Error) -> BoxedResponse {
    log::error!("{err:?}");
    response(
        StatusCode::BAD_REQUEST,
        err.to_string().map_err(anyhow::Error::from).boxed(),
        &HeaderMap::new(),
    )
}

fn ignore(request: &IncomingRequest) -> bool {
    request
        .uri()
        .unwrap()
        .host()
        .is_some_and(|host| host == "favicon.ico" || host == "static")
}

async fn redirect(request: IncomingRequest) -> BoxedResponse {
    log::info!("{}", request.uri()?);
    if ignore(&request) {
        return response_ok(
            Empty::new().map_err(anyhow::Error::from).boxed(),
            &HeaderMap::new(),
        );
    }
    match maybe_redirect(request).await {
        Ok(resp) => Ok(resp),
        Err(err) => bad_request(err),
    }
}

async fn serve(context: Context, stream: TcpStream) {
    spawn(async move {
        let proxy = async |req| redirect(IncomingRequest::new(context.clone(), req)).await;
        if let Err(err) = server::conn::http1::Builder::new()
            .keep_alive(false)
            .serve_connection(TokioIo::new(stream), service_fn(proxy))
            .await
        {
            log::error!("Internal error: {err:?}");
        }
    });
}

fn port() -> u16 {
    args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(3000)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let addr = SocketAddr::from(([0, 0, 0, 0], port()));
    let listener = TcpListener::bind(addr).await?;
    let context = Context {
        cache: Arc::new(Cache::new(PathBuf::from("cache"))),
        client: Arc::new(
            reqwest::ClientBuilder::new()
                .http1_only()
                .redirect(Policy::none())
                .build()
                .unwrap(),
        ),
        config: Arc::new(toml::from_str(include_str!("../config.toml")).unwrap()),
    };
    log::info!("Listening on http://{addr}");
    loop {
        let (stream, _) = listener.accept().await?;
        serve(context.clone(), stream).await;
    }
}
