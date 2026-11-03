// SPDX-License-Identifier: GPL-3.0-or-later
//! The spend animation's numbers, hire_anim run the other way. A build paid with a banked
//! resource lifts its mini die off the tab in a blue glow (step 1), carries it to the piece
//! being built (step 2), and fades it out there as the glow dies (step 3).
//!
//! In trader resource classic mode nothing flies: the player drops the die onto a rolled die
//! themselves. A blue outline then runs once round that die (step 1), the die turns to the
//! resource (step 2) and the outline fades (step 3); see `ring_*`.
//!
//! Every function takes the milliseconds elapsed since the animation began. Negative means it
//! has not begun (several resources spent in one build stagger), and anything past the total
//! means it is over.

use super::board_layers::round_f32;
use super::hire_anim::{LIFT_PX, smooth};
use super::pt::Pt;

pub const LIFT_MS: i32 = 160;
pub const TRAVEL_MS: i32 = 420;
pub const LAND_MS: i32 = 220;

pub const RING_MS: i32 = 320;
pub const TURN_MS: i32 = 80;
pub const RING_FADE_MS: i32 = 260;

/// The whole flight.
pub fn total() -> i32 {
    LIFT_MS + TRAVEL_MS + LAND_MS
}

/// The glow's strength, 0 to 1: rising as the die lifts, full in flight, dying as it lands.
pub fn glow(elapsed: i32) -> f32 {
    if elapsed < 0 {
        0.0
    } else if elapsed < LIFT_MS {
        smooth(elapsed as f32 / LIFT_MS as f32)
    } else if elapsed < LIFT_MS + TRAVEL_MS {
        1.0
    } else if elapsed < total() {
        1.0 - smooth((elapsed - LIFT_MS - TRAVEL_MS) as f32 / LAND_MS as f32)
    } else {
        0.0
    }
}

/// The mini die's opacity, 0 to 255: solid until it lands, then fading into the piece.
pub fn alpha(elapsed: i32) -> i32 {
    if elapsed < 0 || elapsed >= total() {
        return 0;
    }
    let landing = elapsed - LIFT_MS - TRAVEL_MS;
    if landing <= 0 {
        return 255;
    }
    255 - round_f32(255.0 * smooth(landing as f32 / LAND_MS as f32))
}

/// Where the mini die's top left is, given where it rests on the tab (from) and where it
/// comes to rest over the piece (to). It lifts LIFT_PX straight up, travels in a straight
/// line to the same height over the piece, and settles onto it as it fades. Whole pixels.
pub fn die_at(from: Pt, to: Pt, elapsed: i32) -> Pt {
    if elapsed < LIFT_MS {
        let t = smooth(elapsed.max(0) as f32 / LIFT_MS as f32);
        return Pt::new(from.x, from.y - round_f32(LIFT_PX as f32 * t));
    }
    let travelled = elapsed - LIFT_MS;
    if travelled < TRAVEL_MS {
        let e = smooth(travelled as f32 / TRAVEL_MS as f32);
        return Pt::new(
            from.x + round_f32((to.x - from.x) as f32 * e),
            from.y - LIFT_PX + round_f32((to.y - from.y) as f32 * e),
        );
    }
    let t = smooth(((travelled - TRAVEL_MS) as f32 / LAND_MS as f32).min(1.0));
    Pt::new(to.x, to.y - LIFT_PX + round_f32(LIFT_PX as f32 * t))
}

/// The whole classic ring.
pub fn ring_total() -> i32 {
    RING_MS + TURN_MS + RING_FADE_MS
}

/// How much of the outline is drawn, 0 to 1, going once round the die.
pub fn ring_drawn(elapsed: i32) -> f32 {
    if elapsed <= 0 {
        0.0
    } else if elapsed >= RING_MS {
        1.0
    } else {
        smooth(elapsed as f32 / RING_MS as f32)
    }
}

/// Whether the die still shows what it showed before the drop: until the ring closes.
pub fn ring_shows_old_face(elapsed: i32) -> bool {
    elapsed < RING_MS
}

/// The outline's opacity, 0 to 255: full while it runs and the die turns, then fading.
pub fn ring_alpha(elapsed: i32) -> i32 {
    let fading = elapsed - RING_MS - TURN_MS;
    if elapsed < 0 || fading >= RING_FADE_MS {
        return 0;
    }
    if fading <= 0 {
        return 255;
    }
    255 - round_f32(255.0 * smooth(fading as f32 / RING_FADE_MS as f32))
}

/// The first `n` pixels of the outline round a w by h box with its corner pixels cut,
/// clockwise from the middle of the top edge, as (x, y) offsets from the box's top left.
/// The full outline is `ring_len(w, h)` pixels.
pub fn ring_pixels(w: i32, h: i32, n: usize) -> Vec<(i32, i32)> {
    let mut all = Vec::with_capacity(ring_len(w, h));
    let mid = w / 2;
    for x in mid..w - 1 {
        all.push((x, 0));
    }
    for y in 1..h - 1 {
        all.push((w - 1, y));
    }
    for x in (1..w - 1).rev() {
        all.push((x, h - 1));
    }
    for y in (1..h - 1).rev() {
        all.push((0, y));
    }
    for x in 1..mid {
        all.push((x, 0));
    }
    all.truncate(n);
    all
}

/// How many pixels a whole ring round a w by h box has.
pub fn ring_len(w: i32, h: i32) -> usize {
    (2 * (w - 2) + 2 * (h - 2)).max(0) as usize
}
