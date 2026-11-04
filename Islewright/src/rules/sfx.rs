// SPDX-License-Identifier: GPL-3.0-or-later
//! The sixteen moments that have a sound, and a second take of one of them. Each is an
//! audio file named sfx_<name> (see tools/sfx-sources.txt for where the sounds come from).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Sfx {
    Click,
    Die,
    Roll,
    Road,
    Village,
    Castle,
    Trader,
    /// Spending a banked resource. Retriggers: each use cuts off the one still playing.
    Banked,
    Gold,
    TurnEnd,
    EmptyTurn,
    ShipAppears,
    ShipDocks,
    GameOver,
    TopScore,
    Bonus,
    /// The second roll sound. Never asked for by the game: the player plays it for half the
    /// Rolls, so the dice do not always sound the same.
    RollB,
}

impl Sfx {
    pub const ALL: [Sfx; 17] = [
        Sfx::Click,
        Sfx::Die,
        Sfx::Roll,
        Sfx::Road,
        Sfx::Village,
        Sfx::Castle,
        Sfx::Trader,
        Sfx::Banked,
        Sfx::Gold,
        Sfx::TurnEnd,
        Sfx::EmptyTurn,
        Sfx::ShipAppears,
        Sfx::ShipDocks,
        Sfx::GameOver,
        Sfx::TopScore,
        Sfx::Bonus,
        Sfx::RollB,
    ];

    pub fn ordinal(self) -> usize {
        self as usize
    }

    /// The file is "sfx_" + this.
    pub fn file(self) -> &'static str {
        match self {
            Sfx::Click => "click",
            Sfx::Die => "die",
            Sfx::Roll => "roll",
            Sfx::Road => "road",
            Sfx::Village => "village",
            Sfx::Castle => "castle",
            Sfx::Trader => "trader",
            Sfx::Banked => "banked",
            Sfx::Gold => "gold",
            Sfx::TurnEnd => "turn_end",
            Sfx::EmptyTurn => "empty_turn",
            Sfx::ShipAppears => "ship_appears",
            Sfx::ShipDocks => "ship_docks",
            Sfx::GameOver => "game_over",
            Sfx::TopScore => "top_score",
            Sfx::Bonus => "bonus",
            Sfx::RollB => "roll_b",
        }
    }

    /// Zero for a sound that plays out and may overlap itself. Above zero the sound
    /// retriggers: a new request within this many milliseconds of the last one (about the
    /// sample's own length) stops the stream still playing first, so a run of requests
    /// stutters the opening of the sample instead of stacking copies.
    pub fn retrigger_ms(self) -> u64 {
        match self {
            Sfx::Banked => 260,
            _ => 0,
        }
    }

    /// The name the old code logged, upper case with underscores, which the tests compare.
    pub fn constant_name(self) -> &'static str {
        match self {
            Sfx::Click => "CLICK",
            Sfx::Die => "DIE",
            Sfx::Roll => "ROLL",
            Sfx::Road => "ROAD",
            Sfx::Village => "VILLAGE",
            Sfx::Castle => "CASTLE",
            Sfx::Trader => "TRADER",
            Sfx::Banked => "BANKED",
            Sfx::Gold => "GOLD",
            Sfx::TurnEnd => "TURN_END",
            Sfx::EmptyTurn => "EMPTY_TURN",
            Sfx::ShipAppears => "SHIP_APPEARS",
            Sfx::ShipDocks => "SHIP_DOCKS",
            Sfx::GameOver => "GAME_OVER",
            Sfx::TopScore => "TOP_SCORE",
            Sfx::Bonus => "BONUS",
            Sfx::RollB => "ROLL_B",
        }
    }
}

impl fmt::Display for Sfx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.constant_name())
    }
}
