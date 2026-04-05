use base64::{Engine, prelude::BASE64_STANDARD};

pub struct Credentials {
    email: String,
    password: String,
}

impl Credentials {
    pub fn new(email: String, password: String) -> Self {
        Self { email, password }
    }

    pub fn domain(&self) -> anyhow::Result<&str> {
        Ok("spbu.ru") // see https://support.mozilla.org/ga-IE/kb/thunderbird-smtp-ehlo
    }

    pub fn encode(&self) -> String {
        BASE64_STANDARD.encode(format!("\0{}\0{}", self.email, self.password))
    }
}

pub struct Mailbox {
    from: Option<String>,
    email: String,
}

impl Mailbox {
    pub fn anonymous(email: String) -> Self {
        Self { from: None, email }
    }

    pub fn named(name: String, email: String) -> Self {
        Self {
            from: Some(name),
            email,
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

impl ToString for Mailbox {
    fn to_string(&self) -> String {
        format!(
            "{}<{}>",
            self.from.clone().map(|name| name + " ").unwrap_or_default(),
            self.email
        )
    }
}
