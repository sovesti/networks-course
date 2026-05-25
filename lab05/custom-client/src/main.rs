mod address;
mod encoding;
mod message;
mod transport;

use std::{
    env::{self, args},
    fs,
};

use address::Mailbox;
use anyhow::{Context, anyhow};
use message::{Attachment, Contents, Message};
use serde::Deserialize;
use transport::Smtp;

#[derive(Deserialize, Debug)]
struct Config {
    smtp: String,
    port: u16,
    email: String,
    password: String,
    from: String,
}

impl Config {
    fn host(&self) -> String {
        self.smtp.clone()
    }

    fn smtp(&self) -> String {
        format!("{}:{}", self.smtp, self.port)
    }

    fn from(&self) -> Mailbox {
        Mailbox::named(self.from.clone(), self.email.clone())
    }
}

fn usage() -> anyhow::Error {
    anyhow!(
        "Usage: `custom-client.exe aaa@aaa.com \"Some subject\" body.txt image/png/picture.png image/gif/gif.gif`"
    )
}

fn arg(index: usize) -> anyhow::Result<String> {
    env::args().nth(index).ok_or_else(usage)
}

fn to() -> anyhow::Result<Mailbox> {
    Ok(Mailbox::anonymous(arg(1)?.parse().context(arg(1)?)?))
}

fn subject() -> anyhow::Result<String> {
    arg(2)
}

fn body() -> anyhow::Result<String> {
    arg(3)
}

fn attachments() -> anyhow::Result<Vec<Attachment>> {
    args().skip(4).map(|arg| arg.parse()).collect()
}

fn transport(config: &Config) -> anyhow::Result<Smtp> {
    Ok(Smtp::new(
        config.host(),
        config.smtp(),
        config.email.clone(),
        config.password.clone(),
    ))
}

fn email(config: &Config) -> anyhow::Result<Message> {
    Ok(Message::new(
        config.from(),
        to()?,
        Contents::new(subject()?, body()?, attachments()?),
    ))
}

fn main() -> anyhow::Result<()> {
    let config = toml::from_str(&fs::read_to_string("config.toml")?)?;
    transport(&config)?.send(email(&config)?)?;
    Ok(())
}
