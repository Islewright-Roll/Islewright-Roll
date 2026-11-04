// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from DebugScenarioTest: scenario documents are checked against their map and put a
//! game into a known state; every committed file under tools/scenarios parses.

mod common;

use common::*;
use dayapp::rules::scenario::Scenario;
use dayapp::rules::{Game, MapDef};

fn rejects(json: &str, fragment: &str) {
    match Scenario::from_json(json) {
        Ok(_) => panic!("accepted: {json}"),
        Err(e) => assert!(e.0.contains(fragment), "{e}"),
    }
}

fn scenarios_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/scenarios")
}

#[test]
fn no_map_means_the_default_map() {
    assert!(same(
        MapDef::default_map(),
        Scenario::from_json("{}").unwrap().map
    ));
}

#[test]
fn unknown_map_is_rejected() {
    rejects("{\"map\":\"nowhere\"}", "unknown map");
}

#[test]
fn knights_count_becomes_trader_flags() {
    let s = Scenario::from_json("{\"playsheet\":{\"knights\":2}}").unwrap();
    let traders = s.traders.unwrap();
    assert_eq!(MapDef::default_map().traders.len(), traders.len());
    assert!(traders[0] && traders[1]);
    assert!(!traders[2]);
}

#[test]
fn knights_and_traders_must_agree() {
    let n = MapDef::default_map().traders.len();
    let flags: Vec<&str> = (0..n)
        .map(|i| if i < 2 { "true" } else { "false" })
        .collect();
    let flags = flags.join(",");
    Scenario::from_json(&format!(
        "{{\"playsheet\":{{\"knights\":2,\"traders\":[{flags}]}}}}"
    ))
    .unwrap();
    rejects(
        &format!("{{\"playsheet\":{{\"knights\":3,\"traders\":[{flags}]}}}}"),
        "disagree",
    );
}

#[test]
fn ordered_map_rejects_gapped_traders() {
    let n = tarnholm().traders.len();
    let mut flags = vec!["false", "true"];
    flags.extend(std::iter::repeat_n("false", n - 2));
    rejects(
        &format!("{{\"playsheet\":{{\"traders\":[{}]}}}}", flags.join(",")),
        "prefix",
    );
}

#[test]
fn lengths_and_ranges_come_from_the_map() {
    let m = MapDef::default_map();
    rejects(&format!("{{\"turnsTaken\":{}}}", m.turn_cap), "turnsTaken");
    rejects("{\"playsheet\":{\"roads\":[true]}}", "playsheet.roads");
    rejects(
        &format!(
            "{{\"playsheet\":{{\"bankSpent\":[{}]}}}}",
            m.bank_resource.len()
        ),
        "bankSpent",
    );
    rejects(
        "{\"playsheet\":{\"bankSpent\":[1],\"knightResourcesSpent\":[1]}}",
        "not both",
    );
}

#[test]
fn applies_to_a_game() {
    let mut game = Game::new(MapDef::default_map());
    Scenario::from_json("{\"rolls\":3,\"playsheet\":{\"knights\":2,\"knightResourcesSpent\":[1]}}")
        .unwrap()
        .apply_to(&mut game);
    assert_eq!(3, game.rolls);
    assert_eq!(2, game.playsheet.trader_count());
    assert!(!game.playsheet.resources_avail[1]);
    assert!(game.tutorial_paused);
}

#[test]
fn passed_turns_get_running_totals_ending_on_the_score() {
    let json = std::fs::read_to_string(scenarios_dir().join("tarnholm-midgame.json")).unwrap();
    let mut game = Game::new(tarnholm());
    Scenario::from_json(&json).unwrap().apply_to(&mut game);
    let total = game.playsheet.score();
    let totals = &game.playsheet.turn_score;
    assert_eq!(total, totals[6]);
    assert_eq!(0, total - totals[game.turns_taken as usize]);
    for turn in 1..=6 {
        assert!(totals[turn] >= totals[turn - 1]);
    }
    assert_eq!(0, totals[7]);
}

#[test]
fn turn_scores_override_the_spread() {
    let mut game = Game::new(tarnholm());
    Scenario::from_json("{\"map\":\"tarnholm\",\"turnsTaken\":3,\"turnScores\":[2,0,5]}")
        .unwrap()
        .apply_to(&mut game);
    assert_eq!(2, game.playsheet.turn_score[1]);
    assert_eq!(2, game.playsheet.turn_score[2]);
    assert_eq!(7, game.playsheet.turn_score[3]);
}

#[test]
fn turn_scores_must_match_the_completed_turns() {
    rejects(
        "{\"turnsTaken\":3,\"turnScores\":[1,2]}",
        "turnScores must have exactly",
    );
    rejects("{\"turnScores\":[1]}", "turnScores must have exactly");
    rejects("{\"turnsTaken\":1,\"turnScores\":[-1]}", "0 or more");
}

/// Every committed scenario parses against its own map, and applies to a game on it.
#[test]
fn committed_scenario_files_parse() {
    let mut parsed = 0;
    for entry in std::fs::read_dir(scenarios_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path).unwrap();
        let s = Scenario::from_json(&json).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut game = Game::new(s.map);
        s.apply_to(&mut game);
        parsed += 1;
    }
    assert!(parsed > 0);
}
