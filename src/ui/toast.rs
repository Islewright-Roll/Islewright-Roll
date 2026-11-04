// SPDX-License-Identifier: GPL-3.0-or-later
//! A short message over everything that goes by itself and takes no taps: what Android's Toast
//! was to the Java app, which Day has no call for.

use std::cell::Cell;

use day::prelude::*;

use super::metrics::{Metrics, metrics};
use super::paint::{self, kit};
use super::text::{Align, Block, Face};
use super::widget::{self, W};

/// How long a message shows, as Toast.LENGTH_SHORT and LENGTH_LONG.
const SHORT_MS: u32 = 2000;
const LONG_MS: u32 = 3500;

/// The message showing, with its id so a later one's timer leaves it alone.
type Shown = Signal<Option<(u64, String)>>;

thread_local! {
    static SHOWN: Cell<Option<Shown>> = const { Cell::new(None) };
    static NEXT: Cell<u64> = const { Cell::new(1) };
}

fn shown() -> Shown {
    SHOWN.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global(None);
            c.set(Some(s));
            s
        }
    })
}

/// Shows `text` for a moment, in place of any message showing.
pub fn show(text: impl Into<String>, long: bool) {
    let text = text.into();
    announce(&text);
    let id = NEXT.with(|n| n.replace(n.get() + 1));
    shown().set(Some((id, text)));
    day::task(async move {
        day::sleep(if long { LONG_MS } else { SHORT_MS }).await;
        if shown().get_untracked().is_some_and(|(now, _)| now == id) {
            shown().set(None);
        }
    });
}

/// The message showing, low on the screen and centered.
pub fn layer() -> impl Piece {
    keyed(
        move || shown().get().map(|(id, s)| (id, s, metrics())),
        |(_, s, m)| view(s, m),
    )
}

fn view(s: &str, m: &Metrics) -> AnyPiece {
    let k = kit();
    let width = (m.width - m.safe.left - m.safe.right - 32).max(64);
    let block = Block::new(&k.body, Face::Body, m.t, s, width - 16, Align::Center);
    let w = (block.line_width(&k.body) + 16).min(width);
    let h = block.height + 8;
    let mut lines = block.clone();
    lines.width = w - 16;
    let frame = W::new(
        widget::art(m, w, h, move |img| {
            paint::frame(img, &k.panel_dark, 0, 0, w, h);
            lines.draw(&k.body, img, 8, 4, paint::BUTTON_TEXT);
        }),
        w,
        h,
    );
    let x = m.safe.left + (m.width - m.safe.left - m.safe.right - w) / 2;
    let y = m.height - m.safe.bottom - h - 32;
    frame
        .piece
        .padding(Insets::new(m.pt(y), m.pt(x), 0.0, 0.0))
        .any()
}
