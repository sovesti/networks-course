use clap::Parser;

use crate::{
    config::{Command, FtpConfig},
    connection::FtpConnection,
};

mod config;
mod connection;

fn cli() -> anyhow::Result<()> {
    let args = FtpConfig::parse();
    let mut connection = FtpConnection::try_from(&args)?;
    execute_command(args.command, &mut connection)?;
    connection.close()
}

fn execute_command(args: Command, connection: &mut FtpConnection) -> anyhow::Result<()> {
    Ok(match args {
        Command::List => connection
            .raw_list()?
            .iter()
            .for_each(|file| println!("{file}")),
        Command::Upload { file } => println!("Sent {} bytes", connection.upload(&file)?),
        Command::Download { file } => println!("Received {} bytes", connection.download(&file)?),
    })
}

fn success() -> &'static str {
    "Operation completed succesfully"
}

fn error(err: anyhow::Error) -> String {
    log::error!("{err:?}");
    format!("An error occured: {err}")
}

fn main() {
    env_logger::init();
    match cli() {
        Ok(_) => println!("{}", success()),
        Err(err) => println!("{}", error(err)),
    }
}
