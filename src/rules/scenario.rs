// SPDX-License-Identifier: GPL-3.0-or-later
//! A test scenario: dice, and enough of the sheet's board state (roads, villages, cities,
//! traders, spent bank slots) to set up situations that would be impractical to reach by
//! playing, on either island. The files under tools/scenarios are these; a debug build or a
//! dayscript walkthrough can load one.
//!
//! from_json parses and fully validates before returning: every field is checked for shape
//! and range against the map the scenario names (lengths, tier ranges and the turn range all
//! come from that map), so a scenario either comes back whole or is refused and changes
//! nothing. apply_to only ever runs on a scenario that passed that gate, and only on a game
//! already on that same map (the caller starts one), so it has nothing left to refuse.
//!
//! This does not check board legality (whether the pieces given could be reached by legal
//! play). That is the scenario author's job; the point is precision for testing.
//!
//! The document, all fields optional:
//!
//! ```text
//! {
//!   "map": "tarnholm",       a map id; default is the default map. Unknown ids are refused.
//!   "dice": ["Brick", "Brick", "Lumber", "Lumber", "Wool", "Ore"],   exactly 6
//!   "turnsTaken": 0,         0 to turnCap - 1 of the map (any on a map with no cap)
//!   "turnScores": [2, 3, 1], the points gained in each passed turn, one per completed turn;
//!                            without it the running totals the HUD strip reads are spread
//!                            evenly so the scenario's score is reached by the last passed
//!                            turn and the turn in progress shows +0
//!   "rolls": 1,              0 to 3, default 1
//!   "playsheet": {
//!     "roads": [true, false, ...],     one per road of the map, index 0 the start road
//!     "villages": [false, ...],        one per village of the map
//!     "cities": [false, ...],          one per castle of the map
//!     "knights": 3,                    maps whose traders are built in order only: how many
//!     "traders": [true, false, ...],   one per trader in tier order; on an ordered map the
//!                                      flags must form a prefix, and agree with "knights"
//!     "bankSpent": [6],                1 based bank slots already used
//!     "knightResourcesSpent": [1]      an older name for bankSpent (not both)
//!   }
//! }
//! ```

use super::dice::DiceValue;
use super::game::Game;
use super::json::{Json, JsonError};
use super::map_def::MapDef;

#[derive(Clone, Debug)]
pub struct Scenario {
    pub map: &'static MapDef,
    pub dice: Option<[DiceValue; 6]>,
    pub turns_taken: Option<i32>,
    /// Per passed turn gain, or None to spread the score evenly.
    pub turn_scores: Option<Vec<i32>>,
    pub rolls: Option<i32>,
    pub roads: Option<Vec<bool>>,
    pub villages: Option<Vec<bool>>,
    pub cities: Option<Vec<bool>>,
    /// One per tier, already merged from "knights" or "traders".
    pub traders: Option<Vec<bool>>,
    pub bank_spent: Option<Vec<usize>>,
}

fn err<T>(what: String) -> Result<T, JsonError> {
    Err(JsonError(what))
}

fn int(v: &Json, field: &str) -> Result<i32, JsonError> {
    v.as_i64_lenient()
        .map(|n| n as i32)
        .ok_or_else(|| JsonError(format!("{field} is not a number")))
}

fn array<'a>(j: &'a Json, key: &str, field: &str) -> Result<&'a [Json], JsonError> {
    match j.get(key) {
        Some(Json::Array(items)) => Ok(items),
        _ => err(format!("{field} is not an array")),
    }
}

fn booleans(j: &Json, key: &str, length: usize, field: &str) -> Result<Vec<bool>, JsonError> {
    let items = array(j, key, field)?;
    if items.len() != length {
        return err(format!(
            "{field} must have exactly {length} entries, had {}",
            items.len()
        ));
    }
    items
        .iter()
        .map(|v| {
            v.as_bool_lenient()
                .ok_or_else(|| JsonError(format!("{field} holds something that is not a boolean")))
        })
        .collect()
}

impl Scenario {
    pub fn from_json(json: &str) -> Result<Scenario, JsonError> {
        let root = Json::parse(json)?;
        let mut map = MapDef::default_map();
        if let Some(v) = root.get("map") {
            let id = v.as_str().ok_or("map is not a string")?;
            map = MapDef::find(id).ok_or_else(|| JsonError(format!("unknown map \"{id}\"")))?;
        }
        let mut s = Scenario {
            map,
            dice: None,
            turns_taken: None,
            turn_scores: None,
            rolls: None,
            roads: None,
            villages: None,
            cities: None,
            traders: None,
            bank_spent: None,
        };

        if root.has("dice") {
            let items = array(&root, "dice", "dice")?;
            if items.len() != 6 {
                return err(format!(
                    "dice must have exactly 6 entries, had {}",
                    items.len()
                ));
            }
            let mut dice = [DiceValue::None; 6];
            for (d, v) in dice.iter_mut().zip(items) {
                let name = v.as_str().ok_or("a die is not a string")?;
                *d = DiceValue::from_name(name)
                    .ok_or_else(|| JsonError(format!("unrecognised dice value \"{name}\"")))?;
            }
            s.dice = Some(dice);
        }

        if let Some(v) = root.get("turnsTaken") {
            let turns = int(v, "turnsTaken")?;
            // A map with no turn cap takes any number of completed turns
            if turns < 0 || (map.turn_cap > 0 && turns > map.turn_cap - 1) {
                return err(format!(
                    "turnsTaken must be between 0 and {} on {}, was {turns}",
                    map.turn_cap - 1,
                    map.id
                ));
            }
            s.turns_taken = Some(turns);
        }

        if root.has("turnScores") {
            let items = array(&root, "turnScores", "turnScores")?;
            let passed = s.turns_taken.unwrap_or(0) as usize;
            if items.len() != passed {
                return err(format!(
                    "turnScores must have exactly one entry per completed turn ({passed}), had {}",
                    items.len()
                ));
            }
            let mut gains = Vec::new();
            for v in items {
                let gain = int(v, "turnScores")?;
                if gain < 0 {
                    return err(format!("turnScores entries must be 0 or more, was {gain}"));
                }
                gains.push(gain);
            }
            s.turn_scores = Some(gains);
        }

        if let Some(v) = root.get("rolls") {
            let rolls = int(v, "rolls")?;
            if !(0..=3).contains(&rolls) {
                return err(format!("rolls must be between 0 and 3, was {rolls}"));
            }
            s.rolls = Some(rolls);
        }

        if let Some(p) = root.get("playsheet") {
            if !matches!(p, Json::Object(_)) {
                return err("playsheet is not an object".into());
            }
            let tiers = map.traders.len();
            if p.has("roads") {
                s.roads = Some(booleans(p, "roads", map.roads.len(), "playsheet.roads")?);
            }
            if p.has("villages") {
                s.villages = Some(booleans(
                    p,
                    "villages",
                    map.villages.len(),
                    "playsheet.villages",
                )?);
            }
            if p.has("cities") {
                s.cities = Some(booleans(p, "cities", map.cities.len(), "playsheet.cities")?);
            }

            let mut knights = None;
            if let Some(v) = p.get("knights") {
                if !map.ordered_traders {
                    return err(format!(
                        "playsheet.knights needs a map whose traders are built in order; {} is not, use playsheet.traders",
                        map.id
                    ));
                }
                let count = int(v, "playsheet.knights")?;
                if count < 0 || count as usize > tiers {
                    return err(format!(
                        "playsheet.knights must be between 0 and {tiers} on {}, was {count}",
                        map.id
                    ));
                }
                knights = Some(count as usize);
            }
            let mut flags = if p.has("traders") {
                Some(booleans(p, "traders", tiers, "playsheet.traders")?)
            } else {
                None
            };
            if let (Some(flags), Some(k)) = (&flags, knights)
                && let Some(i) = (0..tiers).find(|&i| flags[i] != (i < k))
            {
                return err(format!(
                    "playsheet.traders and playsheet.knights disagree at trader {}",
                    i + 1
                ));
            }
            if flags.is_none()
                && let Some(k) = knights
            {
                flags = Some((0..tiers).map(|i| i < k).collect());
            }
            if let Some(flags) = &flags
                && map.ordered_traders
                && let Some(i) = (1..tiers).find(|&i| flags[i] && !flags[i - 1])
            {
                return err(format!(
                    "playsheet.traders must be a prefix on {}: trader {} needs trader {i}",
                    map.id,
                    i + 1
                ));
            }
            s.traders = flags;

            if p.has("bankSpent") && p.has("knightResourcesSpent") {
                return err("give playsheet.bankSpent or its alias playsheet.knightResourcesSpent, not both".into());
            }
            let key = if p.has("bankSpent") {
                "bankSpent"
            } else {
                "knightResourcesSpent"
            };
            if p.has(key) {
                let size = map.bank_size();
                let mut spent = Vec::new();
                for v in array(p, key, &format!("playsheet.{key}"))? {
                    let slot = int(v, key)?;
                    if slot < 1 || slot as usize > size {
                        return err(format!(
                            "playsheet.{key} entries must be between 1 and {size} on {}, was {slot}",
                            map.id
                        ));
                    }
                    spent.push(slot as usize);
                }
                s.bank_spent = Some(spent);
            }
        }
        Ok(s)
    }

    /// Sets the game to match this scenario. Always starts from Game::reset, so the result
    /// never depends on whatever state the game happened to be in before; every field this
    /// scenario did not mention keeps reset's default. The game must be on this scenario's
    /// map (start one with Game::start_new_game first if it is not).
    pub fn apply_to(&self, game: &mut Game) {
        debug_assert!(
            std::ptr::eq(game.map, self.map),
            "a scenario applies to a game on its own map"
        );
        game.reset();
        if let Some(dice) = self.dice {
            for (die, value) in game.dice.iter_mut().zip(dice) {
                die.reset();
                die.value = value;
            }
        }
        if let Some(turns) = self.turns_taken {
            game.turns_taken = turns;
        }
        game.rolls = self.rolls.unwrap_or(1);
        let sheet = &mut game.playsheet;
        if let Some(roads) = &self.roads {
            sheet.roads.copy_from_slice(roads);
        }
        if let Some(villages) = &self.villages {
            sheet.villages.copy_from_slice(villages);
        }
        if let Some(cities) = &self.cities {
            sheet.cities.copy_from_slice(cities);
        }
        if let Some(traders) = &self.traders {
            sheet.traders[1..].copy_from_slice(traders);
        }
        if let Some(spent) = &self.bank_spent {
            for &slot in spent {
                sheet.resources_avail[slot] = false;
            }
        }
        self.fill_turn_scores(game, self.turns_taken.unwrap_or(0));
        // The tutorial would read this setup as a player's moves and draw over it
        game.tutorial_paused = true;
    }

    /// Fills the running totals for the turns already passed (Playsheet::turn_score, which the
    /// HUD strip's per turn flags read), since the pieces above are set directly and no turn
    /// was ever scored. Default: the final score rises evenly across the passed turns, whole
    /// numbers that never decrease, ending on the current score so the turn in progress shows
    /// +0. With turn_scores the totals are the running sum of the given gains instead.
    fn fill_turn_scores(&self, game: &mut Game, passed: i32) {
        if passed < 1 {
            return;
        }
        let sheet = &mut game.playsheet;
        let passed = passed as usize;
        if sheet.turn_score.len() <= passed {
            // A map with no turn cap grows the vector as turns are scored; do the same here
            sheet.turn_score.resize(passed + 1, 0);
        }
        let total = sheet.score();
        let mut running = 0;
        for turn in 1..=passed {
            sheet.turn_score[turn] = match &self.turn_scores {
                Some(gains) => {
                    running += gains[turn - 1];
                    running
                }
                None => (total * turn as i32).div_euclid(passed as i32),
            };
        }
    }
}
