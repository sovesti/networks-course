use dioxus::html::geometry::{ElementSpace, euclid::Point2D};
use dioxus::prelude::*;
use dioxus_core::Element;
use rkyv::{Archive, Deserialize, Serialize};
use tokio::sync::broadcast::channel;
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::broadcast::{Receiver, Sender},
};

use crate::error::ErrorView;
use crate::streaming::{read_from_socket, write_to_socket};

const WIDTH: u16 = 3;

type HtmlPoint = Point2D<f64, ElementSpace>;

#[derive(Archive, Serialize, Deserialize, Clone, Copy, Debug)]
struct Point {
    x: u16,
    y: u16,
}

impl Point {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl From<HtmlPoint> for Point {
    fn from(point: HtmlPoint) -> Self {
        let point = point.round();
        Self::new(point.x as u16, point.y as u16)
    }
}

#[derive(Archive, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Line {
    from: Point,
    to: Point,
    width: u16,
}

impl Line {
    fn new(from: Point, to: Point, width: u16) -> Self {
        Self { from, to, width }
    }

    fn shader(&self) -> String {
        format!(
            include_str!("canvas.js"),
            canvas_id(),
            self.width,
            self.from.x,
            self.from.y,
            self.to.x,
            self.to.y
        )
    }
}

#[derive(Clone, Copy)]
struct DrawingState {
    held: Signal<bool>,
    previous: Signal<Option<HtmlPoint>>,
}

impl DrawingState {
    fn new() -> Self {
        Self {
            held: use_signal(|| false),
            previous: use_signal(|| None),
        }
    }

    fn hold(&mut self) {
        self.held.set(true);
    }

    fn release(&mut self) {
        self.held.set(false);
        self.previous.set(None);
    }

    fn held(&self) -> bool {
        *self.held.read()
    }

    fn line(&mut self, next: HtmlPoint) -> Option<Line> {
        if !self.held() {
            return None;
        }
        let previous = self.previous.write().replace(next);
        previous.map(|previous| Line::new(previous.into(), next.into(), WIDTH))
    }
}

#[component]
pub fn Canvas(
    mut read: Signal<Option<OwnedReadHalf>>,
    mut write: Signal<Option<OwnedWriteHalf>>,
) -> Element {
    let error = use_signal(|| None);
    let mut state = DrawingState::new();
    draw_from_socket(read, error);
    let tx = draw_from_user(write, error);
    rsx! {
        div {
            class: "flex-1 w-full h-full outline-1",
            onmousedown: move |_| state.hold(),
            onmouseup: move |_| state.release(),
            onmousemove: move |evt| draw(error, tx.clone(), evt, state),
            canvas {
                class: "w-full h-full",
                id: canvas_id()
            }
        }
        ErrorView { error }
    }
}

fn draw_from_socket(read: Signal<Option<OwnedReadHalf>>, error: Signal<Option<anyhow::Error>>) {
    let (tx, rx) = channel::<Line>(32);
    spawn(read_from_socket(read, error, tx.clone()));
    spawn(draw_on_canvas(rx));
}

fn draw_from_user(
    write: Signal<Option<OwnedWriteHalf>>,
    error: Signal<Option<anyhow::Error>>,
) -> Sender<Line> {
    let (tx, rx) = channel::<Line>(32);
    spawn(write_to_socket(write, error, tx.subscribe()));
    spawn(draw_on_canvas(rx));
    tx
}

async fn draw_on_canvas(mut rx: Receiver<Line>) {
    while let Ok(msg) = rx.recv().await {
        document::eval(&msg.shader());
    }
}

fn canvas_id() -> String {
    "canvas".to_string()
}

async fn draw(
    mut error: Signal<Option<anyhow::Error>>,
    tx: Sender<Line>,
    evt: Event<MouseData>,
    mut state: DrawingState,
) {
    if let Some(line) = state.line(evt.element_coordinates()) {
        if let Err(err) = tx.send(line) {
            error.set(Some(err.into()));
        }
    }
}
