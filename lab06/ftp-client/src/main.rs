use std::path::PathBuf;

use clap::Parser;
use dioxus::prelude::*;
use suppaftp::list::File;

use crate::{
    config::{FtpCli, FtpConfig, default_address, default_password, default_user},
    connection::FtpConnection,
};

mod config;
mod connection;

fn cli() -> anyhow::Result<()> {
    let args = FtpCli::parse();
    let mut connection = FtpConnection::try_from(&args.config())?;
    connection.execute_command(args.command)
}

fn run_cli() {
    match cli() {
        Ok(_) => println!("{}", success()),
        Err(err) => println!("{}", display_error(&err)),
    }
}

fn success() -> &'static str {
    "OK"
}

fn display_error(err: &anyhow::Error) -> String {
    log::error!("{err:?}");
    format!("An error occured: {err}")
}

fn list_files(
    connection: &mut Signal<Option<FtpConnection>>,
    files: &mut Signal<Vec<File>>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    files.set(
        connection
            .write()
            .iter_mut()
            .flat_map(|conn| maybe_list_files(conn, error).into_iter())
            .collect(),
    );
}

fn maybe_list_files(
    conn: &mut FtpConnection,
    error: &mut Signal<Option<anyhow::Error>>,
) -> Vec<File> {
    match conn.list() {
        Ok(files) => {
            error.set(None);
            files
        }
        Err(err) => {
            log::error!("{err:?}");
            let _ = error.write().insert(err);
            vec![]
        }
    }
}

fn connect(
    config: &Signal<FtpConfig>,
    connection: &mut Signal<Option<FtpConnection>>,
    files: &mut Signal<Vec<File>>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    connection.set(None);
    match FtpConnection::try_from(&config().clone()) {
        Ok(conn) => {
            connection.set(Some(conn));
            error.set(None);
        }
        Err(err) => error.set(Some(err)),
    }
    list_files(connection, files, error);
}

fn upload_text(
    connection: &mut Signal<Option<FtpConnection>>,
    files: &mut Signal<Vec<File>>,
    path: &Signal<PathBuf>,
    file: &Signal<String>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    connection.write().iter_mut().for_each(|conn| {
        match conn.upload_text(&path.read().clone(), file.cloned()) {
            Ok(_) => error.set(None),
            Err(err) => error.set(Some(err)),
        }
    });
    list_files(connection, files, error);
}

fn download_file(
    connection: &mut Signal<Option<FtpConnection>>,
    path: &Signal<PathBuf>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    connection
        .write()
        .iter_mut()
        .for_each(|conn| match conn.download(&path.read().clone()) {
            Ok(_) => error.set(None),
            Err(err) => error.set(Some(err)),
        });
}

fn download_text(
    connection: &mut Signal<Option<FtpConnection>>,
    path: &Signal<PathBuf>,
    file: &mut Signal<String>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    connection
        .write()
        .iter_mut()
        .for_each(|conn| match conn.download_text(&path.read().clone()) {
            Ok(text) => {
                file.set(text);
                error.set(None);
            }
            Err(err) => error.set(Some(err)),
        });
}

fn delete_file(
    connection: &mut Signal<Option<FtpConnection>>,
    files: &mut Signal<Vec<File>>,
    path: &Signal<PathBuf>,
    error: &mut Signal<Option<anyhow::Error>>,
) {
    connection
        .write()
        .iter_mut()
        .for_each(|conn| match conn.delete(&path.read().clone()) {
            Ok(_) => error.set(None),
            Err(err) => error.set(Some(err)),
        });
    list_files(connection, files, error);
}

fn app() -> Element {
    let mut config = use_signal(FtpConfig::default);
    let mut connection = use_signal(|| None);
    let mut error = use_signal(|| None);
    let mut path = use_signal(PathBuf::new);
    let mut files: Signal<Vec<File>> = use_signal(Vec::new);
    let mut file: Signal<String> = use_signal(|| "".to_string());
    let mut editing = use_signal(|| false);
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        Stylesheet { href: "https://fonts.googleapis.com/icon?family=Material+Icons" }
        div {
            class: "font-mono bg-zinc-50",
            main {
                class: "flex flex-col max-w-100 h-dvh",
                if editing.read().cloned() {
                    div {
                        class: "flex flex-1 flex-col",
                        textarea {
                            class: "outline-1 pl-1 pr-1 m-1 h-full rounded-xs",
                            oninput: move |event| file.set(event.value()),
                            placeholder: "Write file contents here...",
                            initial_value: file.read().cloned(),
                            resize: "none"
                        }
                        button {
                            class: "outline-1 m-1 pl-5 pr-5 hover:bg-zinc-300 rounded-xs",
                            onclick: move |_| {
                                upload_text(&mut connection, &mut files, &path, &file, &mut error);
                                editing.toggle();
                            },
                            "Commit"
                        }
                    }
                } else {
                    div {
                        class: "flex flex-row justify-start",
                        input {
                            class: "outline-1 pl-1 pr-1 m-1 max-w-48 rounded-xs",
                            oninput: move |event| config.write().user = event.value(),
                            placeholder: "User",
                            initial_value: default_user()
                        }
                        input {
                            class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                            oninput: move |event| config.write().password = event.value(),
                            placeholder: "Password",
                            initial_value: default_password(),
                            type: "password"
                        }
                    }
                    div {
                        class: "flex flex-row justify-between",
                        div {
                            class: "flex flex-col",
                            input {
                                class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                                oninput: move |event| config.write().address = event.value(),
                                placeholder: "Host",
                                initial_value: default_address()
                            }
                        }
                        button {
                            class: "outline-1 m-1 pl-5 pr-5 hover:bg-zinc-300 rounded-xs",
                            onclick: move |_| connect(&config, &mut connection, &mut files, &mut error),
                            "Connect"
                        }
                    }
                    div {
                        class: "flex m-1 h-full outline-1 flex-col overflow-auto rounded-xs",
                        for file in files.iter()
                        {
                            div {
                                class: "flex flex-row justify-start",
                                i {
                                    class: "material-icons ml-1 w-6 h-6",
                                    font_size: "20px",
                                    if file.is_directory() { "folder" } else { "description" }
                                }
                                {file.name()}
                            }
                        }
                    }
                    div {
                        class: "flex flex-row justify-between",
                        div {
                            class: "flex flex-col justify-center flex-1",
                            input {
                                class: "outline-1 pl-1 pr-1 m-1 rounded-xs",
                                oninput: move |event| path.set(PathBuf::from(event.value())),
                                placeholder: "Path",
                                resize: "none"
                            }
                        }
                        div {
                            class: "flex flex-col",
                            button {
                                class: "outline-1 pl-5 pr-5 m-1 enabled:hover:bg-zinc-300 rounded-xs disabled:text-zinc-500",
                                disabled: connection.read().is_none(),
                                onclick: move |_| editing.toggle(),
                                "Create"
                            }
                            button {
                                class: "outline-1 pl-5 pr-5 m-1 enabled:hover:bg-zinc-300 rounded-xs disabled:text-zinc-500",
                                disabled: connection.read().is_none(),
                                onclick: move |_| download_file(&mut connection, &path, &mut error),
                                "Retrieve"
                            }
                            button {
                                class: "outline-1 m-1 pl-5 pr-5 enabled:hover:bg-zinc-300 rounded-xs disabled:text-zinc-500",
                                disabled: connection.read().is_none(),
                                onclick: move |_| {
                                    download_text(&mut connection, &path, &mut file, &mut error);
                                    editing.toggle();
                                },
                                "Update"
                            }
                            button {
                                class: "outline-1 m-1 pl-5 pr-5 enabled:hover:bg-zinc-300 rounded-xs disabled:text-zinc-500",
                                disabled: connection.read().is_none(),
                                onclick: move |_| delete_file(&mut connection, &mut files, &path, &mut error),
                                "Delete"
                            }
                        }
                    }
                    div {
                        class: "min-h-20 m-1 overflow-auto",
                        p {
                            class: if error.read().is_some() { "text-red-800" } else { "" },
                            {
                                error.read().as_ref().map(display_error).unwrap_or_else(|| success().to_owned())
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    dioxus::launch(app);
}
