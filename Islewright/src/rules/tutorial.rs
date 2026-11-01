// SPDX-License-Identifier: GPL-3.0-or-later
//! The in-context tutorial's decisions, and nothing else: which callout (a "coach mark") is
//! due now, and which have been seen. The UI gathers a `Snapshot` of the game after every
//! change (Game::tutorial_snapshot), asks `update`, and draws what comes back where
//! tutorial_layout says. The tutorial never blocks play; it only decides what to say.
//!
//! Ten steps. 1 to 7 are met in order, and 8 to 10 whenever their moment first comes. Each
//! step has a trigger (the state in which its callout shows) and is done when the player does
//! the thing it describes, or taps Got it. Of the two banked resource steps a game shows the
//! one for the trader mode it is played in, so a player who never turns classic mode on keeps
//! the tutorial on with nothing left to say, and is shown how to drag the first time they
//! bank a resource in classic mode. A step that has been done is never shown again; the set
//! of them is a bitmask the caller saves, so the tutorial resumes where it left off. Only one
//! callout is up at a time, and the one up stays until it is done or its trigger no longer
//! holds, so a callout never changes under the player's eyes.
//!
//! The "player did X" events (rolled, held, built, undone, turn_ended) are public so a test
//! can speak them directly; update derives them itself by comparing each snapshot with the
//! one before, and from the first snapshot alone (a resumed game) it catches up from the state.

/// The steps, in the order 1 to 7 are met and then the three that wait for their moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    /// Roll button: a new game, before the first roll.
    Roll,
    /// Dice tray: after the first roll of the game.
    Keep,
    /// The costs key in the top bar: after Keep is done.
    Costs,
    /// One spot on offer: the first time anything is, after Costs.
    Build,
    /// The undo button: right after the first build.
    Undo,
    /// The roll button: the first time it would end the turn.
    End,
    /// The turn strip: from the second turn on.
    Ship,
    /// One gold die: the first time two or more dice show gold.
    Gold,
    /// The banked mini dice: the first time one appears, in the usual trader mode.
    Bank,
    /// The banked mini dice: the first time one appears in trader resource classic mode. A
    /// standing hint rather than a tour stop: it is not part of ALL, so finishing the tour
    /// does not wait for it, and it still shows after the tour is finished or skipped, because
    /// a player who turns classic mode on later has never been shown the drag.
    BankClassic,
}

impl Step {
    pub const ALL: [Step; 10] = [
        Step::Roll,
        Step::Keep,
        Step::Costs,
        Step::Build,
        Step::Undo,
        Step::End,
        Step::Ship,
        Step::Gold,
        Step::Bank,
        Step::BankClassic,
    ];

    pub fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

/// Every step there is, the standing hint included: the bits worth keeping.
pub const KNOWN: u32 = (1 << Step::ALL.len()) - 1;

/// Every step of the tour seen. The standing hint is left out (see BankClassic).
pub const ALL: u32 = KNOWN & !(1 << Step::BankClassic as u32);

/// What the tutorial needs to know about the game, as plain numbers and flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Rolls made this turn, 0 to 3.
    pub rolls: i32,
    /// Turns finished so far: 0 on the first turn.
    pub turns_taken: i32,
    /// Whether any die is held.
    pub any_held: bool,
    /// Whether any spot on the island is on offer right now.
    pub offered: bool,
    /// Whether something has been built this turn.
    pub built_this_turn: bool,
    /// Whether the undo button is showing.
    pub can_undo: bool,
    /// Whether the roll button would end the turn: no rolls left or a build made, and the
    /// game not over.
    pub roll_ends_turn: bool,
    /// How many of the dice showing are gold.
    pub gold_dice: i32,
    /// How many banked resources show as mini dice.
    pub mini_dice: i32,
    /// Whether the game is in trader resource classic mode: a banked resource is dragged onto
    /// a die.
    pub classic_bank: bool,
    pub game_over: bool,
}

#[derive(Clone, Debug)]
pub struct Tutorial {
    active: bool,
    seen: u32,
    current: Option<Step>,
    prev: Option<Snapshot>,
    dirty: bool,
}

impl Tutorial {
    pub fn new(active: bool, seen: u32) -> Tutorial {
        Tutorial {
            active,
            seen: seen & KNOWN,
            current: None,
            prev: None,
            dirty: false,
        }
    }

    /// Whether the tutorial is on: false once skipped or finished.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The steps done, one bit each (Step::bit), for the settings document.
    pub fn seen_mask(&self) -> u32 {
        self.seen
    }

    pub fn is_seen(&self, step: Step) -> bool {
        (self.seen & step.bit()) != 0
    }

    /// The step the last update asked to show, or None.
    pub fn current(&self) -> Option<Step> {
        self.current
    }

    /// Takes the stored settings over, when they differ from what is held (the Settings
    /// screen and a backup import can change them). A change starts the watching afresh.
    pub fn configure(&mut self, active: bool, seen: u32) {
        let seen = seen & KNOWN;
        if active == self.active && seen == self.seen {
            return;
        }
        self.active = active;
        self.seen = seen;
        self.current = None;
        self.prev = None;
    }

    /// Stops comparing with the snapshot before, and drops the callout up: the game under the
    /// tutorial was swapped for another, or was set aside for a while. The steps seen are kept.
    pub fn forget(&mut self) {
        self.current = None;
        self.prev = None;
    }

    /// True once after the active flag or the seen steps changed: the caller saves them.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    // ---------------------------------------------------------------- what to show

    /// Looks at the game as it is now and returns the step whose callout is due, or None. Any
    /// "player did X" that happened since the last call is applied first. Nothing is due once
    /// the game is over, or the tutorial is off.
    pub fn update(&mut self, s: &Snapshot) -> Option<Step> {
        if !self.active {
            return self.standing_hint(s);
        }
        if let Some(prev) = self.prev
            && is_new_game(&prev, s)
        {
            self.prev = None;
            self.current = None;
        }
        match self.prev {
            None => self.catch_up(s),
            Some(prev) => self.transitions(&prev, s),
        }
        self.prev = Some(*s);

        if (self.seen & ALL) == ALL {
            self.finish();
            return self.standing_hint(s);
        }
        if s.game_over {
            self.current = None;
            return None;
        }
        if let Some(current) = self.current
            && self.is_seen(current)
        {
            self.current = None;
        }
        if let Some(current) = self.current
            && !self.triggered(current, s)
        {
            // The two moment steps are done when their moment passes; the rest wait to come back
            if matches!(current, Step::Gold | Step::Bank | Step::BankClassic) {
                self.mark_seen(current);
            }
            self.current = None;
        }
        if self.current.is_none() {
            self.current = Step::ALL
                .into_iter()
                .find(|&step| !self.is_seen(step) && self.triggered(step, s));
        }
        self.current
    }

    /// Whether this step's callout shows in this state, if it has not been done.
    fn triggered(&self, step: Step, s: &Snapshot) -> bool {
        match step {
            Step::Roll => s.turns_taken == 0 && s.rolls == 0 && !s.built_this_turn,
            Step::Keep => self.is_seen(Step::Roll) && s.rolls >= 1,
            Step::Costs => self.is_seen(Step::Keep),
            Step::Build => self.is_seen(Step::Costs) && s.offered && !s.built_this_turn,
            Step::Undo => s.can_undo,
            Step::End => s.roll_ends_turn,
            Step::Ship => s.turns_taken >= 1,
            Step::Gold => s.gold_dice >= 2,
            Step::Bank => s.mini_dice >= 1 && !s.classic_bank,
            Step::BankClassic => s.mini_dice >= 1 && s.classic_bank,
        }
    }

    /// The events between two snapshots.
    fn transitions(&mut self, before: &Snapshot, now: &Snapshot) {
        if now.rolls > before.rolls || (now.turns_taken > before.turns_taken && now.rolls > 0) {
            self.rolled(now.rolls);
        }
        if now.any_held && !before.any_held {
            self.held();
        }
        if now.built_this_turn && !before.built_this_turn {
            self.built();
        }
        if before.can_undo && !now.can_undo {
            self.undone();
        }
        if now.turns_taken > before.turns_taken {
            self.turn_ended();
        }
    }

    /// The first look at a game (the app was reopened, the island changed, a new game began):
    /// whatever the state shows has already been done is not shown. A game with no progress
    /// marks nothing.
    fn catch_up(&mut self, s: &Snapshot) {
        if s.rolls >= 1 || s.turns_taken >= 1 || s.built_this_turn {
            self.mark_seen(Step::Roll);
        }
        if s.any_held || s.rolls >= 2 || s.built_this_turn || s.turns_taken >= 1 {
            self.mark_seen(Step::Keep);
        }
        if s.built_this_turn {
            self.mark_seen(Step::Build);
        }
    }

    // ---------------------------------------------------------------- the player did X

    /// A roll was made; rolls_now is the count this turn after it (1 to 3).
    pub fn rolled(&mut self, rolls_now: i32) {
        self.mark_seen(Step::Roll);
        if rolls_now >= 2 {
            self.mark_seen(Step::Keep);
        }
        self.mark_if_current(Step::Costs);
    }

    /// A die was held.
    pub fn held(&mut self) {
        self.mark_seen(Step::Roll);
        self.mark_seen(Step::Keep);
    }

    /// Something was built.
    pub fn built(&mut self) {
        self.mark_seen(Step::Roll);
        self.mark_seen(Step::Keep);
        self.mark_seen(Step::Build);
        self.mark_if_current(Step::Costs);
    }

    /// The last build was taken back, or the chance to was lost to a roll or the turn ending.
    pub fn undone(&mut self) {
        self.mark_if_current(Step::Undo);
    }

    /// The turn ended (by the roll button).
    pub fn turn_ended(&mut self) {
        self.mark_seen(Step::Roll);
        self.mark_seen(Step::Keep);
        self.mark_seen(Step::End);
        self.mark_if_current(Step::Costs);
        self.mark_if_current(Step::Undo);
        self.mark_if_current(Step::Ship);
    }

    /// The Got it button: the callout up is done.
    pub fn got_it(&mut self) {
        if let Some(current) = self.current {
            self.mark_seen(current);
        }
    }

    /// The Skip tutorial button: no more tour callouts, now or in later games. The standing
    /// hint is only dismissed when it is the one up, so skipping the tour does not hide it.
    pub fn skip(&mut self) {
        self.mark_if_current(Step::BankClassic);
        self.current = None;
        self.finish();
    }

    /// The standing hint on its own, once the tour is off: up while its moment lasts, and
    /// done when that moment passes or the player taps Got it.
    fn standing_hint(&mut self, s: &Snapshot) -> Option<Step> {
        let hint = Step::BankClassic;
        if self.is_seen(hint) {
            self.current = None;
            return None;
        }
        if !s.game_over && self.triggered(hint, s) {
            self.current = Some(hint);
            return Some(hint);
        }
        if self.current == Some(hint) {
            self.mark_seen(hint);
        }
        self.current = None;
        None
    }

    fn finish(&mut self) {
        if self.active {
            self.active = false;
            self.dirty = true;
        }
    }

    fn mark_if_current(&mut self, step: Step) {
        if self.current == Some(step) {
            self.mark_seen(step);
        }
    }

    fn mark_seen(&mut self, step: Step) {
        if !self.is_seen(step) {
            self.seen |= step.bit();
            self.dirty = true;
        }
        if self.current == Some(step) {
            self.current = None;
        }
    }
}

/// A restart or a new game puts the turn count or the roll count back.
fn is_new_game(before: &Snapshot, now: &Snapshot) -> bool {
    now.turns_taken < before.turns_taken
        || (now.turns_taken == before.turns_taken && now.rolls < before.rolls)
}
