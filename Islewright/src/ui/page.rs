// SPDX-License-Identifier: GPL-3.0-or-later
//! A full-window page in the kit's look: dark wood all round, the page's title and a brass Done
//! button across the top, and a parchment panel holding content that scrolls. Pages are shown in
//! a Day cover, so the platform keeps them modal and Android's back button closes them; Escape
//! does too.

use std::rc::Rc;

use day::prelude::*;

use super::metrics::Metrics;
use super::paint::{self, kit};
use super::widget::{self, Button, Style, W};

/// The wood showing round the parchment, and the parchment's own padding.
const EDGE: i32 = 4;
const PAD: i32 = 8;
/// The widest the content gets at default text, so lines stay readable on a wide window.
const MAX_CONTENT_W: i32 = 320;
/// Points the scroll view reaches past its clip on the right, where the platform draws its
/// scroll bar: the bar is cut off, so no system widget shows on the parchment. Scrolling itself
/// (wheel, touch, keyboard, screen reader) is the platform's own.
const BAR_ROOM: f64 = 24.0;

/// The page's background color, for the cover behind it.
pub fn wood() -> Color {
    let c = kit().wood();
    Color::rgb(
        ((c >> 16) & 0xFF) as f64 / 255.0,
        ((c >> 8) & 0xFF) as f64 / 255.0,
        (c & 0xFF) as f64 / 255.0,
    )
}

/// `content` in a vertical scroll view `width` x `height` art pixels, its scroll bar clipped.
pub fn scroller(m: &Metrics, content: W, width: i32, height: i32) -> W {
    let (wp, hp) = (m.pt(width), m.pt(height));
    let view = scroll(content.piece).frame(wp + BAR_ROOM, hp);
    let clip = zstack((view,))
        .align(Alignment::TopLeading)
        .frame(wp, hp)
        .corner_radius(0.0);
    W::new(clip, width, height)
}

/// The width a page's content gets in this window.
pub fn content_width(m: &Metrics) -> i32 {
    let inner = m.width - m.safe.left - m.safe.right - 2 * EDGE - 2 * PAD;
    let cap = if m.t > 1 { inner } else { MAX_CONTENT_W };
    inner.min(cap).max(32)
}

/// A page: `title` over the content `body` builds for the width it is given, named `id` for
/// scripts. `done` closes it, from the Done button and from Escape.
pub fn page(
    m: &Metrics,
    id: ElementId,
    title: &str,
    done_label: &str,
    done: Rc<dyn Fn()>,
    body: impl FnOnce(i32) -> Vec<W>,
) -> AnyPiece {
    {
        let done = done.clone();
        widget::on_escape(move || done());
    }
    let (left, top) = (m.safe.left, m.safe.top);
    let right = m.width - m.safe.right;
    let bottom = m.height - m.safe.bottom;

    // A screen reader hears which page opened, and the keyboard lands on its Done button, so
    // Escape and Enter work at once and Tab goes on into the page.
    announce(title);
    let done_button = {
        let done = done.clone();
        Button::new(day::id!(page_done), done_label)
            .autofocus(true)
            .build(m, move || done())
    };
    let title_w = (right - left - 2 * EDGE - done_button.w - 2 * EDGE).max(16);
    // Large text beside a large Done button can leave the title too little room: it steps down
    // a whole size at a time until it fits on one line, rather than breaking inside a word
    let display = &kit().display;
    let ink = display.ink_width(&super::text::printable(title));
    let fit = (1..=m.t).rev().find(|t| ink * t <= title_w).unwrap_or(1);
    let title = widget::text(
        &Metrics { t: fit, ..*m },
        None,
        title_w,
        title,
        Style::WOOD_TITLE,
    );
    let header_h = title.h.max(done_button.h);
    let title_top = (header_h - title.h) / 2;
    let title = widget::vstack(m, vec![widget::gap(m, title_w, title_top), title], 0);
    let header = widget::hstack(
        m,
        vec![
            title,
            widget::gap(m, right - left - 2 * EDGE - title_w - done_button.w, 1),
            done_button,
        ],
        0,
    );

    // The parchment below the header, and the content inside it
    let (px, py) = (left + EDGE, top + EDGE + header_h + EDGE);
    let (pw, ph) = (right - left - 2 * EDGE, (bottom - EDGE - py).max(32));
    let cw = content_width(m);
    let mut items = body(cw);
    items.push(widget::gap(m, cw, PAD));
    let content = widget::vstack(m, items, 4);
    let inner_w = pw - 2 * PAD;
    let content = widget::center(m, content, inner_w);
    // The view reaches down to the parchment's frame, so text scrolls out under its edge
    let view_h = ph - PAD - 4;
    let view = scroller(m, content, inner_w, view_h);

    let parchment = kit().panel.clone();
    let panel = widget::art(m, pw, ph, move |img| {
        paint::frame(img, &parchment, 0, 0, pw, ph)
    });
    let wood_color = wood();
    let background = canvas(move |d, size| {
        d.fill(
            Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
            wood_color,
        )
    })
    .a11y(|a| a.hidden())
    .grow();
    let at = |x: i32, y: i32| Insets::new(m.pt(y), m.pt(x), 0.0, 0.0);
    zstack((
        background,
        panel.padding(at(px, py)),
        header.piece.padding(at(left + EDGE, top + EDGE)),
        view.piece.padding(at(px + PAD, py + PAD)),
    ))
    .align(Alignment::TopLeading)
    .id(id)
    .grow()
    .any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_keeps_a_readable_width() {
        let phone = Metrics::new(412.0, 892.0, 2.625, Insets::ZERO, 1.0);
        assert_eq!(content_width(&phone), 216 - 2 * EDGE - 2 * PAD);
        let wide = Metrics::new(892.0, 412.0, 2.625, Insets::ZERO, 1.0);
        assert_eq!(content_width(&wide), MAX_CONTENT_W);
        let big = Metrics::new(892.0, 412.0, 2.625, Insets::ZERO, 2.0);
        assert!(content_width(&big) > MAX_CONTENT_W);
    }
}
