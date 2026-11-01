// SPDX-License-Identifier: GPL-3.0-or-later
//! The decisions the old game view and its pieces made, now in the rules: which colour a spot
//! is offered in, how it glows, which questions a tap asks first, what the roll button says
//! and does, which spot a tap on the board means, and what the tutorial sees. These had no
//! tests of their own in the Java app; the expectations follow UIEntity, UIDiceRoll and
//! GameView line by line.

mod common;

use common::*;
use dayapp::rules::high_scores::{EARLY_PROMPT_CONSERVATIVE, EARLY_PROMPT_OFF};
use dayapp::rules::view_model::{self, HIGHLIGHT, HIGHLIGHT_KNIGHT_CHAIN, HIGHLIGHT_WILDCARD};
use dayapp::rules::{
    BoardPieces, BuildPrompt, DiceValue, Game, GlowTier, Kind, Offer, OfferPay, Rng, RollLabel,
    RollPress,
};

use DiceValue::{Brick, Gold, Grain, Lumber, Ore, Wool};

#[test]
fn the_offer_colours_are_the_old_ones() {
    assert_eq!(0x6423B40F, HIGHLIGHT);
    assert_eq!(0x552846E6, view_model::HIGHLIGHT_KNIGHT);
    assert_eq!(0x8D2846E6, HIGHLIGHT_WILDCARD);
    // 85 * 55 / 100 = 46
    assert_eq!(0x2E2846E6, HIGHLIGHT_KNIGHT_CHAIN);
}

#[test]
fn a_spot_is_offered_in_the_colour_of_what_pays_for_it() {
    let mut game = Game::new(tarnholm());
    assert_eq!(
        None,
        game.offer(Kind::Road, 1),
        "nothing before the first roll"
    );
    give_dice(&mut game, &[Brick, Lumber]);
    let offer = game.offer(Kind::Road, 1).unwrap();
    assert_eq!(
        Offer {
            pay: OfferPay::Dice,
            now: true
        },
        offer
    );
    assert_eq!(HIGHLIGHT, offer.argb());
    assert_eq!(
        None,
        game.offer(Kind::Road, 0),
        "the start road is never offered"
    );

    // Brick on the dice, lumber from slot 4: blue
    let mut banked = Game::new(tarnholm());
    banked.playsheet.set_trader_count(4);
    banked.playsheet.resources_avail[6] = false;
    give_dice(&mut banked, &[Brick]);
    assert_eq!(OfferPay::Bank, banked.offer(Kind::Road, 1).unwrap().pay);
    // Road 2 comes after road 1, and by then the bank's one lumber is spent: not offered
    assert!(banked.offer(Kind::Road, 2).is_none());

    // Only the "?" left: the heavier blue
    let mut wild = Game::new(tarnholm());
    wild.playsheet.set_trader_count(6);
    for slot in 1..=5 {
        wild.playsheet.resources_avail[slot] = false;
    }
    give_dice(&mut wild, &[Brick]);
    let offer = wild.offer(Kind::Road, 1).unwrap();
    assert_eq!(OfferPay::Wildcard, offer.pay);
    assert_eq!(HIGHLIGHT_WILDCARD, offer.argb());
}

#[test]
fn a_spot_that_needs_another_build_first_is_offered_lighter() {
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[Brick, Lumber, Brick, Lumber, Brick, Lumber]);
    let offer = game.offer(Kind::Road, 4).unwrap();
    assert!(!offer.now);
    assert_eq!(view_model::HIGHLIGHT_CHAIN, offer.argb());
}

#[test]
fn glow_tiers_follow_the_map_with_the_dice_left_out() {
    let mut game = Game::new(tarnholm());
    // Road 1 hangs off the start road; road 2 needs road 1
    assert_eq!(GlowTier::Full, game.glow_tier(Kind::Road, 1));
    assert_eq!(GlowTier::None, game.glow_tier(Kind::Road, 2));
    assert_eq!(GlowTier::None, game.glow_tier(Kind::Road, 0), "built");
    // Village 0 sits on the start road and is first in order
    assert_eq!(GlowTier::Full, game.glow_tier(Kind::Village, 0));
    // Village 1 needs road 3 and village 0: with road 3 down it is waiting for its turn
    assert_eq!(GlowTier::None, game.glow_tier(Kind::Village, 1));
    game.playsheet.roads[1] = true;
    game.playsheet.roads[3] = true;
    assert_eq!(GlowTier::Waiting, game.glow_tier(Kind::Village, 1));
    // Traders fade after the next
    assert_eq!(GlowTier::Full, game.glow_tier(Kind::Knight, 1));
    assert_eq!(GlowTier::Weak, game.glow_tier(Kind::Knight, 2));
    assert_eq!(GlowTier::Faint, game.glow_tier(Kind::Knight, 3));
    assert_eq!(GlowTier::None, game.glow_tier(Kind::Knight, 4));
    game.playsheet.build_knight(1);
    assert_eq!(GlowTier::None, game.glow_tier(Kind::Knight, 1));
    assert_eq!(GlowTier::Full, game.glow_tier(Kind::Knight, 2));
    assert_eq!(0.5, view_model::glow_strength(GlowTier::Weak));
    assert_eq!(0.2, view_model::glow_strength(GlowTier::Faint));

    // On Dunholm nothing is ordered: every trader glows in full, nothing waits
    let dun = Game::new(dunholm());
    for tier in 1..=9 {
        assert_eq!(GlowTier::Full, dun.glow_tier(Kind::Knight, tier));
    }
    assert_eq!(GlowTier::None, dun.glow_tier(Kind::Village, 0));
}

#[test]
fn an_early_build_asks_unless_the_tap_spends_enough_dice() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[Brick, Lumber, Wool, Ore, Ore, Wool]);
    // A road on the first roll spends two dice: ask
    assert_eq!(
        vec![BuildPrompt::EarlyBuild { rolls_made: 1 }],
        game.build_prompts(Kind::Road, 1)
    );
    game.rolls = 2;
    assert_eq!(
        vec![BuildPrompt::EarlyBuild { rolls_made: 2 }],
        game.build_prompts(Kind::Road, 1)
    );
    // On the third roll nothing is forfeit
    game.rolls = 3;
    assert!(game.build_prompts(Kind::Road, 1).is_empty());
    // The setting off never asks
    game.rolls = 1;
    game.early_build_prompt = EARLY_PROMPT_OFF;
    assert!(game.build_prompts(Kind::Road, 1).is_empty());

    // A village and the road its two leftover dice pay for spend all six: no question, even
    // for the cautious setting
    let mut all = Game::new(tarnholm());
    give_dice(&mut all, &[Lumber, Brick, Wool, Grain, Brick, Lumber]);
    all.early_build_prompt = EARLY_PROMPT_CONSERVATIVE;
    assert!(all.build_prompts(Kind::Village, 0).is_empty());
}

#[test]
fn the_winning_build_asks_nothing() {
    let mut game = Game::new(dunholm());
    for i in 0..4 {
        game.playsheet.cities[i] = true;
    }
    game.playsheet.villages[6] = true;
    give_dice(&mut game, &[Lumber, Brick, Wool, Grain]);
    assert!(game.would_forfeit_rolls());
    assert!(game.build_prompts(Kind::Village, 2).is_empty());
}

#[test]
fn starting_a_second_pair_asks_after_the_early_build_question() {
    let mut game = Game::new(dunholm());
    game.playsheet.build_knight(6);
    give_dice(&mut game, &[Wool, Grain, Ore]);
    assert_eq!(
        vec![
            BuildPrompt::EarlyBuild { rolls_made: 1 },
            BuildPrompt::SecondPair
        ],
        game.build_prompts(Kind::Knight, 8)
    );
    game.rolls = 3;
    assert_eq!(
        vec![BuildPrompt::SecondPair],
        game.build_prompts(Kind::Knight, 8)
    );
    assert!(
        game.build_prompts(Kind::Knight, 7).is_empty(),
        "finishing the pair is no new pair"
    );
}

#[test]
fn the_roll_button_says_what_it_does_next() {
    let mut game = Game::new(tarnholm());
    assert_eq!(RollLabel::FirstRoll, game.roll_label());
    assert_eq!(RollPress::Roll, game.roll_press());
    game.rolls = 1;
    assert_eq!(RollLabel::SecondRoll, game.roll_label());
    game.rolls = 2;
    assert_eq!(RollLabel::ThirdRoll, game.roll_label());

    // Rolls spent with a road affordable from dice: a real choice
    give_dice(&mut game, &[Brick, Lumber]);
    game.rolls = 3;
    assert_eq!(RollLabel::Done, game.roll_label());
    assert_eq!(RollPress::ConfirmEndTurn, game.roll_press());
    let outcome = game.press_roll_button();
    assert_eq!(RollPress::ConfirmEndTurn, outcome.press);
    assert_eq!(
        3, game.rolls,
        "the question is the UI's; nothing happened yet"
    );
    game.end_turn_prompt = false;
    assert_eq!(RollPress::EndTurnAndRoll, game.roll_press());

    // Nothing to build and nothing to roll: the button ends the turn and rolls
    let mut idle = Game::new(tarnholm());
    idle.rolls = 3;
    assert_eq!(RollLabel::NextTurn, idle.roll_label());
    let outcome = idle.press_roll_button();
    assert_eq!(RollPress::EndTurnAndRoll, outcome.press);
    assert!(outcome.rolled);
    assert_eq!(1, idle.turns_taken);
    assert_eq!(1, idle.rolls);

    // The last turn reads Finish; the end reads Play again
    let mut last = Game::new(tarnholm());
    empty_turns(&mut last, 14);
    last.rolls = 3;
    assert_eq!(RollLabel::Finish, last.roll_label());
    last.new_turn(false);
    assert!(last.is_game_done());
    assert_eq!(RollLabel::PlayAgain, last.roll_label());
    assert_eq!(RollPress::NewGame, last.roll_press());
    let outcome = last.press_roll_button();
    assert!(!outcome.rolled);
    assert!(!last.is_game_done());
    assert_eq!(0, last.turns_taken);
}

#[test]
fn the_roll_button_is_locked_for_a_moment_after_the_game_ends() {
    let clock = std::rc::Rc::new(std::cell::Cell::new(10_000u64));
    let mut game = Game::new(tarnholm());
    let c = clock.clone();
    game.clock = Some(Box::new(move || c.get()));
    empty_turns(&mut game, 15);
    assert!(game.is_game_done());
    assert_eq!(10_000, game.game_over_at);
    assert!(game.is_roll_locked());
    assert_eq!(RollPress::Locked, game.press_roll_button().press);
    assert!(
        game.is_game_done(),
        "a bump does not wipe the finished board"
    );
    clock.set(10_000 + view_model::ROLL_LOCKOUT_MS);
    assert!(!game.is_roll_locked());
    assert_eq!(RollPress::NewGame, game.press_roll_button().press);
}

#[test]
fn a_tap_means_the_offered_spot_under_it() {
    let pieces = BoardPieces::for_map(tarnholm());
    assert_eq!(17, pieces.roads.len());
    assert_eq!(6 + 4 + 6, pieces.standing.len());
    // The painter's order: further down the island comes later
    for pair in pieces.standing.windows(2) {
        assert!(pair[0].foot_y() <= pair[1].foot_y());
    }
    let mut game = Game::new(tarnholm());
    let b = game.board;
    // Nothing on offer before a roll: a tap means nothing
    assert!(
        pieces
            .piece_at(&game, b.road_anchor[1].x, b.road_anchor[1].y)
            .is_none()
    );
    give_dice(&mut game, &[Brick, Lumber]);
    let p = pieces
        .piece_at(&game, b.road_anchor[1].x, b.road_anchor[1].y)
        .unwrap();
    assert_eq!((Kind::Road, 1), (p.kind, p.index));
    // Near either end of the road still counts
    let end = b.road_ends[1][1];
    let p = pieces.piece_at(&game, end.x - 3, end.y - 3).unwrap();
    assert_eq!((Kind::Road, 1), (p.kind, p.index));
    // Far out at sea, nothing
    assert!(pieces.piece_at(&game, 5, 5).is_none());

    // A building's own body wins over the road ends under it
    let mut village = Game::new(tarnholm());
    give_dice(&mut village, &[Lumber, Brick, Wool, Grain, Brick, Lumber]);
    let v = b.village[0];
    let p = pieces.piece_at(&village, v.x, v.y - 4).unwrap();
    assert_eq!((Kind::Village, 0), (p.kind, p.index));
    assert_eq!(
        Some(p),
        pieces.offered_spot(&village),
        "a building beats a road for the tutorial"
    );
}

#[test]
fn a_tap_on_a_shared_resource_hires_neither_trader() {
    let pieces = BoardPieces::for_map(dunholm());
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[Wool, Grain, Ore]);
    let resting = pieces.resting_shared(&game);
    assert_eq!(2, resting.len());
    let (slot, at) = resting[0];
    assert_eq!(6, slot);
    assert!(pieces.piece_at(&game, at.x + 6, at.y + 6).is_none());
    // Its traders are still there to tap on their own bodies
    let upper = pieces
        .standing
        .iter()
        .find(|p| p.kind == Kind::Knight && p.index == 6)
        .unwrap();
    let p = pieces
        .piece_at(&game, upper.anchor.x, upper.anchor.y - upper.body_rise())
        .unwrap();
    assert_eq!((Kind::Knight, 6), (p.kind, p.index));
    // Once both stand the resource leaves the board
    game.playsheet.build_knight(6);
    game.playsheet.build_knight(7);
    assert_eq!(1, pieces.resting_shared(&game).len());
    assert_eq!(vec![6], game.banked_slots());
    assert_eq!(DiceValue::Any, game.bank_resource(6));
}

#[test]
fn the_tutorial_sees_the_game() {
    let mut game = Game::with_rng(tarnholm(), Rng::new(7));
    let s = game.tutorial_snapshot(0);
    assert_eq!(0, s.rolls);
    assert!(!s.offered);
    assert!(!s.roll_ends_turn);
    give_dice(&mut game, &[Gold, Gold, Brick, Lumber]);
    game.dice[0].held = true;
    game.playsheet.set_trader_count(2);
    let s = game.tutorial_snapshot(1);
    assert_eq!(1, s.rolls);
    assert!(s.any_held);
    assert!(s.offered);
    assert_eq!(2, s.gold_dice);
    assert_eq!(1, s.mini_dice, "two banked, one still flying in");
    game.build(Kind::Road, 1);
    let s = game.tutorial_snapshot(0);
    assert!(s.built_this_turn && s.can_undo && s.roll_ends_turn);
    assert!(game.die_stays(3));
}

#[test]
fn a_hire_reports_its_animation() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[Wool, Grain, Ore, Wool, Grain, Ore]);
    game.build(Kind::Knight, 2);
    let hires = game.take_hire_events();
    assert_eq!(2, hires.len());
    assert_eq!(
        (1, 1, true, 0),
        (
            hires[0].tier,
            hires[0].slot,
            hires[0].completes,
            hires[0].delay_ms
        )
    );
    assert_eq!(
        (2, 2, true, 150),
        (
            hires[1].tier,
            hires[1].slot,
            hires[1].completes,
            hires[1].delay_ms
        )
    );
    assert!(game.take_hire_events().is_empty());

    // The first of a pair banks nothing yet
    let mut dun = Game::new(dunholm());
    give_dice(&mut dun, &[Wool, Grain, Ore]);
    dun.build(Kind::Knight, 6);
    let hires = dun.take_hire_events();
    assert_eq!(1, hires.len());
    assert!(!hires[0].completes);
}

#[test]
fn every_change_moves_the_revision() {
    let mut game = Game::with_rng(tarnholm(), Rng::new(3));
    let r0 = game.revision();
    game.roll();
    let r1 = game.revision();
    assert!(r1 > r0);
    game.dice_touched(0);
    assert!(game.revision() > r1);
    let r2 = game.revision();
    assert!(!game.dice_touched(9));
    assert_eq!(r2, game.revision(), "nothing changed");
}

#[test]
fn a_seeded_game_rolls_the_same_dice_every_time() {
    let mut a = Game::with_rng(tarnholm(), Rng::new(42));
    let mut b = Game::with_rng(tarnholm(), Rng::new(42));
    for _ in 0..3 {
        a.roll();
        b.roll();
        assert_eq!(a.dice, b.dice);
    }
    assert!(a.dice.iter().all(|d| DiceValue::FACES.contains(&d.value)));
}

#[test]
fn a_roll_keeps_held_dice_and_swaps_unchanged_ones_where_it_can() {
    for seed in 0..200 {
        let mut game = Game::with_rng(tarnholm(), Rng::new(seed));
        game.roll();
        game.dice[0].held = true;
        let held = game.dice[0].value;
        game.roll();
        assert_eq!(held, game.dice[0].value, "seed {seed}");
        assert_eq!(2, game.rolls);
    }
}
