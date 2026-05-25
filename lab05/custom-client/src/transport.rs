use std::{
    io::{BufRead, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::Arc,
};

use anyhow::anyhow;
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, Stream, crypto::ring, pki_types::ServerName,
};

use crate::{address::Credentials, message::Message};

type TlsStream<'a> = Stream<'a, ClientConnection, TcpStream>;

enum Command {
    Ehlo,
    Auth,
    MailFrom,
    RcptTo,
    Data,
    Quit,
}

impl Command {
    fn send_with(&self, to: &mut TlsStream, contents: &str) -> anyhow::Result<()> {
        send(to, format!("{} {contents}", self.to_string()))
    }

    fn send_with_path(&self, to: &mut TlsStream, contents: &str) -> anyhow::Result<()> {
        send(to, format!("{}:<{contents}>", self.to_string()))
    }

    fn send(&self, to: &mut TlsStream) -> anyhow::Result<()> {
        send(to, self.to_string())
    }
}

fn send(to: &mut TlsStream, command: String) -> anyhow::Result<()> {
    println!("-> {command}");
    write!(to, "{command}\r\n")?;
    to.flush()?;
    read_response(to)?;
    Ok(())
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Command::Ehlo => "EHLO",
            Command::Auth => "AUTH PLAIN",
            Command::MailFrom => "MAIL FROM",
            Command::RcptTo => "RCPT TO",
            Command::Data => "DATA",
            Command::Quit => "QUIT",
        }
        .to_owned()
    }
}

pub struct Smtp {
    host: String,
    server: String,
    credentials: Credentials,
}

impl Smtp {
    pub fn new(host: String, server: String, email: String, password: String) -> Self {
        Self {
            host,
            server,
            credentials: Credentials::new(email, password),
        }
    }

    pub fn send(&self, message: Message) -> anyhow::Result<()> {
        let mut conn = self.tls_client()?;
        let mut tcp = self.connect()?;
        let mut tls = Stream::new(&mut conn, &mut tcp);
        read_response(&mut tls)?;
        self.start_session(&mut tls, &message)?;
        message.send(&mut tls)?;
        self.finish_session(&mut tls)?;
        tcp.shutdown(Shutdown::Both)?;
        Ok(())
    }

    fn tls_client(&self) -> anyhow::Result<ClientConnection> {
        let _ = ring::default_provider().install_default();
        let config = ClientConfig::builder()
            .with_root_certificates(RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            })
            .with_no_client_auth();
        Ok(ClientConnection::new(
            Arc::new(config),
            ServerName::DnsName(self.host.clone().try_into()?),
        )?)
    }

    fn connect(&self) -> anyhow::Result<TcpStream> {
        let addr = self
            .server
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("couldn't connect to {}", self.server))?;
        Ok(TcpStream::connect(addr)?)
    }

    fn start_session(&self, tx: &mut TlsStream, message: &Message) -> anyhow::Result<()> {
        Command::Ehlo.send_with(tx, self.credentials.domain()?)?;
        Command::Auth.send_with(tx, &self.credentials.encode())?;
        Command::MailFrom.send_with_path(tx, &message.from_email())?;
        Command::RcptTo.send_with_path(tx, &message.to_email())?;
        Command::Data.send(tx)?;
        Ok(())
    }

    fn finish_session(&self, tx: &mut TlsStream) -> anyhow::Result<()> {
        Command::Quit.send(tx)
    }
}

fn read_response(to: &mut TlsStream) -> anyhow::Result<()> {
    let mut buf = String::new();
    while !last_line(&buf) {
        buf.clear();
        if to.read_line(&mut buf)? > 0 {
            print!("<- {buf}");
        }
    }
    Ok(())
}

fn last_line(buf: &str) -> bool {
    buf.bytes()
        .nth(3)
        .is_some_and(|delimiter| delimiter == b' ')
}
