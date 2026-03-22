use std::{
    env::args,
    io::{self, BufRead, BufReader, BufWriter, Write, stdout},
    net::{SocketAddr, TcpStream},
};

fn host() -> String {
    args().nth(1).unwrap_or("127.0.0.1".to_owned())
}

fn port() -> u16 {
    args().nth(2).and_then(|p| p.parse().ok()).unwrap_or(3000)
}

fn filename() -> String {
    args().nth(3).unwrap_or("files/hello".to_owned())
}

struct Request {
    host: String,
    port: u16,
    filename: String,
}

impl Request {
    fn new() -> Self {
        Self {
            host: host(),
            port: port(),
            filename: filename(),
        }
    }

    fn addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(SocketAddr::new(self.host.parse()?, self.port))
    }
}

fn serialize(request: Request, serialized: &mut Vec<u8>) -> anyhow::Result<()> {
    let mut buffered = BufWriter::new(serialized);
    write!(buffered, "GET /{} HTTP/1.1\r\n", request.filename)?;
    write!(buffered, "Host: {}\r\n", request.host)?;
    write!(buffered, "\r\n")?;
    buffered.flush()?;
    Ok(())
}

fn send(request: Request, server: &mut TcpStream) -> anyhow::Result<()> {
    let mut serialized = vec![];
    serialize(request, &mut serialized)?;
    server.write_all(&serialized)?;
    Ok(())
}

fn receive(server: &mut TcpStream, to: &mut impl Write) -> Result<(), anyhow::Error> {
    let mut buf: Vec<u8> = vec![];
    let mut reader = BufReader::new(server.try_clone()?);
    while buf.len() != [b'\r', b'\n'].len() {
        buf.clear();
        reader.read_until(b'\n', &mut buf)?;
    }
    io::copy(&mut reader, to)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let request = Request::new();
    let mut server = TcpStream::connect(request.addr()?)?;
    send(request, &mut server)?;
    receive(&mut server, &mut stdout())?;
    Ok(())
}
