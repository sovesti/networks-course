mod subnet;
mod measure;

use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use dioxus::{
    logger::tracing::{self, Level},
    prelude::*,
};
use serde::Deserialize;
use tokio::time::{sleep_until, Instant};

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

    fn me(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), self.port)
    }
}

async fn subnet_loop(mut subnet: Subnet, config: Signal<Config>) -> anyhow::Result<()> {
    let mut sent = Instant::now();
    loop {
        tokio::select! {
            _ = sleep_until(sent + Duration::from_millis(config.read().interval)) => {
                sent = Instant::now();
                subnet.prune(config.read().timeout);
                subnet.broadcast(Message::Running).await
            }
            result = subnet.handle_message() => result
        }?
    }
}

async fn spawn_subnet(
    config: Signal<Config>,
    copies: KnownCopies,
    mut error: Signal<Option<anyhow::Error>>,
) -> anyhow::Result<()> {
    let subnet = Subnet::setup(config.read().me(), copies).await?;
    spawn(async move {
        if let Err(err) = subnet_loop(subnet, config).await {
            error.set(Some(err));
        }
    });
    Ok(())
}

fn subnet_action(
    config: Signal<Config>,
    remotes: KnownCopies,
    error: Signal<Option<anyhow::Error>>,
) -> Action<(), ()> {
    use_action(move || spawn_subnet(config, remotes, error))
}

fn my_address(config: Config) -> Option<SocketAddr> {
    my_ip().ok().map(|ip| SocketAddr::new(ip, config.port))
}

fn app() -> Element {
    let mut config = use_signal(Config::parse);
    let me = my_address(config.cloned());
    let remotes = use_signal(HashMap::new);
    let error = use_signal(|| None);
    subnet_action(config.clone(), remotes.clone(), error.clone()).call();
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        div {
            class: "font-mono bg-zinc-50",
            main {
                class: "flex flex-col max-w-100 h-dvh",
                p {
                    "Копий запущено: {remotes.len()}"
                }
                div {
                    class: "flex flex-row justify-start items-center",
                    "Ожидание, мс:"
                    input {
                        class: "outline-1 pl-1 pr-1 m-1 max-w-48 rounded-xs",
                        type: "number",
                        oninput: move |event| config.write().timeout = event.value().parse().unwrap(),
                        initial_value: config.read().timeout.to_string()
                    }
                }
                div {
                    class: "flex m-1 h-full outline-1 flex-col overflow-auto rounded-xs",
                    for addr in remotes.read().keys()
                    {
                        div {
                            p {
                                if Some(addr) == me.as_ref() {
                                    {addr.to_string() + " (текущая копия)"}
                                } else {
                                    {addr.to_string()}
                                }
                            }
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

fn main() {
    dioxus::logger::init(Level::DEBUG).unwrap();
    dioxus::launch(app);
}
