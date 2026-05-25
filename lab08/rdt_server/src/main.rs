use std::time::Duration;

use anyhow::Context;
use rdt_common::{
    rdt::{RdtConfig, RdtConnection},
    udt::{self, UdtConfig},
};
use serde::Deserialize;
use tokio::fs::File;

#[derive(Deserialize)]
struct Config {
    port: u16,
    loss: f64,
    timeout: u64,
}

fn config() -> anyhow::Result<Config> {
    Ok(toml::from_str(&std::fs::read_to_string("config.toml")?)?)
}

async fn serve(mut rdt: RdtConnection, mut buffer: Vec<u8>) -> anyhow::Result<()> {
    loop {
        match read_query(&mut rdt, &mut buffer).await?.split_once(":") {
            Some(("download", path)) => rdt.send(File::open(path).await?, &mut buffer).await?,
            Some(("upload", path)) => {
                rdt.recv(&mut File::create(path).await?, &mut buffer)
                    .await?;
            }
            other => println!("bad query {other:?}"),
        }
    }
}

async fn read_query(rdt: &mut RdtConnection, buffer: &mut Vec<u8>) -> anyhow::Result<String> {
    let mut request = vec![];
    let bytes = rdt.recv(&mut request, buffer).await?;
    let request = str::from_utf8(&request[..bytes])
        .with_context(|| format!("query length: {bytes}"))?
        .to_owned();
    println!("query: {request}");
    Ok(request)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config = config()?;
    let udt = udt::listen(UdtConfig::new(config.port, config.loss)).await?;
    let rdt = RdtConnection::new(udt, RdtConfig::new(Duration::from_millis(config.timeout)));
    serve(rdt, vec![0; 32 * 1024]).await
}
