use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct FtpConfig {
    #[arg(short, long, default_value = default_address())]
    pub address: String,

    #[arg(short, long, default_value = default_user())]
    pub user: String,

    #[arg(short, long, default_value = default_password())]
    pub password: String,

    #[command(subcommand)]
    pub command: Command,
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
