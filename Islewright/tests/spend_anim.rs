// SPDX-License-Identifier: GPL-3.0-or-later
//! The spend animation's keyframes: the mini die leaves its slot, arrives over the piece and
//! fades there; the classic ring runs once round the die, then fades.

use dayapp::rules::Pt;
use dayapp::rules::hire_anim::LIFT_PX;
use dayapp::rules::spend_anim::*;

#[test]
fn the_die_lifts_off_its_slot_travels_and_sets_down_on_the_piece() {
    let (from, to) = (Pt::new(10, 200), Pt::new(150, 40));
    assert_eq!(from, die_at(from, to, 0));
    assert_eq!(Pt::new(10, 200 - LIFT_PX), die_at(from, to, LIFT_MS));
    assert_eq!(
        Pt::new(150, 40 - LIFT_PX),
        die_at(from, to, LIFT_MS + TRAVEL_MS)
    );
    assert_eq!(to, die_at(from, to, total()));
}

#[test]
fn it_is_solid_in_flight_and_gone_once_landed() {
    assert_eq!(0, alpha(-1));
    assert_eq!(255, alpha(LIFT_MS + TRAVEL_MS));
    assert!(alpha(LIFT_MS + TRAVEL_MS + LAND_MS / 2) < 255);
    assert_eq!(0, alpha(total()));
    assert_eq!(1.0, glow(LIFT_MS + 1));
    assert_eq!(0.0, glow(total()));
}

#[test]
fn the_ring_goes_once_round_without_repeating_a_pixel() {
    let (w, h) = (34, 34);
    let all = ring_pixels(w, h, usize::MAX);
    assert_eq!(ring_len(w, h), all.len());
    let mut seen = all.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(all.len(), seen.len());
    assert!(!all.contains(&(0, 0)), "the corners are cut");
    assert_eq!((w / 2, 0), all[0], "it starts at the top middle");
}

#[test]
fn the_old_face_shows_until_the_ring_closes_then_the_ring_fades() {
    assert!(ring_shows_old_face(RING_MS - 1));
    assert!(!ring_shows_old_face(RING_MS));
    assert_eq!(1.0, ring_drawn(RING_MS));
    assert_eq!(255, ring_alpha(RING_MS + TURN_MS));
    assert_eq!(0, ring_alpha(ring_total()));
}
