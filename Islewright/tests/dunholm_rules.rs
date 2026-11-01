// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from DunholmRulesTest: Dunholm, the second island, pinned against its tables, its
//! rules played on a sheet, and the bank rules only it can reach: two "?" slots that must
//! both be spent when a build needs both, and a banked resource that pays once.

mod common;

use common::*;
use dayapp::rules::game::{
    self, BuildStatus, RESOURCES_CITY, RESOURCES_KNIGHT, RESOURCES_ROAD, RESOURCES_VILLAGE,
};
use dayapp::rules::map_def::TraderSpot;
use dayapp::rules::{
    Board, BuildPlan, Chain, DiceValue, Game, Kind, MapDef, Playsheet, Pt, Rng, Step,
};

use DiceValue::{Brick, Grain, Lumber, Ore, Wool};

fn sheet(traders: &[usize]) -> Playsheet {
    sheet_with(dunholm(), traders)
}

fn status(recipe: &[DiceValue], dice: &[dayapp::rules::Dice], sheet: &Playsheet) -> BuildStatus {
    game::can_build_status(recipe, dice, sheet, &mut Vec::new())
}

fn pay(
    recipe: &[DiceValue],
    dice: &mut [dayapp::rules::Dice],
    sheet: &mut Playsheet,
) -> game::Spent {
    game::use_resources(recipe, dice, sheet, &mut Vec::new())
}

// ---- the tables -----------------------------------------------------------------------

#[test]
fn the_map_is_registered() {
    let map = dunholm();
    assert!(same(map, MapDef::by_id(Some("dunholm"))));
    assert_eq!("dunholm", map.id);
    assert_eq!(2, MapDef::all().len());
    assert!(same(tarnholm(), MapDef::all()[0]));
    assert!(same(map, MapDef::all()[1]));
    assert_eq!(0xFF0D3947, map.sea_fill);
    assert_eq!(tarnholm().sea_fill, map.sea_fill);
}

#[test]
fn the_tables_are_the_plans_tables() {
    let m = dunholm();
    assert_eq!(tarnholm().hex, m.hex);
    assert_eq!(
        vec![
            [0, 1],
            [0, 2],
            [0, 3],
            [1, 1],
            [1, 2],
            [1, 3],
            [2, 4],
            [2, 3],
            [2, 2],
            [3, 3],
            [3, 2],
            [3, 1],
            [4, 3],
            [4, 2],
            [4, 1],
            [4, 0],
            [5, 1],
            [5, 0]
        ],
        m.roads
    );
    assert_eq!(EXPECTED_ROAD_NEEDS.len(), m.road_needs.len());
    for (i, needs) in EXPECTED_ROAD_NEEDS.iter().enumerate() {
        assert_eq!(needs.to_vec(), m.road_needs[i], "road {i}");
    }
    assert_eq!(
        vec![[5, 2], [5, 0], [0, 0], [4, 0], [1, 0], [3, 0], [2, 4]],
        m.villages
    );
    assert_eq!(
        vec![
            vec![17],
            vec![15, 16],
            vec![0, 1],
            vec![13, 14],
            vec![3, 4],
            vec![10, 11],
            vec![6, 7]
        ],
        m.village_roads
    );
    assert!(!m.ordered_villages);
    assert_eq!(vec![[0, 4], [6, 0], [1, 4], [3, 4]], m.cities);
    assert_eq!(vec![vec![2], vec![12], vec![5], vec![8, 9]], m.city_roads);
    assert!(!m.ordered_cities);
    assert_eq!(
        vec![
            TraderSpot::new(0, 0, 0),
            TraderSpot::new(1, 0, 0),
            TraderSpot::new(2, 0, 0),
            TraderSpot::new(3, 0, 0),
            TraderSpot::new(4, 0, 0),
            TraderSpot::new(5, 0, -15),
            TraderSpot::new(5, 0, 15),
            TraderSpot::new(6, 0, -15),
            TraderSpot::new(6, 0, 15)
        ],
        m.traders
    );
    assert!(!m.ordered_traders);
    assert_eq!(vec![1, 2, 3, 4, 5, 6, 6, 7, 7], m.trader_bank);
    assert_eq!(
        vec![
            DiceValue::None,
            Ore,
            Grain,
            Wool,
            Lumber,
            Brick,
            DiceValue::Any,
            DiceValue::Any
        ],
        m.bank_resource
    );
    assert_eq!(
        vec![
            vec![],
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6, 7],
            vec![8, 9]
        ],
        m.bank_traders
    );
    assert_eq!(0, m.road_points);
    assert_eq!(vec![0; 9], m.trader_points);
    assert_eq!(vec![1; 7], m.village_points);
    assert_eq!(vec![2; 4], m.city_points);
    assert_eq!(0, m.empty_turn_penalty);
    assert_eq!(Some(6), m.long_road);
    assert_eq!(2, m.long_road_bonus);
    assert_eq!(3, m.trader_bonus_at);
    assert_eq!(2, m.trader_bonus);
    assert_eq!(
        0, m.turn_cap,
        "no turn cap: the race ends only at the target"
    );
    assert_eq!(10, m.target_score);
    assert!(m.lower_is_better);
    assert!(m.bank_matches_once);
    assert!(
        !tarnholm().bank_matches_once,
        "Tarnholm keeps its old bank habit"
    );
}

#[test]
fn the_board_points_are_where_plan_put_them() {
    let b = Board::for_map(dunholm());
    assert_eq!(Pt::new(225, 198), b.road_anchor[0]);
    assert_eq!(Pt::new(225, 342), b.road_anchor[6]);
    assert_eq!(Pt::new(252, 144), b.road_anchor[17]);
    assert_eq!(Pt::new(234, 144), b.village[0]);
    assert_eq!(Pt::new(342, 288), b.village[5]);
    assert_eq!(Pt::new(180, 252), b.city[0]);
    assert_eq!(Pt::new(288, 252), b.city[1]);
    assert_eq!(Pt::new(288, 324), b.city[3]);
    assert_eq!(Pt::new(198, 216), b.knight[0]);
    assert_eq!(Pt::new(252, 165), b.knight[5]);
    assert_eq!(Pt::new(252, 195), b.knight[6]);
    assert_eq!(Pt::new(252, 237), b.knight[7]);
    assert_eq!(Pt::new(252, 267), b.knight[8]);
}

// ---- placement rules --------------------------------------------------------------------

#[test]
fn a_fresh_sheet_offers_village_two_and_no_castle() {
    let s = Playsheet::new(dunholm());
    for i in 0..7 {
        assert_eq!(i == 2, s.can_build_village(i), "village {i}");
    }
    for i in 0..4 {
        assert!(!s.can_build_city(i), "castle {i}");
    }
    // The start road touches only road 1
    for i in 1..18 {
        assert_eq!(i == 1, s.can_build_road(i), "road {i}");
    }
    assert_eq!(0, s.score());
}

#[test]
fn villages_go_up_in_any_order() {
    let mut s = Playsheet::new(dunholm());
    for road in [1, 3, 4, 6, 7, 8, 9, 10, 11] {
        s.build_road(road);
    }
    // Last village first, then the one beside the start, then a middle one
    assert!(s.can_build_village(6));
    s.build_village(6);
    assert!(s.can_build_village(2));
    s.build_village(2);
    assert!(s.can_build_village(5));
    s.build_village(5);
    assert!(s.villages[6] && s.villages[2] && s.villages[5]);
    assert!(!(s.villages[0] || s.villages[1]));
    assert!(!s.can_build_village(6), "one village once");
    // Still not reachable: no road touches it
    assert!(!s.can_build_village(0));
    assert!(!s.can_build_village(1));
    assert_eq!(5, s.score(), "three villages and the long road");
}

#[test]
fn castles_go_up_in_any_order() {
    let mut s = Playsheet::new(dunholm());
    for road in [1, 3, 4, 5, 6, 7, 8] {
        s.build_road(road);
    }
    assert!(s.can_build_city(3));
    s.build_city(3);
    assert!(s.can_build_city(2));
    s.build_city(2);
    assert!(!s.can_build_city(0));
    assert!(!s.can_build_city(1));
    s.build_road(2);
    assert!(s.can_build_city(0));
    assert_eq!(6, s.score(), "two castles and the long road");
}

#[test]
fn traders_go_up_in_any_order_once() {
    let mut s = Playsheet::new(dunholm());
    for tier in (1..=9).rev() {
        assert!(s.can_build_knight(tier), "trader {tier}");
        s.build_knight(tier);
        assert!(!s.can_build_knight(tier));
    }
    assert_eq!(9, s.trader_count());
    assert!(!s.can_build_knight(10));
    assert!(!s.can_build_knight(0));
}

/// Road 6 cannot be reached except along roads 1, 3 and 4.
#[test]
fn road_six_needs_roads_one_three_and_four() {
    let withheld: [&[usize]; 5] = [&[1, 3, 4], &[1], &[3], &[4], &[1, 3]];
    for cut in withheld {
        let mut s = Playsheet::new(dunholm());
        // Build every road that can be built, except the ones held back and road 6 itself
        let mut grew = true;
        while grew {
            grew = false;
            for road in 1..18 {
                let skip = road == 6 || cut.contains(&road);
                if !skip && s.can_build_road(road) {
                    s.build_road(road);
                    grew = true;
                }
            }
        }
        assert!(!s.can_build_road(6), "road 6 with {cut:?} held back");
        assert!(!s.roads[6]);
    }
    // And road 6 does open the moment road 4 is down
    let mut s = Playsheet::new(dunholm());
    s.build_road(1);
    s.build_road(3);
    assert!(!s.can_build_road(6));
    s.build_road(4);
    assert!(s.can_build_road(6));
}

// ---- bank slots and scoring -------------------------------------------------------------

#[test]
fn a_wildcard_slot_needs_both_its_traders() {
    let mut s = sheet(&[6]);
    assert!(!s.can_use_knight_resource(6));
    assert!(!s.has_wildcard());
    s.build_knight(7);
    assert!(s.can_use_knight_resource(6));
    assert!(s.has_wildcard());
    assert!(
        !s.can_use_knight_resource(7),
        "slot 7 still waits on the centre pair"
    );

    let mut centre = sheet(&[8]);
    assert!(!centre.can_use_knight_resource(7));
    centre.build_knight(9);
    assert!(centre.can_use_knight_resource(7));
    assert!(!centre.can_use_knight_resource(6));

    // A single trader opens a plain slot at once
    for tier in 1..=5 {
        assert!(sheet(&[tier]).can_use_knight_resource(tier), "slot {tier}");
    }
}

#[test]
fn one_castle_scores_two() {
    let mut s = Playsheet::new(dunholm());
    s.build_road(1);
    s.build_road(2);
    s.build_city(0);
    assert_eq!(2, s.score());
}

#[test]
fn the_long_road_bonus_is_road_six_and_survives_the_bank() {
    let mut s = sheet(&[6, 7]);
    for road in [1, 3, 4, 5, 2] {
        s.build_road(road);
    }
    assert_eq!(0, s.score(), "roads score nothing");
    s.build_road(6);
    assert_eq!(2, s.score());
    s.use_knight_resource(6);
    assert!(s.is_knight_resource_used(6));
    assert_eq!(2, s.score(), "spending slot 6 costs no points");
    s.build_road(7);
    assert_eq!(2, s.score(), "and only road 6 earns it");
}

#[test]
fn the_trader_bonus_comes_at_the_third_trader_whichever_they_are() {
    let sets = [[1, 2, 3], [9, 1, 5], [6, 7, 8], [7, 8, 9], [4, 9, 2]];
    for set in sets {
        let mut s = Playsheet::new(dunholm());
        s.build_knight(set[0]);
        s.build_knight(set[1]);
        assert_eq!(0, s.score(), "{set:?} two traders");
        s.build_knight(set[2]);
        assert_eq!(2, s.score(), "{set:?} three traders");
        for tier in 1..=9 {
            s.build_knight(tier);
        }
        assert_eq!(2, s.score(), "{set:?} nine traders");
    }
}

#[test]
fn empty_turns_cost_nothing() {
    let mut s = Playsheet::new(dunholm());
    s.turns_nothing_built = 12;
    assert_eq!(0, s.score());
}

#[test]
fn the_most_anyone_can_score_is_nineteen() {
    let mut s = Playsheet::new(dunholm());
    for road in 1..18 {
        s.build_road(road);
    }
    let mut grew = true;
    while grew {
        grew = false;
        for road in 1..18 {
            if s.can_build_road(road) {
                s.build_road(road);
                grew = true;
            }
        }
    }
    for i in 0..7 {
        s.build_village(i);
    }
    for i in 0..4 {
        s.build_city(i);
    }
    for tier in 1..=9 {
        s.build_knight(tier);
    }
    assert_eq!(7 + 8 + 2 + 2, s.score());
    // The same sum read from the tables, which is what caps the target setting
    assert_eq!(19, dunholm().max_score());
}

// ---- the early build prompts stand aside for the winning build ----------------------------

/// A Dunholm game on its first roll with these dice, every castle up (8 points).
fn castles_up(values: &[DiceValue]) -> Game {
    let mut g = Game::new(dunholm());
    for i in 0..4 {
        g.playsheet.cities[i] = true;
    }
    g.rolls = 1;
    for (i, &v) in values.iter().enumerate() {
        g.dice[i].value = v;
    }
    g
}

#[test]
fn a_village_that_reaches_ten_wins_the_race() {
    let mut g = castles_up(&[Lumber, Brick, Wool, Grain]);
    g.playsheet.villages[6] = true;
    assert_eq!(9, g.playsheet.score());
    let plan = BuildPlan::compute(&g.dice, &g.playsheet, g.rolls);
    let chain = plan.chain_for(Kind::Village, 2).cloned();
    // A position affordable right now has a chain of one step, not none
    assert_eq!(1, chain.as_ref().unwrap().steps.len());
    assert!(g.would_forfeit_rolls());
    assert!(g.chain_reaches_target(chain.as_ref()));
    assert_eq!(9, g.playsheet.score(), "and the check built nothing");

    g.playsheet.villages[6] = false;
    assert!(
        !g.chain_reaches_target(chain.as_ref()),
        "one short of the target"
    );
    assert!(!g.chain_reaches_target(None), "nothing reaches it");
}

#[test]
fn the_third_traders_bonus_can_win_the_race() {
    let mut g = castles_up(&[Wool, Grain, Ore]);
    g.playsheet.build_knight(1);
    g.playsheet.build_knight(2);
    assert_eq!(8, g.playsheet.score());
    let plan = BuildPlan::compute(&g.dice, &g.playsheet, g.rolls);
    assert!(g.chain_reaches_target(plan.chain_for(Kind::Knight, 6)));

    // A chain of several steps counts every one of them
    g.playsheet.traders = vec![false; dunholm().traders.len() + 1];
    let mut steps = vec![
        Step::new(Kind::Knight, 3),
        Step::new(Kind::Knight, 4),
        Step::new(Kind::Knight, 5),
    ];
    assert!(g.chain_reaches_target(Some(&Chain::new(steps.clone(), false, false, 0))));
    steps.remove(2);
    assert!(!g.chain_reaches_target(Some(&Chain::new(steps, false, false, 0))));
}

#[test]
fn the_winning_build_is_judged_against_the_games_own_target() {
    let mut g = castles_up(&[Lumber, Brick, Wool, Grain]);
    g.playsheet.villages[6] = true;
    let chain = BuildPlan::compute(&g.dice, &g.playsheet, g.rolls)
        .chain_for(Kind::Village, 2)
        .cloned();
    assert!(g.chain_reaches_target(chain.as_ref()));
    g.set_race_target(11);
    assert!(
        !g.chain_reaches_target(chain.as_ref()),
        "ten is no longer enough"
    );
}

// ---- the target setting -------------------------------------------------------------------

#[test]
fn a_new_target_moves_the_game_in_progress_only_when_it_is_still_ahead() {
    let mut g = Game::new(dunholm());
    assert_eq!(10, g.target_score);
    g.playsheet.cities[0] = true;
    g.playsheet.cities[1] = true;
    g.playsheet.cities[2] = true;
    assert_eq!(6, g.playsheet.score());

    g.set_race_target(15);
    assert_eq!(15, g.target_score);
    g.set_race_target(7);
    assert_eq!(7, g.target_score, "lowered, but still ahead of the score");
    g.set_race_target(4);
    assert_eq!(
        7, g.target_score,
        "already passed: the game would end on the spot, so it keeps its target"
    );
    assert!(!g.is_game_done());

    // The next game starts with the setting, clamped to what the island can give
    g.reset();
    assert_eq!(4, g.target_score);
    g.set_race_target(40);
    assert_eq!(19, g.target_score);
    g.reset();
    assert_eq!(19, g.target_score);
}

#[test]
fn a_finished_game_keeps_its_target() {
    let mut g = Game::new(dunholm());
    g.set_race_target(8);
    for i in 0..4 {
        g.playsheet.cities[i] = true;
    }
    assert!(g.is_game_done());
    g.set_race_target(12);
    assert_eq!(8, g.target_score);
    assert!(g.is_game_done());
}

#[test]
fn only_a_race_to_the_islands_own_target_goes_on_the_top_scores() {
    let mut g = Game::new(dunholm());
    for i in 0..4 {
        g.playsheet.cities[i] = true;
    }
    g.playsheet.villages[0] = true;
    g.playsheet.villages[1] = true;
    g.turns_taken = 9;
    assert_eq!(10, g.playsheet.score());
    assert_eq!(9, g.result_to_record());

    let mut other = Game::new(dunholm());
    other.set_race_target(8);
    for i in 0..4 {
        other.playsheet.cities[i] = true;
    }
    other.turns_taken = 6;
    assert!(other.is_game_done());
    assert_eq!(0, other.result_to_record());
}

#[test]
fn tarnholm_is_no_race_so_no_build_wins_it() {
    let g = Game::new(tarnholm());
    let chain = Chain::new(vec![Step::new(Kind::Road, 1)], false, false, 0);
    assert!(!g.chain_reaches_target(Some(&chain)));
}

// ---- paying from the bank: wildcards ------------------------------------------------------

#[test]
fn a_build_missing_one_resource_spends_one_wildcard_slot() {
    // Traders 6 and 7 open slot 6; slot 7 stays shut. A road is Brick and Lumber.
    let mut s = sheet(&[6, 7]);
    let mut d = dice(&[Brick]);
    assert_eq!(BuildStatus::WithKnight, status(RESOURCES_ROAD, &d, &s));
    pay(RESOURCES_ROAD, &mut d, &mut s);
    assert!(!s.resources_avail[6]);
    assert!(s.resources_avail[7], "the shut slot is untouched");
    assert_eq!(0, usable(&d[..1]));
}

#[test]
fn a_build_missing_one_resource_with_two_wildcards_open_spends_only_one() {
    let mut s = sheet(&[6, 7, 8, 9]);
    let mut d = dice(&[Brick]);
    pay(RESOURCES_ROAD, &mut d, &mut s);
    assert!(!s.resources_avail[6], "the lowest slot pays");
    assert!(s.resources_avail[7]);
}

#[test]
fn a_build_missing_two_resources_spends_both_wildcard_slots() {
    let mut s = sheet(&[6, 7, 8, 9]);
    let mut d = dice(&[]);
    assert_eq!(
        BuildStatus::WithKnight,
        status(RESOURCES_ROAD, &d, &s),
        "offered"
    );
    pay(RESOURCES_ROAD, &mut d, &mut s);
    assert!(!s.resources_avail[6], "paid");
    assert!(!s.resources_avail[7], "paid");
    // Nothing is left to pay a second road
    assert_eq!(BuildStatus::None, status(RESOURCES_ROAD, &d, &s));
}

#[test]
fn a_build_missing_three_is_not_offered_on_two_wildcards() {
    let s = sheet(&[6, 7, 8, 9]);
    assert_eq!(
        BuildStatus::None,
        status(RESOURCES_VILLAGE, &dice(&[Lumber]), &s)
    );
}

#[test]
fn wildcards_and_exact_slots_and_dice_share_a_village() {
    // Village: Lumber, Brick, Wool, Grain. Lumber from the dice, Wool from its slot, Brick and
    // Grain from the two wildcards.
    let mut s = sheet(&[3, 6, 7, 8, 9]);
    let mut d = dice(&[Lumber]);
    assert_eq!(BuildStatus::WithKnight, status(RESOURCES_VILLAGE, &d, &s));
    pay(RESOURCES_VILLAGE, &mut d, &mut s);
    assert!(!d[0].is_usable());
    assert!(!s.resources_avail[3], "wool slot");
    assert!(!s.resources_avail[6], "first wildcard");
    assert!(!s.resources_avail[7], "second wildcard");
}

#[test]
fn gold_pays_what_wildcards_leave() {
    // Lumber on the dice and a pair of gold covers one more; two wildcards cover the rest
    let mut s = sheet(&[6, 7, 8, 9]);
    let mut d = dice(&[Lumber, DiceValue::Gold, DiceValue::Gold]);
    assert_eq!(BuildStatus::WithKnight, status(RESOURCES_VILLAGE, &d, &s));
    pay(RESOURCES_VILLAGE, &mut d, &mut s);
    assert_eq!(3, 6 - usable(&d), "every die spent");
    assert!(!s.resources_avail[6]);
    assert!(!s.resources_avail[7]);
}

/// The turn end hands back resources the turn set aside. With two "?" slots, a spent one must
/// not come back just because the other is still open.
#[test]
fn a_spent_wildcard_stays_spent_at_the_next_turn() {
    let mut g = Game::new(dunholm());
    for tier in 6..=9 {
        g.playsheet.build_knight(tier);
    }
    g.rolls = 1;
    g.dice[0].value = Brick;
    g.build_road(1);
    assert!(g.playsheet.roads[1]);
    assert!(
        !g.playsheet.resources_avail[6],
        "slot 6 paid for the lumber"
    );
    assert!(g.playsheet.resources_avail[7]);
    g.new_turn(false);
    assert!(!g.playsheet.resources_avail[6], "and is still spent");
    assert!(g.playsheet.resources_avail[7]);
}

// ---- paying from the bank: one banked resource pays once ----------------------------------

#[test]
fn a_banked_grain_pays_for_one_grain_on_dunholm() {
    // Castle: Grain, Grain, Ore, Ore, Ore. Ore on three dice, one grain on a die, one in the
    // bank. Dunholm spends the grain slot for the second grain.
    let mut s = sheet(&[2]);
    let mut d = dice(&[Ore, Ore, Ore, Grain]);
    assert_eq!(BuildStatus::WithKnight, status(RESOURCES_CITY, &d, &s));
    pay(RESOURCES_CITY, &mut d, &mut s);
    assert!(!s.resources_avail[2]);
    assert_eq!(2, usable(&d), "three ore and a grain gone");
}

#[test]
fn two_grain_needed_one_banked_is_not_affordable_without_a_wildcard() {
    // Both grain from a single banked grain: not offered, not paid
    let s = sheet(&[2]);
    assert_eq!(
        BuildStatus::None,
        status(RESOURCES_CITY, &dice(&[Ore, Ore, Ore]), &s)
    );
}

#[test]
fn the_second_grain_of_a_castle_comes_from_a_wildcard_not_from_nothing() {
    let mut s = sheet(&[2, 6, 7]);
    let mut d = dice(&[Ore, Ore, Ore]);
    assert_eq!(BuildStatus::WithKnight, status(RESOURCES_CITY, &d, &s));
    pay(RESOURCES_CITY, &mut d, &mut s);
    assert!(!s.resources_avail[2], "the grain slot");
    assert!(
        !s.resources_avail[6],
        "and the wildcard for the second grain"
    );
}

#[test]
fn tarnholm_still_gets_its_free_second_grain() {
    // The old quirk, kept for Tarnholm: one banked grain pays both grain of a castle
    let mut s = Playsheet::new(tarnholm());
    s.set_trader_count(2);
    let mut d = dice(&[Ore, Ore, Ore, Grain]);
    pay(RESOURCES_CITY, &mut d, &mut s);
    assert!(!s.resources_avail[2]);
}

// ---- offered is paid ----------------------------------------------------------------------

/// On random dice and random banks, whenever a build is offered it is paid in full: the
/// resources it needs equal the dice spent, plus one per pair of gold, plus the bank slots
/// spent. Dunholm only, since Tarnholm's free grain is a deliberate quirk.
#[test]
fn whatever_is_offered_is_exactly_paid() {
    let recipes = [
        RESOURCES_ROAD,
        RESOURCES_VILLAGE,
        RESOURCES_KNIGHT,
        RESOURCES_CITY,
    ];
    let mut offered = 0;
    let mut bank_paid = 0;
    let available = |s: &Playsheet| {
        (1..=s.bank_size())
            .filter(|&i| s.resources_avail[i])
            .count()
    };
    let usable_gold = |d: &[dayapp::rules::Dice]| {
        d.iter()
            .filter(|d| d.is_usable() && d.value == DiceValue::Gold)
            .count()
    };
    for seed in 0..3000u64 {
        let mut random = Rng::new(seed);
        let mut s = Playsheet::new(dunholm());
        for tier in 1..=9 {
            if random.next_bool() {
                s.build_knight(tier);
            }
        }
        for slot in 1..=7 {
            if random.below(4) == 0 {
                s.resources_avail[slot] = false;
            }
        }
        let mut d: Vec<dayapp::rules::Dice> = (0..6)
            .map(|_| {
                let mut die = dayapp::rules::Dice::new();
                die.value = if random.below(8) == 0 {
                    DiceValue::None
                } else {
                    DiceValue::FACES[random.below(6) as usize]
                };
                die.used = random.below(8) == 0;
                die
            })
            .collect();
        let recipe = recipes[random.below(4) as usize];
        let st = status(recipe, &d, &s);
        if st == BuildStatus::None {
            continue;
        }
        offered += 1;
        let bank_before = available(&s);
        let dice_before = usable(&d);
        let gold_before = usable_gold(&d);
        pay(recipe, &mut d, &mut s);
        let bank_spent = bank_before - available(&s);
        let gold_spent = gold_before - usable_gold(&d);
        let plain_spent = dice_before - usable(&d) - gold_spent;
        assert_eq!(0, gold_spent % 2, "seed {seed} gold is spent in pairs");
        assert_eq!(
            recipe.len(),
            plain_spent + gold_spent / 2 + bank_spent,
            "seed {seed}"
        );
        if bank_spent > 0 {
            bank_paid += 1;
            assert_eq!(
                BuildStatus::WithKnight,
                st,
                "seed {seed} a bank spend was announced"
            );
        }
    }
    assert!(offered > 500, "the run offered builds: {offered}");
    assert!(bank_paid > 200, "and some paid from the bank: {bank_paid}");
}
