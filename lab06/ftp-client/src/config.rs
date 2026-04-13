use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct FtpCli {
    #[arg(short, long, default_value = default_address())]
    pub address: String,

    #[arg(short, long, default_value = default_user())]
    pub user: String,

    #[arg(short, long, default_value = default_password())]
    pub password: String,

    #[command(subcommand)]
    pub command: Command,
}

impl FtpCli {
    pub fn config(&self) -> FtpConfig {
        FtpConfig {
            address: self.address.clone(),
            user: self.user.clone(),
            password: self.password.clone(),
        }
    }
}

#[derive(Clone)]
pub struct FtpConfig {
    pub address: String,
    pub user: String,
    pub password: String,
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            address: default_address().to_owned(),
            user: default_user().to_owned(),
            password: default_password().to_owned(),
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    List,
    Upload { file: PathBuf },
    Download { file: PathBuf },
}

pub fn default_address() -> &'static str {
    "127.0.0.1:21"
}

pub fn default_user() -> &'static str {
    "TestUser"
}

pub fn default_password() -> &'static str {
    "12345678"
}
