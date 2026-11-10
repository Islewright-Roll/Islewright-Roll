// SPDX-License-Identifier: GPL-3.0-or-later
//! The hire animation's numbers. A trader fades in on its spot (step 1). If the hire banked a
//! resource, the resource then lifts off the cart, a mini die plate appears behind it, it
//! travels to its slot on the tab, settles into it and the glow round it fades (steps 2 to 6).
//!
//! Every function takes the milliseconds elapsed since the hire began. Negative means it has
//! not begun (a chain of hires staggers them), and anything past the total means it is over.

use super::board_layers::round_f32;
use super::pt::Pt;

pub const FADE_MS: i32 = 180;
pub const LIFT_MS: i32 = 160;
pub const PLATE_MS: i32 = 80;
pub const TRAVEL_MS: i32 = 450;
pub const SETTLE_MS: i32 = 120;
pub const GLOW_MS: i32 = 200;
/// How far the resource lifts off the cart, and sets down again into its slot.
pub const LIFT_PX: i32 = 3;
/// A second hire in the same tap begins this long after the one before it.
pub const CHAIN_STAGGER_MS: i32 = 150;
/// How far toward white the player's colour is taken for the banking glow.
pub const GLOW_LIGHTEN: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Wait,
    Fade,
    Lift,
    Plate,
    Travel,
    Settle,
    Glow,
    Done,
}

const ORDER: [Phase; 6] = [
    Phase::Fade,
    Phase::Lift,
    Phase::Plate,
    Phase::Travel,
    Phase::Settle,
    Phase::Glow,
];
const LENGTH: [i32; 6] = [FADE_MS, LIFT_MS, PLATE_MS, TRAVEL_MS, SETTLE_MS, GLOW_MS];

/// The whole sequence, or step 1 alone.
pub fn total(flies: bool) -> i32 {
    if flies { LENGTH.iter().sum() } else { FADE_MS }
}

/// Smoothstep.
pub fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// The step under way, and how far into it. Without flying there is only Fade, then Done.
fn locate(elapsed: i32, flies: bool) -> (Phase, f32) {
    if elapsed < 0 {
        return (Phase::Wait, 0.0);
    }
    let mut left = elapsed;
    let steps = if flies { ORDER.len() } else { 1 };
    for i in 0..steps {
        if left < LENGTH[i] {
            return (ORDER[i], left as f32 / LENGTH[i] as f32);
        }
        left -= LENGTH[i];
    }
    (Phase::Done, 1.0)
}

/// The step under way. Without flying there is only Fade, then Done.
pub fn phase(elapsed: i32, flies: bool) -> Phase {
    locate(elapsed, flies).0
}

/// How far through its step it is, 0 to 1 (0 while waiting, 1 once done).
pub fn progress(elapsed: i32, flies: bool) -> f32 {
    locate(elapsed, flies).1
}

/// The cart's opacity, 0 to 255: nothing until it begins, full once step 1 is over.
pub fn cart_alpha(elapsed: i32) -> i32 {
    if elapsed <= 0 {
        return 0;
    }
    if elapsed >= FADE_MS {
        return 255;
    }
    round_f32(255.0 * smooth(elapsed as f32 / FADE_MS as f32))
}

/// The resource's opacity while it stays on the cart. A resource that flies is solid
/// throughout; one that does not trades places with the cart as it fades in, and is gone when
/// the cart is whole.
pub fn icon_alpha(elapsed: i32, flies: bool) -> i32 {
    if flies {
        255
    } else {
        255 - cart_alpha(elapsed)
    }
}

/// The glow's strength, 0 to 1, in the flying sequence.
pub fn glow(elapsed: i32) -> f32 {
    match phase(elapsed, true) {
        Phase::Lift => smooth(progress(elapsed, true)),
        Phase::Plate | Phase::Travel | Phase::Settle => 1.0,
        Phase::Glow => 1.0 - smooth(progress(elapsed, true)),
        _ => 0.0,
    }
}

/// How many pixels the resource stands above its resting place, 0 to LIFT_PX.
pub fn lift(elapsed: i32) -> i32 {
    match phase(elapsed, true) {
        Phase::Lift => round_f32(LIFT_PX as f32 * smooth(progress(elapsed, true))),
        Phase::Plate | Phase::Travel => LIFT_PX,
        Phase::Settle => LIFT_PX - round_f32(LIFT_PX as f32 * smooth(progress(elapsed, true))),
        _ => 0,
    }
}

/// Whether the mini die's plate is drawn behind the resource: from step 3 to the end.
pub fn plate_shown(elapsed: i32) -> bool {
    matches!(
        phase(elapsed, true),
        Phase::Plate | Phase::Travel | Phase::Settle | Phase::Glow
    )
}

/// Where the resource's top left is, in art pixels, given where it rests on the cart's bed
/// (from) and where it rests in its slot (to). It starts on the cart, lifts straight up,
/// travels in a straight line to LIFT_PX above the slot, and sets down onto it. Whole pixels.
pub fn icon_at(from: Pt, to: Pt, elapsed: i32) -> Pt {
    let phase = phase(elapsed, true);
    if phase == Phase::Travel {
        let e = smooth(progress(elapsed, true));
        return Pt::new(
            from.x + round_f32((to.x - from.x) as f32 * e),
            from.y - LIFT_PX + round_f32((to.y - from.y) as f32 * e),
        );
    }
    if matches!(phase, Phase::Settle | Phase::Glow | Phase::Done) {
        return Pt::new(to.x, to.y - lift(elapsed));
    }
    Pt::new(from.x, from.y - lift(elapsed))
}

/// Whether a pixel of a d by d mini die plate is solid: the square with its four corner
/// pixels cut. The glow's outline follows this shape.
pub fn plate_has(x: i32, y: i32, d: i32) -> bool {
    if x < 0 || y < 0 || x >= d || y >= d {
        return false;
    }
    let corner_x = x == 0 || x == d - 1;
    let corner_y = y == 0 || y == d - 1;
    !(corner_x && corner_y)
}
