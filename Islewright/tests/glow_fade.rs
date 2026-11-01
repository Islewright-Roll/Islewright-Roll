// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from GlowFadeTest: the next trader glows in full and the ones after it fade.

use dayapp::rules::glow_fade;

fn hired(count: usize) -> Vec<bool> {
    (0..7).map(|tier| tier >= 1 && tier <= count).collect()
}

#[test]
fn the_next_trader_glows_in_full_and_the_rest_fade() {
    let h = hired(0);
    assert_eq!(Some(0), glow_fade::step(&h, 1));
    assert_eq!(Some(1), glow_fade::step(&h, 2));
    assert_eq!(Some(2), glow_fade::step(&h, 3));
    assert_eq!(Some(3), glow_fade::step(&h, 4));
    assert_eq!(Some(5), glow_fade::step(&h, 6));
}

#[test]
fn the_fade_moves_up_as_traders_are_hired() {
    let h = hired(2);
    assert_eq!(None, glow_fade::step(&h, 1));
    assert_eq!(None, glow_fade::step(&h, 2));
    assert_eq!(Some(0), glow_fade::step(&h, 3));
    assert_eq!(Some(1), glow_fade::step(&h, 4));
    assert_eq!(Some(2), glow_fade::step(&h, 5));
    assert_eq!(Some(3), glow_fade::step(&h, 6));
}

#[test]
fn strength_is_full_then_half_then_a_fifth_then_none() {
    assert_eq!(1.0, glow_fade::strength(Some(0)));
    assert_eq!(0.5, glow_fade::strength(Some(1)));
    assert_eq!(0.2, glow_fade::strength(Some(2)));
    assert_eq!(0.0, glow_fade::strength(Some(3)));
    assert_eq!(0.0, glow_fade::strength(Some(5)));
    assert_eq!(0.0, glow_fade::strength(None));
}

#[test]
fn a_hired_or_missing_tier_has_no_step() {
    let h = hired(3);
    assert_eq!(None, glow_fade::step(&h, 3));
    assert_eq!(None, glow_fade::step(&h, 0));
    assert_eq!(None, glow_fade::step(&h, 7));
}

#[test]
fn a_skipped_trader_still_counts_as_ahead_of_the_ones_after_it() {
    // Not reachable on Tarnholm, where traders are hired in order, but the rule is only "how
    // many unhired ones come before this one"
    let mut h = vec![false; 7];
    h[2] = true;
    assert_eq!(Some(0), glow_fade::step(&h, 1));
    assert_eq!(Some(1), glow_fade::step(&h, 3));
}
