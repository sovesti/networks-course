use std::{env, fs};

use anyhow::{Context, anyhow, bail};
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    smtp: String,
    email: String,
    password: String,
    from: String,
}

impl Config {
    fn credentials(&self) -> Credentials {
        Credentials::new(self.email.clone(), self.password.clone())
    }
}

enum Format {
    Html,
    Txt,
}

impl Format {
    fn content_type(&self) -> ContentType {
        match self {
            Format::Html => ContentType::TEXT_HTML,
            Format::Txt => ContentType::TEXT_PLAIN,
        }
    }

    fn parse(raw: &str) -> anyhow::Result<Format> {
        match raw {
            "txt" => Ok(Format::Txt),
            "html" => Ok(Format::Html),
            _ => bail!("wrong format {raw}"),
        }
    }
}

fn usage() -> anyhow::Error {
    anyhow!(
        "Usage: `lettre-client.exe html aaa@aaa.com \"Some subject\" body.html`
            or `lettre-client.exe txt aaa@aaa.com \"Some subject\" body.txt`"
    )
}

fn arg(index: usize) -> anyhow::Result<String> {
    env::args().nth(index).ok_or_else(usage)
}

fn format() -> anyhow::Result<Format> {
    Ok(Format::parse(&arg(1)?)?)
}

fn to() -> anyhow::Result<Mailbox> {
    Ok(Mailbox::new(None, arg(2)?.parse().context(arg(2)?)?))
}

fn subject() -> anyhow::Result<String> {
    arg(3)
}

fn body() -> anyhow::Result<String> {
    Ok(fs::read_to_string(arg(4)?)?)
}

fn transport(config: &Config) -> SmtpTransport {
    SmtpTransport::relay(&config.smtp)
        .unwrap()
        .credentials(config.credentials())
        .build()
}

fn email(config: &Config) -> anyhow::Result<Message> {
    Ok(Message::builder()
        .from(Mailbox::new(
            Some(config.from.clone()),
            config.email.parse().context(config.email.clone())?,
        ))
        .to(to()?)
        .subject(subject()?)
        .header(format()?.content_type())
        .body(body()?)?)
}

fn main() -> anyhow::Result<()> {
    let config = toml::from_str(include_str!("../config.toml"))?;
    transport(&config).send(&email(&config)?)?;
    Ok(())
}
