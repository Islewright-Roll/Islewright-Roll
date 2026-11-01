// SPDX-License-Identifier: GPL-3.0-or-later
//! The pixel-art widget kit every dialog and page is built from, so nothing the player sees is
//! in the platform's default style.
//!
//! A canvas is one accessible element to Day, so the kit does not draw a whole page into one:
//! each widget is its own canvas that both shows sharp pixel art (the game's button and panel
//! art and its bitmap fonts, at a whole number of device pixels per art pixel) and is a real
//! accessible element with a role, a label, a value where it has one, a stable id, keyboard
//! focus and Enter and Space to activate it. Layout and scrolling are Day's own containers.
//!
//! - `metrics`: the art pixel and the text size on this screen, large text included.
//! - `text`: wrapping and drawing the bitmap fonts.
//! - `paint`: the frames, boxes and focus ring.
//! - `widget`: text, buttons, check boxes, radio buttons, the stepper, links, pictures.
//! - `dialog`: overlays over the game, with an awaitable `ask`.
//! - `page`: full-window pages for Day covers.

pub mod dialog;
pub mod metrics;
pub mod page;
pub mod paint;
pub mod text;
pub mod widget;

use day::prelude::*;

pub use dialog::{Dialog, ask, confirm};
pub use metrics::{Metrics, metrics};
pub use paint::Icon;
pub use widget::{Button, Style, W};

/// The app's content with the kit's layers round it: the probes that measure the window and the
/// text size beneath, the dialog layer above.
pub fn host(content: impl Piece) -> impl Piece {
    zstack((metrics::probes(), content, dialog::layer())).align(Alignment::TopLeading)
}
