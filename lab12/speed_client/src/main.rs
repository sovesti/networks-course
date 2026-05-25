mod error;
mod random;
mod subnet;

use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use dioxus::{
    logger::tracing::{self, Level},
    prelude::*,
};
use serde::Deserialize;
use tokio::time::{Instant, sleep_until};

use crate::subnet::{KnownCopies, Message, Subnet};

fn success() -> &'static str {
    "OK"
}

fn display_error(err: &anyhow::Error) -> String {
    tracing::error!("{err:?}");
    format!("An error occured: {err}")
}

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

#[component]
fn TextField(label: String, initial: String, value: Signal<String>) -> Element {
    rsx! {
        div {
            class: "flex flex-row justify-end",
            {label},
            input {
                class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                oninput: move |event| value.set(event.value()),
                initial_value: initial
            }
        }
    }
}

fn app() -> Element {
    let tcp = use_signal(|| false);
    let mut config = use_signal(Config::parse);
    let remotes = use_signal(HashMap::new);
    let mut target = use_signal(|| "".to_string());
    let mut port = use_signal(|| "".to_string());
    let mut volume = use_signal(|| "".to_string());
    let error = use_signal(|| None);
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        div {
            class: "font-mono bg-zinc-50",
            main {
                class: "flex flex-col max-w-100 h-dvh",
                div {
                    class: "flex flex-row",
                    button {
                        class: format!("p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1"),
                        disabled: *tcp.read(),
                        onclick: move |_| tcp.toggle(),
                        "TCP"
                    }
                    button {
                        class: "p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1",
                        disabled: !*tcp.read(),
                        onclick: move |_| tcp.toggle(),
                        "UDP"
                    }
                }
                if *tcp.read() {
                    p {
                        class: "pl-1 pr-1 m-1 w-48 rounded-xs",
                        "Ожидание, адрес: {me}"
                    }
                } else {
                    TextField { "Введите адрес получателя", "127.0.0.1", target }
                    TextField { "Введите порт получателя", "8001", port }
                    TextField { "Введите количество пакетов для отправки", "127.0.0.1", volume }
                    div {
                        class: "m-1 w-48 rounded-xs text-xl",
                        button {
                            class: "hover:bg-zinc-200",
                            disabled: *connected.read(),
                            onclick: move |_| connect(read, write, error, connected, target),
                            p {
                                "Отправить"
                            }
                        }
                    }
                }
                ErrorView { error }
            }
        }
    }
}

fn main() {
    dioxus::logger::init(Level::DEBUG).unwrap();
    dioxus::launch(app);
}
