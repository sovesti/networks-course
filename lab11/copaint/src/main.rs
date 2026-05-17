mod canvas;
mod error;
mod interfaces;
mod streaming;

use std::{fs::File, io::Read, net::SocketAddr, str::FromStr};

use dioxus::{logger::tracing::Level, prelude::*};
use serde::Deserialize;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};

use crate::canvas::Canvas;
use crate::{error::ErrorView, interfaces::my_ip};

#[derive(Deserialize, Clone, Copy)]
struct Config {
    port: u16,
}

impl Config {
    fn parse() -> Self {
        let mut config = String::new();
        File::open("config.toml")
            .unwrap()
            .read_to_string(&mut config)
            .unwrap();
        toml::from_str(&config).unwrap()
    }
}

fn my_address(config: Config) -> Option<SocketAddr> {
    my_ip().ok().map(|ip| SocketAddr::new(ip, config.port))
}

fn connect(
    read: Signal<Option<OwnedReadHalf>>,
    write: Signal<Option<OwnedWriteHalf>>,
    mut error: Signal<Option<anyhow::Error>>,
    mut connected: Signal<bool>,
    addr: Signal<String>,
) {
    connected.set(true);
    let addr = SocketAddr::from_str(addr.read().as_ref());
    if let Err(err) = addr {
        error.set(Some(err.into()));
        connected.set(false);
        return;
    }
    spawn(async move {
        match TcpStream::connect(addr.unwrap()).await {
            Ok(connected) => split_stream(read, write, connected),
            Err(err) => {
                connected.set(false);
                error.set(Some(err.into()));
            }
        }
    });
}

fn search(mut serving: Signal<bool>, mut accept: Action<(), ()>) {
    accept.cancel();
    serving.set(false);
}

async fn accept(
    mut serving: Signal<bool>,
    read: Signal<Option<OwnedReadHalf>>,
    write: Signal<Option<OwnedWriteHalf>>,
) -> anyhow::Result<()> {
    let listener = use_context::<NetContext>().listener;
    match listener.read().as_ref() {
        Some(listener) => {
            serving.set(true);
            split_stream(read, write, listener.accept().await?.0);
        }
        _ => (),
    }
    Ok(())
}

fn split_stream(
    mut read: Signal<Option<OwnedReadHalf>>,
    mut write: Signal<Option<OwnedWriteHalf>>,
    accepted: TcpStream,
) {
    let accepted = accepted.into_split();
    read.set(Some(accepted.0));
    write.set(Some(accepted.1));
}

#[component]
fn Connected(
    serving: Signal<bool>,
    mut read: Signal<Option<OwnedReadHalf>>,
    mut write: Signal<Option<OwnedWriteHalf>>,
    mut accept: Action<(), ()>,
) -> Element {
    let error = use_signal(|| None);
    let me = use_context::<NetContext>().me;
    let connected = use_signal(|| false);
    let mut target = use_signal(|| me.read().to_string());
    rsx! {
        div {
            class: "flex flex-row",
            button {
                class: format!("p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1"),
                disabled: *serving.read(),
                onclick: move |_| accept.call(),
                "Создать сервер"
            }
            button {
                class: "p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1",
                disabled: !*serving.read(),
                onclick: move |_| search(serving, accept),
                "Присоединиться к серверу"
            }
        }
        if *serving.read() {
            p {
                class: "pl-1 pr-1 m-1 w-48 rounded-xs",
                "Ожидание, адрес: {me}"
            }
        } else {
            input {
                class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                oninput: move |event| target.set(event.value()),
                placeholder: "Адрес",
                initial_value: me.read().to_string()
            }
            div {
                class: "m-1 w-48 rounded-xs text-xl",
                button {
                    class: "hover:bg-zinc-200",
                    disabled: *connected.read(),
                    onclick: move |_| connect(read, write, error, connected, target),
                    p {
                        "Подключиться"
                    }
                }
            }
        }
        ErrorView { error }
    }
}

#[derive(Clone, Copy)]
struct NetContext {
    me: Signal<SocketAddr>,
    listener: Signal<Option<TcpListener>>,
}

impl NetContext {
    fn new(me: Signal<SocketAddr>, listener: Signal<Option<TcpListener>>) -> Self {
        spawn(start_listening(me, listener));
        Self { me, listener }
    }
}

async fn start_listening(me: Signal<SocketAddr>, mut listener: Signal<Option<TcpListener>>) {
    listener.set(TcpListener::bind(*me.read()).await.ok());
}

fn app() -> Element {
    let serving = use_signal(|| false);
    let read = use_signal(|| None);
    let write = use_signal(|| None);
    let config = use_signal(Config::parse);
    let me = use_signal(|| my_address(config.read().clone()).unwrap());
    let listener = use_signal(|| None);
    use_context_provider(|| NetContext::new(me, listener));
    let accept = use_action(move || accept(serving, read, write));
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        div {
            class: "font-mono bg-zinc-50",
            main {
                class: "flex flex-col max-w-100 h-dvh",
                if read.read().is_some() {
                    Canvas { read, write }
                } else {
                    Connected { serving, read, write, accept }
                }
            }
        }
    }
}

fn main() {
    dioxus::logger::init(Level::DEBUG).unwrap();
    dioxus::launch(app);
}
