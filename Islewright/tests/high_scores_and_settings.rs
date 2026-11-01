// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from HighScoresAndSettingsTest: the scores and settings document. The first island's
//! list stays under "highScores", every other map gets its own list in the order its rules
//! say, and each setting is written only once it has left its default or been named.

mod common;

use common::*;
use dayapp::rules::high_scores::{self, HighScoresAndSettings, MAX_SCORES, ScoreAndDate, Theme};
use dayapp::rules::json::Json;
use dayapp::rules::palette::Palette;
use dayapp::rules::tutorial;

const RACE: &str = "race-fixture";

/// A document as the Android app wrote it: scores, colour, prompts and haptics, no map anywhere.
const CURRENT: &str = "{\n  \"schemaVersion\": 1,\n  \"highScores\": [\n    {\"score\": 31, \"playedAt\": \"2026-03-01T10:00:00Z\"},\n    {\"score\": 18, \"playedAt\": \"2026-02-01T10:00:00Z\"},\n    {\"score\": 7, \"playedAt\": \"2026-01-01T10:00:00Z\"}\n  ],\n  \"settings\": {\"playerColor\": \"#C03030\", \"earlyBuildPrompt\": 6, \"endTurnPrompt\": false, \"hapticFeedback\": false}\n}";

fn load(json: &str) -> HighScoresAndSettings {
    HighScoresAndSettings::from_json_with_maps(json, vec![tarnholm(), race()]).unwrap()
}

fn store() -> HighScoresAndSettings {
    HighScoresAndSettings::with_maps(vec![tarnholm(), race()])
}

fn values(list: &[ScoreAndDate]) -> Vec<i32> {
    list.iter().map(|s| s.score).collect()
}

fn add(store: &mut HighScoresAndSettings, map: &str, score: i32) -> usize {
    store.add_score(map, score, 0).unwrap()
}

/// CURRENT with something more in its settings.
fn with(extra: &str) -> String {
    CURRENT.replace(
        "\"hapticFeedback\": false",
        &format!("\"hapticFeedback\": false, {extra}"),
    )
}

fn settings(store: &HighScoresAndSettings) -> Json {
    Json::parse(&store.to_json())
        .unwrap()
        .get("settings")
        .unwrap()
        .clone()
}

#[test]
fn the_json_reader_reads_a_number() {
    assert_eq!(
        Some(1),
        Json::parse("{\"a\":1}")
            .unwrap()
            .get("a")
            .and_then(Json::as_i64_lenient)
    );
}

#[test]
fn a_current_file_loads_unchanged() {
    let loaded = load(CURRENT);
    assert_eq!(vec![31, 18, 7], values(&loaded.scores));
    assert_eq!(vec![31, 18, 7], values(loaded.scores_for("tarnholm")));
    assert!(loaded.scores_for(RACE).is_empty());
    assert_eq!(0xFFC03030, loaded.player_color);
    assert!(loaded.player_color_was_specified);
    assert_eq!(6, loaded.early_build_prompt);
    assert!(!loaded.end_turn_prompt);
    assert!(!loaded.haptic_feedback);
    assert_eq!("tarnholm", loaded.map);
    assert!(!loaded.map_was_specified);
}

#[test]
fn a_current_file_rewrites_as_the_same_document() {
    let before = Json::parse(CURRENT).unwrap();
    let after = Json::parse(&load(CURRENT).to_json()).unwrap();
    assert!(!after.has("highScoresByMap"));
    assert!(!after.get("settings").unwrap().has("map"));
    assert_eq!(before.canonical(), after.canonical());
}

#[test]
fn a_new_file_keeps_the_default_maps_scores_under_the_old_key() {
    let mut s = load(CURRENT);
    add(&mut s, RACE, 12);
    add(&mut s, "tarnholm", 25);
    s.map = RACE.into();

    let written = Json::parse(&s.to_json()).unwrap();
    assert_eq!(
        Some(1),
        written.get("schemaVersion").and_then(Json::as_i64_lenient)
    );
    let scores = written.get("highScores").unwrap().as_array().unwrap();
    assert_eq!(4, scores.len());
    assert_eq!(
        Some(31),
        scores[0].get("score").and_then(Json::as_i64_lenient)
    );
    assert_eq!(
        Some(25),
        scores[1].get("score").and_then(Json::as_i64_lenient)
    );
    let by_map = written.get("highScoresByMap").unwrap();
    assert_eq!(
        Some(12),
        by_map.get(RACE).unwrap().as_array().unwrap()[0]
            .get("score")
            .and_then(Json::as_i64_lenient)
    );
    assert!(!by_map.has("tarnholm"));
    assert_eq!(
        Some(RACE),
        written
            .get("settings")
            .unwrap()
            .get("map")
            .unwrap()
            .as_str()
    );

    let back = load(&s.to_json());
    assert_eq!(vec![31, 25, 18, 7], values(&back.scores));
    assert_eq!(vec![12], values(back.scores_for(RACE)));
    assert_eq!(RACE, back.map);
    assert!(back.map_was_specified);
}

#[test]
fn a_setting_named_the_default_map_is_kept_once_the_file_has_one() {
    let loaded = load(&with("\"map\": \"tarnholm\""));
    assert!(loaded.map_was_specified);
    assert_eq!(
        Some("tarnholm"),
        settings(&loaded).get("map").unwrap().as_str()
    );
}

#[test]
fn a_map_nobody_knows_in_the_settings_leaves_the_default() {
    let loaded = load(&with("\"map\": \"atlantis\""));
    assert_eq!("tarnholm", loaded.map);
    assert!(!loaded.map_was_specified);
}

#[test]
fn maps_never_share_a_list() {
    let mut s = store();
    add(&mut s, "tarnholm", 40);
    add(&mut s, RACE, 9);
    add(&mut s, RACE, 14);
    add(&mut s, "tarnholm", 3);

    assert_eq!(vec![40, 3], values(s.scores_for("tarnholm")));
    assert_eq!(vec![9, 14], values(s.scores_for(RACE)));
    assert_eq!(vec![40, 3], values(&s.scores));

    // And after a trip through the document
    let back = load(&s.to_json());
    assert_eq!(vec![40, 3], values(back.scores_for("tarnholm")));
    assert_eq!(vec![9, 14], values(back.scores_for(RACE)));
}

#[test]
fn the_default_map_lists_highest_first_and_an_added_tie_enters_above() {
    let mut s = store();
    for score in [10, 30, 20, 20] {
        add(&mut s, "tarnholm", score);
    }
    assert_eq!(vec![30, 20, 20, 10], values(s.scores_for("tarnholm")));

    // A tie goes at or above the equal score: this one is first among the 20s
    s.scores[1].date = 1;
    s.scores[2].date = 2;
    assert_eq!(2, add(&mut s, "tarnholm", 20));
    assert_eq!((1, 2), (s.scores[2].date, s.scores[3].date));
}

#[test]
fn a_race_map_lists_fewest_turns_first_and_trims_the_worst() {
    let mut s = store();
    assert_eq!(1, add(&mut s, RACE, 14));
    assert_eq!(1, add(&mut s, RACE, 11));
    assert_eq!(3, add(&mut s, RACE, 17));
    assert_eq!(2, add(&mut s, RACE, 12));
    assert_eq!(vec![11, 12, 14, 17], values(s.scores_for(RACE)));
    // A tie enters at or above the equal result, in this direction too
    assert_eq!(3, add(&mut s, RACE, 14));
    assert_eq!(vec![11, 12, 14, 14, 17], values(s.scores_for(RACE)));

    // Full table: more turns than the worst entry does not make it, fewer pushes the worst out
    assert_eq!(0, add(&mut s, RACE, 18));
    assert_eq!(1, add(&mut s, RACE, 8));
    assert_eq!(vec![8, 11, 12, 14, 14], values(s.scores_for(RACE)));
    assert_eq!(MAX_SCORES, s.scores_for(RACE).len());
}

#[test]
fn a_file_is_sorted_for_each_map_on_load() {
    let json = format!(
        "{{\"schemaVersion\":1,\"highScores\":[{{\"score\":5,\"playedAt\":\"2026-01-01T00:00:00Z\"}},{{\"score\":50,\"playedAt\":\"2026-01-02T00:00:00Z\"}}],\"highScoresByMap\":{{\"{RACE}\":[{{\"score\":15,\"playedAt\":\"2026-01-01T00:00:00Z\"}},{{\"score\":9,\"playedAt\":\"2026-01-02T00:00:00Z\"}},{{\"score\":12,\"playedAt\":\"2026-01-03T00:00:00Z\"}},{{\"score\":20,\"playedAt\":\"2026-01-04T00:00:00Z\"}},{{\"score\":10,\"playedAt\":\"2026-01-05T00:00:00Z\"}},{{\"score\":11,\"playedAt\":\"2026-01-06T00:00:00Z\"}}]}}}}"
    );
    let loaded = load(&json);
    assert_eq!(vec![50, 5], values(&loaded.scores));
    assert_eq!(vec![9, 10, 11, 12, 15], values(loaded.scores_for(RACE)));
}

#[test]
fn a_bad_entry_or_a_map_list_that_is_not_a_list_is_skipped() {
    let json = format!(
        "{{\"schemaVersion\":1,\"highScoresByMap\":{{\"{RACE}\":[{{\"score\":\"x\"}},{{\"score\":13}},7,{{\"nope\":1}}],\"tarnholm\":[{{\"score\":99}}],\"junk\":3}}}}"
    );
    let loaded = load(&json);
    assert_eq!(vec![13], values(loaded.scores_for(RACE)));
    // The first island's list lives under "highScores" only
    assert!(loaded.scores.is_empty());
}

#[test]
fn a_map_this_build_does_not_know_is_kept_as_read_and_never_mixed() {
    let json = "{\"schemaVersion\":1,\"highScores\":[{\"score\":5}],\"highScoresByMap\":{\"third-isle\":[{\"score\":9,\"playedAt\":\"2026-01-01T00:00:00Z\"},{\"score\":2,\"playedAt\":\"2026-01-02T00:00:00Z\"}]}}";
    let mut loaded = load(json);
    assert_eq!(vec![5], values(&loaded.scores));
    assert_eq!(vec![9, 2], values(loaded.scores_for("third-isle")));
    assert_eq!(
        vec![9, 2],
        values(load(&loaded.to_json()).scores_for("third-isle"))
    );
    assert!(
        loaded.add_score("third-isle", 1, 0).is_err(),
        "a result for a map the build does not know has no list to go in"
    );
    assert_eq!(vec![9, 2], values(loaded.scores_for("third-isle")));
}

#[test]
fn merging_is_per_map_and_dedupes_on_score_and_date() {
    let mut s = store();
    s.merge_scores(
        RACE,
        &[ScoreAndDate::new(13, 100), ScoreAndDate::new(10, 200)],
    );
    let added = s.merge_scores(
        RACE,
        &[
            ScoreAndDate::new(13, 100),
            ScoreAndDate::new(9, 300),
            ScoreAndDate::new(15, 400),
        ],
    );
    assert_eq!(2, added);
    assert_eq!(vec![9, 10, 13, 15], values(s.scores_for(RACE)));
    assert!(s.scores_for("tarnholm").is_empty());

    // The default map's list is "highScores"
    assert_eq!(1, s.merge_scores("tarnholm", &[ScoreAndDate::new(4, 1)]));
    assert_eq!(vec![4], values(&s.scores));
}

#[test]
fn export_then_import_round_trips_everything() {
    let mut source = load(CURRENT);
    add(&mut source, RACE, 12);
    add(&mut source, RACE, 16);
    source.map = RACE.into();
    source.map_was_specified = true;
    source.player_color = 0xFF2040C0;
    source.haptic_feedback = true;
    let exported = source.to_json();

    let mut fresh = store();
    let added = fresh.import_from(&load(&exported));
    assert_eq!(5, added);
    assert_eq!(vec![31, 18, 7], values(fresh.scores_for("tarnholm")));
    assert_eq!(vec![12, 16], values(fresh.scores_for(RACE)));
    assert_eq!(RACE, fresh.map);
    assert_eq!(0xFF2040C0, fresh.player_color);
    assert!(fresh.haptic_feedback);
    assert_eq!(
        Json::parse(&exported)
            .unwrap()
            .get("highScoresByMap")
            .unwrap()
            .canonical(),
        Json::parse(&fresh.to_json())
            .unwrap()
            .get("highScoresByMap")
            .unwrap()
            .canonical()
    );

    // Importing it again adds nothing
    assert_eq!(0, fresh.import_from(&load(&exported)));
}

#[test]
fn importing_an_old_export_still_works_and_leaves_the_map_alone() {
    let mut s = store();
    add(&mut s, RACE, 11);
    s.map = RACE.into();
    s.map_was_specified = true;

    let added = s.import_from(&load(CURRENT));
    assert_eq!(3, added);
    assert_eq!(vec![31, 18, 7], values(s.scores_for("tarnholm")));
    assert_eq!(vec![11], values(s.scores_for(RACE)));
    // The document named no map, so the choice on this device stands
    assert_eq!(RACE, s.map);
    assert_eq!(0xFFC03030, s.player_color);
}

#[test]
fn a_file_with_only_scores_still_loads() {
    let loaded =
        load("{\"schemaVersion\":1,\"highScores\":[{\"score\":4,\"playedAt\":1700000000000}]}");
    assert_eq!(vec![4], values(&loaded.scores));
    assert_eq!(1_700_000_000_000, loaded.scores[0].date);
}

#[test]
fn dates_are_written_in_utc_and_read_back() {
    let ms = 1_772_359_200_000; // 2026-03-01T10:00:00Z
    assert_eq!("2026-03-01T10:00:00Z", high_scores::format_date(ms));
    assert_eq!(
        ms,
        high_scores::parse_date(Some(&Json::Str("2026-03-01T10:00:00Z".into())))
    );
    assert_eq!("1970-01-01T00:00:00Z", high_scores::format_date(0));
    assert_eq!(
        0,
        high_scores::parse_date(Some(&Json::Str("yesterday".into())))
    );
    let mut s = store();
    s.add_score("tarnholm", 9, ms).unwrap();
    assert_eq!(ms, load(&s.to_json()).scores[0].date);
}

// ---- Theme and the welcome dialog -----------------------------------------------------

#[test]
fn theme_and_welcome_default_to_system_and_on() {
    let fresh = store();
    assert_eq!(Theme::System, fresh.theme);
    assert!(fresh.show_welcome);

    let old = load(CURRENT);
    assert_eq!(Theme::System, old.theme);
    assert!(old.show_welcome);
    assert!(!old.theme_was_specified);
    assert!(!old.show_welcome_was_specified);
}

#[test]
fn theme_and_welcome_round_trip_through_the_document() {
    for theme in [Theme::System, Theme::Light, Theme::Dark] {
        let mut source = load(CURRENT);
        source.theme = theme;
        source.show_welcome = false;
        let back = load(&source.to_json());
        assert_eq!(theme, back.theme);
        assert!(!back.show_welcome);
        assert_eq!(theme != Theme::System, back.theme_was_specified);
        assert!(back.show_welcome_was_specified);
    }

    // Defaults are not written
    assert!(!settings(&store()).has("theme"));
    assert!(!settings(&store()).has("showWelcome"));

    let mut dark = store();
    dark.theme = Theme::Dark;
    assert_eq!(Some("dark"), settings(&dark).get("theme").unwrap().as_str());
}

#[test]
fn an_unknown_theme_name_is_ignored() {
    let loaded = load(&with("\"theme\": \"sepia\""));
    assert_eq!(Theme::System, loaded.theme);
    assert!(!loaded.theme_was_specified);
}

#[test]
fn export_then_import_carries_theme_and_welcome() {
    let mut source = load(CURRENT);
    source.theme = Theme::Dark;
    source.show_welcome = false;
    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert_eq!(Theme::Dark, fresh.theme);
    assert!(!fresh.show_welcome);
}

#[test]
fn importing_an_old_export_leaves_theme_and_welcome_alone() {
    let mut s = store();
    s.theme = Theme::Light;
    s.show_welcome = false;
    s.import_from(&load(CURRENT));
    assert_eq!(Theme::Light, s.theme);
    assert!(!s.show_welcome);
}

// ---- The tutorial -----------------------------------------------------------------------

#[test]
fn tutorial_defaults_to_on_with_nothing_seen() {
    let fresh = store();
    assert!(fresh.tutorial);
    assert_eq!(0, fresh.tutorial_seen);

    let old = load(CURRENT);
    assert!(old.tutorial);
    assert_eq!(0, old.tutorial_seen);
    assert!(!old.tutorial_was_specified);
    assert!(!old.tutorial_seen_was_specified);
}

#[test]
fn tutorial_round_trips_through_the_document() {
    let mut source = load(CURRENT);
    source.tutorial = false;
    source.tutorial_seen = 0x15;
    let back = load(&source.to_json());
    assert!(!back.tutorial);
    assert_eq!(0x15, back.tutorial_seen);
    assert!(back.tutorial_was_specified);
    assert!(back.tutorial_seen_was_specified);

    // Defaults are not written
    let s = settings(&store());
    assert!(!s.has("tutorial"));
    assert!(!s.has("tutorialSeen"));

    // A value that left its default is written, and so is one the document already carried
    let mut partway = store();
    partway.tutorial_seen = 3;
    assert_eq!(
        Some(3),
        settings(&partway)
            .get("tutorialSeen")
            .and_then(Json::as_i64_lenient)
    );
    let mut reset = load(&source.to_json());
    reset.tutorial = true;
    reset.tutorial_seen = 0;
    let written = settings(&reset);
    assert_eq!(
        Some(true),
        written.get("tutorial").and_then(Json::as_bool_lenient)
    );
    assert_eq!(
        Some(0),
        written.get("tutorialSeen").and_then(Json::as_i64_lenient)
    );
}

#[test]
fn a_stray_tutorial_seen_is_cut_down_to_the_steps_that_exist() {
    let loaded = load(&with("\"tutorialSeen\": 65535"));
    assert_eq!(tutorial::ALL, loaded.tutorial_seen);
    assert!(loaded.tutorial_seen_was_specified);
}

#[test]
fn export_then_import_carries_the_tutorial() {
    let mut source = load(CURRENT);
    source.tutorial = false;
    source.tutorial_seen = 7;
    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert!(!fresh.tutorial);
    assert_eq!(7, fresh.tutorial_seen);
}

#[test]
fn importing_an_old_export_leaves_the_tutorial_alone() {
    let mut s = store();
    s.tutorial = false;
    s.tutorial_seen = 5;
    s.import_from(&load(CURRENT));
    assert!(!s.tutorial);
    assert_eq!(5, s.tutorial_seen);
}

// ---- Music volume -----------------------------------------------------------------------

#[test]
fn music_volume_defaults_to_sixty_and_is_not_written_while_default() {
    let fresh = store();
    assert_eq!(60, fresh.music_volume);
    assert!(!fresh.music_volume_was_specified);
    assert!(!settings(&fresh).has("musicVolume"));

    let old = load(CURRENT);
    assert_eq!(60, old.music_volume);
    assert!(!old.music_volume_was_specified);
    assert!(!settings(&old).has("musicVolume"));
}

#[test]
fn music_volume_round_trips_through_the_document() {
    for volume in [0, 1, 59, 61, 100] {
        let mut source = load(CURRENT);
        source.music_volume = volume;
        assert_eq!(
            Some(volume as i64),
            settings(&source)
                .get("musicVolume")
                .and_then(Json::as_i64_lenient)
        );
        let back = load(&source.to_json());
        assert_eq!(volume, back.music_volume);
        assert!(back.music_volume_was_specified);
    }

    // Once the document names it, it stays named even at the default
    let named = load(&with("\"musicVolume\": 60"));
    assert!(named.music_volume_was_specified);
    assert_eq!(
        Some(60),
        settings(&named)
            .get("musicVolume")
            .and_then(Json::as_i64_lenient)
    );
}

#[test]
fn a_stray_music_volume_is_clamped_or_ignored() {
    assert_eq!(100, load(&with("\"musicVolume\": 250")).music_volume);
    assert_eq!(0, load(&with("\"musicVolume\": -5")).music_volume);
    let text = load(&with("\"musicVolume\": \"loud\""));
    assert_eq!(60, text.music_volume);
    assert!(!text.music_volume_was_specified);
}

#[test]
fn export_then_import_carries_music_volume_and_an_old_export_leaves_it_alone() {
    let mut source = load(CURRENT);
    source.music_volume = 15;
    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert_eq!(15, fresh.music_volume);
    assert!(fresh.music_volume_was_specified);

    let mut kept = store();
    kept.music_volume = 85;
    kept.import_from(&load(CURRENT));
    assert_eq!(85, kept.music_volume);
}

// ---- Dunholm target -----------------------------------------------------------------------

#[test]
fn race_target_defaults_to_the_islands_own_and_is_not_written_while_default() {
    let fresh = store();
    assert_eq!(dunholm().target_score, fresh.race_target);
    assert_eq!(10, fresh.race_target);
    assert_eq!(dunholm().target_score, high_scores::RACE_TARGET_DEFAULT);
    assert!(!fresh.race_target_was_specified);
    assert!(!settings(&fresh).has("raceTarget"));

    let old = load(CURRENT);
    assert_eq!(10, old.race_target);
    assert!(!old.race_target_was_specified);
    assert!(!settings(&old).has("raceTarget"));
}

#[test]
fn race_target_round_trips_through_the_document() {
    for target in [1, 9, 11, 15, 19] {
        let mut source = load(CURRENT);
        source.race_target = target;
        assert_eq!(
            Some(target as i64),
            settings(&source)
                .get("raceTarget")
                .and_then(Json::as_i64_lenient)
        );
        let back = load(&source.to_json());
        assert_eq!(target, back.race_target);
        assert!(back.race_target_was_specified);
    }
}

#[test]
fn a_stray_race_target_is_clamped_to_what_the_island_can_give_or_ignored() {
    assert_eq!(19, high_scores::race_target_max());
    assert_eq!(19, load(&with("\"raceTarget\": 40")).race_target);
    assert_eq!(1, load(&with("\"raceTarget\": 0")).race_target);
    let text = load(&with("\"raceTarget\": \"lots\""));
    assert_eq!(10, text.race_target);
    assert!(!text.race_target_was_specified);
}

#[test]
fn export_then_import_carries_race_target_and_an_old_export_leaves_it_alone() {
    let mut source = load(CURRENT);
    source.race_target = 15;
    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert_eq!(15, fresh.race_target);
    assert!(fresh.race_target_was_specified);

    let mut kept = store();
    kept.race_target = 7;
    kept.import_from(&load(CURRENT));
    assert_eq!(7, kept.race_target);
}

#[test]
fn palette_follows_the_theme_and_the_os_only_for_system() {
    assert!(!Palette::is_dark(Theme::System, false));
    assert!(Palette::is_dark(Theme::System, true));
    assert!(!Palette::is_dark(Theme::Light, true));
    assert!(Palette::is_dark(Theme::Dark, false));
    assert_eq!(&Palette::DARK, Palette::for_mode(Theme::Dark, false));
    assert_eq!(&Palette::LIGHT, Palette::for_mode(Theme::Light, true));
}

#[test]
fn sound_effects_are_on_by_default_and_a_file_without_them_rewrites_without() {
    let loaded = load(CURRENT);
    assert!(loaded.sound_effects);
    assert!(!loaded.sound_effects_was_specified);
    assert!(!settings(&loaded).has("soundEffects"));
    assert!(store().sound_effects);
}

#[test]
fn sound_effects_off_round_trips_through_the_document() {
    let mut source = load(CURRENT);
    source.sound_effects = false;
    assert_eq!(
        Some(false),
        settings(&source)
            .get("soundEffects")
            .and_then(Json::as_bool_lenient)
    );
    let back = load(&source.to_json());
    assert!(!back.sound_effects);
    assert!(back.sound_effects_was_specified);

    // Once the document names it, it stays named even when back to the default
    let named = load(&with("\"soundEffects\": true"));
    assert!(named.sound_effects);
    assert!(named.sound_effects_was_specified);
    assert_eq!(
        Some(true),
        settings(&named)
            .get("soundEffects")
            .and_then(Json::as_bool_lenient)
    );
}

#[test]
fn export_then_import_carries_sound_effects_and_an_old_export_leaves_them_alone() {
    let mut source = load(CURRENT);
    source.sound_effects = false;
    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert!(!fresh.sound_effects);
    assert!(fresh.sound_effects_was_specified);

    let mut kept = store();
    kept.sound_effects = false;
    kept.import_from(&load(CURRENT));
    assert!(
        !kept.sound_effects,
        "a backup from before the setting does not turn sound back on"
    );

    let mut on = store();
    let mut was_on = load(CURRENT);
    was_on.sound_effects = true;
    was_on.sound_effects_was_specified = true;
    on.sound_effects = false;
    on.import_from(&load(&was_on.to_json()));
    assert!(on.sound_effects);
}

#[test]
fn trader_classic_is_off_by_default_and_a_file_without_it_rewrites_without() {
    let loaded = load(CURRENT);
    assert!(!loaded.trader_classic);
    assert!(!loaded.trader_classic_was_specified);
    assert!(!settings(&loaded).has("traderClassic"));
    assert!(!store().trader_classic);
}

#[test]
fn trader_classic_round_trips_through_the_document_and_an_import() {
    let mut source = load(CURRENT);
    source.trader_classic = true;
    assert_eq!(
        Some(true),
        settings(&source)
            .get("traderClassic")
            .and_then(Json::as_bool_lenient)
    );
    let mut back = load(&source.to_json());
    assert!(back.trader_classic);
    assert!(back.trader_classic_was_specified);

    // Once the document names it, it stays named when turned back off
    back.trader_classic = false;
    assert_eq!(
        Some(false),
        settings(&back)
            .get("traderClassic")
            .and_then(Json::as_bool_lenient)
    );

    let mut fresh = store();
    fresh.import_from(&load(&source.to_json()));
    assert!(fresh.trader_classic);
    let mut kept = store();
    kept.trader_classic = true;
    kept.import_from(&load(CURRENT));
    assert!(
        kept.trader_classic,
        "a backup from before the setting leaves it alone"
    );
}
