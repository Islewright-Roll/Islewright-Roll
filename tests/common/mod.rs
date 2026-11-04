// SPDX-License-Identifier: GPL-3.0-or-later
//! Fixtures the rules tests share: race maps that are not shipped, dice, and a sound
//! recorder. Each test file uses only some of them.
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use dayapp::rules::{Dice, DiceValue, Game, MapDef, Playsheet, Sfx, SoundSink};

/// Dunholm's road table as the geometry gives it.
pub const EXPECTED_ROAD_NEEDS: [&[usize]; 18] = [
    &[],
    &[0, 2, 3],
    &[1, 3],
    &[1, 2, 4],
    &[3, 5, 6],
    &[4, 6],
    &[4, 5, 7],
    &[6, 8],
    &[7, 9],
    &[8, 10],
    &[9, 11],
    &[10, 12, 13],
    &[11, 13],
    &[11, 12, 14],
    &[13, 15],
    &[14, 16],
    &[15, 17],
    &[16],
];

/// Dunholm's tables under another id, so a test can break one table or build a map that is
/// not among the shipped ones without disturbing the shipped map or its cached board.
pub fn race_builder() -> MapDef {
    let mut b = MapDef::dunholm_tables();
    b.id = "race-fixture".into();
    b
}

pub fn leak(map: MapDef) -> &'static MapDef {
    Box::leak(Box::new(map))
}

/// The race fixture map.
pub fn race() -> &'static MapDef {
    static MAP: OnceLock<&'static MapDef> = OnceLock::new();
    MAP.get_or_init(|| leak(race_builder().check().expect("race fixture")))
}

/// Dunholm's tables with the twenty turn cap the island once had. No shipped map is a race
/// with a cap any more, but the code still handles one (the ship on the board, a race that
/// runs out of turns with nothing to record), and these keep that path pinned.
pub fn capped() -> &'static MapDef {
    static MAP: OnceLock<&'static MapDef> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut b = MapDef::dunholm_tables();
        b.id = "race-capped".into();
        b.turn_cap = 20;
        leak(b.check().expect("capped fixture"))
    })
}

pub fn tarnholm() -> &'static MapDef {
    MapDef::tarnholm()
}

pub fn dunholm() -> &'static MapDef {
    MapDef::dunholm()
}

pub fn same(a: &MapDef, b: &MapDef) -> bool {
    std::ptr::eq(a, b)
}

/// Six dice showing these values, the rest showing nothing.
pub fn dice(values: &[DiceValue]) -> Vec<Dice> {
    (0..6)
        .map(|i| Dice::showing(values.get(i).copied().unwrap_or(DiceValue::None)))
        .collect()
}

/// The game's dice reset to these values, the rest showing nothing, on the first roll.
pub fn give_dice(game: &mut Game, values: &[DiceValue]) {
    for (i, die) in game.dice.iter_mut().enumerate() {
        die.used = false;
        die.value = values.get(i).copied().unwrap_or(DiceValue::None);
    }
    game.rolls = 1;
}

pub fn empty_turns(game: &mut Game, n: usize) {
    for _ in 0..n {
        game.new_turn(false);
    }
}

/// Six villages and a castle: eight points, short of the target, with road 12 down for castle 1.
pub fn score_eight(game: &mut Game) {
    let sheet = &mut game.playsheet;
    for i in 0..6 {
        sheet.villages[i] = true;
    }
    sheet.cities[0] = true;
    sheet.roads[12] = true;
    assert_eq!(8, sheet.score());
}

pub fn usable(dice: &[Dice]) -> usize {
    dice.iter().filter(|d| d.is_usable()).count()
}

/// A sheet's traders as a string of 0 and 1, index 0 first.
pub fn bits(s: &str) -> Vec<bool> {
    s.chars().map(|c| c == '1').collect()
}

pub fn sheet_with(map: &'static MapDef, traders: &[usize]) -> Playsheet {
    let mut sheet = Playsheet::new(map);
    for &tier in traders {
        sheet.build_knight(tier);
    }
    sheet
}

/// A sound sink that writes down what it was asked for, as "SFX@delay".
#[derive(Clone, Default)]
pub struct Recorder {
    pub log: Rc<RefCell<Vec<String>>>,
}

impl Recorder {
    /// What was asked for since the last take, as "[A@0, B@300]".
    pub fn take(&self) -> String {
        let s = format!("[{}]", self.log.borrow().join(", "));
        self.log.borrow_mut().clear();
        s
    }
}

impl SoundSink for Recorder {
    fn play(&mut self, sfx: Sfx, delay_ms: i32) {
        self.log.borrow_mut().push(format!("{sfx}@{delay_ms}"));
    }
}

/// A game whose sounds go to a recorder.
pub fn recorded(map: &'static MapDef) -> (Game, Recorder) {
    let r = Recorder::default();
    let mut game = Game::new(map);
    game.sounds = Some(Box::new(r.clone()));
    (game, r)
}

/// A game whose results all take this rank on the top scores.
pub fn ranking(game: &mut Game, rank: i32) {
    game.recorder = Some(Box::new(move |_, _| rank));
}
