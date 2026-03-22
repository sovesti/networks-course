use std::{env::args, u16};

use crate::server::Server;

mod http;
mod pool;
mod server;

fn port() -> u16 {
    args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(3000)
}

fn concurrency() -> u16 {
    args()
        .nth(2)
        .and_then(|c| c.parse().ok())
        .unwrap_or(u16::MAX)
}

fn main() {
    env_logger::init();
    let server = Server::new(port(), concurrency());
    loop {
        server.accept();
    }
}
