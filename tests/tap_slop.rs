// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TapSlopTest: a tap is a touch that ends near where it began.

use dayapp::rules::tap_slop::is_tap;

#[test]
fn a_touch_that_never_moved_is_a_tap() {
    assert!(is_tap(100.0, 200.0, 100.0, 200.0, 42.0));
}

#[test]
fn a_wobble_inside_the_slop_is_a_tap() {
    assert!(is_tap(100.0, 200.0, 110.0, 215.0, 42.0));
    assert!(is_tap(100.0, 200.0, 80.0, 190.0, 42.0));
}

#[test]
fn exactly_the_slop_is_still_a_tap() {
    assert!(is_tap(0.0, 0.0, 30.0, 40.0, 50.0));
    assert!(is_tap(0.0, 0.0, 0.0, -50.0, 50.0));
}

#[test]
fn anything_further_is_a_swipe() {
    assert!(!is_tap(0.0, 0.0, 31.0, 40.0, 50.0));
    assert!(!is_tap(100.0, 200.0, 100.0, 400.0, 42.0));
    assert!(!is_tap(500.0, 500.0, 100.0, 500.0, 42.0));
}

#[test]
fn distance_is_straight_line_not_per_axis() {
    // 35 across and 35 down is 49.5 away: past 42 though neither axis is
    assert!(!is_tap(0.0, 0.0, 35.0, 35.0, 42.0));
    assert!(is_tap(0.0, 0.0, 29.0, 29.0, 42.0));
}
