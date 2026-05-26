mod error;
mod random;

use std::{
    fs::File,
    io::Read,
    net::{SocketAddr, ToSocketAddrs},
};

use anyhow::anyhow;
use dioxus::{logger::tracing::{self, Level}, prelude::*};
use serde::Deserialize;

use crate::{error::ErrorView, random::RandomTraffic};

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
    value.set(initial.clone());
    rsx! {
        div {
            class: "flex flex-row justify-between",
            p {
                class: "p-1",
                {label},
            }
            input {
                class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                oninput: move |event| value.set(event.value()),
                initial_value: initial
            }
        }
    }
}

fn parse_target(port: Signal<String>, target: Signal<String>) -> anyhow::Result<SocketAddr> {
    let port = port().parse()?;
    (target().to_string(), port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("couldn't resolve {}:{}", target(), port))
}

fn empty_string() -> Signal<String> {
    use_signal(|| "".to_string())
}

async fn send_random(
    config: Signal<Config>,
    tcp: Signal<bool>,
    target: anyhow::Result<SocketAddr>,
    volume: anyhow::Result<usize>,
) -> anyhow::Result<()> {
    let mut traffic = RandomTraffic::new(32 * 1024);
    tracing::debug!("{volume:?}");
    if tcp() {
        traffic.send_tcp(target?, volume?).await
    } else {
        traffic.send_udp(config.read().port, target?, volume?).await
    }
}

fn app() -> Element {
    let mut tcp = use_signal(|| false);
    let config = use_signal(Config::parse);
    let target: Signal<String> = empty_string();
    let port: Signal<String> = empty_string();
    let volume: Signal<String> = empty_string();
    let mut send = use_action(move || {
        send_random(
            config,
            tcp,
            parse_target(port, target),
            volume().parse().map_err(anyhow::Error::from),
        )
    });
    let error = use_signal(|| None);
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        div {
            class: "font-mono bg-zinc-50",
            main {
                class: "flex flex-col max-w-150 h-dvh",
                div {
                    class: "flex flex-row",
                    button {
                        class: format!("p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1"),
                        disabled: tcp(),
                        onclick: move |_| tcp.toggle(),
                        "TCP"
                    }
                    button {
                        class: "p-1 m-1 rounded-md hover:bg-zinc-200 disabled:bg-zinc-300 disabled:outline-1",
                        disabled: !tcp(),
                        onclick: move |_| tcp.toggle(),
                        "UDP"
                    }
                }
                TextField { label: "Введите адрес получателя", initial: "127.0.0.1", value: target }
                TextField { label: "Введите порт получателя", initial: "8001", value: port }
                TextField { label: "Введите количество пакетов для отправки", initial: "5", value: volume }
                div {
                    class: "m-1 w-48 rounded-xs text-xl",
                    button {
                        class: "hover:bg-zinc-200",
                        disabled: send.pending(),
                        onclick: move |_| send.call(),
                        p {
                            class: "p-1",
                            "Отправить"
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
