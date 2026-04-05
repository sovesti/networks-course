use std::env::args;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{self, Stdio};
use std::thread::{self, JoinHandle};

use anyhow::anyhow;

fn wait(mut out: TcpStream, mut child: process::Child) -> anyhow::Result<()> {
    let status = child.wait()?;
    pipe_out(out.try_clone()?, &mut child)?;
    writeln!(&mut out, "{status}")?;
    out.flush()?;
    Ok(())
}

fn execute(command: &str, out: TcpStream) -> anyhow::Result<()> {
    let parts: Vec<_> = split(command);
    if parts.is_empty() {
        return Ok(());
    }
    let child = spawn(parts)?;
    wait(out, child)?;
    Ok(())
}

fn split(buf: &str) -> Vec<String> {
    buf.split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|program| program.to_owned())
        .collect()
}

fn spawn(parts: Vec<String>) -> Result<process::Child, io::Error> {
    log::info!("running command: {}", parts.first().unwrap());
    process::Command::new(parts.first().unwrap())
        .args(parts.iter().skip(1))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn pipe_out(out: TcpStream, child: &mut process::Child) -> anyhow::Result<()> {
    let stdout = pipe_stdout(out.try_clone()?, child)?;
    let stderr = pipe_stderr(out.try_clone()?, child)?;
    stdout
        .join()
        .map_err(|_| anyhow!("failed to redirect stdout"))?;
    stderr
        .join()
        .map_err(|_| anyhow!("failed to redirect stderr"))?;
    Ok(())
}

fn pipe_stdout(out: TcpStream, child: &mut process::Child) -> anyhow::Result<JoinHandle<u64>> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("no stdout captured"))?;
    let mut cloned = out.try_clone()?;
    Ok(thread::spawn(move || {
        io::copy(&mut stdout, &mut cloned).unwrap()
    }))
}

fn pipe_stderr(out: TcpStream, child: &mut process::Child) -> anyhow::Result<JoinHandle<u64>> {
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("no stderr captured"))?;
    let mut cloned = out.try_clone()?;
    Ok(thread::spawn(move || {
        io::copy(&mut stderr, &mut cloned).unwrap()
    }))
}

fn talk(client: TcpStream) -> anyhow::Result<()> {
    let mut read = BufReader::new(client.try_clone()?);
    let mut buf = String::new();
    while let Ok(_) = read.read_line(&mut buf)
        && buf.trim() != "exit"
    {
        execute(&buf, client.try_clone()?)?;
        buf.clear();
    }
    Ok(())
}

fn port() -> u16 {
    args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(3000)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port())).unwrap();
    while let Ok((mut socket, _)) = listener.accept() {
        if let Err(err) = talk(socket.try_clone()?) {
            log::error!("{err}");
            writeln!(socket, "{err}")?;
        }
        socket.shutdown(Shutdown::Both)?;
    }
    Ok(())
}
