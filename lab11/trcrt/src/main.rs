mod icmp;
mod message;
mod message_types;

use std::{
    io::{Write, stdout},
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use clap::ArgAction::Help;
use clap::Parser;

use crate::{icmp::IcmpConnection, message::Message, message_types::MessageType};

#[derive(clap::Parser)]
#[clap(disable_help_flag = true)]
struct Cli {
    /// Maximum number of hops to search for target.
    #[arg(short, long, default_value_t = 30)]
    hops: u16,
    /// Wait timeout milliseconds for each reply.
    #[arg(short, long, default_value_t = 1000)]
    wait: u64,
    /// Number of retries for each host.
    #[arg(short, long, default_value_t = 3)]
    retries: u16,

    /// Display this message.
    #[arg(long, action = Help)]
    help: Option<bool>,

    target: String,
}

fn show_prologue(host: String, addr: SocketAddr, hops: u16) {
    let addr = if host == addr.ip().to_string() {
        host.clone()
    } else {
        format!("{} [{}]", host, addr.ip())
    };
    println!();
    println!("Tracing route to {addr}");
    println!("over a maximum of {} hops:", hops);
    println!()
}

const BYTES: usize = 64;

fn message() -> String {
    "*".repeat(BYTES)
}

enum Response {
    Lost(String),
    Received(Duration, Message),
}

impl Response {
    fn rtt(&self) -> Option<Duration> {
        match self {
            Response::Lost(_) => None,
            Response::Received(duration, _) => Some(*duration),
        }
    }

    fn addr(&self) -> Option<SocketAddr> {
        match self {
            Response::Lost(_) => None,
            Response::Received(_, message) => Some(SocketAddr::new(message.source().into(), 0)),
        }
    }

    fn is_target(&self) -> bool {
        match self {
            Response::Lost(_) => false,
            Response::Received(_, message) => !message.failure(),
        }
    }

    fn unwrap_err(&self) -> String {
        match self {
            Response::Lost(err) => err.clone(),
            Response::Received(_, _) => panic!(),
        }
    }
}

fn attempt_ping(seq: u16, socket: &mut IcmpConnection) -> anyhow::Result<Response> {
    let start = Instant::now();
    socket.send(MessageType::Echo, seq, message().as_bytes())?;
    Ok(match socket.recv(seq) {
        Ok(None) => Response::Lost("Request timed out.".to_string()),
        Ok(Some(message)) if message.rejected() => Response::Lost(message.show_error()),
        Ok(Some(message)) => Response::Received(start.elapsed(), message),
        Err(err) => Response::Lost(format!("Error: {err:?}")),
    })
}

fn show_addr(addr: SocketAddr) -> String {
    match dns_lookup::getnameinfo(&addr, 0) {
        Ok((host, _)) if host != addr.ip().to_string() => format!("{host} [{}]", addr.ip()),
        _ => addr.ip().to_string(),
    }
}

fn report_host(responses: &Vec<Response>) {
    print!(
        "  {}",
        responses
            .iter()
            .filter_map(Response::addr)
            .next()
            .map(show_addr)
            .unwrap_or_else(|| responses.last().unwrap().unwrap_err())
    );
}

fn report_response(
    response: Response,
    rtts: &mut Vec<Option<Duration>>,
    responses: &mut Vec<Response>,
) {
    let rtt = response.rtt();
    print!(
        "{:>9}",
        rtt.map(|duration| duration.as_millis().to_string() + " ms")
            .unwrap_or("*   ".to_string())
    );
    stdout().flush().unwrap();
    rtts.push(rtt);
    responses.push(response);
}

fn try_hops(hops: u16, retries: u16, socket: &mut IcmpConnection) -> anyhow::Result<bool> {
    socket.set_ttl(hops as u32)?;
    let mut rtts = vec![];
    let mut responses = vec![];
    print!("{hops:>3}");
    stdout().flush().unwrap();
    for retry in 0..retries {
        let response = attempt_ping(hops * retries + retry, socket)?;
        report_response(response, &mut rtts, &mut responses);
    }
    report_host(&responses);
    println!();
    Ok(responses.iter().any(Response::is_target))
}

fn addr(target: &str) -> anyhow::Result<SocketAddr> {
    Ok((target, 0)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("Failed to resolve host {target}"))?)
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let addr = addr(&args.target)?;
    let mut socket = IcmpConnection::new(addr, 122, Duration::from_millis(args.wait))?;
    show_prologue(args.target, addr, args.hops);
    for hops in 1..=args.hops {
        if try_hops(hops, args.retries, &mut socket)? {
            break;
        }
    }
    println!();
    println!("Trace complete.");
    Ok(())
}
