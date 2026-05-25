use std::{io, net::IpAddr};

use clap::Parser;

use crate::{interfaces::show_wireless_interfaces, scanning::show_open_ports};

mod interfaces;
mod scanning;

#[derive(Parser)]
enum Command {
    Ip,
    Scan { host: IpAddr, from: u16, to: u16 },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> io::Result<()> {
    env_logger::init();
    match Command::parse() {
        Command::Ip => show_wireless_interfaces(),
        Command::Scan { host, from, to } => Ok(show_open_ports(host, from, to).await),
    }
}
