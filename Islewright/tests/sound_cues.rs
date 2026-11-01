// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from SoundCuesTest: which sounds a turn end and a build ask for.

use dayapp::rules::Sfx;
use dayapp::rules::sound_cues::{after_build, bonus_gained, describe, turn_end};

#[test]
fn an_ordinary_turn_end_is_the_turn_sound() {
    assert_eq!(
        "[TURN_END@0]",
        describe(&turn_end(false, false, false, false, false))
    );
}

#[test]
fn an_empty_turn_with_a_penalty_replaces_it() {
    assert_eq!(
        "[EMPTY_TURN@0]",
        describe(&turn_end(true, false, false, false, false))
    );
}

#[test]
fn the_ship_appears_after_the_turn_sound() {
    assert_eq!(
        "[TURN_END@0, SHIP_APPEARS@300]",
        describe(&turn_end(false, true, false, false, false))
    );
}

#[test]
fn the_last_turn_ending_is_the_turn_then_the_short_fanfare_with_no_bell() {
    assert_eq!(
        "[TURN_END@0, GAME_OVER@300]",
        describe(&turn_end(false, false, false, true, false))
    );
}

#[test]
fn a_new_best_at_the_end_plays_the_big_fanfare_instead_of_the_short_one() {
    assert_eq!(
        "[TURN_END@0, TOP_SCORE@300]",
        describe(&turn_end(false, false, false, true, true))
    );
}

#[test]
fn an_empty_last_turn_ending_has_the_same_fanfare() {
    assert_eq!(
        "[EMPTY_TURN@0, GAME_OVER@300]",
        describe(&turn_end(true, false, false, true, false))
    );
}

#[test]
fn the_bell_rings_when_the_ship_moors_for_the_last_turn() {
    assert_eq!(
        "[TURN_END@0, SHIP_DOCKS@300]",
        describe(&turn_end(false, false, true, false, false))
    );
}

#[test]
fn the_bell_never_comes_before_the_build_or_turn_sound_at_zero() {
    for cue in turn_end(false, false, true, false, false) {
        if cue.sfx == Sfx::ShipDocks {
            assert!(cue.delay_ms >= 300);
        }
    }
}

#[test]
fn nothing_follows_a_build_that_earned_and_ended_nothing() {
    assert_eq!("[]", describe(&after_build(false, false, false)));
}

#[test]
fn a_bonus_chimes_after_the_build() {
    assert_eq!("[BONUS@250]", describe(&after_build(true, false, false)));
}

#[test]
fn a_build_that_wins_a_race_is_followed_by_the_fanfare() {
    assert_eq!(
        "[GAME_OVER@300]",
        describe(&after_build(false, true, false))
    );
    assert_eq!("[TOP_SCORE@300]", describe(&after_build(false, true, true)));
}

#[test]
fn a_winning_build_that_also_earned_a_bonus_chimes_first() {
    assert_eq!(
        "[BONUS@250, TOP_SCORE@700]",
        describe(&after_build(true, true, true))
    );
}

#[test]
fn bonus_transition_is_detected_only_when_a_bit_is_newly_set() {
    assert!(!bonus_gained(0, 0));
    assert!(bonus_gained(0, 1));
    assert!(bonus_gained(0, 2));
    assert!(bonus_gained(1, 3));
    assert!(!bonus_gained(3, 3), "already had it");
    assert!(!bonus_gained(2, 2));
}
