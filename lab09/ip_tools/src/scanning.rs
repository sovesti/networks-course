use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use tokio::{
    io,
    net::TcpStream,
    task::JoinSet,
    time::{error, timeout},
};

pub async fn show_open_ports(host: IpAddr, from: u16, to: u16) {
    println!("Scanning {host}, ports {from}-{to}:");
    let mut open = open_ports(host, from, to).await;
    open.sort();
    open.into_iter().for_each(|port| println!("  {port}: open"));
}

async fn open_ports(host: IpAddr, from: u16, to: u16) -> Vec<u16> {
    (from..=to)
        .map(|port| scan_port(host, port))
        .collect::<JoinSet<_>>()
        .join_all()
        .await
        .iter()
        .filter(|&(_, open)| *open)
        .map(|&(port, _)| port)
        .collect()
}

async fn scan_port(host: IpAddr, port: u16) -> (u16, bool) {
    let stream = try_connect(host, port).await;
    log::debug!("{port}: {stream:?}");
    (port, stream.is_ok_and(|connection| connection.is_ok()))
}

async fn try_connect(host: IpAddr, port: u16) -> Result<io::Result<TcpStream>, error::Elapsed> {
    timeout(
        Duration::from_secs(10),
        TcpStream::connect(SocketAddr::new(host, port)),
    )
    .await
}
