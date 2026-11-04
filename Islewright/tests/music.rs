// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from MusicTest: the pure parts of the soundtrack, the play order and the volume curve.

use std::collections::HashSet;

use dayapp::rules::Rng;
use dayapp::rules::music_gain::gain_for;
use dayapp::rules::music_playlist::MusicPlaylist;

#[test]
fn gain_is_the_setting_as_a_fraction_squared() {
    assert_eq!(0.0, gain_for(0));
    assert_eq!(1.0, gain_for(100));
    assert!((0.36 - gain_for(60)).abs() < 1e-6);
    assert!((0.25 - gain_for(50)).abs() < 1e-6);
    assert!((0.0001 - gain_for(1)).abs() < 1e-8);
}

#[test]
fn gain_rises_with_the_setting_and_stays_in_range() {
    let mut previous = -1.0;
    for setting in 0..=100 {
        let gain = gain_for(setting);
        assert!(gain > previous);
        assert!((0.0..=1.0).contains(&gain));
        previous = gain;
    }
    assert_eq!(0.0, gain_for(-20));
    assert_eq!(1.0, gain_for(400));
}

#[test]
fn every_track_plays_once_per_pass_in_a_shuffled_order() {
    for seed in 0..50 {
        let mut list = MusicPlaylist::new(10, Rng::new(seed));
        for _ in 0..3 {
            let mut seen = HashSet::new();
            for _ in 0..10 {
                let track = list.next_track().unwrap();
                assert!(track < 10);
                assert!(seen.insert(track), "repeat inside a pass");
            }
        }
    }
}

#[test]
fn no_track_repeats_straight_away_across_the_loop_boundary() {
    for count in 2..=10 {
        for seed in 0..300 {
            let mut list = MusicPlaylist::new(count, Rng::new(seed));
            let mut previous = list.next_track().unwrap();
            for i in 1..count * 6 {
                let track = list.next_track().unwrap();
                assert_ne!(track, previous, "count {count} seed {seed} step {i}");
                previous = track;
            }
        }
    }
}

#[test]
fn the_order_changes_with_the_seed() {
    let firsts: HashSet<usize> = (0..40)
        .map(|seed| MusicPlaylist::new(10, Rng::new(seed)).next_track().unwrap())
        .collect();
    assert!(firsts.len() > 3);
}

#[test]
fn one_track_repeats_and_no_tracks_give_none() {
    let mut one = MusicPlaylist::new(1, Rng::new(1));
    assert_eq!(Some(0), one.next_track());
    assert_eq!(Some(0), one.next_track());
    assert_eq!(None, MusicPlaylist::new(0, Rng::new(1)).next_track());
}
