use std::{fs::File, io, path::PathBuf};

use anyhow::anyhow;
use suppaftp::FtpStream;

use crate::config::FtpConfig;

pub struct FtpConnection {
    stream: FtpStream,
}

impl TryFrom<&FtpConfig> for FtpConnection {
    type Error = anyhow::Error;

    fn try_from(config: &FtpConfig) -> anyhow::Result<Self> {
        let mut stream = FtpStream::connect(&config.address)?;
        stream.login(&config.user, &config.password)?;
        Ok(Self { stream })
    }
}

impl FtpConnection {
    pub fn raw_list(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.stream.list(None)?)
    }

    pub fn upload(&mut self, path: &PathBuf) -> anyhow::Result<u64> {
        Ok(self
            .stream
            .put_file(from_os(path)?, &mut File::open(path)?)?)
    }

    pub fn download(&mut self, path: &PathBuf) -> anyhow::Result<u64> {
        Ok(self.stream.retr(from_os(path)?, |mut file| {
            Ok(File::create(path).and_then(|mut local| io::copy(&mut file, &mut local)))
        })??)
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        Ok(self.stream.quit()?)
    }
}

fn from_os<'a>(path: &'a PathBuf) -> anyhow::Result<&'a str> {
    path.to_str()
        .ok_or_else(|| anyhow!("invalid path {path:?}"))
}
