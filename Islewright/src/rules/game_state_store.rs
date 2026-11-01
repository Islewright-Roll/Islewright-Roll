// SPDX-License-Identifier: GPL-3.0-or-later
//! The games in progress, one JSON document per island, deliberately apart from the scores
//! and settings document: that one is exported and imported by the player as a backup, and
//! a half finished game does not belong in it.
//!
//! Each island keeps its own document under its own key, so switching islands never loses a
//! game. The UI stores the string under `key_for(map)` in its preferences: `persist` says
//! after each change whether to write the game or clear the key (a finished game is never
//! kept), and `load` turns what was stored back into a game, or says to clear a key that
//! holds something it must not restore.
//!
//! Only the current format is read. The App Fair build is a new install, so nothing written
//! by the Android app ever reaches it, and the old formats' migration paths are gone.

use super::dice::DiceValue;
use super::game::Game;
use super::json::{Json, JsonError};
use super::map_def::MapDef;

pub const SCHEMA_VERSION: i64 = 2;

/// The default island's key. Every other island's is "game-state-<mapId>.json".
pub const DEFAULT_KEY: &str = "game-state.json";

/// The storage key for one island's game.
pub fn key_for(map: &MapDef) -> String {
    if map.is_default() {
        DEFAULT_KEY.to_owned()
    } else {
        format!("game-state-{}.json", map.id)
    }
}

/// What to do with an island's stored game after a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Persist {
    /// Store this document under the island's key.
    Save(String),
    /// The game is over: remove the island's key, so a finished game never comes back.
    Clear,
}

/// Writes the game if it is still going and clears its island's save if it is over. The one
/// place that decides between the two.
pub fn persist(game: &Game) -> Persist {
    if game.is_game_done() {
        Persist::Clear
    } else {
        Persist::Save(to_json(game))
    }
}

/// What came of reading an island's stored game.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The game to go on with, or None for a fresh one.
    pub game: Option<Game>,
    /// True when the stored document must be removed: it did not parse, it holds another
    /// island's game, or it somehow reached the end of the game. Such a document is set aside
    /// rather than left to fail again on the next launch.
    pub clear_stored: bool,
    /// Why the document was refused, for a log.
    pub error: Option<String>,
}

/// Turns one island's stored document back into its game. Nothing stored is no game. A
/// document that does not parse, or holds another island's game, is refused and cleared. So
/// is a save that reached the end of the game: that should never happen, since the save is
/// cleared as soon as the game is done, but returning nothing instead of resurrecting a
/// finished game keeps a stray document from ever surfacing one.
pub fn load(stored: Option<&str>, map: &'static MapDef) -> Loaded {
    load_with_maps(stored, map, &MapDef::all())
}

/// As `load`, with the maps a save may name given by the caller.
pub fn load_with_maps(
    stored: Option<&str>,
    map: &'static MapDef,
    maps: &[&'static MapDef],
) -> Loaded {
    let Some(json) = stored else {
        return Loaded::default();
    };
    let refused = |error: String| Loaded {
        game: None,
        clear_stored: true,
        error: Some(error),
    };
    match from_json_with_maps(json, maps) {
        Err(e) => refused(e.0),
        Ok(game) if !std::ptr::eq(game.map, map) => refused(format!(
            "the document for \"{}\" holds a game on \"{}\"",
            map.id, game.map.id
        )),
        Ok(game) if game.is_game_done() => Loaded {
            game: None,
            clear_stored: true,
            error: None,
        },
        Ok(game) => Loaded {
            game: Some(game),
            clear_stored: false,
            error: None,
        },
    }
}

/// The game as a compact document, keys in a fixed order.
pub fn to_json(game: &Game) -> String {
    build_json(game).to_compact()
}

/// The game as a document tree.
pub fn build_json(game: &Game) -> Json {
    let mut root = Json::object();
    root.put("schemaVersion", SCHEMA_VERSION);

    let mut g = Json::object();
    g.put("mapId", game.map.id.as_str());
    g.put("rolls", game.rolls);
    g.put("turnsTaken", game.turns_taken);
    g.put("builtResourceThisTurn", game.built_resource_this_turn);
    g.put("scored", game.scored);
    // Only a race has a target, and it is the game's own: the setting may have moved since
    // the game began
    if game.is_race() {
        g.put("targetScore", game.target_score);
    }
    g.put(
        "knightResources",
        game.knight_resources()
            .iter()
            .map(|v| v.name())
            .collect::<Vec<_>>(),
    );

    let dice: Vec<Json> = game
        .dice
        .iter()
        .map(|die| {
            let mut d = Json::object();
            d.put("value", die.value.name());
            d.put("held", die.held);
            d.put("used", die.used);
            d.put("locked", die.locked);
            d
        })
        .collect();
    g.put("dice", Json::Array(dice));

    let sheet = &game.playsheet;
    let mut p = Json::object();
    // The vector itself, not a count: a map whose traders can be built in any order has
    // states a count cannot say. Index 0 is unused, as in the sheet.
    p.put("traders", sheet.traders.clone());
    p.put("resourcesAvail", sheet.resources_avail.clone());
    p.put("roads", sheet.roads.clone());
    p.put("villages", sheet.villages.clone());
    p.put("cities", sheet.cities.clone());
    p.put("turnsNothingBuilt", sheet.turns_nothing_built);
    p.put("turnScore", sheet.turn_score.clone());
    g.put("playsheet", p);

    root.put("game", g);
    root
}

/// Parses a game over the shipped maps. See from_json_with_maps.
pub fn from_json(json: &str) -> Result<Game, JsonError> {
    from_json_with_maps(json, &MapDef::all())
}

/// Parses and fully applies a save in one pass. Reads the map first, then starts from a
/// plain `Game::new(map)` and overwrites every field this module stores. A map id nobody
/// knows is refused rather than loaded as the default map, where it would mangle a game a
/// newer build could still read. Anything that does not match the expected shape is refused,
/// so the caller can fall back to a fresh game rather than restore something half wrong.
pub fn from_json_with_maps(json: &str, maps: &[&'static MapDef]) -> Result<Game, JsonError> {
    let root = Json::parse(json)?;
    let schema = root
        .get("schemaVersion")
        .and_then(Json::as_i64_lenient)
        .unwrap_or(-1);
    if schema != SCHEMA_VERSION {
        return Err(format!("unsupported schemaVersion {schema}").into());
    }
    let g = object(&root, "game")?;
    let map_id = string(g, "mapId")?;
    let map = maps
        .iter()
        .copied()
        .find(|m| m.id == map_id)
        .ok_or_else(|| JsonError(format!("unknown mapId \"{map_id}\"")))?;
    let mut game = Game::new(map);

    game.rolls = int(g, "rolls")?;
    game.turns_taken = int(g, "turnsTaken")?;
    game.built_resource_this_turn = boolean(g, "builtResourceThisTurn")?;
    game.scored = boolean(g, "scored")?;
    // A save from before the target could be set raced to the island's own. A stray value
    // from a hand edit is pulled into the range a game can reach.
    if game.is_race() {
        let target = g
            .get("targetScore")
            .and_then(Json::as_i64_lenient)
            .map_or(map.target_score, |v| v as i32);
        game.target_score = target.min(map.max_score()).max(1);
    }

    let mut set_aside = Vec::new();
    for v in array(g, "knightResources")? {
        set_aside.push(dice_value(v)?);
    }
    *game.knight_resources_mut() = set_aside;

    let dice = array(g, "dice")?;
    if dice.len() != game.dice.len() {
        return Err(format!(
            "dice must have exactly {} entries, had {}",
            game.dice.len(),
            dice.len()
        )
        .into());
    }
    for (die, d) in game.dice.iter_mut().zip(dice) {
        die.value = dice_value(d.get("value").ok_or("a die has no value")?)?;
        die.held = boolean(d, "held")?;
        die.used = boolean(d, "used")?;
        die.locked = d
            .get("locked")
            .and_then(Json::as_bool_lenient)
            .unwrap_or(false);
    }

    let p = object(g, "playsheet")?;
    let sheet = &mut game.playsheet;
    sheet.traders = booleans(p, "traders", sheet.traders.len())?;
    sheet.traders[0] = false;
    sheet.resources_avail = booleans(p, "resourcesAvail", sheet.resources_avail.len())?;
    sheet.roads = booleans(p, "roads", sheet.roads.len())?;
    sheet.villages = booleans(p, "villages", sheet.villages.len())?;
    sheet.cities = booleans(p, "cities", sheet.cities.len())?;
    sheet.turns_nothing_built = int(p, "turnsNothingBuilt")?;
    // A map with no turn cap grows its turn scores as the game goes on, so any length is
    // right for it
    let scores = array(p, "turnScore")?;
    if map.turn_cap != 0 && scores.len() != sheet.turn_score.len() {
        return Err(format!(
            "turnScore must have exactly {} entries, had {}",
            sheet.turn_score.len(),
            scores.len()
        )
        .into());
    }
    let mut turn_score = vec![0; scores.len().max(1)];
    for (i, v) in scores.iter().enumerate() {
        turn_score[i] =
            v.as_i64_lenient()
                .ok_or("turnScore holds something that is not a number")? as i32;
    }
    sheet.turn_score = turn_score;
    Ok(game)
}

fn object<'a>(j: &'a Json, key: &str) -> Result<&'a Json, JsonError> {
    match j.get(key) {
        Some(v @ Json::Object(_)) => Ok(v),
        _ => Err(format!("{key} is not an object").into()),
    }
}

fn array<'a>(j: &'a Json, key: &str) -> Result<&'a [Json], JsonError> {
    match j.get(key) {
        Some(Json::Array(items)) => Ok(items),
        _ => Err(format!("{key} is not an array").into()),
    }
}

fn string(j: &Json, key: &str) -> Result<String, JsonError> {
    match j.get(key) {
        Some(Json::Str(s)) => Ok(s.clone()),
        Some(v) if !matches!(v, Json::Null) => Ok(v.to_compact()),
        _ => Err(format!("{key} is missing").into()),
    }
}

fn int(j: &Json, key: &str) -> Result<i32, JsonError> {
    j.get(key)
        .and_then(Json::as_i64_lenient)
        .map(|v| v as i32)
        .ok_or_else(|| format!("{key} is not a number").into())
}

fn boolean(j: &Json, key: &str) -> Result<bool, JsonError> {
    j.get(key)
        .and_then(Json::as_bool_lenient)
        .ok_or_else(|| format!("{key} is not a boolean").into())
}

fn booleans(j: &Json, key: &str, length: usize) -> Result<Vec<bool>, JsonError> {
    let items = array(j, key)?;
    if items.len() != length {
        return Err(format!(
            "{key} must have exactly {length} entries, had {}",
            items.len()
        )
        .into());
    }
    items
        .iter()
        .map(|v| {
            v.as_bool_lenient()
                .ok_or_else(|| JsonError(format!("{key} holds something that is not a boolean")))
        })
        .collect()
}

fn dice_value(v: &Json) -> Result<DiceValue, JsonError> {
    let name = v.as_str().ok_or("a dice value is not a string")?;
    DiceValue::from_name(name).ok_or_else(|| format!("unrecognised dice value \"{name}\"").into())
}
