// SPDX-License-Identifier: GPL-3.0-or-later
//! Where a tutorial callout goes, in art pixels: pure arithmetic, tested on both canvas sizes.
//! Given the rectangle of the thing a callout describes, the callout's text and its two
//! buttons' labels, it wraps the text, sizes the panel and puts it beside the target (never
//! over it, never off the canvas), with a small pointer on the panel's edge that faces the
//! target. The target's own 1 px outline is the caller's to draw; the panel stays clear of it.

use std::fmt;

/// Which side of the target the panel sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Above,
    Below,
    Left,
    Right,
}

// The pointer: a triangle POINTER pixels deep whose base is 2 * HALF_BASE + 1 wide
pub const POINTER: i32 = 5;
pub const HALF_BASE: i32 = 4;
// The target's outline is one pixel outside its rectangle
pub const OUTLINE: i32 = 1;
// The least space left between a panel and the canvas edge
pub const MARGIN: i32 = 3;
// The panel's frame is 4 px of art; text starts this far in from the panel's edge
pub const PAD_X: i32 = 8;
pub const PAD_TOP: i32 = 7;
pub const PAD_BOTTOM: i32 = 5;
// Distance from one line of text's top to the next, and a line's height in ink (the dots
// over an umlaut down to a descender). The pitch leaves a clear row between one line's
// descenders and the next line's accents.
pub const LINE_PITCH: i32 = 12;
pub const LINE_INK: i32 = 12;
// The button row: hit areas this tall (at least 14), the plates drawn inside them
pub const BUTTON_H: i32 = 14;
pub const BUTTON_PAD: i32 = 5;
pub const BUTTON_GAP: i32 = 8;
// Space between the last line of text and the buttons
pub const TEXT_TO_BUTTONS: i32 = 3;
// The widest a panel is allowed to be (the HUD's width less a margin each side)
pub const MAX_PANEL_W: i32 = 210;
// The narrowest panel that is worth drawing beside a target (left and right)
pub const MIN_SIDE_W: i32 = 90;

/// A rectangle in art pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{} {}x{}]", self.x, self.y, self.w, self.h)
    }
}

/// Everything the UI needs to draw one callout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub side: Side,
    /// The panel, frame included.
    pub panel: Rect,
    /// The wrapped text, one line per entry, the first line's top at (text_x, text_y).
    pub lines: Vec<String>,
    pub text_x: i32,
    pub text_y: i32,
    /// The pointer's tip, on the target's outline, and the centre of its base on the panel's edge.
    pub tip_x: i32,
    pub tip_y: i32,
    pub base_x: i32,
    pub base_y: i32,
    /// The hit areas of the two buttons, and where each label's top left goes.
    pub got_it: Rect,
    pub skip: Rect,
    pub got_it_label_x: i32,
    pub got_it_label_y: i32,
    pub skip_label_x: i32,
    pub skip_label_y: i32,
}

/// The text broken into lines no wider than max_width, as `measure` says how wide a piece of
/// text is in art pixels. Breaks fall on spaces; a single word wider than the line is left
/// whole on its own line.
pub fn wrap(text: &str, max_width: i32, measure: &dyn Fn(&str) -> i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let joined = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && measure(&joined) > max_width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = joined;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The usable canvas a callout is placed on: canvas_w wide, its usable height running from
/// top_limit to bottom_limit (the system bars' padding is outside it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Canvas {
    pub w: i32,
    pub top_limit: i32,
    pub bottom_limit: i32,
}

/// Places a callout for a target rectangle. A target in the lower half gets its panel above
/// if there is room, then below, then to a side; one in the upper half the other way round.
/// None only when there is room on no side at all, which the two canvas sizes never have.
pub fn place(
    target: Rect,
    text: &str,
    got_it_label: &str,
    skip_label: &str,
    canvas: Canvas,
    measure: &dyn Fn(&str) -> i32,
) -> Option<Placement> {
    let lower = target.y + target.h / 2 > (canvas.top_limit + canvas.bottom_limit) / 2;
    let order = if lower {
        [Side::Above, Side::Below, Side::Left, Side::Right]
    } else {
        [Side::Below, Side::Above, Side::Right, Side::Left]
    };
    order.into_iter().find_map(|side| {
        try_place(
            side,
            target,
            text,
            got_it_label,
            skip_label,
            canvas,
            measure,
        )
    })
}

fn try_place(
    side: Side,
    t: Rect,
    text: &str,
    got_it_label: &str,
    skip_label: &str,
    canvas: Canvas,
    measure: &dyn Fn(&str) -> i32,
) -> Option<Placement> {
    let Canvas {
        w: canvas_w,
        top_limit,
        bottom_limit,
    } = canvas;
    let gap = OUTLINE + POINTER;
    // The room on this side of the target, as a width and a height the panel may use
    let (max_w, max_h) = match side {
        Side::Above => (
            MAX_PANEL_W.min(canvas_w - 2 * MARGIN),
            t.y - gap - (top_limit + MARGIN),
        ),
        Side::Below => (
            MAX_PANEL_W.min(canvas_w - 2 * MARGIN),
            (bottom_limit - MARGIN) - (t.y + t.h + gap),
        ),
        Side::Left => (
            MAX_PANEL_W.min(t.x - gap - MARGIN),
            bottom_limit - top_limit - 2 * MARGIN,
        ),
        Side::Right => (
            MAX_PANEL_W.min(canvas_w - MARGIN - (t.x + t.w + gap)),
            bottom_limit - top_limit - 2 * MARGIN,
        ),
    };
    if max_w < MIN_SIDE_W {
        return None;
    }

    let got_w = measure(got_it_label) + 2 * BUTTON_PAD;
    let skip_w = measure(skip_label) + 2 * BUTTON_PAD;
    let inner = max_w - 2 * PAD_X;
    let lines = wrap(text, inner, measure);
    let text_w = lines.iter().map(|l| measure(l)).max().unwrap_or(0);
    // One row of buttons if both fit with a gap between, else the two stacked
    let one_row = got_w + BUTTON_GAP + skip_w <= inner;
    let buttons_w = if one_row {
        got_w + BUTTON_GAP + skip_w
    } else {
        got_w.max(skip_w)
    };
    let mut w = max_w.min(text_w.max(buttons_w) + 2 * PAD_X);
    // Space for the pointer's base inside the panel's width
    w = w.max(2 * (PAD_X + HALF_BASE) + 1);
    let button_rows = if one_row { 1 } else { 2 };
    let h = PAD_TOP
        + (lines.len() as i32 - 1) * LINE_PITCH
        + LINE_INK
        + TEXT_TO_BUTTONS
        + button_rows * BUTTON_H
        + PAD_BOTTOM;
    if h > max_h {
        return None;
    }

    let tcx = t.x + t.w / 2;
    let tcy = t.y + t.h / 2;
    let (px, py) = match side {
        Side::Above => (
            clamp(tcx - w / 2, MARGIN, canvas_w - MARGIN - w),
            t.y - gap - h,
        ),
        Side::Below => (
            clamp(tcx - w / 2, MARGIN, canvas_w - MARGIN - w),
            t.y + t.h + gap,
        ),
        Side::Left => (
            t.x - gap - w,
            clamp(tcy - h / 2, top_limit + MARGIN, bottom_limit - MARGIN - h),
        ),
        Side::Right => (
            t.x + t.w + gap,
            clamp(tcy - h / 2, top_limit + MARGIN, bottom_limit - MARGIN - h),
        ),
    };

    // The pointer faces the target at its centre, pulled in to keep its base inside the
    // panel's straight edge, and kept within the target's own span where the target allows
    let (tip_x, tip_y, base_x, base_y) = if matches!(side, Side::Above | Side::Below) {
        let lo = (px + PAD_X).max(t.x);
        let hi = (px + w - PAD_X - 1).min(t.x + t.w - 1);
        let bx = if lo <= hi {
            clamp(tcx, lo, hi)
        } else {
            clamp(tcx, px + PAD_X, px + w - PAD_X - 1)
        };
        if side == Side::Above {
            (bx, t.y - OUTLINE - 1, bx, py + h)
        } else {
            (bx, t.y + t.h + OUTLINE, bx, py - 1)
        }
    } else {
        let lo = (py + PAD_X).max(t.y);
        let hi = (py + h - PAD_X - 1).min(t.y + t.h - 1);
        let by = if lo <= hi {
            clamp(tcy, lo, hi)
        } else {
            clamp(tcy, py + PAD_X, py + h - PAD_X - 1)
        };
        if side == Side::Left {
            (t.x - OUTLINE - 1, by, px + w, by)
        } else {
            (t.x + t.w + OUTLINE, by, px - 1, by)
        }
    };

    // The buttons: Got it at the left, Skip at the right, or stacked at the left
    let row_y = py + h - PAD_BOTTOM - button_rows * BUTTON_H;
    let left_x = px + PAD_X;
    let got_it = Rect::new(left_x, row_y, got_w, BUTTON_H);
    let skip = if one_row {
        Rect::new(px + w - PAD_X - skip_w, row_y, skip_w, BUTTON_H)
    } else {
        Rect::new(left_x, row_y + BUTTON_H, skip_w, BUTTON_H)
    };
    Some(Placement {
        side,
        panel: Rect::new(px, py, w, h),
        lines,
        text_x: px + PAD_X,
        text_y: py + PAD_TOP,
        tip_x,
        tip_y,
        base_x,
        base_y,
        got_it,
        skip,
        got_it_label_x: got_it.x + BUTTON_PAD,
        got_it_label_y: got_it.y,
        skip_label_x: skip.x + BUTTON_PAD,
        skip_label_y: skip.y,
    })
}

fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    // Not i32::clamp: that panics when lo > hi, where this (as the old Math.max/min pair)
    // lets the low bound win
    lo.max(hi.min(v))
}
