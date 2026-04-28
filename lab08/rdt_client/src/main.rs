use std::{net::ToSocketAddrs, time::Duration};

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
    server: u16,
    receive: String,
    send: String,
}

fn config() -> anyhow::Result<Config> {
    Ok(toml::from_str(&std::fs::read_to_string("config.toml")?)?)
}

async fn request(
    rdt: &mut RdtConnection,
    buffer: &mut Vec<u8>,
    query: &str,
    param: &str,
) -> anyhow::Result<()> {
    rdt.send(format!("{query}:{param}").as_bytes(), buffer)
        .await
}

async fn requests(
    mut rdt: RdtConnection,
    mut buffer: Vec<u8>,
    config: &Config,
) -> anyhow::Result<()> {
    download(&mut rdt, &mut buffer, config).await?;
    upload(&mut rdt, &mut buffer, config).await?;
    Ok(())
}

async fn download(
    rdt: &mut RdtConnection,
    buffer: &mut Vec<u8>,
    config: &Config,
) -> Result<(), anyhow::Error> {
    request(rdt, buffer, "download", &config.receive).await?;
    rdt.recv(&mut File::create(&config.receive).await?, buffer)
        .await?;
    println!("File received successfully");
    Ok(())
}

async fn upload(
    rdt: &mut RdtConnection,
    buffer: &mut Vec<u8>,
    config: &Config,
) -> Result<(), anyhow::Error> {
    request(rdt, buffer, "upload", &config.send).await?;
    rdt.send(File::open(&config.send).await?, buffer).await?;
    println!("File sent successfully");
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config = config()?;
    let udt = udt::connect(
        UdtConfig::new(config.port, config.loss),
        format!("127.0.0.1:{}", config.server)
            .to_socket_addrs()?
            .next()
            .unwrap(),
    )
    .await?;
    let rdt = RdtConnection::new(udt, RdtConfig::new(Duration::from_millis(config.timeout)));
    requests(rdt, vec![0; 32 * 1024], &config).await
}
