use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    net::TcpStream,
    time::SystemTime,
};

use anyhow::anyhow;
use httparse::{EMPTY_HEADER, Request};
use httpdate::fmt_http_date;

enum FileResponse {
    Opened(File, u64),
    NotFound,
}

fn open_file(path: &str) -> anyhow::Result<FileResponse> {
    let mut file = File::open(&path[1..])?;
    let len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;
    Ok(FileResponse::Opened(file, len))
}

impl FileResponse {
    fn status(&self) -> &'static str {
        match self {
            FileResponse::Opened(_, _) => "200 OK",
            FileResponse::NotFound => "404 Not Found",
        }
    }
}

fn file_response(path: &str) -> FileResponse {
    match open_file(path) {
        Ok(opened) => opened,
        Err(err) => {
            log::error!("{}", err.context(path.to_owned()));
            FileResponse::NotFound
        }
    }
}

fn file(request: Request) -> anyhow::Result<FileResponse> {
    let path = request.path.ok_or_else(|| anyhow!("bad request"))?;
    let method = request.method.ok_or_else(|| anyhow!("bad request"))?;
    log::info!("Requested file: {}", path);
    Ok(match method {
        "GET" => file_response(path),
        _ => FileResponse::NotFound,
    })
}

fn _slow_send_file(output: &mut impl Write, mut file: File) -> anyhow::Result<usize> {
    let mut total = 0;
    let mut buf = [0; 8];
    loop {
        if file.read_exact(&mut buf).is_err() {
            break;
        }
        total += buf.len();
        output.write_all(&buf)?;
    }
    Ok(total)
}

fn write_body(response: FileResponse, buffered: &mut impl Write) -> anyhow::Result<u64> {
    match response {
        FileResponse::Opened(mut file, len) => {
            write!(buffered, "Content-Length: {len}\r\n")?;
            write!(buffered, "\r\n")?;
            Ok(io::copy(&mut file, buffered)?)
            // Ok(_slow_send_file(buffered, file)? as u64)
        }
        FileResponse::NotFound => {
            write!(buffered, "\r\n")?;
            Ok(0)
        }
    }
}

fn serialize(response: FileResponse, serialized: &mut impl Write) -> anyhow::Result<()> {
    let mut buffered = BufWriter::new(serialized);
    // let mut buffered = serialized;
    write!(buffered, "HTTP/1.1 {}\r\n", response.status())?;
    write!(buffered, "Date: {}\r\n", fmt_http_date(SystemTime::now()))?;
    write_body(response, &mut buffered)?;
    buffered.flush()?;
    Ok(())
}

fn respond(mut connection: TcpStream, response: FileResponse) -> anyhow::Result<()> {
    serialize(response, &mut connection)?;
    connection.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}

fn title(connection: TcpStream) -> Result<Vec<u8>, anyhow::Error> {
    let mut buffer = vec![];
    let mut reader = BufReader::new(connection);
    reader.read_until(b'\n', &mut buffer)?;
    Ok(buffer)
}

pub fn talk_to(connection: TcpStream) -> anyhow::Result<()> {
    let mut headers = [EMPTY_HEADER; 64];
    let mut request = Request::new(&mut headers);
    let title = title(connection.try_clone()?)?;
    request.parse(&title)?;
    respond(connection, file(request)?)
}
