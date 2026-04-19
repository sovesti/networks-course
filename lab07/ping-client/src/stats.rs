use std::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    time::Duration,
};

pub struct Session {
    addr: SocketAddr,
    bytes: usize,
}

impl Session {
    pub fn new(addr: SocketAddr, bytes: usize) -> Self {
        Self { addr, bytes }
    }
}

impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pinging {} with {} bytes of data:",
            self.addr, self.bytes
        )
    }
}

struct Reply {
    addr: SocketAddr,
    bytes: usize,
    time: u128,
}

impl Reply {
    fn new(addr: SocketAddr, bytes: usize, time: Duration) -> Self {
        Self {
            addr,
            bytes,
            time: time.as_millis(),
        }
    }
}

impl Display for Reply {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Reply from {}: bytes={} time={}ms",
            self.addr, self.bytes, self.time
        )
        // write!(f, "RTT: {}ms", self.time)
    }
}

#[derive(PartialEq)]
enum Outcome {
    Received,
    Lost,
}

pub struct Stats {
    session: Session,
    lost: Vec<Outcome>,
    replies: Vec<Reply>,
}

impl Stats {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            lost: vec![],
            replies: vec![],
        }
    }

    pub fn lost(&mut self) {
        self.lost.push(Outcome::Lost);
        println!("Request timed out.");
    }

    pub fn received(&mut self, bytes: usize, addr: SocketAddr, time: Duration) {
        self.lost.push(Outcome::Received);
        let reply = Reply::new(addr, bytes, time);
        println!("{reply}");
        self.replies.push(reply);
    }

    fn sent_packets(&self) -> usize {
        self.lost.len()
    }

    fn received_packets(&self) -> usize {
        self.lost
            .iter()
            .filter(|l| **l == Outcome::Received)
            .count()
    }

    fn lost_packets(&self) -> usize {
        self.lost.iter().filter(|l| **l == Outcome::Lost).count()
    }

    fn loss(&self) -> usize {
        self.lost_packets() * 100 / self.sent_packets()
    }

    fn minimum_rtt(&self) -> u128 {
        self.rtts().map(|rtt| rtt.time).min().unwrap_or_default()
    }

    fn maximum_rtt(&self) -> u128 {
        self.rtts().map(|rtt| rtt.time).max().unwrap_or_default()
    }

    fn average_rtt(&self) -> u128 {
        Some(self.received_packets() as u128)
            .filter(|&total| total != 0)
            .map(|total| self.rtts().map(|rtt| rtt.time).sum::<u128>() / total)
            .unwrap_or_default()
    }

    fn rtts(&self) -> impl Iterator<Item = &Reply> {
        self.replies.iter()
    }

    fn write_packets(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\tPackets: Sent = {}, Received = {}, Lost = {} ({}% loss),",
            self.sent_packets(),
            self.received_packets(),
            self.lost_packets(),
            self.loss()
        )
    }

    fn write_rtts(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Approximate round trip times in milli-seconds:")?;
        writeln!(
            f,
            "\tMinimum = {}ms, Maximum = {}ms, Average = {}ms",
            self.minimum_rtt(),
            self.maximum_rtt(),
            self.average_rtt()
        )
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "")?;
        writeln!(f, "Ping statistics for {}:", self.session.addr)?;
        self.write_packets(f)?;
        if self.rtts().next().is_some() {
            self.write_rtts(f)?;
        }
        writeln!(f, "")?;
        Ok(())
    }
}
