use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
};

use crate::{http::talk_to, pool::Pool};

pub struct Server {
    listener: TcpListener,
    threads: Arc<Pool>,
}

impl Server {
    pub fn new(port: u16, concurrency: u16) -> Self {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).unwrap();
        log::info!("Listening on http://{}", addr);
        Self {
            listener,
            threads: Arc::new(Pool::new(concurrency)),
        }
    }

    fn _single_threaded(&self) -> anyhow::Result<()> {
        let (connection, _) = self.listener.accept()?;
        talk_to(connection)
    }

    fn multi_threaded(&self) -> anyhow::Result<()> {
        log::info!("waiting...");
        let (connection, addr) = self.listener.accept()?;
        log::info!("connected to {}", addr);
        let pool = self.threads.clone();
        pool.take();
        std::thread::spawn(move || {
            log(talk_to(connection));
            log::info!("closed connection to {}", addr);
            pool.release();
        });
        Ok(())
    }

    pub fn accept(&self) {
        // log(self._single_threaded());
        log(self.multi_threaded());
    }
}

fn log(result: anyhow::Result<()>) {
    if let Err(err) = result {
        log::error!("{}", err);
    }
}
