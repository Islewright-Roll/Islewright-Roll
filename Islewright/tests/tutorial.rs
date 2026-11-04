// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TutorialTest: which callout is due, step by step, as a player plays.

use dayapp::rules::tutorial::{self, Snapshot, Step, Tutorial};

/// The game as the tutorial sees it, with the moves a player makes.
struct Table {
    tutorial: Tutorial,
    rolls: i32,
    turns: i32,
    held: bool,
    offered: bool,
    built: bool,
    undo: bool,
    ends_turn: bool,
    gold: i32,
    mini: i32,
    classic: bool,
    over: bool,
}

impl Table {
    fn new(tutorial: Tutorial) -> Table {
        Table {
            tutorial,
            rolls: 0,
            turns: 0,
            held: false,
            offered: false,
            built: false,
            undo: false,
            ends_turn: false,
            gold: 0,
            mini: 0,
            classic: false,
            over: false,
        }
    }

    fn look(&mut self) -> Option<Step> {
        let s = Snapshot {
            rolls: self.rolls,
            turns_taken: self.turns,
            any_held: self.held,
            offered: self.offered,
            built_this_turn: self.built,
            can_undo: self.undo,
            roll_ends_turn: self.ends_turn && !self.over,
            gold_dice: self.gold,
            mini_dice: self.mini,
            classic_bank: self.classic,
            game_over: self.over,
        };
        self.tutorial.update(&s)
    }

    fn roll(&mut self) -> Option<Step> {
        self.rolls += 1;
        self.ends_turn = self.rolls >= 3 || self.built;
        self.look()
    }

    fn hold(&mut self) -> Option<Step> {
        self.held = true;
        self.look()
    }

    fn build(&mut self) -> Option<Step> {
        self.built = true;
        self.undo = true;
        self.ends_turn = true;
        self.offered = false;
        self.look()
    }

    fn take_back(&mut self) -> Option<Step> {
        self.built = false;
        self.undo = false;
        self.ends_turn = self.rolls >= 3;
        self.look()
    }

    /// The roll button when it ends the turn.
    fn end_turn(&mut self) -> Option<Step> {
        self.turns += 1;
        self.rolls = 0;
        self.held = false;
        self.built = false;
        self.undo = false;
        self.ends_turn = false;
        self.offered = false;
        self.gold = 0;
        self.look()
    }
}

/// The tour's steps up to the gold: Roll, Keep, the four costs, Build, Undo, End, Ship and Saved.
const FIRST: u32 = 0x3C7F;

/// Got it on each of the costs tour's four callouts, which come one after another.
fn through_costs(t: &mut Table) {
    for step in [
        Step::Costs,
        Step::CostVillage,
        Step::CostCastle,
        Step::CostTrader,
    ] {
        assert_eq!(Some(step), t.look());
        t.tutorial.got_it();
    }
}

fn fresh() -> Table {
    Table::new(Tutorial::new(true, 0))
}

fn reach_build(t: &mut Table) {
    t.look();
    t.roll();
    t.hold();
    through_costs(t);
    t.offered = true;
    assert_eq!(Some(Step::Build), t.look());
}

// ---- the order of 1 to 7 ------------------------------------------------------------------

#[test]
fn a_new_game_starts_with_the_roll_button() {
    assert_eq!(Some(Step::Roll), fresh().look());
}

#[test]
fn the_first_seven_steps_come_in_order() {
    let mut t = fresh();
    assert_eq!(Some(Step::Roll), t.look());
    assert_eq!(Some(Step::Keep), t.roll());
    assert_eq!(Some(Step::Costs), t.hold());
    t.offered = true;
    through_costs(&mut t);
    assert_eq!(Some(Step::Build), t.look());
    assert_eq!(Some(Step::Undo), t.build());
    t.tutorial.got_it();
    assert_eq!(Some(Step::End), t.look());
    // The second turn: the ship, then that the game saves itself
    assert_eq!(Some(Step::Ship), t.end_turn());
    t.tutorial.got_it();
    assert_eq!(Some(Step::Saved), t.look());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
    assert_eq!(FIRST, t.tutorial.seen_mask());
    assert!(t.tutorial.is_active());
}

#[test]
fn rolling_the_dice_ends_the_roll_callout() {
    let mut t = fresh();
    assert_eq!(Some(Step::Roll), t.look());
    assert!(!t.tutorial.is_seen(Step::Roll));
    t.roll();
    assert!(t.tutorial.is_seen(Step::Roll));
}

#[test]
fn keep_shows_after_the_first_roll_and_ends_on_a_hold() {
    let mut t = fresh();
    t.look();
    assert_eq!(Some(Step::Keep), t.roll());
    assert!(!t.tutorial.is_seen(Step::Keep));
    assert_eq!(Some(Step::Costs), t.hold());
    assert!(t.tutorial.is_seen(Step::Keep));
}

#[test]
fn keep_ends_on_the_second_roll_too() {
    let mut t = fresh();
    t.look();
    assert_eq!(Some(Step::Keep), t.roll());
    assert_eq!(Some(Step::Costs), t.roll());
    assert!(t.tutorial.is_seen(Step::Keep));
}

#[test]
fn keep_ends_when_the_first_roll_is_followed_by_a_build() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.offered = true;
    t.build();
    assert!(t.tutorial.is_seen(Step::Keep));
    assert!(t.tutorial.is_seen(Step::Build));
}

#[test]
fn costs_ends_on_the_next_roll() {
    let mut t = fresh();
    t.look();
    t.roll();
    assert_eq!(Some(Step::Costs), t.hold());
    assert_eq!(None, t.roll());
    assert!(t.tutorial.is_seen(Step::Costs));
}

#[test]
fn the_costs_tour_goes_entry_by_entry_on_got_it() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.hold();
    through_costs(&mut t);
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::CostTrader));
}

#[test]
fn rolling_during_the_costs_tour_ends_the_rest_of_it() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.hold();
    t.tutorial.got_it();
    assert_eq!(Some(Step::CostVillage), t.look());
    assert_eq!(None, t.roll());
    assert!(t.tutorial.is_seen(Step::CostTrader));
}

#[test]
fn costs_ends_on_a_build_and_the_undo_callout_follows() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.hold();
    t.offered = true;
    assert_eq!(Some(Step::Costs), t.look());
    assert_eq!(Some(Step::Undo), t.build());
    assert!(t.tutorial.is_seen(Step::Costs));
    assert!(t.tutorial.is_seen(Step::Build));
}

#[test]
fn build_waits_for_something_to_be_on_offer() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.hold();
    through_costs(&mut t);
    assert_eq!(None, t.look());
    t.offered = true;
    assert_eq!(Some(Step::Build), t.look());
    // The offer goes (the dice changed): the callout goes with it but is not used up
    t.offered = false;
    assert_eq!(None, t.look());
    assert!(!t.tutorial.is_seen(Step::Build));
    t.offered = true;
    assert_eq!(Some(Step::Build), t.look());
}

#[test]
fn build_needs_costs_to_be_done_first() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.hold();
    t.offered = true;
    assert_eq!(Some(Step::Costs), t.look());
}

#[test]
fn undo_shows_right_after_the_first_build_and_ends_on_got_it() {
    let mut t = fresh();
    reach_build(&mut t);
    assert_eq!(Some(Step::Undo), t.build());
    t.tutorial.got_it();
    assert!(t.tutorial.is_seen(Step::Undo));
    assert_eq!(Some(Step::End), t.look());
}

#[test]
fn undo_ends_when_the_build_is_taken_back() {
    let mut t = fresh();
    reach_build(&mut t);
    assert_eq!(Some(Step::Undo), t.build());
    t.offered = true;
    // Nothing follows: the build was done, the undo was done
    assert_eq!(None, t.take_back());
    assert!(t.tutorial.is_seen(Step::Undo));
    assert!(t.tutorial.is_seen(Step::Build));
}

#[test]
fn undo_ends_when_the_turn_ends_without_a_got_it() {
    let mut t = fresh();
    reach_build(&mut t);
    assert_eq!(Some(Step::Undo), t.build());
    t.end_turn();
    assert!(t.tutorial.is_seen(Step::Undo));
    // The player used the end turn button, so there is nothing left to say about it
    assert!(t.tutorial.is_seen(Step::End));
    assert_eq!(Some(Step::Ship), t.look());
}

#[test]
fn end_shows_when_the_rolls_are_used_up_with_nothing_to_build() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.roll();
    through_costs(&mut t);
    assert_eq!(Some(Step::End), t.roll());
    assert_eq!(Some(Step::Ship), t.end_turn());
    assert!(t.tutorial.is_seen(Step::End));
}

#[test]
fn end_waits_behind_a_build_callout_when_something_is_on_offer() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.roll();
    through_costs(&mut t);
    t.offered = true;
    t.roll();
    assert_eq!(Some(Step::Build), t.look());
}

#[test]
fn end_shows_after_a_build_once_undo_is_done() {
    let mut t = fresh();
    reach_build(&mut t);
    assert_eq!(Some(Step::Undo), t.build());
    t.tutorial.got_it();
    assert_eq!(Some(Step::End), t.look());
    assert_eq!(Some(Step::Ship), t.end_turn());
    assert!(t.tutorial.is_seen(Step::End));
}

#[test]
fn ship_shows_from_the_second_turn_and_ends_with_got_it_or_the_turn() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.end_turn();
    // Turn one ended after a single roll: nothing was built, nothing held
    assert!(t.tutorial.is_seen(Step::Keep));
    through_costs(&mut t);
    assert_eq!(Some(Step::Ship), t.look());
    t.tutorial.got_it();
    assert_eq!(Some(Step::Saved), t.look());

    let mut u = fresh();
    reach_build(&mut u);
    u.build();
    u.tutorial.got_it();
    assert_eq!(Some(Step::End), u.look());
    assert_eq!(Some(Step::Ship), u.end_turn());
    // Rolling does not take it away; the turn ending does
    assert_eq!(Some(Step::Ship), u.roll());
    assert_eq!(Some(Step::Ship), u.roll());
    assert_eq!(Some(Step::Ship), u.roll());
    // The turn ending takes the ship away, and that the game saves itself follows
    assert_eq!(Some(Step::Saved), u.end_turn());
    assert!(u.tutorial.is_seen(Step::Ship));
}

// ---- gold and the bank wait for their moment ----------------------------------------------

#[test]
fn gold_needs_two_gold_dice_and_waits_behind_another_callout() {
    let mut t = fresh();
    t.look();
    t.gold = 1;
    assert_eq!(Some(Step::Roll), t.look());
    t.gold = 2;
    // The roll callout is up: gold waits
    assert_eq!(Some(Step::Roll), t.look());
    t.tutorial.got_it();
    assert_eq!(Some(Step::Gold), t.look());
}

#[test]
fn a_single_gold_shows_nothing() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 3;
    t.rolls = 1;
    t.gold = 1;
    t.look();
    assert_eq!(None, t.look());
}

#[test]
fn gold_is_done_when_the_gold_is_gone() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 3;
    t.rolls = 1;
    t.look();
    t.gold = 2;
    assert_eq!(Some(Step::Gold), t.look());
    t.gold = 0;
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::Gold));
    t.gold = 2;
    assert_eq!(None, t.look());
}

#[test]
fn gold_does_not_bump_a_callout_that_is_up() {
    let mut t = fresh();
    t.look();
    t.roll();
    t.gold = 2;
    assert_eq!(Some(Step::Keep), t.look());
    assert_eq!(Some(Step::Costs), t.hold());
    through_costs(&mut t);
    assert_eq!(Some(Step::Gold), t.look());
}

#[test]
fn bank_shows_while_a_mini_die_is_waiting_and_ends_on_got_it() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 4;
    t.look();
    t.mini = 1;
    assert_eq!(Some(Step::Bank), t.look());
    assert_eq!(Some(Step::Bank), t.roll());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::Bank));
}

#[test]
fn bank_ends_when_the_mini_die_is_spent() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 4;
    t.look();
    t.mini = 1;
    assert_eq!(Some(Step::Bank), t.look());
    t.mini = 0;
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::Bank));
}

#[test]
fn gold_comes_before_bank_and_each_waits_for_the_one_up() {
    let mut t = Table::new(Tutorial::new(true, 0));
    t.look();
    t.gold = 2;
    t.mini = 1;
    assert_eq!(Some(Step::Roll), t.look());
    t.tutorial.got_it();
    assert_eq!(Some(Step::Gold), t.look());
    t.tutorial.got_it();
    assert_eq!(Some(Step::Bank), t.look());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
}

#[test]
fn classic_mode_shows_the_drag_callout_instead_and_it_ends_when_the_die_is_dragged() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 4;
    t.classic = true;
    t.look();
    t.mini = 1;
    assert_eq!(Some(Step::BankClassic), t.look());
    t.mini = 0;
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::BankClassic));
    assert!(!t.tutorial.is_seen(Step::Bank));
}

#[test]
fn a_player_who_learned_the_usual_bank_still_meets_the_drag_once_classic_is_on() {
    let mut t = Table::new(Tutorial::new(true, FIRST | Step::Gold.bit()));
    t.turns = 4;
    t.look();
    t.mini = 1;
    assert_eq!(Some(Step::Bank), t.look());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
    // The tour is over without waiting for the drag, which is a standing hint
    assert!(!t.tutorial.is_active());
    t.classic = true;
    assert_eq!(Some(Step::BankClassic), t.look());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
    assert_eq!(tutorial::KNOWN, t.tutorial.seen_mask());
}

#[test]
fn skipping_the_tour_leaves_the_drag_hint_for_when_classic_mode_banks_a_resource() {
    let mut t = Table::new(Tutorial::new(true, FIRST));
    t.turns = 4;
    t.classic = true;
    t.look();
    t.tutorial.skip();
    assert!(!t.tutorial.is_active());
    t.mini = 1;
    assert_eq!(Some(Step::BankClassic), t.look());
    // Skip with the hint up dismisses it for good
    t.tutorial.skip();
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::BankClassic));
}

#[test]
fn a_player_who_finished_the_tour_long_ago_meets_the_drag_hint_once() {
    let mut t = Table::new(Tutorial::new(false, tutorial::ALL));
    t.turns = 9;
    t.mini = 1;
    assert_eq!(None, t.look());
    t.classic = true;
    assert_eq!(Some(Step::BankClassic), t.look());
    assert!(!t.tutorial.take_dirty());
    t.mini = 0;
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_seen(Step::BankClassic));
    assert!(t.tutorial.take_dirty());
    t.mini = 1;
    assert_eq!(None, t.look());
}

// ---- skip, finish, game over ----------------------------------------------------------------

#[test]
fn skip_ends_everything_for_good() {
    let mut t = fresh();
    assert_eq!(Some(Step::Roll), t.look());
    t.tutorial.skip();
    assert!(!t.tutorial.is_active());
    assert!(t.tutorial.take_dirty());
    assert_eq!(None, t.look());
    assert_eq!(None, t.roll());
    t.gold = 2;
    t.mini = 2;
    assert_eq!(None, t.look());
}

#[test]
fn skip_works_with_no_callout_up() {
    let mut tutorial = Tutorial::new(true, 0);
    tutorial.skip();
    assert!(!tutorial.is_active());
}

#[test]
fn a_tutorial_that_is_off_shows_nothing() {
    let mut t = Table::new(Tutorial::new(false, 0));
    assert_eq!(None, t.look());
    t.gold = 2;
    assert_eq!(None, t.look());
}

#[test]
fn the_tutorial_is_over_when_every_tour_step_is_seen() {
    let mut t = Table::new(Tutorial::new(true, tutorial::ALL & !Step::Bank.bit()));
    t.turns = 5;
    assert_eq!(None, t.look());
    assert!(t.tutorial.is_active());
    t.mini = 1;
    assert_eq!(Some(Step::Bank), t.look());
    t.tutorial.got_it();
    assert_eq!(None, t.look());
    assert!(!t.tutorial.is_active());
    assert_eq!(tutorial::ALL, t.tutorial.seen_mask());
    assert!(t.tutorial.take_dirty());
}

#[test]
fn nothing_shows_once_the_game_is_over() {
    let mut t = fresh();
    assert_eq!(Some(Step::Roll), t.look());
    t.over = true;
    assert_eq!(None, t.look());
    t.gold = 2;
    t.mini = 3;
    assert_eq!(None, t.look());
    assert_eq!(None, t.tutorial.current());
}

#[test]
fn a_new_game_after_the_end_goes_on_where_it_left_off() {
    let mut t = fresh();
    reach_build(&mut t);
    t.build();
    t.tutorial.got_it();
    t.look();
    t.end_turn();
    t.tutorial.got_it();
    assert_eq!(Some(Step::Saved), t.look());
    t.tutorial.got_it();
    assert_eq!(FIRST, t.tutorial.seen_mask());
    t.turns = 15;
    t.over = true;
    assert_eq!(None, t.look());
    // Play again: the turn count is back to zero and the rolls too
    t.turns = 0;
    t.rolls = 0;
    t.over = false;
    t.built = false;
    t.undo = false;
    t.ends_turn = false;
    t.held = false;
    assert_eq!(None, t.look());
    assert_eq!(FIRST, t.tutorial.seen_mask());
}

#[test]
fn a_restart_mid_tutorial_shows_the_next_pending_step_after_it() {
    let mut t = fresh();
    t.look();
    t.roll();
    assert_eq!(Some(Step::Keep), t.look());
    // Restart from the menu: back to turn one, no rolls. Roll is done, Keep is not.
    t.rolls = 0;
    t.held = false;
    t.ends_turn = false;
    assert_eq!(None, t.look());
    assert_eq!(Some(Step::Keep), t.roll());
}

// ---- resuming --------------------------------------------------------------------------------

#[test]
fn a_saved_mask_resumes_where_it_left_off() {
    // Roll, Keep, Costs and Build seen; the game is at the start of turn two
    let mut t = Table::new(Tutorial::new(
        true,
        Step::Roll.bit() | Step::Keep.bit() | Step::Costs.bit() | Step::Build.bit(),
    ));
    t.turns = 1;
    assert_eq!(Some(Step::Ship), t.look());
    // A mask saved before the costs tour had four entries: past the build, the rest is done
    assert!(t.tutorial.is_seen(Step::CostTrader));
}

#[test]
fn a_fresh_install_reopened_before_the_first_roll_shows_the_roll_callout_again() {
    let mut first = fresh();
    assert_eq!(Some(Step::Roll), first.look());
    let mut reopened = Table::new(Tutorial::new(true, first.tutorial.seen_mask()));
    assert_eq!(Some(Step::Roll), reopened.look());
}

#[test]
fn reopened_after_the_first_roll_the_keep_callout_is_still_due() {
    let mut first = fresh();
    first.look();
    first.roll();
    assert_eq!(Step::Roll.bit(), first.tutorial.seen_mask());
    let mut reopened = Table::new(Tutorial::new(true, first.tutorial.seen_mask()));
    reopened.rolls = 1;
    assert_eq!(Some(Step::Keep), reopened.look());
}

#[test]
fn reopened_with_an_unseen_tutorial_in_the_middle_of_a_game_catches_up() {
    // A saved game on turn five, three rolls used: the early steps are moot
    let mut t = fresh();
    t.turns = 4;
    t.rolls = 3;
    t.ends_turn = true;
    assert_eq!(Some(Step::Costs), t.look());
    assert!(t.tutorial.is_seen(Step::Roll));
    assert!(t.tutorial.is_seen(Step::Keep));
    through_costs(&mut t);
    assert_eq!(Some(Step::End), t.look());
}

#[test]
fn configure_takes_new_settings_and_restarts_the_watching() {
    let mut tutorial = Tutorial::new(true, 0x1F);
    tutorial.take_dirty();
    tutorial.configure(true, 0x1F);
    assert!(!tutorial.take_dirty());
    tutorial.configure(false, 0x1F);
    assert!(!tutorial.is_active());
    tutorial.configure(true, 0);
    assert!(tutorial.is_active());
    assert_eq!(0, tutorial.seen_mask());
    let mut t = Table::new(tutorial);
    assert_eq!(Some(Step::Roll), t.look());
}

#[test]
fn progress_is_reported_dirty_only_when_it_changes() {
    let mut t = fresh();
    t.look();
    assert!(!t.tutorial.take_dirty());
    t.roll();
    assert!(t.tutorial.take_dirty());
    assert!(!t.tutorial.take_dirty());
    t.look();
    assert!(!t.tutorial.take_dirty());
}

#[test]
fn a_seen_mask_keeps_only_the_bits_for_real_steps() {
    assert_eq!(tutorial::KNOWN, Tutorial::new(true, u32::MAX).seen_mask());
    assert_eq!(14, Step::ALL.len());
}

// ---- the events can be spoken directly --------------------------------------------------------

#[test]
fn the_event_methods_mark_their_steps_done() {
    let mut tutorial = Tutorial::new(true, 0);
    tutorial.rolled(1);
    assert_eq!(Step::Roll.bit(), tutorial.seen_mask());
    tutorial.held();
    assert_eq!(Step::Roll.bit() | Step::Keep.bit(), tutorial.seen_mask());
    tutorial.built();
    assert!(tutorial.is_seen(Step::Build));
    // The ones that need their callout up first do nothing without it
    tutorial.undone();
    assert!(!tutorial.is_seen(Step::Undo));
    tutorial.turn_ended();
    assert!(tutorial.is_seen(Step::End));
    assert!(!tutorial.is_seen(Step::Costs));
}
