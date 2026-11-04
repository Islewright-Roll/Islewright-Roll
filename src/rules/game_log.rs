// SPDX-License-Identifier: GPL-3.0-or-later
//! The played games: one short record for each game that reached its end, newest last. A
//! record keeps what the end of the game looked like (the pieces on the board, the bank slots
//! spent, the running score of each turn) as strings of 0 and 1, so the board can be drawn
//! again from it by the same renderer that drew the game. A few hundred bytes each.
//!
//! The document is a JSON array of objects:
//!
//! ```text
//! { "map": "dunholm", "date": 1793600000000, "turns": 12, "score": 10,
//!   "roads": "110100...", "villages": "0010100", "cities": "1000", "traders": "110000000",
//!   "bank": "0100000", "empty": 0, "turnScores": [0, 1, 1, 3, ...] }
//! ```
//!
//! "traders" and "bank" are 1 based on the sheet and written from tier and slot 1. "bank"
//! marks the slots spent. "turnScores" is the running score at the end of each turn, from
//! turn 1. A record that does not fit its island (an unknown island, a length that does not
//! match) is skipped when read, never mended.

use super::game::Game;
use super::json::{Json, JsonError};
use super::map_def::MapDef;
use super::rng::Rng;

/// How many records are kept; the oldest go first.
pub const KEPT: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayedGame {
    pub map: String,
    /// When it ended, in ms since the epoch; 0 when not known.
    pub date: i64,
    pub turns: i32,
    pub score: i32,
    pub roads: Vec<bool>,
    pub villages: Vec<bool>,
    pub cities: Vec<bool>,
    /// From tier 1.
    pub traders: Vec<bool>,
    /// From slot 1: true where the slot was spent.
    pub bank_spent: Vec<bool>,
    pub turns_nothing_built: i32,
    /// From turn 1.
    pub turn_scores: Vec<i32>,
}

fn bits(v: &[bool]) -> String {
    v.iter().map(|&b| if b { '1' } else { '0' }).collect()
}

fn read_bits(j: &Json, key: &str, length: usize) -> Result<Vec<bool>, JsonError> {
    let s = j
        .get(key)
        .and_then(Json::as_str)
        .ok_or_else(|| JsonError(format!("{key} is not a string")))?;
    if s.len() != length || !s.chars().all(|c| c == '0' || c == '1') {
        return Err(JsonError(format!("{key} is not {length} bits")));
    }
    Ok(s.chars().map(|c| c == '1').collect())
}

fn read_int(j: &Json, key: &str) -> Result<i64, JsonError> {
    j.get(key)
        .and_then(Json::as_i64_lenient)
        .ok_or_else(|| JsonError(format!("{key} is not a number")))
}

impl PlayedGame {
    /// The record of a game as it stands, which is how it ended when it is over.
    pub fn of(game: &Game, date: i64) -> PlayedGame {
        let s = &game.playsheet;
        let bank = s.bank_size();
        PlayedGame {
            map: game.map.id.clone(),
            date,
            turns: game.turns_taken,
            score: s.score(),
            roads: s.roads.clone(),
            villages: s.villages.clone(),
            cities: s.cities.clone(),
            traders: s.traders[1..].to_vec(),
            bank_spent: (1..=bank).map(|slot| !s.resources_avail[slot]).collect(),
            turns_nothing_built: s.turns_nothing_built,
            turn_scores: (1..=game.turns_taken as usize)
                .map(|t| s.turn_score.get(t).copied().unwrap_or(0))
                .collect(),
        }
    }

    /// The game's end again, ready to draw: its island, its pieces, its score and turns, and
    /// nothing left to roll or build. None for an island this version does not have.
    pub fn restore(&self) -> Option<Game> {
        let map = MapDef::find(&self.map)?;
        let mut game = Game::with_rng(map, Rng::new(0));
        game.start_new_game(map);
        game.reset();
        game.turns_taken = self.turns;
        game.rolls = 0;
        for die in game.dice.iter_mut() {
            die.reset();
        }
        let s = &mut game.playsheet;
        s.roads.copy_from_slice(&self.roads);
        s.villages.copy_from_slice(&self.villages);
        s.cities.copy_from_slice(&self.cities);
        s.traders[1..].copy_from_slice(&self.traders);
        for (i, &spent) in self.bank_spent.iter().enumerate() {
            s.resources_avail[i + 1] = !spent;
        }
        s.turns_nothing_built = self.turns_nothing_built;
        if s.turn_score.len() <= self.turn_scores.len() {
            s.turn_score.resize(self.turn_scores.len() + 1, 0);
        }
        for (t, &score) in self.turn_scores.iter().enumerate() {
            s.turn_score[t + 1] = score;
        }
        // A race is over at the score it reached, whatever the target is set to now
        if game.is_race() {
            game.target_score = self.score.max(1);
        }
        game.scored = true;
        game.tutorial_paused = true;
        Some(game)
    }

    pub fn to_json(&self) -> Json {
        let mut j = Json::object();
        j.put("map", self.map.as_str());
        j.put("date", self.date);
        j.put("turns", self.turns);
        j.put("score", self.score);
        j.put("roads", bits(&self.roads));
        j.put("villages", bits(&self.villages));
        j.put("cities", bits(&self.cities));
        j.put("traders", bits(&self.traders));
        j.put("bank", bits(&self.bank_spent));
        j.put("empty", self.turns_nothing_built);
        j.put(
            "turnScores",
            Json::Array(self.turn_scores.iter().map(|&s| Json::from(s)).collect()),
        );
        j
    }

    /// A record read back, checked against its island's sizes.
    pub fn from_json(j: &Json) -> Result<PlayedGame, JsonError> {
        let id = j
            .get("map")
            .and_then(Json::as_str)
            .ok_or_else(|| JsonError("map is not a string".into()))?;
        let map = MapDef::find(id).ok_or_else(|| JsonError(format!("no island {id}")))?;
        let sheet = super::playsheet::Playsheet::new(map);
        let turns = read_int(j, "turns")? as i32;
        let turn_scores = match j.get("turnScores") {
            Some(Json::Array(items)) => items
                .iter()
                .map(|v| v.as_i64_lenient().map(|n| n as i32))
                .collect::<Option<Vec<i32>>>()
                .ok_or_else(|| JsonError("turnScores holds a non-number".into()))?,
            _ => return Err(JsonError("turnScores is not an array".into())),
        };
        if turns < 0 || turn_scores.len() != turns as usize {
            return Err(JsonError("turnScores does not match turns".into()));
        }
        Ok(PlayedGame {
            map: map.id.clone(),
            date: read_int(j, "date")?,
            turns,
            score: read_int(j, "score")? as i32,
            roads: read_bits(j, "roads", sheet.roads.len())?,
            villages: read_bits(j, "villages", sheet.villages.len())?,
            cities: read_bits(j, "cities", sheet.cities.len())?,
            traders: read_bits(j, "traders", sheet.traders.len() - 1)?,
            bank_spent: read_bits(j, "bank", sheet.bank_size())?,
            turns_nothing_built: read_int(j, "empty")? as i32,
            turn_scores,
        })
    }
}

/// The played games, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameLog {
    pub games: Vec<PlayedGame>,
}

impl GameLog {
    /// Adds a finished game, letting the oldest go past KEPT.
    pub fn add(&mut self, game: PlayedGame) {
        self.games.push(game);
        if self.games.len() > KEPT {
            let extra = self.games.len() - KEPT;
            self.games.drain(..extra);
        }
    }

    pub fn to_json(&self) -> String {
        Json::Array(self.games.iter().map(PlayedGame::to_json).collect()).to_compact()
    }

    /// The log read back. A record that does not read is left out; a document that is not an
    /// array at all is an error, so the caller can keep it aside rather than overwrite it.
    pub fn from_json(text: &str) -> Result<GameLog, JsonError> {
        let doc = Json::parse(text)?;
        let items = doc
            .as_array()
            .ok_or_else(|| JsonError("the played games are not an array".into()))?;
        Ok(GameLog {
            games: items
                .iter()
                .filter_map(|j| PlayedGame::from_json(j).ok())
                .collect(),
        })
    }
}
