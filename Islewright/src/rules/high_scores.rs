// SPDX-License-Identifier: GPL-3.0-or-later
//! The top scores and settings, held in one JSON document. The UI keeps the document as a
//! string in its preferences (and the player can export it as a backup and import it again);
//! this module reads and writes that string and holds the rules for filing a result.
//!
//! The first island's list stays under the "highScores" key and every other island's list
//! sits in "highScoresByMap". A setting left at its default, and never written by the
//! player, is left out of the document, so importing an old backup can tell "not said" from
//! "said the default".
//!
//! Compatibility with the documents the Android app wrote is not a goal (the App Fair build
//! is a new install), but the shape is kept, so such a document still imports.

use super::json::{Json, JsonError};
use super::map_def::MapDef;
use super::player_color;
use super::tutorial;

pub const SCHEMA_VERSION: i64 = 1;
pub const MAX_SCORES: usize = 5;

/// When to ask before building early, which forfeits the rolls not yet used.
///
/// The number is how many dice the turn must be able to spend, counting the whole sequence of
/// builds reachable from the tap and not just the tapped one, before the prompt is skipped as
/// obviously deliberate.
///
/// SMART settles for five and so asks less often. CONSERVATIVE wants every die and so asks
/// more often, which is what makes it the cautious one. OFF never asks.
pub const EARLY_PROMPT_OFF: i32 = 0;
pub const EARLY_PROMPT_SMART: i32 = 5;
pub const EARLY_PROMPT_CONSERVATIVE: i32 = 6;

/// The Music volume setting, 0 to 100. 0 is music off. How loud that is on top of the
/// device's media volume is music_gain's business.
pub const MUSIC_VOLUME_MIN: i32 = 0;
pub const MUSIC_VOLUME_MAX: i32 = 100;
pub const MUSIC_VOLUME_DEFAULT: i32 = 60;

/// The Dunholm target setting: the score that wins the race, 1 to the most the island can
/// give (MapDef::max_score). Read when a race starts and kept by the game, so a saved game
/// goes on with the target it was started with. Top Scores record only a game played to the
/// island's own target, since fewer turns to another target is not the same feat.
pub const RACE_TARGET_MIN: i32 = 1;
/// Dunholm's own target (MapDef::dunholm().target_score; a test holds them together).
pub const RACE_TARGET_DEFAULT: i32 = 10;

/// The Theme setting: follow the OS night mode, or always light, or always dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// The name a Theme is stored under: plain words, so a hand edit is easy.
    pub fn name(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// The Theme for a stored name, or None for one this build does not know.
    pub fn from_name(name: &str) -> Option<Theme> {
        match name {
            "system" => Some(Theme::System),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

/// One result on a list: the points or turns, and when it was played (milliseconds since
/// the epoch, 0 for unknown).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScoreAndDate {
    pub score: i32,
    pub date: i64,
}

impl ScoreAndDate {
    pub const fn new(score: i32, date: i64) -> ScoreAndDate {
        ScoreAndDate { score, date }
    }
}

#[derive(Clone, Debug)]
pub struct HighScoresAndSettings {
    /// The default map's scores, best first. They stay under the original "highScores" key.
    /// Every other map's list lives in other_scores; reach either through scores_for.
    pub scores: Vec<ScoreAndDate>,
    /// Scores of every other map by map id, in the order first seen, written as
    /// "highScoresByMap". A map this build does not know is kept exactly as read (not sorted,
    /// not trimmed, since its order is not ours to guess) so a document loses nothing.
    other_scores: Vec<(String, Vec<ScoreAndDate>)>,
    /// The maps a score list can belong to: what orders a list and what a result may name.
    maps: Vec<&'static MapDef>,

    pub player_color: u32,
    /// Whether the parsed document actually carried a playerColor, as opposed to falling back
    /// to the default. Import needs the distinction: applying a defaulted colour would
    /// silently repaint the board red whenever an imported file simply lacked the field.
    pub player_color_was_specified: bool,
    /// The id of the chosen map. A stored field only; the game reads it, this does not act on it.
    pub map: String,
    pub map_was_specified: bool,
    pub early_build_prompt: i32,
    /// Whether to warn when ending a turn with something still affordable from dice.
    pub end_turn_prompt: bool,
    /// Whether the game gives haptic feedback at all.
    pub haptic_feedback: bool,
    pub haptic_feedback_was_specified: bool,
    /// Whether the game plays its sound effects at all.
    pub sound_effects: bool,
    pub sound_effects_was_specified: bool,
    /// Trader resource classic mode: a banked resource is not spent by itself but dragged onto
    /// a rolled die, which it overwrites. Off by default. See Playsheet::bank_pays_builds.
    pub trader_classic: bool,
    pub trader_classic_was_specified: bool,
    pub theme: Theme,
    pub theme_was_specified: bool,
    /// Whether the welcome dialog is shown when the app starts. On for a new install.
    pub show_welcome: bool,
    pub show_welcome_was_specified: bool,
    /// Whether the in-context tutorial's callouts show during play. On for a new install;
    /// Skip tutorial, or seeing every callout, turns it off.
    pub tutorial: bool,
    pub tutorial_was_specified: bool,
    /// Which tutorial steps have been seen, one bit each (tutorial::Step::bit), so the
    /// tutorial resumes where it left off after the app is closed. 0 for none.
    pub tutorial_seen: u32,
    pub tutorial_seen_was_specified: bool,
    pub music_volume: i32,
    pub music_volume_was_specified: bool,
    pub race_target: i32,
    pub race_target_was_specified: bool,
}

impl Default for HighScoresAndSettings {
    fn default() -> Self {
        HighScoresAndSettings::new()
    }
}

impl HighScoresAndSettings {
    pub fn new() -> HighScoresAndSettings {
        HighScoresAndSettings::with_maps(MapDef::all().to_vec())
    }

    /// The same store over a different set of maps (for tests).
    pub fn with_maps(maps: Vec<&'static MapDef>) -> HighScoresAndSettings {
        HighScoresAndSettings {
            scores: Vec::new(),
            other_scores: Vec::new(),
            maps,
            player_color: player_color::DEFAULT,
            player_color_was_specified: false,
            map: MapDef::default_map().id.clone(),
            map_was_specified: false,
            early_build_prompt: EARLY_PROMPT_SMART,
            end_turn_prompt: true,
            haptic_feedback: true,
            haptic_feedback_was_specified: false,
            sound_effects: true,
            sound_effects_was_specified: false,
            trader_classic: false,
            trader_classic_was_specified: false,
            theme: Theme::System,
            theme_was_specified: false,
            show_welcome: true,
            show_welcome_was_specified: false,
            tutorial: true,
            tutorial_was_specified: false,
            tutorial_seen: 0,
            tutorial_seen_was_specified: false,
            music_volume: MUSIC_VOLUME_DEFAULT,
            music_volume_was_specified: false,
            race_target: RACE_TARGET_DEFAULT,
            race_target_was_specified: false,
        }
    }

    /// Inserts a score into one map's list at the first position whose entry it beats or ties
    /// (a tie enters at or above the equal score), trims to MAX_SCORES, and returns the 1 based
    /// rank, or 0 if it did not make the table. "Beats" follows the map: more points on a map
    /// where higher is better, fewer turns where lower is. now_ms is when it was played. A map
    /// this store does not know is a bug in the caller, not a score to file under some other
    /// map, so it is refused.
    pub fn add_score(&mut self, map_id: &str, score: i32, now_ms: i64) -> Result<usize, String> {
        let Some(def) = self.known_map(map_id) else {
            return Err(format!("unknown map \"{map_id}\""));
        };
        let lower = def.lower_is_better;
        let list = self.list_for_mut(map_id);
        let insert_at = list
            .iter()
            .position(|e| {
                if lower {
                    score <= e.score
                } else {
                    score >= e.score
                }
            })
            .unwrap_or(list.len());
        if insert_at >= MAX_SCORES {
            return Ok(0);
        }
        list.insert(insert_at, ScoreAndDate::new(score, now_ms));
        list.truncate(MAX_SCORES);
        Ok(insert_at + 1)
    }

    /// One map's results, best first: highest score first where higher is better, fewest
    /// turns first where lower is. Empty for an id this store has no list for.
    pub fn scores_for(&self, map_id: &str) -> &[ScoreAndDate] {
        if map_id == MapDef::default_map().id {
            return &self.scores;
        }
        self.other_scores
            .iter()
            .find(|(id, _)| id == map_id)
            .map(|(_, l)| l.as_slice())
            .unwrap_or(&[])
    }

    /// The ids of every map this store holds a list for, the default map first.
    pub fn map_ids_with_scores(&self) -> Vec<String> {
        let mut ids = vec![MapDef::default_map().id.clone()];
        ids.extend(self.other_scores.iter().map(|(id, _)| id.clone()));
        ids
    }

    /// Unions another list of one map's results into that map's list, deduping on (score,
    /// date), keeping the best MAX_SCORES in the map's own order. Returns how many of the given
    /// scores were new. A map this store does not know has the new entries appended and
    /// nothing else done to them.
    pub fn merge_scores(&mut self, map_id: &str, other: &[ScoreAndDate]) -> usize {
        let mut added = 0;
        {
            let list = self.list_for_mut(map_id);
            for sad in other {
                if !list
                    .iter()
                    .any(|e| e.score == sad.score && e.date == sad.date)
                {
                    list.push(*sad);
                    added += 1;
                }
            }
        }
        if added > 0 {
            self.order(map_id);
        }
        added
    }

    /// What importing a backup does: every map's results in the document are merged into this
    /// store, and each setting is taken over only where the document actually carried one
    /// (the parser defaults a missing value, and applying that would overwrite a real choice
    /// with a guess). Returns how many scores were new.
    pub fn import_from(&mut self, imported: &HighScoresAndSettings) -> usize {
        let mut added = 0;
        for map_id in imported.map_ids_with_scores() {
            added += self.merge_scores(&map_id, imported.scores_for(&map_id));
        }
        // A backup carries the player's colour and restoring one should bring it back
        if imported.player_color_was_specified {
            self.player_color = imported.player_color;
        }
        if imported.haptic_feedback_was_specified {
            self.haptic_feedback = imported.haptic_feedback;
        }
        if imported.sound_effects_was_specified {
            self.sound_effects = imported.sound_effects;
            self.sound_effects_was_specified = true;
        }
        if imported.trader_classic_was_specified {
            self.trader_classic = imported.trader_classic;
            self.trader_classic_was_specified = true;
        }
        if imported.theme_was_specified {
            self.theme = imported.theme;
            self.theme_was_specified = true;
        }
        if imported.show_welcome_was_specified {
            self.show_welcome = imported.show_welcome;
            self.show_welcome_was_specified = true;
        }
        if imported.tutorial_was_specified {
            self.tutorial = imported.tutorial;
            self.tutorial_was_specified = true;
        }
        if imported.tutorial_seen_was_specified {
            self.tutorial_seen = imported.tutorial_seen;
            self.tutorial_seen_was_specified = true;
        }
        if imported.music_volume_was_specified {
            self.music_volume = imported.music_volume;
            self.music_volume_was_specified = true;
        }
        if imported.race_target_was_specified {
            self.race_target = imported.race_target;
            self.race_target_was_specified = true;
        }
        if imported.map_was_specified {
            self.map = imported.map.clone();
            self.map_was_specified = true;
        }
        added
    }

    /// This store's map with the given id, or None when it has none.
    fn known_map(&self, map_id: &str) -> Option<&'static MapDef> {
        self.maps.iter().copied().find(|m| m.id == map_id)
    }

    /// The list for a map id, made empty if there is none.
    fn list_for_mut(&mut self, map_id: &str) -> &mut Vec<ScoreAndDate> {
        if map_id == MapDef::default_map().id {
            return &mut self.scores;
        }
        let at = match self.other_scores.iter().position(|(id, _)| id == map_id) {
            Some(at) => at,
            None => {
                self.other_scores.push((map_id.to_owned(), Vec::new()));
                self.other_scores.len() - 1
            }
        };
        &mut self.other_scores[at].1
    }

    /// Sorts a list best first for its map and cuts it to MAX_SCORES. A map nobody knows is
    /// left alone.
    fn order(&mut self, map_id: &str) {
        let Some(def) = self.known_map(map_id) else {
            return;
        };
        let lower = def.lower_is_better;
        let list = self.list_for_mut(map_id);
        if lower {
            list.sort_by_key(|e| e.score);
        } else {
            list.sort_by_key(|e| std::cmp::Reverse(e.score));
        }
        list.truncate(MAX_SCORES);
    }

    /// The document, indented, keys in a fixed order.
    pub fn to_json(&self) -> String {
        let mut root = Json::object();
        root.put("schemaVersion", SCHEMA_VERSION);
        root.put("highScores", scores_to_json(&self.scores));

        // Only for the maps that have results, so a player who never leaves the first island
        // has a document no different from one written before there were two
        let mut by_map = Json::object();
        for (map_id, list) in &self.other_scores {
            if !list.is_empty() {
                by_map.put(map_id, scores_to_json(list));
            }
        }
        if !by_map.keys().is_empty() {
            root.put("highScoresByMap", by_map);
        }

        let mut settings = Json::object();
        settings.put("playerColor", player_color::to_hex(self.player_color));
        settings.put("earlyBuildPrompt", self.early_build_prompt);
        settings.put("endTurnPrompt", self.end_turn_prompt);
        settings.put("hapticFeedback", self.haptic_feedback);
        // Each of these is left out while it is the default and was never in the document
        if self.sound_effects_was_specified || !self.sound_effects {
            settings.put("soundEffects", self.sound_effects);
        }
        if self.trader_classic_was_specified || self.trader_classic {
            settings.put("traderClassic", self.trader_classic);
        }
        if self.theme_was_specified || self.theme != Theme::System {
            settings.put("theme", self.theme.name());
        }
        if self.show_welcome_was_specified || !self.show_welcome {
            settings.put("showWelcome", self.show_welcome);
        }
        if self.tutorial_was_specified || !self.tutorial {
            settings.put("tutorial", self.tutorial);
        }
        if self.tutorial_seen_was_specified || self.tutorial_seen != 0 {
            settings.put("tutorialSeen", self.tutorial_seen);
        }
        if self.music_volume_was_specified || self.music_volume != MUSIC_VOLUME_DEFAULT {
            settings.put("musicVolume", self.music_volume);
        }
        if self.race_target_was_specified || self.race_target != RACE_TARGET_DEFAULT {
            settings.put("raceTarget", self.race_target);
        }
        if self.map_was_specified || self.map != MapDef::default_map().id {
            settings.put("map", self.map.as_str());
        }
        root.put("settings", settings);
        root.to_pretty(2)
    }

    /// Parses the document over the shipped maps. See from_json_with_maps.
    pub fn from_json(json: &str) -> Result<HighScoresAndSettings, JsonError> {
        HighScoresAndSettings::from_json_with_maps(json, MapDef::all().to_vec())
    }

    /// Parses the document. A schemaVersion other than the one this code understands is
    /// refused, since a newer or unknown shape cannot be read safely. Missing "settings" or
    /// "playerColor" fall back to the default preset; a missing "highScores" is an empty
    /// list; a score entry with a non integer "score" is skipped rather than failing the
    /// whole document, so one bad hand edit does not cost the rest of the table. Each list is
    /// ordered for its own map on load. A "tarnholm" key in "highScoresByMap" is ignored,
    /// since that list lives in "highScores" and nowhere else.
    pub fn from_json_with_maps(
        json: &str,
        maps: Vec<&'static MapDef>,
    ) -> Result<HighScoresAndSettings, JsonError> {
        let root = Json::parse(json)?;
        if !matches!(root, Json::Object(_)) {
            return Err("the document is not an object".into());
        }
        let schema = root
            .get("schemaVersion")
            .and_then(Json::as_i64_lenient)
            .unwrap_or(-1);
        if schema != SCHEMA_VERSION {
            return Err(format!("unsupported schemaVersion {schema}").into());
        }

        let mut store = HighScoresAndSettings::with_maps(maps);
        if let Some(Json::Array(array)) = root.get("highScores") {
            store.scores.extend(scores_from_json(array));
        }
        store.scores.sort_by_key(|e| std::cmp::Reverse(e.score));
        store.scores.truncate(MAX_SCORES);

        if let Some(Json::Object(by_map)) = root.get("highScoresByMap") {
            for (map_id, list) in by_map {
                let Json::Array(list) = list else {
                    continue;
                };
                if *map_id == MapDef::default_map().id {
                    continue;
                }
                let entries = scores_from_json(list);
                store.list_for_mut(map_id).extend(entries);
                store.order(map_id);
            }
        }

        let Some(settings) = root
            .get("settings")
            .filter(|s| matches!(s, Json::Object(_)))
        else {
            return Ok(store);
        };
        if let Some(color) = settings.get("playerColor") {
            match player_color::parse_hex(&opt_string(color)) {
                Ok(c) => {
                    store.player_color = c;
                    store.player_color_was_specified = true;
                }
                Err(_) => store.player_color = player_color::DEFAULT,
            }
            store.early_build_prompt = settings
                .get("earlyBuildPrompt")
                .and_then(Json::as_i64_lenient)
                .map_or(EARLY_PROMPT_SMART, |v| v as i32);
            store.end_turn_prompt = settings
                .get("endTurnPrompt")
                .and_then(Json::as_bool_lenient)
                .unwrap_or(true);
        }
        if let Some(map) = settings.get("map") {
            // An id this build has no map for is left unspecified, so the default stands
            let id = opt_string(map);
            if store.known_map(&id).is_some() {
                store.map = id;
                store.map_was_specified = true;
            }
        }
        if let Some(v) = settings.get("hapticFeedback") {
            store.haptic_feedback = v.as_bool_lenient().unwrap_or(true);
            store.haptic_feedback_was_specified = true;
        }
        if let Some(v) = settings.get("soundEffects") {
            store.sound_effects = v.as_bool_lenient().unwrap_or(true);
            store.sound_effects_was_specified = true;
        }
        if let Some(v) = settings.get("traderClassic") {
            store.trader_classic = v.as_bool_lenient().unwrap_or(false);
            store.trader_classic_was_specified = true;
        }
        if let Some(theme) = settings
            .get("theme")
            .and_then(|v| Theme::from_name(&opt_string(v)))
        {
            store.theme = theme;
            store.theme_was_specified = true;
        }
        if let Some(v) = settings.get("showWelcome") {
            store.show_welcome = v.as_bool_lenient().unwrap_or(true);
            store.show_welcome_was_specified = true;
        }
        if let Some(v) = settings.get("tutorial") {
            store.tutorial = v.as_bool_lenient().unwrap_or(true);
            store.tutorial_was_specified = true;
        }
        // A stray value from a hand edit is cut down to the bits there are tour steps for
        if let Some(n) = settings.get("tutorialSeen").and_then(Json::as_number_i64) {
            store.tutorial_seen = (n as i32 as u32) & tutorial::ALL;
            store.tutorial_seen_was_specified = true;
        }
        // A stray value from a hand edit is pulled into range rather than refused
        if let Some(n) = settings.get("musicVolume").and_then(Json::as_number_i64) {
            store.music_volume = clamp_music_volume(n as i32);
            store.music_volume_was_specified = true;
        }
        if let Some(n) = settings.get("raceTarget").and_then(Json::as_number_i64) {
            store.race_target = clamp_race_target(n as i32);
            store.race_target_was_specified = true;
        }
        Ok(store)
    }
}

pub fn clamp_music_volume(volume: i32) -> i32 {
    volume.clamp(MUSIC_VOLUME_MIN, MUSIC_VOLUME_MAX)
}

/// The highest Dunholm target there is: everything the island has to build, built.
pub fn race_target_max() -> i32 {
    MapDef::dunholm().max_score()
}

pub fn clamp_race_target(target: i32) -> i32 {
    target.min(race_target_max()).max(RACE_TARGET_MIN)
}

/// org.json's optString: a string as it is, anything else as its JSON text, null as nothing.
fn opt_string(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        Json::Null => String::new(),
        other => other.to_compact(),
    }
}

fn scores_to_json(list: &[ScoreAndDate]) -> Json {
    Json::Array(
        list.iter()
            .map(|sad| {
                let mut entry = Json::object();
                entry.put("score", sad.score);
                entry.put("playedAt", format_date(sad.date));
                entry
            })
            .collect(),
    )
}

/// Reads a list of {score, playedAt} entries, skipping any that is not one.
fn scores_from_json(array: &[Json]) -> Vec<ScoreAndDate> {
    array
        .iter()
        .filter_map(|entry| {
            let score = entry.get("score")?.as_i64_lenient()? as i32;
            Some(ScoreAndDate::new(score, parse_date(entry.get("playedAt"))))
        })
        .collect()
}

/// "yyyy-MM-ddTHH:mm:ssZ" in UTC, from milliseconds since the epoch.
pub fn format_date(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// The milliseconds a playedAt holds: the date text format_date writes, or a number of
/// milliseconds as a hand edit may give. Anything else is 0, unknown.
pub fn parse_date(value: Option<&Json>) -> i64 {
    match value {
        Some(Json::Int(n)) => *n,
        Some(Json::Float(f)) if f.is_finite() => f.trunc() as i64,
        Some(Json::Str(s)) => parse_date_text(s).unwrap_or(0),
        _ => 0,
    }
}

fn parse_date_text(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let num = |from: usize, to: usize| s.get(from..to)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (
        num(0, 4)?,
        num(5, 7)?,
        num(8, 10)?,
        num(11, 13)?,
        num(14, 16)?,
        num(17, 19)?,
    );
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    Some((days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se) * 1000)
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
