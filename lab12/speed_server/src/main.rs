mod error;
mod measure;

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::anyhow;
use dioxus::{logger::tracing::Level, prelude::*};

use crate::{
    error::ErrorView,
    measure::{MeasureTraffic, MeasuringState},
};

#[component]
fn TextField(label: String, initial: String, value: Signal<String>) -> Element {
    value.set(initial.clone());
    rsx! {
        div {
            class: "flex flex-row justify-between",
            {label},
            input {
                class: "outline-1 pl-1 pr-1 m-1 w-48 rounded-xs",
                oninput: move |event| value.set(event.value()),
                initial_value: initial
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
struct AcceptingRequest {
    target: Signal<String>,
    port: Signal<String>,
}

impl AcceptingRequest {
    fn new() -> Self {
        Self::default()
    }

    fn port(&self) -> anyhow::Result<u16> {
        Ok(self.port.read().parse()?)
    }

    fn target(&self) -> anyhow::Result<SocketAddr> {
        let port = self.port()?;
        (self.target.read().to_string(), port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("couldn't resolve {}:{}", self.target.read(), port))
    }
}

async fn measure(
    tcp: Signal<bool>,
    target: anyhow::Result<SocketAddr>,
    state: MeasuringState,
) -> anyhow::Result<()> {
    let mut measure = MeasureTraffic::new(state);
    if tcp() {
        measure.measure_tcp(target?).await
    } else {
        measure.measure_udp(target?).await
    }
}

fn scale(bytes: Signal<usize>) -> f64 {
    (bytes() as f64) / 1024.0
}

fn app() -> Element {
    let mut tcp = use_signal(|| false);
    let request = AcceptingRequest::new();
    let bps = use_signal(|| 0);
    let received = use_signal(|| 0);
    let packets = use_signal(|| 0);
    let total = use_signal(|| 0);
    let state = MeasuringState::new(bps, received, packets, total);
    let mut accept = use_action(move || measure(tcp, request.target(), state));
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
                TextField { label: "Введите адрес получения", initial: "127.0.0.1", value: request.target }
                TextField { label: "Введите порт получения", initial: "8001", value: request.port }
                p {
                    "Скорость передачи: {scale(bps):.1} kb/s"
                }
                p {
                    if tcp() {
                        "Получено данных: {scale(received) / 1024.0:.1} mb of {scale(total) / 1024.0:.1} mb"
                    } else {
                        "Получено пакетов: {packets()} of {total()}"
                    }
                }
                div {
                    class: "m-1 w-48 rounded-xs text-xl",
                    button {
                        class: "hover:bg-zinc-200",
                        disabled: accept.pending(),
                        onclick: move |_| accept.call(),
                        p {
                            class: "p-1",
                            "Получить"
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
