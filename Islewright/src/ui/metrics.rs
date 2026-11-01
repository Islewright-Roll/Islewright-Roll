// SPDX-License-Identifier: GPL-3.0-or-later
//! How big an art pixel is, and how big the text is, on this screen.
//!
//! An art pixel is a whole number of device pixels (`k`), chosen from the screen the same way the
//! game screen chooses its scale, but from the screen's short and long sides rather than its
//! width and height, so a page keeps its size when the phone turns. Text is drawn at `t` art
//! pixels per font pixel. Day has no call that reports the accessibility text size, so the kit
//! measures it: a hidden native label in the body text style is laid out by the platform at the
//! user's text size, and its height against the same text measured at the style's nominal 17
//! points is the user's text scale. When that is larger, the kit trades a smaller art pixel for
//! more of them per font pixel, so text grows by whole device pixels and stays sharp.

use std::cell::Cell;

use day::prelude::*;

use crate::art::{DESIGN_W, MIN_H};

/// Insets in whole art pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Edges {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

/// The sizes everything in the kit is laid out from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Device pixels per art pixel.
    pub k: i32,
    /// Art pixels per font pixel: 1, or more for large text.
    pub t: i32,
    /// Device pixels per point.
    pub dpr: f64,
    /// The window in whole art pixels, and the part of it the system bars cover.
    pub width: i32,
    pub height: i32,
    pub safe: Edges,
}

impl Metrics {
    /// The metrics of a window `w` x `h` points at `dpr` device pixels per point, with `safe`
    /// insets in points, for a user text scale of `ratio` (1 is the platform's default).
    pub fn new(w: f64, h: f64, dpr: f64, safe: Insets, ratio: f64) -> Metrics {
        let dpr = if dpr > 0.0 { dpr } else { 1.0 };
        let (dw, dh) = ((w * dpr).floor() as i32, (h * dpr).floor() as i32);
        let base = art_scale(dw, dh);
        let (k, t) = text_scales(base, ratio);
        let up = |pts: f64| ((pts * dpr) / k as f64).ceil().max(0.0) as i32;
        Metrics {
            k,
            t,
            dpr,
            width: dw / k,
            height: dh / k,
            safe: Edges {
                top: up(safe.top),
                left: up(safe.leading),
                bottom: up(safe.bottom),
                right: up(safe.trailing),
            },
        }
    }

    /// Art pixels as points: exactly `k` device pixels each.
    pub fn pt(&self, art: i32) -> f64 {
        art as f64 * self.k as f64 / self.dpr
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics::new(412.0, 892.0, 1.0, Insets::ZERO, 1.0)
    }
}

/// The base art pixel for a screen of `w` x `h` device pixels: the largest whole number at
/// which the design's 216 x 352 art pixels fit the short and long sides.
pub fn art_scale(w: i32, h: i32) -> i32 {
    let (short, long) = (w.min(h), w.max(h));
    (short / DESIGN_W as i32).min(long / MIN_H as i32).max(1)
}

/// Below this text scale the platform's default sizes differ more than any user chose, so it
/// counts as the default.
const LARGE_TEXT: f64 = 1.15;

/// The art pixel `k` and the text enlargement `t` for a base art pixel and a user text scale.
/// Text then is `k * t` device pixels per font pixel: the whole number nearest `base * ratio`,
/// never smaller than `base`, with the largest art pixel that gets there, so frames shrink only
/// as much as the text needs room.
pub fn text_scales(base: i32, ratio: f64) -> (i32, i32) {
    let base = base.max(1);
    if !ratio.is_finite() || ratio < LARGE_TEXT {
        return (base, 1);
    }
    let want = base as f64 * ratio.min(4.0);
    let mut best = (base, 1);
    let mut best_err = f64::MAX;
    for k in (1..=base).rev() {
        for t in 2..=4 {
            if k * t < base {
                continue;
            }
            let err = ((k * t) as f64 - want).abs();
            if err < best_err - 1e-9 {
                best = (k, t);
                best_err = err;
            }
        }
    }
    best
}

thread_local! {
    /// The window in points, as the probe last measured it.
    static WINDOW: Cell<Option<Signal<(f64, f64)>>> = const { Cell::new(None) };
    /// The body text style's line height in points, as the probe last measured it.
    static BODY_LINE: Cell<Option<Signal<f64>>> = const { Cell::new(None) };
}

fn window_signal() -> Signal<(f64, f64)> {
    WINDOW.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global((0.0, 0.0));
            c.set(Some(s));
            s
        }
    })
}

fn body_line_signal() -> Signal<f64> {
    BODY_LINE.with(|c| match c.get() {
        Some(s) => s,
        None => {
            let s = Signal::global(0.0);
            c.set(Some(s));
            s
        }
    })
}

/// Device pixels per point, from the primary display (the canvases are drawn for it).
pub fn device_scale() -> f64 {
    let monitors = day::monitors();
    monitors
        .iter()
        .find(|m| m.primary)
        .or(monitors.first())
        .map(|m| m.scale)
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0)
}

/// The sample both text measurements use.
const SAMPLE: &str = "Mg";
/// The body style's nominal size on the platforms Day targets.
const BODY_PT: f64 = 17.0;

/// The user's text scale: the probe's line against the same sample at the nominal size.
fn text_ratio(line: f64) -> f64 {
    if line <= 0.0 {
        return 1.0;
    }
    let nominal = day::measure_text(SAMPLE, BODY_PT, &CanvasFont::default()).height;
    if nominal > 0.0 { line / nominal } else { 1.0 }
}

/// The current metrics. Tracked: a piece built from them rebuilds when the window turns or
/// resizes, or the text size changes. Before the probes have measured anything, the window is
/// taken as a phone's.
pub fn metrics() -> Metrics {
    let (w, h) = window_signal().get();
    let line = body_line_signal().get();
    let (w, h) = if w > 0.0 && h > 0.0 {
        (w, h)
    } else {
        (412.0, 892.0)
    };
    Metrics::new(w, h, device_scale(), day::safe_area(), text_ratio(line))
}

/// Sets a probe's signal after the current layout pass, not inside it.
fn report<T: PartialEq + 'static>(signal: Signal<T>, value: T) {
    if signal.with_untracked(|v| *v != value) {
        day::task(async move { signal.set_if_changed(value) });
    }
}

/// The probes: a canvas that fills the window and reports its size, and a hidden body text label
/// that reports its line height. Both sit under everything else and are hidden from screen
/// readers; they draw nothing.
pub fn probes() -> impl Piece {
    let window = window_signal();
    let line = body_line_signal();
    zstack((
        canvas(move |_, size| report(window, (size.width, size.height))).grow(),
        label(SAMPLE)
            .font(Font::Body)
            .single_line()
            .overlay(canvas(move |_, size| report(line, size.height)))
            .opacity(0.0)
            .a11y(|a| a.hidden()),
    ))
    .a11y(|a| a.hidden())
    .grow()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_ignores_which_way_the_phone_is_turned() {
        // A 412 x 892 point phone at 2.625: 1081 x 2341 device pixels
        assert_eq!(art_scale(1081, 2341), 5);
        assert_eq!(art_scale(2341, 1081), 5);
        // A desktop browser window
        assert_eq!(art_scale(960, 640), 2);
        assert_eq!(art_scale(100, 100), 1);
    }

    #[test]
    fn default_text_keeps_the_base_scale() {
        assert_eq!(text_scales(5, 1.0), (5, 1));
        assert_eq!(text_scales(5, 0.76), (5, 1));
        assert_eq!(text_scales(5, 1.1), (5, 1));
        assert_eq!(text_scales(5, f64::NAN), (5, 1));
    }

    #[test]
    fn large_text_grows_by_whole_device_pixels() {
        // 1.3 times 5 is 6.5: three art pixels of two device pixels each is 6
        assert_eq!(text_scales(5, 1.3), (3, 2));
        // Twice the text: the full art pixel, doubled
        assert_eq!(text_scales(5, 2.0), (5, 2));
        assert_eq!(text_scales(5, 3.0), (5, 3));
        // Never smaller than the default text
        for r in [1.15, 1.2, 1.5, 2.5, 3.5, 10.0] {
            for base in 1..=6 {
                let (k, t) = text_scales(base, r);
                assert!(k * t >= base && k <= base && t >= 1, "{base} {r}");
            }
        }
    }

    #[test]
    fn metrics_count_whole_art_pixels() {
        let safe = Insets::new(24.0, 0.0, 16.0, 0.0);
        let m = Metrics::new(412.0, 892.0, 2.625, safe, 1.0);
        assert_eq!((m.k, m.t), (5, 1));
        assert_eq!((m.width, m.height), (216, 468));
        // 24 points is 63 device pixels: 13 art pixels, rounded up
        assert_eq!(m.safe.top, 13);
        assert_eq!(m.safe.bottom, 9);
        assert!((m.pt(21) * 2.625 - 105.0).abs() < 1e-9);
    }
}
