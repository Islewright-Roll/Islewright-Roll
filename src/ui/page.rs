// SPDX-License-Identifier: GPL-3.0-or-later
//! A full-window page in the kit's look: the panel from edge to edge, a centered title over a
//! rule, content that scrolls, and an OK button across the bottom (rules.xml, about.xml,
//! high_scores.xml). Pages are shown in a Day cover, so the platform keeps them modal and
//! Android's back button closes them; Escape does too.

use std::rc::Rc;

use day::prelude::*;

use super::metrics::Metrics;
use super::paint::{self, kit, palette};
use super::widget::{self, Button, Style, W};

/// The panel's padding, all round.
const PAD: i32 = 14;
/// The widest the content gets at default text, so lines stay readable on a wide window.
const MAX_CONTENT_W: i32 = 320;
/// Points the scroll view reaches past its clip on the right, where the platform draws its
/// scroll bar: a view with no thumb has it cut off.
const BAR_ROOM: f64 = 24.0;

/// What shows round a page: under Android's navigation bar, what the bar's scrim made of the
/// panel beneath it in the Java app.
pub fn surround() -> Color {
    let (r, g, b) = surround_rgb();
    Color::rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

/// `surround`'s colour in bytes, for the window bridge to dim it.
pub fn surround_rgb() -> (u8, u8, u8) {
    if palette().dark {
        (34, 24, 13)
    } else {
        (72, 61, 45)
    }
}

/// `content` in a vertical scroll view `width` x `height` art pixels. With `thumb` it shows the
/// kit's scroll thumb, which the app's theme gives every scroll view at its right edge.
pub fn scroller(m: &Metrics, content: W, width: i32, height: i32, thumb: bool) -> W {
    let (wp, hp) = (m.pt(width), m.pt(height));
    let room = if thumb { 0.0 } else { BAR_ROOM };
    if thumb {
        super::window::scroll_thumb(palette().dark);
    }
    let view = scroll(content.piece).frame(wp + room, hp);
    let clip = zstack((view,))
        .align(Alignment::TopLeading)
        .frame(wp, hp)
        .corner_radius(0.0);
    W::new(clip, width, height)
}

/// The width a page's content gets in this window.
pub fn content_width(m: &Metrics) -> i32 {
    let inner = m.width - m.safe.left - m.safe.right - 2 * PAD;
    let cap = if m.t > 1 { inner } else { MAX_CONTENT_W };
    inner.min(cap).max(32)
}

/// How a page's content fills the room over its OK button.
pub enum Fill {
    /// Scrolls, showing the thumb or not, 8 art pixels clear of the button.
    Scroll { thumb: bool },
    /// Takes exactly the room there is, right down to the button, and scrolls, with the thumb,
    /// only when the content is taller than that (Top Scores on a tablet held sideways).
    Exact,
}

/// A page: `title` over the content `body` builds for the width and height it is given, named
/// `id` for scripts. `done` closes it, from the OK button and from Escape.
#[allow(clippy::too_many_arguments)]
pub fn page(
    m: &Metrics,
    id: ElementId,
    title: &str,
    ok_label: &str,
    done: Rc<dyn Fn()>,
    fill: Fill,
    body: impl FnOnce(i32, i32) -> W,
) -> AnyPiece {
    {
        let done = done.clone();
        widget::on_escape(move || done());
    }
    let (left, top) = (m.safe.left, m.safe.top);
    let right = m.width - m.safe.right;
    let bottom = m.height - m.safe.bottom;

    // A screen reader hears which page opened, and the keyboard lands on its OK button, so
    // Escape and Enter work at once and Tab goes on into the page.
    announce(title);
    let pw = right - left;
    let inner = pw - 2 * PAD;
    let ok = {
        let done = done.clone();
        Button::new(day::id!(page_done), ok_label)
            .width(inner)
            .autofocus(true)
            .build(m, move || done())
    };
    let heading = widget::text(m, None, inner, title, Style::TITLE);
    let head_h = heading.h + 2 + 1 + 6;
    let head = widget::vstack(
        m,
        vec![
            heading,
            widget::gap(m, inner, 2),
            widget::rule(m, inner),
            widget::gap(m, inner, 6),
        ],
        0,
    );

    let ok_y = bottom - PAD - ok.h;
    let cw = content_width(m);
    let room = ok_y - (top + PAD + head_h);
    let center = |content: W| widget::center(m, content, inner);
    let view = match fill {
        Fill::Scroll { thumb } => {
            let h = room - 8;
            scroller(m, center(body(cw, h)), inner, h, thumb)
        }
        Fill::Exact => {
            let content = body(cw, room);
            if content.h > room {
                scroller(m, center(content), inner, room - 8, true)
            } else {
                center(content)
            }
        }
    };

    // On a phone the panel runs on under the navigation bar: its bottom edge never shows
    let under = if cfg!(target_os = "android") && m.height > m.width {
        2 * paint::CORNER
    } else {
        0
    };
    let ph = bottom - top;
    let k = kit();
    // A share of a pixel left over at the foot of the root belongs to the panel too
    let foot = if under > 0 { 1 } else { 0 };
    let panel = widget::art(m, pw, ph + foot, move |img| {
        paint::frame(img, k.panel_art(), 0, 0, pw, ph + under)
    });
    let at = |x: i32, y: i32| Insets::new(m.pt(y), m.pt(x), 0.0, 0.0);
    zstack((
        panel.padding(at(left, top)),
        head.piece.padding(at(left + PAD, top + PAD)),
        view.piece.padding(at(left + PAD, top + PAD + head_h)),
        ok.piece.padding(at(left + PAD, ok_y)),
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
        assert_eq!(content_width(&phone), 216 - 2 * PAD);
        let wide = Metrics::new(892.0, 412.0, 2.625, Insets::ZERO, 1.0);
        assert_eq!(content_width(&wide), MAX_CONTENT_W);
        let big = Metrics::new(892.0, 412.0, 2.625, Insets::ZERO, 2.0);
        assert!(content_width(&big) > MAX_CONTENT_W);
    }
}
