// SPDX-License-Identifier: GPL-3.0-or-later
//! Which sounds a game event asks for and when. Delays are milliseconds from the moment of
//! the event.
//!
//! At the end of a game the order is: the sound of what ended it (the turn ending, or the
//! build that reached a race's target), then 300 ms later the fanfare: the bigger one if
//! the result is a new best for the island, else the short one. The Crown's ship docks a
//! turn earlier, at the turn end that starts the last turn, so its bell is not part of the
//! ending.

use std::fmt;

use super::build_plan::Kind;
use super::sfx::Sfx;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    pub sfx: Sfx,
    pub delay_ms: i32,
}

impl Cue {
    pub const fn new(sfx: Sfx, delay_ms: i32) -> Cue {
        Cue { sfx, delay_ms }
    }
}

impl fmt::Display for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.sfx, self.delay_ms)
    }
}

/// A list of cues as the old tests printed one: "[TURN_END@0, GAME_OVER@300]".
pub fn describe(cues: &[Cue]) -> String {
    let parts: Vec<String> = cues.iter().map(|c| c.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

/// Between the steps of a chain build, so they are heard as a sequence and not a smear.
pub const CHAIN_STAGGER_MS: i32 = 90;
/// Between banked resources spent in one build: each cuts the one before off at about this point.
pub const BANKED_GAP_MS: i32 = 110;
/// The ship's horn, the dock bell and the build's own fanfare follow what caused them by this much.
pub const FOLLOW_MS: i32 = 300;
/// A bonus chime follows the build that earned it.
pub const BONUS_DELAY_MS: i32 = 250;
/// A roll that starts right after the turn ended (the button reading "1st Roll") waits for
/// the turn end sound.
pub const ROLL_AFTER_TURN_END_MS: i32 = 250;

/// A turn ending. empty_penalised is a turn with nothing built on an island that charges for
/// it (it replaces the plain turn end sound); ship_appeared is the turn end that leaves three
/// turns; docked is the turn end that leaves one, the ship mooring for the last turn;
/// game_over is the end by the turn limit (the ship docked already, so no bell).
pub fn turn_end(
    empty_penalised: bool,
    ship_appeared: bool,
    docked: bool,
    game_over: bool,
    new_best: bool,
) -> Vec<Cue> {
    let mut cues = vec![Cue::new(
        if empty_penalised {
            Sfx::EmptyTurn
        } else {
            Sfx::TurnEnd
        },
        0,
    )];
    let at = FOLLOW_MS;
    if ship_appeared && !game_over {
        cues.push(Cue::new(Sfx::ShipAppears, at));
    }
    if docked && !game_over {
        cues.push(Cue::new(Sfx::ShipDocks, at));
    }
    if game_over {
        cues.push(Cue::new(
            if new_best {
                Sfx::TopScore
            } else {
                Sfx::GameOver
            },
            at,
        ));
    }
    cues
}

/// What follows a build: a bonus chime, and the fanfare if the build ended a race.
pub fn after_build(bonus_gained: bool, game_ended: bool, new_best: bool) -> Vec<Cue> {
    let mut cues = Vec::new();
    let mut at = FOLLOW_MS;
    if bonus_gained {
        cues.push(Cue::new(Sfx::Bonus, BONUS_DELAY_MS));
        at = BONUS_DELAY_MS + FOLLOW_MS + 150;
    }
    if game_ended {
        cues.push(Cue::new(
            if new_best {
                Sfx::TopScore
            } else {
                Sfx::GameOver
            },
            at,
        ));
    }
    cues
}

/// The build's own sound.
pub fn for_build(kind: Kind) -> Sfx {
    match kind {
        Kind::Road => Sfx::Road,
        Kind::Village => Sfx::Village,
        Kind::City => Sfx::Castle,
        Kind::Knight => Sfx::Trader,
    }
}

/// True when a bonus bit is set after a build that was not set before (see
/// Playsheet::bonus_state).
pub fn bonus_gained(before: u32, after: u32) -> bool {
    (after & !before) != 0
}
