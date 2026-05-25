use std::{io::Write, str::FromStr};

use anyhow::anyhow;
use uuid::Uuid;

use crate::{address::Mailbox, encoding::EncodedFile};

enum Header {
    From,
    To,
    Subject,
    MimeVersion,
    ContentDisposition,
    ContentTransferEncoding,
    ContentType,
}

impl Header {
    fn send_with(&self, to: &mut impl Write, contents: &str) -> anyhow::Result<()> {
        write!(to, "{}: {contents}\r\n", self.to_string())?;
        Ok(())
    }
}

impl ToString for Header {
    fn to_string(&self) -> String {
        match self {
            Header::From => "From",
            Header::To => "To",
            Header::Subject => "Subject",
            Header::MimeVersion => "MIME-Version",
            Header::ContentDisposition => "Content-Disposition",
            Header::ContentTransferEncoding => "Content-Transfer-Encoding",
            Header::ContentType => "Content-Type",
        }
        .to_owned()
    }
}

pub struct Attachment {
    mime: String,
    file: String,
}

fn wrong_format(raw: &str) -> anyhow::Error {
    anyhow!("wrong attachment format: {raw}")
}

impl Attachment {
    fn new(mime: String, file: String) -> Self {
        Self { mime, file }
    }

    fn send(&self, to: &mut impl Write, boundary: &str) -> anyhow::Result<()> {
        send_boundary(to, boundary)?;
        Header::ContentDisposition
            .send_with(to, &format!("inline; filename=\"{}\"", &self.file))?;
        Header::ContentTransferEncoding.send_with(to, "base64")?;
        Header::ContentType.send_with(to, &self.mime)?;
        send_file(to, &self.file)?;
        Ok(())
    }
}

impl FromStr for Attachment {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> anyhow::Result<Self> {
        let mut parts = raw.split('/');
        let mime = format!(
            "{}/{}",
            parts.next().ok_or_else(|| wrong_format(raw))?,
            parts.next().ok_or_else(|| wrong_format(raw))?
        );
        let file = parts.fold(String::new(), |acc, next| acc + "/" + next);
        Ok(Self::new(mime, file[1..].to_owned()))
    }
}

pub struct Contents {
    subject: String,
    boundary: String,
    text: String,
    attachments: Vec<Attachment>,
}

impl Contents {
    pub fn new(subject: String, text: String, attachments: Vec<Attachment>) -> Self {
        Self {
            subject,
            boundary: Uuid::new_v4().to_string(),
            text,
            attachments,
        }
    }

    pub fn send(&self, to: &mut impl Write) -> anyhow::Result<()> {
        self.send_text(to)?;
        self.attachments
            .iter()
            .map(|attachment| attachment.send(to, &self.boundary))
            .collect::<anyhow::Result<Vec<_>>>()?;
        send_final_boundary(to, &self.boundary)?;
        Ok(())
    }

    fn send_text(&self, to: &mut impl Write) -> anyhow::Result<()> {
        send_boundary(to, &self.boundary)?;
        Header::ContentType.send_with(to, "text/plain; charset=utf-8")?;
        Header::ContentTransferEncoding.send_with(to, "base64")?;
        send_file(to, &self.text)?;
        Ok(())
    }
}

pub struct Message {
    from: Mailbox,
    to: Mailbox,
    contents: Contents,
}

impl Message {
    pub fn new(from: Mailbox, to: Mailbox, contents: Contents) -> Self {
        Self { from, to, contents }
    }

    pub fn send(&self, to: &mut impl Write) -> anyhow::Result<()> {
        self.headers(to)?;
        self.contents.send(to)?;
        Ok(())
    }

    fn headers(&self, to: &mut impl Write) -> anyhow::Result<()> {
        Header::From.send_with(to, &self.from.to_string())?;
        Header::To.send_with(to, &self.to.to_string())?;
        Header::Subject.send_with(to, &self.contents.subject)?;
        Header::MimeVersion.send_with(to, "1.0")?;
        Header::ContentType.send_with(
            to,
            &format!("multipart/mixed; boundary={}", &self.contents.boundary),
        )?;
        Ok(())
    }

    pub fn from_email(&self) -> &str {
        &self.from.email()
    }

    pub fn to_email(&self) -> &str {
        &self.to.email()
    }
}

fn new_line(to: &mut impl Write) -> anyhow::Result<()> {
    write!(to, "\r\n")?;
    Ok(())
}

fn send_file(to: &mut impl Write, file: &str) -> anyhow::Result<()> {
    let mut encoded = EncodedFile::new(to, file)?;
    let mut remainder = 0;
    while let Some(len) = encoded.read_from_file(remainder) {
        remainder = encoded.chunked_encode(len as usize)?;
    }
    encoded.encode_remainder(remainder)?;
    Ok(())
}

fn send_boundary(to: &mut impl Write, boundary: &str) -> anyhow::Result<()> {
    new_line(to)?;
    write!(to, "--{boundary}\r\n")?;
    Ok(())
}

fn send_final_boundary(to: &mut impl Write, boundary: &str) -> anyhow::Result<()> {
    new_line(to)?;
    write!(to, "--{boundary}--\r\n.\r\n")?;
    Ok(())
}
