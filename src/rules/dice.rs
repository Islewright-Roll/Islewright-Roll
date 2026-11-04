// SPDX-License-Identifier: GPL-3.0-or-later
//! One die: the face it shows and whether it is held, spent or locked.

use super::rng::Rng;

/// What a die can show, and what a bank slot can hold. None is a die not yet rolled; Any is
/// the "?" a bank slot holds, standing for whichever resource a build needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DiceValue {
    None,
    Wool,
    Grain,
    Brick,
    Ore,
    Lumber,
    Gold,
    Any,
}

impl DiceValue {
    pub const ALL: [DiceValue; 8] = [
        DiceValue::None,
        DiceValue::Wool,
        DiceValue::Grain,
        DiceValue::Brick,
        DiceValue::Ore,
        DiceValue::Lumber,
        DiceValue::Gold,
        DiceValue::Any,
    ];

    /// The six faces a roll can land on, in the order of the old 1 to 6 roll.
    pub const FACES: [DiceValue; 6] = [
        DiceValue::Wool,
        DiceValue::Grain,
        DiceValue::Brick,
        DiceValue::Ore,
        DiceValue::Lumber,
        DiceValue::Gold,
    ];

    /// The five resources a "?" can be turned into, in the order the choice offers them.
    pub const RESOURCES: [DiceValue; 5] = [
        DiceValue::Wool,
        DiceValue::Grain,
        DiceValue::Brick,
        DiceValue::Ore,
        DiceValue::Lumber,
    ];

    /// The position in the enum, which the build plan's fingerprint is made of.
    pub fn ordinal(self) -> usize {
        self as usize
    }

    /// The name the save file writes.
    pub fn name(self) -> &'static str {
        match self {
            DiceValue::None => "None",
            DiceValue::Wool => "Wool",
            DiceValue::Grain => "Grain",
            DiceValue::Brick => "Brick",
            DiceValue::Ore => "Ore",
            DiceValue::Lumber => "Lumber",
            DiceValue::Gold => "Gold",
            DiceValue::Any => "Any",
        }
    }

    /// The value with this exact name, or None for a name nobody knows.
    pub fn from_name(name: &str) -> Option<DiceValue> {
        DiceValue::ALL.into_iter().find(|v| v.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Dice {
    pub value: DiceValue,
    pub held: bool,
    pub used: bool,
    /// Overwritten by a banked resource in trader resource classic mode (Game::overwrite_die):
    /// held, and a tap does not let it go, until the turn ends.
    pub locked: bool,
}

impl Default for Dice {
    fn default() -> Self {
        Dice::new()
    }
}

impl Dice {
    pub const fn new() -> Dice {
        Dice {
            value: DiceValue::None,
            held: false,
            used: false,
            locked: false,
        }
    }

    /// A fresh die showing this value, for tests and scenarios.
    pub const fn showing(value: DiceValue) -> Dice {
        Dice {
            value,
            held: false,
            used: false,
            locked: false,
        }
    }

    /// One of the six faces, each equally likely.
    pub fn roll(&mut self, rng: &mut Rng) {
        self.value = DiceValue::FACES[rng.below(6) as usize];
    }

    pub fn swap(&mut self, other: &mut Dice) {
        std::mem::swap(&mut self.value, &mut other.value);
    }

    pub fn is_held(&self) -> bool {
        self.held
    }

    pub fn hold(&mut self, held: bool) {
        self.held = held;
    }

    pub fn is_usable(&self) -> bool {
        !self.used
    }

    pub fn use_die(&mut self) {
        self.used = true;
    }

    pub fn reset(&mut self) {
        *self = Dice::new();
    }
}
