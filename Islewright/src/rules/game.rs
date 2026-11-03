// SPDX-License-Identifier: GPL-3.0-or-later
//! One game on one island: the dice, the sheet, the turn and its rolls, and every action
//! the player can take. The UI queries a `Game` for what to draw and calls its actions; see
//! also view_model.rs for the decisions the old game view made on top of these (which
//! colour a spot is offered in, which question to ask before a build, what the roll button
//! says and does).
//!
//! Three things the old Android class reached for are injected instead: the random numbers
//! (`rng`), the time (`clock`, for the moment the game ended) and the top scores
//! (`recorder`, which files a result and returns its rank). Where the old class redrew and
//! saved after every change, this one counts revisions (`revision`); the UI redraws and
//! stores `game_state_store::to_json` whenever the count moves.

use std::cell::RefCell;
use std::rc::Rc;

use super::board::Board;
use super::build_plan::{self, BuildPlan, Chain, Kind};
use super::dice::{Dice, DiceValue};
use super::high_scores::{EARLY_PROMPT_SMART, RACE_TARGET_DEFAULT};
use super::hire_anim;
use super::map_def::MapDef;
use super::player_color;
use super::playsheet::Playsheet;
use super::rng::Rng;
use super::sfx::Sfx;
use super::sound_cues;
use super::sound_sink::{self, SoundSink};
use super::voyage::{self, Stage};

pub const RESOURCES_ROAD: &[DiceValue] = &[DiceValue::Brick, DiceValue::Lumber];
pub const RESOURCES_VILLAGE: &[DiceValue] = &[
    DiceValue::Lumber,
    DiceValue::Brick,
    DiceValue::Wool,
    DiceValue::Grain,
];
pub const RESOURCES_KNIGHT: &[DiceValue] = &[DiceValue::Wool, DiceValue::Grain, DiceValue::Ore];
pub const RESOURCES_CITY: &[DiceValue] = &[
    DiceValue::Grain,
    DiceValue::Grain,
    DiceValue::Ore,
    DiceValue::Ore,
    DiceValue::Ore,
];

/// How many dice a game rolls.
pub const DICE_COUNT: usize = 6;
/// Rolls per turn.
pub const MAX_ROLLS: i32 = 3;

/// Whether a recipe is affordable at all, and if so, whether paying for it would spend a
/// banked knight resource rather than just the dice in front of the player this turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildStatus {
    None,
    WithDice,
    WithKnight,
}

/// What paying for a build took: banked resources spent, and gold dice (always in pairs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    pub banked: usize,
    pub gold_dice: usize,
}

/// A trader was just hired: the UI plays its hire animation (hire_anim) from this, starting
/// `delay_ms` after the tap. Nothing else starts one, so a load, a scenario and an undo
/// never play one. `epoch` is `Game::state_epoch` at the hire; when that has moved on the
/// game was replaced under the animation, which then ends at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HireEvent {
    pub tier: usize,
    /// The bank slot the trader feeds.
    pub slot: usize,
    /// Whether this hire completed its bank slot, so there is a resource to bank.
    pub completes: bool,
    pub delay_ms: i32,
    pub epoch: u32,
}

/// A build just spent a banked resource by itself: the UI flies its mini die from the tab to
/// the piece (spend_anim), starting `delay_ms` after the tap, in time with its tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpendEvent {
    pub slot: usize,
    /// Where on the tab its mini die sat just before it was spent: how many usable slots
    /// came before it.
    pub place: i32,
    pub value: DiceValue,
    pub kind: Kind,
    pub index: usize,
    pub delay_ms: i32,
    pub epoch: u32,
}

/// In trader resource classic mode a banked resource was just dropped onto a die: the UI runs
/// a ring round it (spend_anim::ring_*), showing `was` until the ring closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingEvent {
    pub die: usize,
    pub was: DiceValue,
    pub epoch: u32,
}

/// Files a result on an island's top scores: (map id, result) to its 1 based rank, or 0 if
/// it did not make the list. See HighScoresAndSettings::add_score.
pub type ResultRecorder = Box<dyn FnMut(&str, i32) -> i32>;

/// Milliseconds on a monotonic clock.
pub type Clock = Box<dyn Fn() -> u64>;

/// The game as it stood before a build tap, for the undo button.
#[derive(Clone, Debug)]
struct Snapshot {
    rolls: i32,
    turns_taken: i32,
    built_resource_this_turn: bool,
    scored: bool,
    knight_resources: Vec<DiceValue>,
    dice: [Dice; DICE_COUNT],
    traders: Vec<bool>,
    resources_avail: Vec<bool>,
    roads: Vec<bool>,
    villages: Vec<bool>,
    cities: Vec<bool>,
    turns_nothing_built: i32,
    turn_score: Vec<i32>,
}

pub struct Game {
    /// The island being played. Everything map specific in the rules is read from it.
    pub map: &'static MapDef,
    /// That island's geometry, for the view.
    pub board: &'static Board,
    pub playsheet: Playsheet,
    pub dice: [Dice; DICE_COUNT],
    /// Rolls made this turn, 0 to 3.
    pub rolls: i32,
    /// What the bank offered at the last affordability check, for new_turn to hand back what
    /// a turn set aside. Every check refreshes it, so it sits behind a RefCell and the checks
    /// can be asked of a shared game.
    knight_resources: RefCell<Vec<DiceValue>>,
    pub built_resource_this_turn: bool,
    /// Turns finished.
    pub turns_taken: i32,
    /// Whether the result has been filed (or the game ended with nothing to file).
    pub scored: bool,
    /// The score that wins this race, 0 on a map that is not one. The game's own and saved
    /// with it, so a change of setting cannot end or lengthen a finished game, and a saved
    /// game goes on with the target it was played to (see set_race_target).
    pub target_score: i32,
    /// When the game became done, on `clock`. The roll button ignores taps for a moment after
    /// this so a bump cannot start a new game over the final board. Not saved: a done game
    /// is never saved.
    pub game_over_at: u64,
    plan_cache: RefCell<Option<(String, Rc<BuildPlan>)>>,

    /// Where this game's sounds go. None means the thread's sink (sound_sink::set_sink); a
    /// test sets its own to see what was asked for.
    pub sounds: Option<Box<dyn SoundSink>>,
    /// Milliseconds added to every sound of the build in progress: the stagger of a chain build.
    sound_base: i32,
    /// The roll that follows a turn end in one tap waits for the turn end sound.
    roll_sound_delay: i32,

    /// The game as it stood before the latest build tap, for the undo button, or None when
    /// there is nothing to undo. In memory only: an undo does not survive the process.
    /// Cleared by anything that moves the game on past the point where taking a build back
    /// would be fair: a roll, a turn ending, a new game, and a build that ended the game.
    undo: Option<Snapshot>,

    /// Counts the moments the game is replaced under the view rather than played: a restart,
    /// a new island, a scenario (all through reset) and an undo. The hire animation notes
    /// the number when it begins and ends at once if it has moved on.
    pub state_epoch: u32,
    /// Traders hired so far in the build tap in progress, to stagger their animations.
    hires_this_tap: i32,
    hires: Vec<HireEvent>,
    /// Banked resources spent in the build tap in progress, to stagger their flights.
    spends_this_tap: i32,
    spends: Vec<SpendEvent>,
    rings: Vec<RingEvent>,
    /// The banked resources the player chose to spend on the build tap in progress, when
    /// gold could have paid for some of them instead (bank_choices).
    bank_spend: Option<Vec<DiceValue>>,

    /// True while a scenario holds this game (see scenario::Scenario::apply_to): the
    /// tutorial neither watches nor draws over it. Cleared by reset, so a restart ends it.
    pub tutorial_paused: bool,

    /// The player's chosen colour (opaque ARGB), used to tint everything they have built.
    /// Held here so drawing never reads the settings document.
    pub player_color: u32,
    /// See HighScoresAndSettings::early_build_prompt. Held for the same reason.
    pub early_build_prompt: i32,
    pub end_turn_prompt: bool,
    /// The Dunholm target setting, held for the same reason. A race starts with it.
    pub race_target: i32,
    /// Trader resource classic mode (see HighScoresAndSettings::trader_classic). Set through
    /// set_trader_classic, which passes it on to the playsheet.
    trader_classic: bool,

    /// The dice's random numbers. Seed it for a game that plays out the same every time.
    pub rng: Rng,
    /// Files a result on the top scores. None files nothing and ranks it 0, as the old game
    /// did with no screen to tell.
    pub recorder: Option<ResultRecorder>,
    /// The time, for game_over_at. None reads as 0.
    pub clock: Option<Clock>,
    revision: u64,
}

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Game")
            .field("map", &self.map.id)
            .field("rolls", &self.rolls)
            .field("turns_taken", &self.turns_taken)
            .field("dice", &self.dice)
            .field("score", &self.playsheet.score())
            .finish_non_exhaustive()
    }
}

impl Game {
    pub fn new(map: &'static MapDef) -> Game {
        Game::with_rng(map, Rng::default())
    }

    /// A game whose dice follow from this generator.
    pub fn with_rng(map: &'static MapDef, rng: Rng) -> Game {
        let mut playsheet = Playsheet::new(map);
        playsheet.bank_pays_builds = true;
        let mut game = Game {
            map,
            board: Board::for_map(map),
            playsheet,
            dice: [Dice::new(); DICE_COUNT],
            rolls: 0,
            knight_resources: RefCell::new(Vec::new()),
            built_resource_this_turn: false,
            turns_taken: 0,
            scored: false,
            target_score: map.target_score,
            game_over_at: 0,
            plan_cache: RefCell::new(None),
            sounds: None,
            sound_base: 0,
            roll_sound_delay: 0,
            undo: None,
            state_epoch: 0,
            hires_this_tap: 0,
            hires: Vec::new(),
            spends_this_tap: 0,
            spends: Vec::new(),
            rings: Vec::new(),
            bank_spend: None,
            tutorial_paused: false,
            player_color: player_color::DEFAULT,
            early_build_prompt: EARLY_PROMPT_SMART,
            end_turn_prompt: true,
            race_target: RACE_TARGET_DEFAULT,
            trader_classic: false,
            rng,
            recorder: None,
            clock: None,
            revision: 0,
        };
        game.new_turn(true);
        game
    }

    // ------------------------------------------------------------ bookkeeping

    /// Moves on after every action that changes the game: a roll, a build, a turn ending, a
    /// die held, a knight resource taken. The UI redraws and saves when it sees it move.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    /// The hires since the last call, for the UI's animations.
    pub fn take_hire_events(&mut self) -> Vec<HireEvent> {
        std::mem::take(&mut self.hires)
    }

    /// The banked resources builds spent since the last call, for the UI's animations.
    pub fn take_spend_events(&mut self) -> Vec<SpendEvent> {
        std::mem::take(&mut self.spends)
    }

    /// The classic drops since the last call, for the UI's animations.
    pub fn take_ring_events(&mut self) -> Vec<RingEvent> {
        std::mem::take(&mut self.rings)
    }

    fn now(&self) -> u64 {
        self.clock.as_ref().map_or(0, |c| c())
    }

    /// What the bank offered at the last affordability check.
    pub fn knight_resources(&self) -> Vec<DiceValue> {
        self.knight_resources.borrow().clone()
    }

    /// The list itself, for a save being restored or a test.
    pub fn knight_resources_mut(&mut self) -> &mut Vec<DiceValue> {
        self.knight_resources.get_mut()
    }

    pub fn trader_classic(&self) -> bool {
        self.trader_classic
    }

    fn sound(&mut self, sfx: Sfx, delay_ms: i32) {
        let delay = self.sound_base + delay_ms;
        match self.sounds.as_mut() {
            Some(sink) => sink.play(sfx, delay),
            None => sound_sink::play_after(sfx, delay),
        }
    }

    // ------------------------------------------------------------ new games and turns

    /// A new game on the same map.
    pub fn reset(&mut self) {
        self.state_epoch += 1;
        self.tutorial_paused = false;
        self.undo = None;
        self.turns_taken = 0;
        self.playsheet.reset();
        self.scored = false;
        self.target_score = if self.is_race() {
            self.race_target.min(self.map.max_score()).max(1)
        } else {
            self.map.target_score
        };
        self.new_turn(true);
        self.changed();
    }

    /// A new game on the given map. The UI holds piece lists and layers sized for the map it
    /// was built with, so it rebuilds them when the map changes.
    pub fn start_new_game(&mut self, map: &'static MapDef) {
        self.map = map;
        self.board = Board::for_map(map);
        self.playsheet = Playsheet::new(map);
        self.playsheet.bank_pays_builds = !self.trader_classic;
        // Anything set aside from the old map's bank means nothing on this one
        self.knight_resources.get_mut().clear();
        *self.plan_cache.get_mut() = None;
        self.undo = None;
        self.reset();
    }

    /// Turns trader resource classic mode on or off for this game from now on. In classic
    /// mode a build never pays with a banked resource by itself; the player spends one by
    /// dragging it onto a die (overwrite_die).
    pub fn set_trader_classic(&mut self, on: bool) {
        let was = self.trader_classic;
        self.trader_classic = on;
        self.playsheet.bank_pays_builds = !on;
        if was == on {
            return;
        }
        if on {
            // The list is what the bank offered at the last check, for new_turn to hand back
            // what a turn set aside. Classic mode sets nothing aside, and a stale entry could
            // return a slot a drag has since spent, so it starts empty and stays so.
            self.knight_resources.get_mut().clear();
        }
        self.changed();
    }

    /// Ends the turn (or, with first_turn, starts the game's first one): unused banked
    /// resources go back, the dice are cleared, an empty turn is charged, and the turn is
    /// counted and scored.
    pub fn new_turn(&mut self, first_turn: bool) {
        if self.is_game_done() {
            return;
        }
        self.undo = None;

        // For the sounds: the ship's stage and whether this turn was empty, before anything changes
        let stage_before =
            voyage::stage_for(self.map, self.turns_taken, false, self.playsheet.score());
        let empty_turn = !first_turn && !self.built_resource_this_turn;
        let mut rank = 0;

        self.rolls = 0;

        // Return unused knight resources. The list is the bank slots that were open at the
        // last check, so a value that still has an open slot is not something set aside.
        // Returning it would hand a spent "?" back whenever the other "?" is open, which only
        // a map with two can do; on Tarnholm every value has just one slot and this changes
        // nothing.
        let set_aside = std::mem::take(self.knight_resources.get_mut());
        for v in set_aside {
            if self.playsheet.first_available_slot(v) == 0 {
                self.playsheet.return_to_bank(v);
            }
        }
        for die in &mut self.dice {
            die.reset();
        }
        if !first_turn && !self.built_resource_this_turn {
            self.playsheet.turns_nothing_built += 1;
        }
        self.built_resource_this_turn = false;

        if !first_turn {
            self.turns_taken += 1;
            self.playsheet.score_turn(self.turns_taken);
            if self.is_game_done() {
                self.game_over_at = self.now();
                // A race that reaches its target ends in finish_if_target_reached, so a race
                // that is done here ran out of turns and has no result to record. Nothing is
                // written for it, and `scored` stays false. A race with no turn cap (Dunholm)
                // never runs out, so it never comes here.
                if !self.scored && !self.is_race() {
                    self.scored = true;
                    rank = self.record_result();
                }
            }
            let done = self.is_game_done();
            let stage_after =
                voyage::stage_for(self.map, self.turns_taken, done, self.playsheet.score());
            let cues = sound_cues::turn_end(
                empty_turn && self.map.empty_turn_penalty > 0,
                stage_before == Stage::None && stage_after == Stage::Far,
                stage_before != Stage::Moored && stage_after == Stage::Moored && !done,
                done,
                rank == 1,
            );
            for cue in cues {
                self.sound(cue.sfx, cue.delay_ms);
            }
        }
        self.changed();
    }

    /// True when starting another game would throw something away: a game that is not over
    /// and in which a turn has passed, the dice have been rolled or anything has been built.
    /// A finished game has nothing left to lose, and neither has one nobody has touched.
    pub fn has_progress(&self) -> bool {
        if self.is_game_done() {
            return false;
        }
        if self.turns_taken > 0 || self.rolls > 0 || self.playsheet.trader_count() > 0 {
            return true;
        }
        // Road 0 is the start road, which every game begins with
        self.playsheet.roads.iter().skip(1).any(|&b| b)
            || self.playsheet.villages.iter().any(|&b| b)
            || self.playsheet.cities.iter().any(|&b| b)
    }

    /// True on a map that is a race to a target score, not a fixed number of turns.
    pub fn is_race(&self) -> bool {
        self.map.target_score > 0
    }

    /// Takes a new Dunholm target setting. It moves the game in progress too when that game
    /// is a race, is not over and has not already passed the new target: a player who sets a
    /// higher target expects to play on, and one already passed would end the game on the
    /// spot. Otherwise it waits for the next new game, which reset starts with it.
    pub fn set_race_target(&mut self, target: i32) {
        self.race_target = target;
        if self.is_race() && !self.is_game_done() {
            let clamped = target.min(self.map.max_score()).max(1);
            if self.playsheet.score() < clamped {
                self.target_score = clamped;
            }
        }
    }

    /// What this game puts on the top scores, or 0 for nothing. On a points map it is the
    /// score, once there is one. On a race it is the turns taken, and only once the target
    /// has been reached; a race that ran out of turns has no result. Nor has a race to any
    /// target but the island's own: fewer turns to another number is not the same feat, and
    /// the list would mix them.
    pub fn result_to_record(&self) -> i32 {
        if self.is_race() {
            return if self.target_score == self.map.target_score
                && self.playsheet.score() >= self.target_score
            {
                self.turns_taken
            } else {
                0
            };
        }
        self.playsheet.score().max(0)
    }

    /// Records the result and returns its rank on the island's list (1 is a new best), or 0
    /// for none.
    fn record_result(&mut self) -> i32 {
        let result = self.result_to_record();
        if result > 0 {
            self.record_score(result)
        } else {
            0
        }
    }

    /// Writes a result to the island's top scores through the recorder. With none it writes
    /// nothing.
    pub fn record_score(&mut self, result: i32) -> i32 {
        let id = self.map.id.clone();
        match self.recorder.as_mut() {
            Some(record) => record(&id, result),
            None => 0,
        }
    }

    /// Called at the end of every build. On a race map the game is over the moment the
    /// score reaches the target, in the middle of the turn: that turn counts as taken and is
    /// the result. The same four things as the end of new_turn: count the turn, score it,
    /// stamp the time, record.
    ///
    /// Returns -1 if the game did not end here, otherwise the rank of the result (0 if none).
    fn finish_if_target_reached(&mut self) -> i32 {
        if !self.is_race() || self.scored || self.playsheet.score() < self.target_score {
            return -1;
        }
        self.turns_taken += 1;
        self.playsheet.score_turn(self.turns_taken);
        self.game_over_at = self.now();
        self.scored = true;
        self.record_result()
    }

    /// Whether running this chain of builds would end a race. The early build prompts ask
    /// the player to think twice about giving up rolls or spreading their traders thin, and
    /// neither matters when the game ends on that build. Pure: works on a copy of the
    /// playsheet. No chain (nothing reaches the position) never wins.
    pub fn chain_reaches_target(&self, chain: Option<&Chain>) -> bool {
        let Some(chain) = chain else {
            return false;
        };
        if !self.is_race() {
            return false;
        }
        let mut copy = self.playsheet.clone();
        for step in &chain.steps {
            match step.kind {
                Kind::Road => copy.roads[step.index] = true,
                Kind::Village => copy.villages[step.index] = true,
                Kind::City => copy.cities[step.index] = true,
                Kind::Knight => copy.traders[step.index] = true,
            }
        }
        copy.score() >= self.target_score
    }

    pub fn can_use_knight_resource(&self, i: usize) -> bool {
        self.playsheet.can_use_knight_resource(i)
    }

    /// Sets a bank slot's resource aside for the turn. The old view never called this;
    /// kept for completeness.
    pub fn consume_knight_resource(&mut self, i: usize) {
        let val = self.playsheet.knight_resource(i);
        if val != DiceValue::None {
            self.knight_resources.get_mut().push(val);
        }
        self.changed();
    }

    // ------------------------------------------------------------ dice

    /// A tap on a die: holds it, or lets it go. Before the first roll, out of range, or on a
    /// locked die it does nothing. Returns whether the die changed, for the UI's haptics.
    pub fn dice_touched(&mut self, i: usize) -> bool {
        if self.rolls == 0 || i >= self.dice.len() {
            return false;
        }
        // A die a banked resource overwrote stays for the rest of the turn (see overwrite_die)
        if self.dice[i].locked {
            return false;
        }
        let held = !self.dice[i].is_held();
        self.dice[i].hold(held);
        self.sound(Sfx::Die, 0);
        self.changed();
        true
    }

    /// The roll button's plain action: a new game once this one is over, the turn's end once
    /// it cannot roll, else a roll of every die not held.
    pub fn roll(&mut self) {
        // A new roll replaces the dice the snapshot holds: restoring them would be a free re-roll
        self.undo = None;
        if self.is_game_done() {
            self.reset();
            return;
        }
        if !self.can_roll() {
            self.new_turn(false);
            return;
        }

        let len = self.dice.len();
        let mut old = [DiceValue::None; DICE_COUNT];
        for (die, was) in self.dice.iter_mut().zip(old.iter_mut()) {
            if !die.is_held() {
                *was = die.value;
                die.roll(&mut self.rng);
            }
        }

        // People think the dice are not random when they don't change, so keep the same
        // dice, but switch order if possible to help remove that disconcerting "I rolled, but
        // some of the dice didn't change" effect.
        for i in 0..len {
            if self.dice[i].is_held() || old[i] != self.dice[i].value {
                continue;
            }
            for j in 0..len {
                if i == j || self.dice[j].is_held() {
                    continue;
                }
                if old[j] != self.dice[i].value && old[i] != self.dice[j].value {
                    let (vi, vj) = (self.dice[i].value, self.dice[j].value);
                    self.dice[i].value = vj;
                    self.dice[j].value = vi;
                    break;
                }
            }
        }

        self.rolls += 1;
        let delay = self.roll_sound_delay;
        self.sound(Sfx::Roll, delay);
        self.changed();
    }

    pub fn can_roll(&self) -> bool {
        self.rolls < MAX_ROLLS && !self.built_resource_this_turn && !self.is_game_done()
    }

    /// Ends the turn and immediately rolls, for the case where the roll button reads "1st
    /// Roll" because nothing can be built and nothing can be rolled. Plain roll only advances
    /// the turn there, which would leave the player tapping a button labelled "1st Roll" and
    /// watching nothing roll.
    pub fn end_turn_and_roll(&mut self) {
        self.new_turn(false);
        if self.can_roll() {
            self.roll_sound_delay = sound_cues::ROLL_AFTER_TURN_END_MS;
            self.roll();
            self.roll_sound_delay = 0;
        }
    }

    /// True when the roll button would end the turn while the player can still afford to
    /// build something. Tapping it then is nearly always a misclick, since the button reads
    /// "Done" in the same spot the player has just tapped three times to roll, so the UI
    /// confirms first. When nothing is affordable, ending the turn is the only legal move
    /// left and confirming would just be a nag, so this returns false.
    pub fn should_confirm_end_turn(&self) -> bool {
        if !self.end_turn_prompt || self.is_game_done() || self.can_roll() {
            return false;
        }
        // Only dice are lost when the turn ends. new_turn hands unused knight resources back
        // to the bank, so a build that would have been paid from the bank is still there next
        // turn and there is nothing to warn about. Warning on it anyway would be a nag on
        // every turn that a banked resource exists.
        self.build_plan().has_dice_only_build()
    }

    /// True when building right now would forfeit rolls the player has not used yet.
    /// can_roll goes false as soon as anything is built, so building on the first or second
    /// roll silently throws the rest of the turn away.
    pub fn would_forfeit_rolls(&self) -> bool {
        self.rolls > 0
            && self.rolls < MAX_ROLLS
            && !self.built_resource_this_turn
            && !self.is_game_done()
    }

    // ------------------------------------------------------------ building

    fn can_build_kind_now(&self, recipe: &[DiceValue]) -> bool {
        if self.rolls == 0 || self.is_game_done() {
            return false;
        }
        self.can_build(recipe)
    }

    /// Whether this position can be built right now, from the dice and the bank as they are.
    pub fn can_build_at(&self, kind: Kind, i: usize) -> bool {
        match kind {
            Kind::Road => self.can_build_road(i),
            Kind::Village => self.can_build_village(i),
            Kind::City => self.can_build_city(i),
            Kind::Knight => self.can_build_knight(i),
        }
    }

    pub fn can_build_village(&self, i: usize) -> bool {
        self.can_build_any_village() && self.playsheet.can_build_village(i)
    }

    /// Whether a village's price can be paid right now, wherever it would go.
    pub fn can_build_any_village(&self) -> bool {
        self.can_build_kind_now(RESOURCES_VILLAGE)
    }

    pub fn can_build_knight(&self, i: usize) -> bool {
        self.can_build_any_knight() && self.playsheet.can_build_knight(i)
    }

    pub fn can_build_any_knight(&self) -> bool {
        self.can_build_kind_now(RESOURCES_KNIGHT)
    }

    pub fn can_build_road(&self, i: usize) -> bool {
        self.can_build_any_road() && self.playsheet.can_build_road(i)
    }

    pub fn can_build_any_road(&self) -> bool {
        self.can_build_kind_now(RESOURCES_ROAD)
    }

    pub fn can_build_city(&self, i: usize) -> bool {
        self.can_build_any_city() && self.playsheet.can_build_city(i)
    }

    pub fn can_build_any_city(&self) -> bool {
        self.can_build_kind_now(RESOURCES_CITY)
    }

    /// Builds one position directly, paying for it now; nothing happens if it cannot be.
    /// `build` is what a tap does (it also runs chains and records the undo).
    pub fn build_village(&mut self, i: usize) {
        self.build_one(Kind::Village, i);
    }

    pub fn build_knight(&mut self, i: usize) {
        self.build_one(Kind::Knight, i);
    }

    pub fn build_road(&mut self, i: usize) {
        self.build_one(Kind::Road, i);
    }

    pub fn build_city(&mut self, i: usize) {
        self.build_one(Kind::City, i);
    }

    fn build_one(&mut self, kind: Kind, i: usize) {
        if !self.can_build_at(kind, i) {
            return;
        }
        let bonuses = self.playsheet.bonus_state();
        let usable: Vec<bool> = (0..=self.playsheet.bank_size())
            .map(|s| self.playsheet.can_use_knight_resource(s))
            .collect();
        let spent = self.use_resources(kind.recipe());
        for slot in 1..usable.len() {
            if usable[slot] && !self.playsheet.can_use_knight_resource(slot) {
                self.spends.push(SpendEvent {
                    slot,
                    place: usable[1..slot].iter().filter(|&&u| u).count() as i32,
                    value: self.map.bank_resource[slot],
                    kind,
                    index: i,
                    delay_ms: self.spends_this_tap * sound_cues::BANKED_GAP_MS,
                    epoch: self.state_epoch,
                });
                self.spends_this_tap += 1;
            }
        }
        build_plan::apply_build(kind, i, &mut self.playsheet);
        if kind == Kind::Knight {
            let slot = self.map.trader_bank[i - 1];
            if slot > 0 {
                let completes = self.map.bank_traders[slot]
                    .iter()
                    .all(|&t| self.playsheet.is_trader_built(t));
                self.hires.push(HireEvent {
                    tier: i,
                    slot,
                    completes,
                    delay_ms: self.hires_this_tap * hire_anim::CHAIN_STAGGER_MS,
                    epoch: self.state_epoch,
                });
            }
            self.hires_this_tap += 1;
        }
        self.built_resource_this_turn = true;
        let ended = self.finish_if_target_reached();
        self.built_sounds(kind, spent, bonuses, ended);
        self.changed();
    }

    /// The current build plan: which positions are reachable right now (a chain of one step)
    /// or after building something else first (a chain of two or more), and whether paying
    /// for the chain dips into the resource bank. Recomputed only when the state fingerprint
    /// changes, so repeated calls between rolls, builds and taps are cheap.
    pub fn build_plan(&self) -> Rc<BuildPlan> {
        // A finished game offers nothing, even mid-turn (a race ends the moment it is won)
        let plan_rolls = if self.is_game_done() { 0 } else { self.rolls };
        let fp = format!(
            "{}:{}:{}",
            self.playsheet.bank_pays_builds,
            plan_rolls,
            build_plan::fingerprint(&self.dice, &self.playsheet)
        );
        let mut cache = self.plan_cache.borrow_mut();
        if let Some((key, plan)) = cache.as_ref()
            && *key == fp
        {
            return plan.clone();
        }
        let plan = Rc::new(BuildPlan::compute(&self.dice, &self.playsheet, plan_rolls));
        *cache = Some((fp, plan.clone()));
        plan
    }

    /// The sounds of one build step: its own sound, a banked resource's tick for each one
    /// spent (a gap apart, so each cuts the last off; see sound_playback), one coin sound if
    /// a pair of gold paid, a chime if it earned a bonus, and the fanfare if it ended a race.
    /// ended_rank is -1 if the game did not end, otherwise the result's rank.
    fn built_sounds(&mut self, kind: Kind, spent: Spent, bonuses_before: u32, ended_rank: i32) {
        self.sound(sound_cues::for_build(kind), 0);
        for n in 0..spent.banked {
            self.sound(Sfx::Banked, n as i32 * sound_cues::BANKED_GAP_MS);
        }
        if spent.gold_dice > 0 {
            self.sound(Sfx::Gold, 0);
        }
        let gained = sound_cues::bonus_gained(bonuses_before, self.playsheet.bonus_state());
        for cue in sound_cues::after_build(gained, ended_rank >= 0, ended_rank == 1) {
            self.sound(cue.sfx, cue.delay_ms);
        }
    }

    /// What a tap on a position does: builds it directly if it is affordable right now,
    /// otherwise runs the recorded chain that reaches it (each step spending resources and
    /// applying its playsheet effect before the next step is attempted), so a single tap on a
    /// position that needs another build first builds everything on the way to it. One undo
    /// takes the whole tap back.
    pub fn build(&mut self, kind: Kind, index: usize) {
        self.hires_this_tap = 0;
        self.spends_this_tap = 0;
        let before = if self.is_game_done() {
            None
        } else {
            Some(self.snapshot())
        };
        if self.can_build_at(kind, index) {
            self.undo = before;
            self.build_one(kind, index);
            self.after_build();
            return;
        }

        let Some(chain) = self.build_plan().chain_for(kind, index).cloned() else {
            return;
        };

        // One snapshot covers the whole chain, so one undo takes the whole tap back
        self.undo = before;

        // Each step sounds a little after the one before, so a chain is heard as a sequence
        self.sound_base = 0;
        self.build_chain(&chain);
        self.sound_base = 0;
        self.after_build();
    }

    fn build_chain(&mut self, chain: &Chain) {
        for step in &chain.steps {
            // A race can end partway along a chain; nothing is built after it does
            if self.is_game_done() {
                return;
            }
            self.build_one(step.kind, step.index);
            if !self.is_built(step.kind, step.index) {
                return;
            }
            self.sound_base += sound_cues::CHAIN_STAGGER_MS;
        }
    }

    /// Whether this position already holds something the player built.
    pub fn is_built(&self, kind: Kind, index: usize) -> bool {
        let sheet = &self.playsheet;
        match kind {
            Kind::Road => sheet.roads.get(index).copied().unwrap_or(false),
            Kind::Village => sheet.villages.get(index).copied().unwrap_or(false),
            Kind::City => sheet.cities.get(index).copied().unwrap_or(false),
            Kind::Knight => sheet.is_trader_built(index),
        }
    }

    /// Trader resource classic mode: whether the banked resource in this slot may be dragged
    /// onto this die right now. After a roll, before the game is over, onto a die not spent on
    /// a build and not already overwritten, from a slot that holds a resource.
    pub fn can_overwrite_die(&self, die_index: usize, slot: usize) -> bool {
        self.trader_classic
            && self.rolls >= 1
            && !self.is_game_done()
            && die_index < self.dice.len()
            && self.dice[die_index].is_usable()
            && !self.dice[die_index].locked
            && self.playsheet.can_use_knight_resource(slot)
    }

    /// Trader resource classic mode: spends the banked resource in this slot by turning a
    /// rolled die to show it. value is what the die becomes: the slot's own resource, or for
    /// a "?" slot the one the player chose. The die is held and locked for the rest of the
    /// turn, since the resource cost a trader to earn and one stray tap and a roll would lose
    /// it. One undo takes it back, as it does a build. Returns whether it happened.
    pub fn overwrite_die(&mut self, die_index: usize, slot: usize, value: DiceValue) -> bool {
        if !self.can_overwrite_die(die_index, slot) {
            return false;
        }
        let banked = self.playsheet.knight_resource(slot);
        if banked != DiceValue::Any && value != banked {
            return false;
        }
        if matches!(value, DiceValue::None | DiceValue::Any | DiceValue::Gold) {
            return false;
        }
        self.undo = Some(self.snapshot());
        let die = &mut self.dice[die_index];
        self.rings.push(RingEvent {
            die: die_index,
            was: die.value,
            epoch: self.state_epoch,
        });
        die.value = value;
        die.held = true;
        die.locked = true;
        self.playsheet.use_knight_resource(slot);
        self.knight_resources.get_mut().clear();
        self.sound(Sfx::Banked, 0);
        self.changed();
        true
    }

    /// A build that ended the game has been recorded as its result, and stays.
    fn after_build(&mut self) {
        if self.is_game_done() {
            self.undo = None;
        }
    }

    /// True when the latest build tap of this turn can be taken back.
    pub fn can_undo(&self) -> bool {
        self.undo.is_some() && !self.is_game_done()
    }

    /// Takes back the latest build tap: the pieces, the dice it spent, the rolls it
    /// forfeited. One level only; the snapshot is gone afterwards. Returns whether anything
    /// was taken back, for the UI's haptics.
    pub fn undo(&mut self) -> bool {
        if !self.can_undo() {
            return false;
        }
        let Some(saved) = self.undo.take() else {
            return false;
        };
        self.restore(saved);
        self.sound(Sfx::Click, 0);
        self.changed();
        true
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            rolls: self.rolls,
            turns_taken: self.turns_taken,
            built_resource_this_turn: self.built_resource_this_turn,
            scored: self.scored,
            knight_resources: self.knight_resources(),
            dice: self.dice,
            traders: self.playsheet.traders.clone(),
            resources_avail: self.playsheet.resources_avail.clone(),
            roads: self.playsheet.roads.clone(),
            villages: self.playsheet.villages.clone(),
            cities: self.playsheet.cities.clone(),
            turns_nothing_built: self.playsheet.turns_nothing_built,
            turn_score: self.playsheet.turn_score.clone(),
        }
    }

    /// Puts back every field the snapshot holds. All but the target: that is a setting the
    /// player may have changed since the snapshot, and an undo takes back a build, not a
    /// choice made in Settings.
    fn restore(&mut self, s: Snapshot) {
        self.state_epoch += 1;
        self.rolls = s.rolls;
        self.turns_taken = s.turns_taken;
        self.built_resource_this_turn = s.built_resource_this_turn;
        self.scored = s.scored;
        *self.knight_resources.get_mut() = s.knight_resources;
        self.dice = s.dice;
        // The traders' first entry is unused, and a snapshot never set it
        self.playsheet.traders = s.traders;
        self.playsheet.traders[0] = false;
        self.playsheet.resources_avail = s.resources_avail;
        self.playsheet.roads = s.roads;
        self.playsheet.villages = s.villages;
        self.playsheet.cities = s.cities;
        self.playsheet.turns_nothing_built = s.turns_nothing_built;
        self.playsheet.turn_score = s.turn_score;
    }

    /// Pays for a recipe from this game's dice and bank.
    pub fn use_resources(&mut self, resources: &[DiceValue]) -> Spent {
        use_resources_spending(
            resources,
            &mut self.dice,
            &mut self.playsheet,
            self.knight_resources.get_mut(),
            self.bank_spend.as_deref(),
        )
    }

    /// The choices of banked resources a tap on this position could spend, when there is a
    /// real one to make (see bank_options); empty otherwise, and for a chain.
    pub fn bank_choices(&self, kind: Kind, index: usize) -> Vec<Vec<DiceValue>> {
        if !self.can_build_at(kind, index) {
            return Vec::new();
        }
        bank_options(kind.recipe(), &self.dice, &self.playsheet)
    }

    /// What a tap does once the player has chosen which banked resources to spend.
    pub fn build_spending(&mut self, kind: Kind, index: usize, spend: Vec<DiceValue>) {
        self.bank_spend = Some(spend);
        self.build(kind, index);
        self.bank_spend = None;
    }

    pub fn can_build(&self, resources_required: &[DiceValue]) -> bool {
        self.can_build_status(resources_required) != BuildStatus::None
    }

    pub fn can_build_status(&self, resources_required: &[DiceValue]) -> BuildStatus {
        can_build_status(
            resources_required,
            &self.dice,
            &self.playsheet,
            &mut self.knight_resources.borrow_mut(),
        )
    }

    /// Per recipe status, with the rolls == 0 guard each can_build_x applies. The board reads
    /// these to tell green (dice) from blue (knight resource) without re-deriving the
    /// affordability itself; can_build_status is the single source of truth.
    pub fn build_status(&self, kind: Kind) -> BuildStatus {
        if self.rolls == 0 || self.is_game_done() {
            return BuildStatus::None;
        }
        self.can_build_status(kind.recipe())
    }

    /// Out of turns (on a map with a cap), or (on a map that has one) at the target score.
    pub fn is_game_done(&self) -> bool {
        (self.map.turn_cap > 0 && self.turns_taken >= self.map.turn_cap)
            || (self.target_score > 0 && self.playsheet.score() >= self.target_score)
    }
}

/// Fills the list with what the bank can pay right now, unless the sheet says the bank
/// pays for nothing by itself (trader resource classic mode).
fn offered_by_bank(playsheet: &Playsheet, knight_resources: &mut Vec<DiceValue>) {
    knight_resources.clear();
    if !playsheet.bank_pays_builds {
        return;
    }
    for i in 1..=playsheet.bank_size() {
        if playsheet.can_use_knight_resource(i) {
            knight_resources.push(playsheet.knight_resource(i));
        }
    }
}

fn remove_first(list: &mut Vec<DiceValue>, v: DiceValue) {
    if let Some(at) = list.iter().position(|&x| x == v) {
        list.remove(at);
    }
}

/// Pays for a recipe: matching dice first, then exact bank slots, then pairs of gold, then
/// the "?" slots for what is left. `knight_resources` is left holding what the bank offered
/// less what was spent. Ask can_build_status first; this pays what it can either way.
pub fn use_resources(
    resources: &[DiceValue],
    dice: &mut [Dice],
    playsheet: &mut Playsheet,
    knight_resources: &mut Vec<DiceValue>,
) -> Spent {
    use_resources_spending(resources, dice, playsheet, knight_resources, None)
}

/// use_resources, told which banked resources to spend where gold could stand in for some of
/// them (`spend`, one of bank_options' answers). None spends the ones the bank lists last.
pub fn use_resources_spending(
    resources: &[DiceValue],
    dice: &mut [Dice],
    playsheet: &mut Playsheet,
    knight_resources: &mut Vec<DiceValue>,
    spend: Option<&[DiceValue]>,
) -> Spent {
    offered_by_bank(playsheet, knight_resources);

    let mut gold = dice
        .iter()
        .filter(|d| d.is_usable() && d.value == DiceValue::Gold)
        .count();
    if gold % 2 != 0 {
        gold -= 1;
    }

    let mut res_list: Vec<DiceValue> = resources.to_vec();
    for &val in resources {
        if let Some(die) = dice.iter_mut().find(|d| d.is_usable() && d.value == val) {
            die.use_die();
            remove_first(&mut res_list, val);
        }
    }

    // Where the map says a banked resource pays for one needed resource, a slot that has
    // matched is not offered to the next one. Tarnholm keeps its old behaviour, in which a
    // castle's two grain both match a single banked grain and the second is free.
    let mut res_list_found: Vec<DiceValue> = Vec::new();
    let mut matched = vec![false; knight_resources.len()];
    for &val in &res_list {
        for i in (0..knight_resources.len()).rev() {
            if knight_resources[i] == val && !(playsheet.map.bank_matches_once && matched[i]) {
                matched[i] = true;
                res_list_found.push(val);
                break;
            }
        }
    }
    for &val in &res_list_found {
        remove_first(&mut res_list, val);
    }

    let mut gold_used;
    // How many bank slots standing for any resource pay for what the dice and the exact bank
    // slots did not cover and gold cannot
    let mut wildcards_needed = 0;
    let pairs = gold / 2;
    if pairs == res_list.len() {
        gold_used = gold;
    } else if pairs > res_list.len() {
        gold_used = res_list.len() * 2;
        let mut i = 0;
        while i < pairs - res_list.len() && !res_list_found.is_empty() {
            // Gold pays instead of a banked resource the player chose to keep
            let at = spend
                .and_then(|keep| {
                    res_list_found.iter().position(|v| {
                        let found = res_list_found.iter().filter(|&w| w == v).count();
                        found > keep.iter().filter(|&w| w == v).count()
                    })
                })
                .unwrap_or(0);
            res_list_found.remove(at);
            gold_used += 2;
            i += 1;
        }
    } else {
        wildcards_needed = res_list.len() - pairs;
        gold_used = gold;
    }

    let gold_paid = gold_used;
    let mut banked_spent = 0;
    for die in dice.iter_mut() {
        if gold_used == 0 {
            break;
        }
        if die.is_usable() && die.value == DiceValue::Gold {
            die.use_die();
            gold_used -= 1;
        }
    }

    for &val in &res_list_found {
        if let Some(i) = (0..knight_resources.len())
            .rev()
            .find(|&i| knight_resources[i] == val)
        {
            let index = playsheet.first_available_slot(knight_resources[i]);
            playsheet.use_knight_resource(index);
            knight_resources.remove(i);
            banked_spent += 1;
        }
    }
    // Each open wildcard slot is spent once, as many as the build needs. can_build_status
    // offers a build on the same count, so what is offered is what is paid. Tarnholm has one
    // such slot, so for it this is the single spend it always was.
    for _ in 0..wildcards_needed {
        if let Some(i) = (0..knight_resources.len())
            .rev()
            .find(|&i| knight_resources[i] == DiceValue::Any)
        {
            let index = playsheet.first_available_slot(DiceValue::Any);
            playsheet.use_knight_resource(index);
            knight_resources.remove(i);
            banked_spent += 1;
        }
    }
    Spent {
        banked: banked_spent,
        gold_dice: gold_paid,
    }
}

/// The different sets of banked resources that could pay for a recipe, when the player has
/// a real choice: gold pairs can stand in for some of the banked resources that match, and
/// the ones left to spend differ by what they are. Each set is sorted; fewer than two means
/// there is nothing to ask. A "?" never enters into it, since a resource that matches is
/// always spent before one.
pub fn bank_options(
    resources: &[DiceValue],
    dice: &[Dice],
    playsheet: &Playsheet,
) -> Vec<Vec<DiceValue>> {
    let mut knight_resources = Vec::new();
    offered_by_bank(playsheet, &mut knight_resources);

    // What the dice leave short, and which banked resources match it
    let mut left = dice.to_vec();
    let mut short: Vec<DiceValue> = resources.to_vec();
    for &val in resources {
        if let Some(die) = left.iter_mut().find(|d| d.is_usable() && d.value == val) {
            die.use_die();
            remove_first(&mut short, val);
        }
    }
    let mut found: Vec<DiceValue> = Vec::new();
    let mut matched = vec![false; knight_resources.len()];
    for &val in &short {
        for i in (0..knight_resources.len()).rev() {
            if knight_resources[i] == val && !(playsheet.map.bank_matches_once && matched[i]) {
                matched[i] = true;
                found.push(val);
                break;
            }
        }
    }
    for &val in &found {
        remove_first(&mut short, val);
    }
    let gold = left
        .iter()
        .filter(|d| d.is_usable() && d.value == DiceValue::Gold)
        .count();
    let spare = (gold / 2).saturating_sub(short.len());
    let keep = found.len().saturating_sub(spare);
    if spare == 0 || keep == 0 {
        return Vec::new();
    }

    // Every distinct choice of `keep` of them to spend, gold paying for the rest
    found.sort_by_key(|v| *v as i32);
    let n = found.len();
    let mut sets: Vec<Vec<DiceValue>> = Vec::new();
    for mask in 0u32..(1 << n) {
        if mask.count_ones() as usize != keep {
            continue;
        }
        let set: Vec<DiceValue> = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| found[i])
            .collect();
        if !sets.contains(&set) {
            sets.push(set);
        }
    }
    if sets.len() < 2 {
        return Vec::new();
    }
    sets
}

/// Whether a recipe can be paid from these dice and this bank, and whether paying would
/// touch the bank. `knight_resources` is refilled with what the bank offers.
pub fn can_build_status(
    resources_required: &[DiceValue],
    dice: &[Dice],
    playsheet: &Playsheet,
    knight_resources: &mut Vec<DiceValue>,
) -> BuildStatus {
    let mut dice_used = vec![false; dice.len()];
    offered_by_bank(playsheet, knight_resources);

    let mut knight_used = vec![false; knight_resources.len()];
    let mut used_knight_resource = false;
    let gold = dice
        .iter()
        .filter(|d| d.is_usable() && d.value == DiceValue::Gold)
        .count();
    let mut unfound = 0;
    for &val in resources_required {
        let mut found = false;
        if let Some(i) =
            (0..dice.len()).find(|&i| dice[i].is_usable() && !dice_used[i] && dice[i].value == val)
        {
            dice_used[i] = true;
            found = true;
        }
        if !found
            && let Some(i) =
                (0..knight_resources.len()).find(|&i| !knight_used[i] && knight_resources[i] == val)
        {
            knight_used[i] = true;
            found = true;
            used_knight_resource = true;
        }
        if !found
            && let Some(i) = (0..knight_resources.len())
                .find(|&i| !knight_used[i] && knight_resources[i] == DiceValue::Any)
        {
            knight_used[i] = true;
            found = true;
            used_knight_resource = true;
        }
        if !found {
            unfound += 1;
        }
    }

    if unfound > gold / 2 {
        BuildStatus::None
    } else if used_knight_resource {
        BuildStatus::WithKnight
    } else {
        BuildStatus::WithDice
    }
}
