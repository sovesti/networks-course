use std::{collections::VecDeque, fs::File, io, path::PathBuf};

use anyhow::anyhow;
use suppaftp::{FtpStream, list};

use crate::config::{Command, FtpConfig};

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
    pub fn execute_command(&mut self, args: Command) -> anyhow::Result<()> {
        Ok(match args {
            Command::List => self.raw_list()?.iter().for_each(|file| println!("{file}")),
            Command::Upload { file } => println!("Sent {} bytes", self.upload(&file)?),
            Command::Download { file } => println!("Received {} bytes", self.download(&file)?),
        })
    }

    pub fn raw_list(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.stream.list(None)?)
    }

    pub fn list(&mut self) -> anyhow::Result<Vec<list::File>> {
        Ok(self
            .stream
            .list(None)?
            .iter()
            .map(|file| file.parse::<list::File>())
            .collect::<Result<_, _>>()?)
    }

    pub fn upload(&mut self, path: &PathBuf) -> anyhow::Result<u64> {
        Ok(self
            .stream
            .put_file(from_os(path)?, &mut File::open(path)?)?)
    }

    pub fn upload_text(&mut self, path: &PathBuf, text: String) -> anyhow::Result<u64> {
        Ok(self
            .stream
            .put_file(from_os(path)?, &mut VecDeque::from(text.into_bytes()))?)
    }

    pub fn download(&mut self, path: &PathBuf) -> anyhow::Result<u64> {
        Ok(self.stream.retr(from_os(path)?, |mut file| {
            Ok(File::create(path).and_then(|mut local| io::copy(&mut file, &mut local)))
        })??)
    }

    pub fn download_text(&mut self, path: &PathBuf) -> anyhow::Result<String> {
        Ok(self
            .stream
            .retr_as_buffer(from_os(path)?)?
            .into_inner()
            .try_into()?)
    }

    pub fn delete(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        Ok(self.stream.rm(from_os(path)?)?)
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        Ok(self.stream.quit()?)
    }
}

impl Drop for FtpConnection {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn from_os<'a>(path: &'a PathBuf) -> anyhow::Result<&'a str> {
    path.to_str()
        .ok_or_else(|| anyhow!("invalid path {path:?}"))
}
