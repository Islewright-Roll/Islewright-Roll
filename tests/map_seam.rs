// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from MapSeamTest: every rule reads the map, so a race map with unordered pieces,
//! paired traders and a long road plays by its own tables.

mod common;

use common::*;
use dayapp::rules::build_plan::fingerprint;
use dayapp::rules::game::{self, RESOURCES_VILLAGE};
use dayapp::rules::{Board, BuildPlan, DiceValue, Game, Kind, Playsheet, Pt};

use DiceValue::{Brick, Grain, Lumber, Ore, Wool};

#[test]
fn geometry_passes_and_roads_follow_from_it() {
    let map = race();
    let b = Board::for_map(map);
    assert_eq!(18, b.road_anchor.len());
    assert_eq!(EXPECTED_ROAD_NEEDS.len(), map.road_needs.len());
    for (i, needs) in EXPECTED_ROAD_NEEDS.iter().enumerate() {
        assert_eq!(needs.to_vec(), map.road_needs[i], "road {i}");
    }
    assert_eq!(Pt::new(252, 165), b.knight[5]);
    assert_eq!(Pt::new(252, 267), b.knight[8]);
}

#[test]
fn sheet_is_sized_by_the_map() {
    let mut s = Playsheet::new(race());
    assert_eq!(18, s.roads.len());
    assert_eq!(7, s.villages.len());
    assert_eq!(4, s.cities.len());
    assert_eq!(10, s.traders.len());
    assert_eq!(7, s.bank_size());
    // No turn cap: the turn scores grow as turns are taken
    assert_eq!(1, s.turn_score.len());
    s.score_turn(30);
    assert_eq!(31, s.turn_score.len());
    assert_eq!(0, s.score());
}

#[test]
fn any_touching_road_permits_and_nothing_is_ordered() {
    let mut s = Playsheet::new(race());
    // The start road touches village 2 and road 1, and nothing else
    assert!(s.can_build_village(2));
    assert!(!s.can_build_village(0));
    assert!(s.can_build_road(1));
    assert!(!s.can_build_road(2));
    assert!(!s.can_build_road(17));
    assert!(!s.can_build_city(0));

    s.build_road(1);
    assert!(s.can_build_road(2));
    assert!(s.can_build_road(3));
    s.build_road(3);
    // Village 4 without villages 0 to 3, by either of its roads
    assert!(s.can_build_village(4));
    s.build_road(2);
    assert!(s.can_build_city(0));
    s.build_city(0);
    assert!(!s.can_build_city(0));
    assert!(!s.can_build_city(1));

    // Any trader, any time, once
    assert!(s.can_build_knight(9));
    assert!(s.can_build_knight(4));
    assert!(!s.can_build_knight(10));
    assert!(!s.can_build_knight(0));
    s.build_knight(9);
    assert!(s.is_trader_built(9));
    assert!(!s.is_trader_built(8));
    assert!(!s.can_build_knight(9));
    assert_eq!(1, s.trader_count());
}

#[test]
fn a_bank_slot_behind_two_traders_needs_both() {
    let mut s = Playsheet::new(race());
    s.build_knight(6);
    assert!(!s.can_use_knight_resource(6));
    assert!(!s.is_knight_resource_used(6));
    assert!(!s.has_wildcard());
    assert_eq!(DiceValue::None, s.use_knight_resource(6));
    assert!(s.resources_avail[6], "a refused use spends nothing");

    s.build_knight(7);
    assert!(s.can_use_knight_resource(6));
    assert_eq!(6, s.first_available_slot(DiceValue::Any));
    assert_eq!(DiceValue::Any, s.knight_resource(6));
    s.use_knight_resource(6);
    assert!(s.is_knight_resource_used(6));
    assert_eq!(0, s.first_available_slot(DiceValue::Any));

    s.build_knight(9);
    s.build_knight(8);
    assert_eq!(7, s.first_available_slot(DiceValue::Any));
    s.build_knight(2);
    assert_eq!(2, s.first_available_slot(Grain));
    assert_eq!(Grain, s.knight_resource(2));

    s.return_to_bank(DiceValue::Any);
    assert_eq!(6, s.first_available_slot(DiceValue::Any));
}

#[test]
fn score_is_pieces_plus_two_derived_bonuses() {
    let mut s = Playsheet::new(race());
    for road in [1, 3, 4] {
        s.build_road(road);
    }
    s.turns_nothing_built = 5;
    assert_eq!(0, s.score(), "roads and empty turns score nothing");
    s.build_village(2);
    s.build_village(4);
    assert_eq!(2, s.score());
    s.build_road(6);
    assert_eq!(4, s.score(), "the long road");
    s.build_road(5);
    s.build_city(2);
    assert_eq!(6, s.score());
    s.build_knight(8);
    s.build_knight(9);
    assert_eq!(6, s.score());
    s.build_knight(1);
    assert_eq!(8, s.score(), "three traders");
    s.build_knight(2);
    assert_eq!(8, s.score(), "and only once");
    s.use_knight_resource(7);
    s.use_knight_resource(1);
    assert_eq!(8, s.score(), "spending the bank costs no points");
}

#[test]
fn the_game_ends_at_the_target_or_the_cap() {
    let mut g = Game::new(race());
    assert!(!g.is_game_done());
    for road in [1, 3, 4, 6, 2, 5] {
        g.playsheet.build_road(road);
    }
    g.playsheet.build_village(2);
    g.playsheet.build_village(4);
    g.playsheet.build_village(6);
    g.playsheet.build_city(0);
    assert_eq!(7, g.playsheet.score());
    assert!(!g.is_game_done());
    g.playsheet.build_city(2);
    assert_eq!(9, g.playsheet.score());
    assert!(!g.is_game_done());
    g.playsheet.build_knight(3);
    g.playsheet.build_knight(5);
    g.playsheet.build_knight(7);
    assert_eq!(11, g.playsheet.score());
    assert!(g.is_game_done());

    // With no cap, no number of turns ends it
    let mut slow = Game::new(race());
    for _ in 1..=40 {
        assert!(!slow.is_game_done());
        slow.new_turn(false);
    }
    assert!(!slow.is_game_done());
    assert_eq!(40, slow.turns_taken);
    assert_eq!(0, slow.playsheet.score());

    // A race with a cap still ends there
    let mut c = Game::new(capped());
    for _ in 1..=20 {
        assert!(!c.is_game_done());
        c.new_turn(false);
    }
    assert!(c.is_game_done());

    // And a game can move from one map to the other
    slow.start_new_game(tarnholm());
    assert_eq!(17, slow.playsheet.roads.len());
    assert!(same(tarnholm(), slow.board.map));
    assert!(!slow.is_game_done());
}

#[test]
fn the_plan_walks_the_map_and_sees_either_wildcard() {
    // Brick, lumber and wool on the dice; the grain for village 2 has to be a "?". Slot 6 is
    // shut (one trader of its pair), slot 7 is open.
    let mut s = Playsheet::new(race());
    s.build_knight(6);
    s.build_knight(8);
    s.build_knight(9);
    let mut d = dice(&[Brick, Lumber, Wool]);
    let plan = BuildPlan::compute(&d, &s, 1);
    let village = plan
        .chain_for(Kind::Village, 2)
        .cloned()
        .expect("village 2");
    assert!(village.spends_bank);
    assert!(village.spends_wildcard);
    let road = plan
        .chain_for(Kind::Road, 1)
        .expect("a road from dice alone");
    assert!(!road.spends_bank);
    assert!(plan.chain_for(Kind::Road, 17).is_none());
    assert!(BuildPlan::max_dice_spendable_through(&d, &s, &village) >= 3);

    game::use_resources(RESOURCES_VILLAGE, &mut d, &mut s, &mut Vec::new());
    assert!(s.resources_avail[6], "slot 6 was never open");
    assert!(!s.resources_avail[7], "slot 7 paid");

    // A trader on the last tier is reachable by the plan
    let trader_dice = dice(&[Wool, Grain, Ore]);
    assert!(
        BuildPlan::compute(&trader_dice, &Playsheet::new(race()), 1)
            .chain_for(Kind::Knight, 9)
            .is_some()
    );
}

#[test]
fn the_fingerprint_tells_trader_sets_apart() {
    let mut a = Playsheet::new(race());
    let mut b = Playsheet::new(race());
    a.build_knight(1);
    b.build_knight(2);
    assert_eq!(a.trader_count(), b.trader_count());
    assert_ne!(fingerprint(&dice(&[]), &a), fingerprint(&dice(&[]), &b));
}
