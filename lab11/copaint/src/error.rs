use anyhow::Error;
use dioxus::{logger::tracing, prelude::*};

#[component]
pub fn ErrorView(error: Signal<Option<Error>>) -> Element {
    rsx! {
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

fn display_error(err: &anyhow::Error) -> String {
    tracing::error!("{err:?}");
    format!("An error occured: {err}")
}

fn success() -> &'static str {
    "OK"
}
