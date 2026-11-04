// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from HireAnimTest: the hire animation's steps, opacities and path.

use dayapp::rules::Pt;
use dayapp::rules::hire_anim::{self as h, Phase};

const FROM: Pt = Pt::new(40, 100);
const TO: Pt = Pt::new(7, 300);

fn close(a: f32, b: f32, d: f32) -> bool {
    (a - b).abs() <= d
}

#[test]
fn the_whole_sequence_is_the_six_steps_added() {
    assert_eq!(180 + 160 + 80 + 450 + 120 + 200, h::total(true));
    assert_eq!(h::FADE_MS, h::total(false));
}

#[test]
fn steps_come_in_order_and_cover_the_total() {
    let order = [
        Phase::Fade,
        Phase::Lift,
        Phase::Plate,
        Phase::Travel,
        Phase::Settle,
        Phase::Glow,
    ];
    let mut seen = 0;
    let mut last = None;
    for t in 0..h::total(true) {
        let p = h::phase(t, true);
        if Some(p) != last {
            assert_eq!(order[seen], p);
            seen += 1;
            last = Some(p);
        }
    }
    assert_eq!(6, seen);
    assert_eq!(Phase::Done, h::phase(h::total(true), true));
    assert_eq!(Phase::Done, h::phase(h::total(true) + 5000, true));
    assert_eq!(Phase::Wait, h::phase(-1, true));
}

#[test]
fn a_hire_that_banks_nothing_plays_the_fade_alone() {
    assert_eq!(Phase::Fade, h::phase(h::FADE_MS - 1, false));
    assert_eq!(Phase::Done, h::phase(h::FADE_MS, false));
}

#[test]
fn progress_runs_zero_to_one_in_each_step() {
    assert_eq!(0.0, h::progress(0, true));
    assert!(close(0.5, h::progress(h::FADE_MS / 2, true), 0.01));
    assert_eq!(0.0, h::progress(h::FADE_MS, true));
    assert_eq!(1.0, h::progress(h::total(true), true));
    assert_eq!(0.0, h::progress(-50, true));
    let mut last = -1.0;
    for t in 0..h::FADE_MS {
        let p = h::progress(t, true);
        assert!(p >= last);
        last = p;
    }
}

#[test]
fn the_cart_fades_in_monotonically_from_nothing_to_full() {
    assert_eq!(0, h::cart_alpha(-10));
    assert_eq!(0, h::cart_alpha(0));
    assert_eq!(255, h::cart_alpha(h::FADE_MS));
    assert_eq!(255, h::cart_alpha(5000));
    let mut last = 0;
    for t in 0..=h::FADE_MS + 20 {
        let a = h::cart_alpha(t);
        assert!(a >= last);
        assert!((0..=255).contains(&a));
        last = a;
    }
}

#[test]
fn a_resource_that_flies_is_solid_and_one_that_does_not_trades_places_with_the_cart() {
    for t in (-20..400).step_by(10) {
        assert_eq!(255, h::icon_alpha(t, true));
        assert_eq!(255, h::icon_alpha(t, false) + h::cart_alpha(t));
    }
    assert_eq!(0, h::icon_alpha(h::FADE_MS, false));
}

#[test]
fn the_lift_is_three_pixels_and_settles_back_to_nothing() {
    assert_eq!(3, h::LIFT_PX);
    let mut lift = 0;
    let mut top = 0;
    for t in 0..h::total(true) {
        let l = h::lift(t);
        assert!((0..=3).contains(&l));
        top = top.max(l);
        lift = l;
    }
    assert_eq!(3, top);
    assert_eq!(0, h::lift(h::FADE_MS - 1));
    assert_eq!(0, h::lift(h::FADE_MS));
    assert_eq!(3, h::lift(h::FADE_MS + h::LIFT_MS));
    assert_eq!(
        3,
        h::lift(h::FADE_MS + h::LIFT_MS + h::PLATE_MS + h::TRAVEL_MS)
    );
    assert_eq!(0, h::lift(h::total(true) - h::GLOW_MS));
    assert_eq!(0, lift);
}

#[test]
fn lift_rises_then_settles_without_wobbling() {
    let settle_start = h::FADE_MS + h::LIFT_MS + h::PLATE_MS + h::TRAVEL_MS;
    let mut last = 0;
    for t in 0..settle_start {
        let l = h::lift(t);
        assert!(l >= last, "rising at {t}");
        last = l;
    }
    for t in settle_start..h::total(true) {
        let l = h::lift(t);
        assert!(l <= last, "falling at {t}");
        last = l;
    }
}

#[test]
fn the_glow_comes_up_with_the_lift_holds_and_fades_at_the_end() {
    assert_eq!(0.0, h::glow(0));
    assert_eq!(0.0, h::glow(h::FADE_MS - 1));
    assert_eq!(1.0, h::glow(h::FADE_MS + h::LIFT_MS));
    let settle_end = h::total(true) - h::GLOW_MS;
    assert_eq!(1.0, h::glow(settle_end - 1));
    assert_eq!(0.0, h::glow(h::total(true)));
    let mut up = 0.0;
    for t in h::FADE_MS..=h::FADE_MS + h::LIFT_MS {
        let g = h::glow(t);
        assert!(g >= up);
        up = g;
    }
    let mut down = 1.0;
    for t in settle_end..h::total(true) {
        let g = h::glow(t);
        assert!(g <= down);
        assert!((0.0..=1.0).contains(&g));
        down = g;
    }
}

#[test]
fn the_plate_is_there_from_step_three_to_the_end() {
    let plate_at = h::FADE_MS + h::LIFT_MS;
    assert!(!h::plate_shown(plate_at - 1));
    assert!(h::plate_shown(plate_at));
    assert!(h::plate_shown(h::total(true) - 1));
    assert!(!h::plate_shown(h::total(true)));
}

#[test]
fn the_resource_starts_on_the_cart_and_ends_exactly_on_the_slot() {
    assert_eq!(FROM, h::icon_at(FROM, TO, -30));
    assert_eq!(FROM, h::icon_at(FROM, TO, 0));
    assert_eq!(FROM, h::icon_at(FROM, TO, h::FADE_MS - 1));
    assert_eq!(TO, h::icon_at(FROM, TO, h::total(true) - h::GLOW_MS));
    assert_eq!(TO, h::icon_at(FROM, TO, h::total(true) - 1));
    assert_eq!(TO, h::icon_at(FROM, TO, h::total(true)));
    assert_eq!(TO, h::icon_at(FROM, TO, 99999));
}

#[test]
fn the_travel_ends_three_pixels_above_the_slot() {
    let travel_end = h::total(true) - h::GLOW_MS - h::SETTLE_MS;
    let just = h::icon_at(FROM, TO, travel_end - 1);
    assert!(just.y <= TO.y - 3 + 1);
    assert_eq!(Pt::new(TO.x, TO.y - 3), h::icon_at(FROM, TO, travel_end));
    let travel_start = h::FADE_MS + h::LIFT_MS + h::PLATE_MS;
    assert_eq!(
        Pt::new(FROM.x, FROM.y - 3),
        h::icon_at(FROM, TO, travel_start)
    );
}

#[test]
fn the_travel_is_monotonic_and_never_leaves_the_straight_lines_bounds() {
    let travel_start = h::FADE_MS + h::LIFT_MS + h::PLATE_MS;
    let travel_end = travel_start + h::TRAVEL_MS;
    let mut last = h::icon_at(FROM, TO, travel_start);
    for t in travel_start..=travel_end {
        let p = h::icon_at(FROM, TO, t);
        assert!(p.x <= last.x, "x at {t}");
        assert!(p.y >= last.y, "y at {t}");
        assert!(p.x >= TO.x && p.x <= FROM.x);
        last = p;
    }
}

#[test]
fn the_travel_is_eased_so_it_is_slow_at_both_ends() {
    let s = h::FADE_MS + h::LIFT_MS + h::PLATE_MS;
    let early = h::icon_at(FROM, TO, s + 50).y - h::icon_at(FROM, TO, s).y;
    let middle = h::icon_at(FROM, TO, s + 275).y - h::icon_at(FROM, TO, s + 225).y;
    assert!(early < middle);
}

#[test]
fn a_slot_left_of_and_above_the_cart_also_works() {
    let (from, to) = (Pt::new(20, 300), Pt::new(200, 120));
    let s = h::FADE_MS + h::LIFT_MS + h::PLATE_MS;
    let mut last = h::icon_at(from, to, s);
    let travel_end = h::total(true) - h::GLOW_MS - h::SETTLE_MS;
    for t in s..=travel_end {
        let p = h::icon_at(from, to, t);
        assert!(p.x >= last.x);
        assert!(p.y <= last.y);
        last = p;
    }
    assert_eq!(Pt::new(200, 117), last);
}

#[test]
fn a_staggered_hire_waits_before_it_begins() {
    assert_eq!(Phase::Wait, h::phase(-h::CHAIN_STAGGER_MS, true));
    assert_eq!(0, h::cart_alpha(-h::CHAIN_STAGGER_MS));
    assert_eq!(150, h::CHAIN_STAGGER_MS);
}

#[test]
fn the_plate_is_the_square_with_its_four_corner_pixels_cut() {
    let d = 16;
    let mut solid = 0;
    for y in -1..=d {
        for x in -1..=d {
            if h::plate_has(x, y, d) {
                solid += 1;
            }
        }
    }
    assert_eq!(d * d - 4, solid);
    assert!(!h::plate_has(0, 0, d));
    assert!(!h::plate_has(d - 1, d - 1, d));
    assert!(h::plate_has(1, 0, d));
    assert!(h::plate_has(0, 1, d));
}
