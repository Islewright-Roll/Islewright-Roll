// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from VoyageTest: which stage the ship is at, where the dock goes, how the HUD strip
//! fits 216 px, how the traders are centred, and where the ship's stops lie. The tests that
//! measure the PNG art (the ship's sprite, the dock's pixels over the board) are left out: they
//! check the assets, not the rules, and reading a PNG would need a decoder this crate lacks.

mod common;

use common::*;
use dayapp::rules::art;
use dayapp::rules::board::{self, Board};
use dayapp::rules::trader_layout;
use dayapp::rules::voyage::{self, Stage, Strip};
use dayapp::rules::{MapDef, Pt};

const DESIGN_W: i32 = art::DESIGN_W;
// The old game view's bars and inset
const TOP_H: i32 = 64;
const BOTTOM_H: i32 = 66;
const HUD_INSET: i32 = 6;

/// The two screens the ship has to work on, in art pixels: width, height, top inset and
/// bottom inset. A 1080 x 2400 phone at scale 5 and a 720 x 1280 one at scale 3.
const SCREENS: [[i32; 4]; 2] = [[216, 480, 0, 13], [240, 426, 0, 16]];

// ---------------------------------------------------------------- stages

#[test]
fn tarnholm_ship_appears_three_turns_from_the_end_and_moors_for_the_last_turn() {
    let map = tarnholm();
    for taken in 0..=11 {
        assert_eq!(
            Stage::None,
            voyage::stage_for(map, taken, false, 0),
            "turn {}",
            taken + 1
        );
    }
    assert_eq!(Stage::Far, voyage::stage_for(map, 12, false, 0));
    assert_eq!(Stage::Mid, voyage::stage_for(map, 13, false, 0));
    assert_eq!(
        Stage::Moored,
        voyage::stage_for(map, 14, false, 0),
        "the last turn is played with the ship at the dock"
    );
    assert_eq!(
        Stage::Done,
        voyage::stage_for(map, 15, true, 40),
        "the 15th turn taken is over: the flag is up"
    );
}

#[test]
fn dunholm_has_no_cap_so_its_ship_never_comes_onto_the_board() {
    for taken in 0..=60 {
        assert_eq!(Stage::None, voyage::stage_for(dunholm(), taken, false, 4));
        assert_eq!(Stage::None, voyage::stage_for(dunholm(), taken, true, 10));
    }
}

#[test]
fn a_capped_race_ship_follows_the_turn_cap() {
    let map = capped();
    assert_eq!(Stage::None, voyage::stage_for(map, 16, false, 4));
    assert_eq!(Stage::Far, voyage::stage_for(map, 17, false, 4));
    assert_eq!(Stage::Mid, voyage::stage_for(map, 18, false, 4));
    assert_eq!(Stage::Moored, voyage::stage_for(map, 19, false, 4));
    assert_eq!(
        Stage::Done,
        voyage::stage_for(map, 20, true, 9),
        "out of turns"
    );
}

#[test]
fn a_race_won_early_leaves_the_ship_where_it_was() {
    let map = capped();
    // Reaching the target on turn 1 or turn 10: no ship yet
    assert_eq!(Stage::None, voyage::stage_for(map, 1, true, 10));
    assert_eq!(Stage::None, voyage::stage_for(map, 10, true, 11));
    // On turn 18 (turns_taken counts it): the far stop was 17 taken, 18th turn is 3 remaining
    assert_eq!(Stage::Far, voyage::stage_for(map, 18, true, 10));
    // On the last turn: moored as it is for the whole last turn, but the King has not arrived,
    // so there is no flag
    assert_eq!(Stage::Moored, voyage::stage_for(map, 20, true, 10));
}

#[test]
fn cells_run_from_the_first_turn_to_the_pier() {
    assert_eq!(0, voyage::cell(15, 0, false, false));
    assert_eq!(14, voyage::cell(15, 14, false, false));
    assert_eq!(15, voyage::cell(15, 15, true, false));
    assert_eq!(19, voyage::cell(20, 20, true, true));
    assert_eq!(0, voyage::cell(20, 1, true, true));
    assert_eq!(20, voyage::cell(20, 20, true, false));
    // No cap: the turn being played, or the one the target was reached in
    assert_eq!(0, voyage::cell(0, 0, false, false));
    assert_eq!(40, voyage::cell(0, 40, false, false));
    assert_eq!(39, voyage::cell(0, 40, true, true));
}

// ---------------------------------------------------------------- the dock

/// A line of the bake's geometry file for a map, as its numbers.
fn geometry(map: &MapDef, key: &str) -> Vec<i32> {
    let path = format!(
        "{}/tools/board_geometry_{}.txt",
        env!("CARGO_MANIFEST_DIR"),
        map.id
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{key} ")))
        .unwrap_or_else(|| panic!("{key} is not in {path}"));
    line.split_whitespace()
        .skip(1)
        .map(|w| w.parse().unwrap())
        .collect()
}

/// The hex centres of the bake's geometry file, in the board's order (NW, SW, S, SE, NE, N,
/// CENTRE), from lines like "hex NW mountain col=3 row=2 centre=198,216".
fn hex_centres(map: &MapDef) -> Vec<[i32; 2]> {
    let path = format!(
        "{}/tools/board_geometry_{}.txt",
        env!("CARGO_MANIFEST_DIR"),
        map.id
    );
    let text = std::fs::read_to_string(&path).unwrap();
    let mut centres = Vec::new();
    for name in ["NW", "SW", "S", "SE", "NE", "N", "CENTRE"] {
        let line = text
            .lines()
            .find(|l| l.starts_with(&format!("hex {name} ")))
            .unwrap();
        let centre = line
            .split_whitespace()
            .find_map(|w| w.strip_prefix("centre="))
            .unwrap();
        let (x, y) = centre.split_once(',').unwrap();
        centres.push([x.parse().unwrap(), y.parse().unwrap()]);
    }
    centres
}

#[test]
fn the_dock_is_where_the_start_road_meets_the_sea_on_both_maps_as_the_bake_wrote_it() {
    for map in MapDef::all() {
        let b = Board::for_map(map);
        let vertex = voyage::coastal_end(&map.hex, &b.road_ends[0]).unwrap();
        // The start road is the NE edge of the NW hex: its upper left end is the sea's
        assert_eq!(voyage::DOCK_FOOT, vertex, "{}", map.id);
        assert!(b.road_touches(0, vertex));
        // The numbers in voyage are the ones the bake wrote beside the board it drew the dock on
        let foot = geometry(map, "dock_foot");
        let mooring = geometry(map, "dock_mooring");
        let sprite = geometry(map, "dock_sprite");
        assert_eq!(voyage::DOCK_FOOT, Pt::new(foot[0], foot[1]), "{}", map.id);
        assert_eq!(
            voyage::DOCK_MOORING,
            Pt::new(mooring[0], mooring[1]),
            "{}",
            map.id
        );
        assert_eq!(
            voyage::DOCK_SPRITE,
            Pt::new(sprite[0], sprite[1]),
            "{}",
            map.id
        );
        // And the board and its hex centres are the bake's too
        assert_eq!(
            vec![board::WIDTH, board::HEIGHT],
            geometry(map, "board_size")
        );
        assert_eq!(map.hex.to_vec(), hex_centres(map));
    }
}

#[test]
fn a_start_road_with_no_coastal_end_is_an_error() {
    // Two hexes side by side, and a road along an outer edge of one: both ends touch it alone
    let hex = [[100, 100], [154, 136]];
    let ends = [Pt::new(82, 64), Pt::new(118, 64)];
    assert!(voyage::coastal_end(&hex, &ends).is_err());
}

// ---------------------------------------------------------------- the HUD strip

fn check_fits(strip: &Strip) {
    assert!(strip.left >= voyage::STRIP_MARGIN, "margin left");
    assert!(
        strip.left + strip.width <= DESIGN_W - voyage::STRIP_MARGIN,
        "margin right"
    );
    // The water ends at the later of the pier and the ship that lies at it
    assert_eq!(
        strip.width,
        (strip.pier_x() + art::TRACK_PIER_W).max(strip.ship_x(strip.turns) + art::TRACK_SHIP_W)
    );
    // Everything inside the water, from the first sprite to the pier, left to right until the
    // ship is at the pier
    assert!(strip.ship_x(0) >= voyage::STRIP_LEAD);
    for c in 1..strip.turns {
        assert!(
            strip.ship_x(c) > strip.ship_x(c - 1),
            "ship cells run left to right"
        );
    }
    assert_eq!(
        strip.pier_x() - art::TRACK_SHIP_W + 1,
        strip.ship_x(strip.turns - 1),
        "the last turn's ship lies alongside the pier"
    );
    assert_eq!(
        strip.pier_x() + voyage::PIER_SHIP_IN,
        strip.ship_x(strip.turns),
        "the finished ship is in harbour"
    );
    // No empty cell between the last cell and the pier
    assert!(strip.pier_x() - strip.cell_x(strip.turns - 1) <= strip.pitch + art::TRACK_FLAG_W);
}

#[test]
fn fifteen_turns_fit_with_flags_in_the_hud() {
    let strip = Strip::new(15, true, DESIGN_W);
    assert_eq!(12, strip.pitch);
    check_fits(&strip);
    // A flag is 13 wide, so at 12 the next one stands on its right edge
    assert_eq!(strip.cell_x(0) + 12, strip.cell_x(1));
    // The pier directly follows the last flag; the last turn's ship lies alongside it, and the
    // finished ship is in harbour behind it
    assert_eq!(strip.cell_x(14) + art::TRACK_FLAG_W + 1, strip.pier_x());
    assert_eq!(strip.pier_x() - art::TRACK_SHIP_W + 1, strip.ship_x(14));
    assert!(strip.ship_x(14) >= strip.cell_x(14));
    assert_eq!(strip.pier_x() + voyage::PIER_SHIP_IN, strip.ship_x(15));
    // and the ship before it is in the cell of its turn, left of the last flag
    assert!(strip.ship_x(13) + art::TRACK_SHIP_W <= strip.cell_x(14) + 2);
    // and in a cell, the ship stands where its flag will
    assert_eq!(strip.cell_x(7), strip.ship_x(7));
    // Every turn to come is marked here: each becomes a flag
    for c in 0..15 {
        assert!(strip.marked(c));
    }
}

#[test]
fn dunholms_strip_is_a_window_of_tarnholms_size() {
    let dun = Strip::for_map(dunholm(), DESIGN_W);
    let tarn = Strip::for_map(tarnholm(), DESIGN_W);
    assert_eq!(voyage::FLAG_WINDOW, dun.turns);
    assert!(dun.flags);
    assert_eq!(tarn.turns, dun.turns);
    assert_eq!(tarn.pitch, dun.pitch);
    assert_eq!(tarn.width, dun.width);
    check_fits(&dun);
    // A race with a cap keeps the narrow strip without flags
    assert!(!Strip::for_map(capped(), DESIGN_W).flags);
}

#[test]
fn until_the_window_fills_every_turn_has_its_flag_and_the_ship_follows() {
    for taken in 0..=voyage::FLAG_WINDOW - 2 {
        assert_eq!(0, voyage::window_start(taken, false));
        assert_eq!(taken, voyage::window_flags(taken, false));
        assert_eq!(taken, voyage::window_ship_cell(taken, false));
    }
}

#[test]
fn once_the_window_is_full_the_oldest_flag_falls_off_the_left() {
    let full = voyage::FLAG_WINDOW - 2;
    for taken in full + 1..=60 {
        let start = voyage::window_start(taken, false);
        assert_eq!(taken - full, start);
        assert_eq!(full, voyage::window_flags(taken, false));
        // The newest flag is the last turn taken
        assert_eq!(taken, start + voyage::window_flags(taken, false));
        // The ship never reaches the cell beside the pier while the race is on
        assert_eq!(
            voyage::FLAG_WINDOW - 2,
            voyage::window_ship_cell(taken, false)
        );
    }
    // After 15 turns: turns 3 to 15 fly flags, the 16th is being played
    assert_eq!(2, voyage::window_start(15, false));
}

#[test]
fn a_won_race_flags_the_ships_cell_and_puts_the_ship_in_harbour() {
    let strip = Strip::for_map(dunholm(), DESIGN_W);
    // Won on turn 7: seven flags from the first turn, the ship in harbour
    assert_eq!(0, voyage::window_start(7, true));
    assert_eq!(7, voyage::window_flags(7, true));
    assert_eq!(voyage::FLAG_WINDOW, voyage::window_ship_cell(7, true));
    assert_eq!(
        strip.pier_x() + voyage::PIER_SHIP_IN,
        strip.ship_x(voyage::window_ship_cell(7, true))
    );
    // Won in the 31st turn: the flags already up stay where they were, and the winning turn's
    // flag goes up in the cell the ship was sailing in
    assert_eq!(
        voyage::window_start(30, false),
        voyage::window_start(31, true)
    );
    assert_eq!(
        voyage::window_ship_cell(30, false) + 1,
        voyage::window_flags(31, true)
    );
    for taken in 1..=60 {
        assert_eq!(
            voyage::window_start(taken - 1, false),
            voyage::window_start(taken, true),
            "won in turn {taken}"
        );
        // The cell beside the pier never has a flag
        assert!(voyage::window_flags(taken, true) < voyage::FLAG_WINDOW);
    }
}

/// The strip's cells, ship and pier lie inside the framed band of water.
#[test]
fn both_strips_lie_inside_the_framed_water_band() {
    for map in MapDef::all() {
        let strip = Strip::for_map(map, DESIGN_W);
        assert!(strip.left > HUD_INSET, "{}", map.id);
        assert!(
            strip.left + strip.width < DESIGN_W - HUD_INSET,
            "{}",
            map.id
        );
    }
}

#[test]
fn twenty_turns_fit_without_flags_in_the_hud() {
    let strip = Strip::new(20, false, DESIGN_W);
    assert_eq!(9, strip.pitch);
    check_fits(&strip);
    // The pier hugs the last buoy
    assert!(strip.pier_x() > strip.buoy_x(19) + art::TRACK_BUOY_W);
    assert!(
        strip.pier_x()
            <= strip.buoy_x(19) + art::TRACK_BUOY_W + 1 + (strip.pitch - art::TRACK_BUOY_W) / 2
    );
    assert_eq!(strip.pier_x() - art::TRACK_SHIP_W + 1, strip.ship_x(19));
    assert_eq!(strip.pier_x() + voyage::PIER_SHIP_IN, strip.ship_x(20));
    // A buoy is centred in its cell
    assert_eq!(strip.cell_x(3) + 2, strip.buoy_x(3));
    // A ship is centred on its cell, and at most its last column touches the next cell's buoy
    for c in 0..19 {
        assert!(strip.ship_x(c) + art::TRACK_SHIP_W <= strip.buoy_x(c + 1) + 1);
    }
    // Half the turns are marked: ten buoys, on the even turns, evenly spaced
    let mut marked = 0;
    let mut last = -1;
    for c in 0..20 {
        if strip.marked(c) {
            marked += 1;
            if last >= 0 {
                assert_eq!(2 * strip.pitch, strip.buoy_x(c) - strip.buoy_x(last));
                assert!(2 * strip.pitch >= 2 * art::TRACK_BUOY_W);
            }
            last = c;
        }
    }
    assert_eq!(10, marked);
    assert_eq!(19, last);
    assert!(strip.marked(1) && !strip.marked(0));
}

// ---------------------------------------------------------------- the traders

#[test]
fn a_single_trader_is_already_centred_and_a_pair_is_moved_to_centre_its_box() {
    for map in MapDef::all() {
        for t in &map.traders {
            let count = map.traders.iter().filter(|o| o.hex == t.hex).count();
            if count == 1 {
                assert_eq!([0, 0], trader_layout::pair_shift(map, t.hex));
            }
        }
    }
    // Dunholm's desert pair: one cart above the other, boxes at (0,-15) and (0,15) from the anchor
    assert_eq!([0, 4], trader_layout::pair_shift(dunholm(), board::N));
    let at = trader_layout::anchors(dunholm());
    let hex = dunholm().hex[board::N];
    let left = at[5].x.min(at[6].x) - art::TRADER_AX;
    let right = at[5].x.max(at[6].x) - art::TRADER_AX + art::TRADER_BOX_W;
    let top = at[5].y.min(at[6].y) - art::TRADER_AY;
    let bottom = at[5].y.max(at[6].y) - art::TRADER_AY + art::TRADER_BOX_H;
    assert_eq!(hex[0], (left + right) / 2);
    assert_eq!(hex[1], (top + bottom) / 2);
}

// ---------------------------------------------------------------- the ship and the board

/// What the screen shows of the board between its bars: left, top, right, bottom, in board pixels.
fn shown(screen: [i32; 4]) -> [i32; 4] {
    let bar_bottom = screen[2] + TOP_H;
    let tray_top = screen[1] - BOTTOM_H - screen[3];
    let origin = voyage::board_origin(screen[0], bar_bottom, tray_top);
    [
        -origin.x,
        bar_bottom - origin.y,
        screen[0] - origin.x,
        tray_top - origin.y,
    ]
}

#[test]
fn the_stops_follow_what_the_screen_shows_but_never_stray_far() {
    let moor = voyage::DOCK_MOORING;
    let moored = voyage::ship_origin(Stage::Moored, moor, 144);
    assert_eq!(
        Pt::new(moor.x - art::SHIP_CONTACT_X, moor.y - art::SHIP_CONTACT_Y),
        moored
    );
    // Room to the left: the far stop hangs half off the screen's edge, level with the mooring
    let roomy = moored.x - 2 * voyage::MIN_STEP - 4 + voyage::FAR_OUT_X;
    assert_eq!(
        Pt::new(roomy - voyage::FAR_OUT_X, moored.y),
        voyage::ship_origin(Stage::Far, moor, roomy)
    );
    // Less room: it starts higher instead, and still sails two full steps
    let far = voyage::ship_origin(Stage::Far, moor, 150);
    assert_eq!(150 - voyage::FAR_OUT_X, far.x);
    assert!(far.y < moored.y);
    let (dx, dy) = (moored.x - far.x, moored.y - far.y);
    assert!(dx * dx + dy * dy >= 4 * voyage::MIN_STEP * voyage::MIN_STEP);
    // A screen that shows the whole board: the far stop is two full steps out, no more
    assert_eq!(
        Pt::new(moored.x - 2 * voyage::MAX_STEP, moored.y),
        voyage::ship_origin(Stage::Far, moor, -200)
    );
    // A screen that shows no sea to the left at all: straight down from above
    assert_eq!(moored.x, voyage::ship_origin(Stage::Far, moor, 400).x);
    // Done is moored: the flag is the dock's, the ship does not move
    assert_eq!(moored, voyage::ship_origin(Stage::Done, moor, 144));
}

/// On both screens the ship sails in level (the old test also measured how much of its sprite
/// shows, which needs the PNG).
#[test]
fn the_far_stop_on_each_screen_is_level_with_the_dock() {
    let boards = [Pt::new(-144, -23), Pt::new(-132, -52)];
    for (i, screen) in SCREENS.iter().enumerate() {
        let s = shown(*screen);
        let origin = voyage::board_origin(screen[0], TOP_H, screen[1] - BOTTOM_H - screen[3]);
        assert_eq!(boards[i], origin);
        let far = voyage::ship_origin(Stage::Far, voyage::DOCK_MOORING, s[0]);
        assert_eq!(
            voyage::ship_origin(Stage::Moored, voyage::DOCK_MOORING, s[0]).y,
            far.y
        );
    }
}

// ---------------------------------------------------------------- sailing

#[test]
fn only_the_next_stage_is_a_sail_and_nothing_else_is_anything_but_a_jump() {
    assert_eq!(
        voyage::SAIL_IN_MS,
        voyage::sail_ms(Some(Stage::None), Stage::Far)
    );
    assert_eq!(
        voyage::SAIL_STEP_MS,
        voyage::sail_ms(Some(Stage::Far), Stage::Mid)
    );
    assert_eq!(
        voyage::SAIL_STEP_MS,
        voyage::sail_ms(Some(Stage::Mid), Stage::Moored)
    );
    // The first frame, no change, the flag going up, undoing, and skipping a stage all snap
    assert_eq!(0, voyage::sail_ms(None, Stage::Far));
    assert_eq!(0, voyage::sail_ms(Some(Stage::Mid), Stage::Mid));
    assert_eq!(0, voyage::sail_ms(Some(Stage::Moored), Stage::Done));
    assert_eq!(0, voyage::sail_ms(Some(Stage::Mid), Stage::Far));
    assert_eq!(0, voyage::sail_ms(Some(Stage::Far), Stage::Moored));
    assert_eq!(0, voyage::sail_ms(Some(Stage::None), Stage::Mid));
    assert_eq!(0, voyage::sail_ms(Some(Stage::Moored), Stage::None));
}

/// The ship sails in from wholly off the left edge of the screen, level with the far stop.
#[test]
fn a_ship_sailing_in_starts_wholly_off_the_screens_left_edge_level_with_the_far_stop() {
    for _map in MapDef::all() {
        for screen in SCREENS {
            let s = shown(screen);
            let start = voyage::sail_start(Stage::None, voyage::DOCK_MOORING, s[0]);
            let far = voyage::ship_origin(Stage::Far, voyage::DOCK_MOORING, s[0]);
            assert_eq!(far.y, start.y);
            assert_eq!(s[0], start.x + art::SHIP_W);
            assert!(start.x < far.x);
        }
    }
    // Between stops it starts from the stop it was at
    let mid = voyage::ship_origin(Stage::Mid, voyage::DOCK_MOORING, 144);
    assert_eq!(
        mid,
        voyage::sail_start(Stage::Mid, voyage::DOCK_MOORING, 144)
    );
}

#[test]
fn the_sail_runs_from_start_to_end_eased_and_on_whole_pixels() {
    let (from, to) = (Pt::new(100, 50), Pt::new(140, 63));
    assert_eq!(from, voyage::sail_at(from, to, 0, 650));
    assert_eq!(from, voyage::sail_at(from, to, -5, 650));
    assert_eq!(to, voyage::sail_at(from, to, 650, 650));
    assert_eq!(to, voyage::sail_at(from, to, 9999, 650));
    assert_eq!(
        to,
        voyage::sail_at(from, to, 0, 0),
        "no duration is no sail"
    );
    // Halfway is the halfway point exactly (smoothstep(0.5) is 0.5)
    assert_eq!(Pt::new(120, 57), voyage::sail_at(from, to, 325, 650));
    // Slow at the start and at the end, quick in the middle
    let early = voyage::sail_at(from, to, 65, 650);
    let late = voyage::sail_at(from, to, 585, 650);
    assert!(early.x - from.x < 4 * 40 / 10 / 2, "early {early}");
    assert!(to.x - late.x < 4 * 40 / 10 / 2, "late {late}");
    // Never goes backwards, never overshoots
    let (mut last_x, mut last_y) = (from.x, from.y);
    for ms in (0..=650).step_by(10) {
        let at = voyage::sail_at(from, to, ms, 650);
        assert!(at.x >= last_x && at.x <= to.x && at.y >= last_y && at.y <= to.y);
        last_x = at.x;
        last_y = at.y;
    }
    // Sailing left works the same way
    assert_eq!(Pt::new(120, 57), voyage::sail_at(to, from, 325, 650));
}

/// The land sits in the middle of the water between the bars, the King's ship left out of it,
/// never with its top under the top bar, and never with the island's foot under the bottom bar.
#[test]
fn the_land_is_centred_between_the_bars() {
    for (top, bottom) in [(62, 401), (62, 500), (80, 640)] {
        let at = voyage::board_origin(216, top, bottom);
        let above = at.y + voyage::LAND_TOP - top;
        let below = bottom - (at.y + voyage::LAND_FOOT);
        assert!((below - above).abs() <= 1, "{top}");
        assert!(above >= 0 && below >= 0);
    }
    // A little short: the land's top stays under the top bar, its shore runs under the bottom one
    assert_eq!(62 - voyage::LAND_TOP, voyage::board_origin(216, 62, 300).y);
    // Shorter still: the island's lowest building stays above the bottom bar, whatever that hides
    assert_eq!(
        250 - voyage::ISLAND_FOOT,
        voyage::board_origin(216, 62, 250).y
    );
}
