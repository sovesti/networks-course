use std::{
    env::args,
    io::{self, Write, stdin, stdout},
    net::TcpStream,
    thread,
};

fn host() -> String {
    args().nth(1).unwrap_or("localhost".to_owned())
}

fn port() -> u16 {
    args().nth(2).and_then(|p| p.parse().ok()).unwrap_or(3000)
}

fn print_results(mut conn: TcpStream) {
    io::copy(&mut conn, &mut stdout()).unwrap();
}

fn main() {
    let mut command = String::new();
    let mut conn = TcpStream::connect(format!("{}:{}", host(), port())).unwrap();
    let out = conn.try_clone().unwrap();
    let out = thread::spawn(move || print_results(out));
    while let Ok(_) = stdin().read_line(&mut command) {
        write!(&mut conn, "{command}").unwrap();
        command.clear();
    }
    out.join().unwrap();
}
