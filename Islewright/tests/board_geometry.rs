// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from BoardGeometryTest: both islands' boards build from their tables, Tarnholm's
//! tables and points are the old hand made ones, and a table that disagrees with the
//! geometry is refused.

mod common;

use common::*;
use dayapp::rules::board::{self, Board, RoadShape};
use dayapp::rules::map_def::{self, TraderSpot};
use dayapp::rules::{DiceValue, MapDef, Pt};

// FROZEN: the old Board's tables, as plain numbers. Hexes NW 0, SW 1, S 2, SE 3, NE 4, N 5,
// CENTRE 6. Edges N 0, NE 1, SE 2, S 3, SW 4, NW 5. Vertices E 0, NE 1, NW 2, W 3, SW 4, SE 5.
const OLD_HEX: [[i32; 2]; 7] = [
    [198, 216],
    [198, 288],
    [252, 324],
    [306, 288],
    [306, 216],
    [252, 180],
    [252, 252],
];
const OLD_VERTEX: [[i32; 2]; 6] = [
    [36, 0],
    [18, -36],
    [-18, -36],
    [-36, 0],
    [-18, 36],
    [18, 36],
];
const OLD_EDGE: [[usize; 2]; 6] = [[2, 1], [1, 0], [0, 5], [4, 5], [3, 4], [3, 2]];
const OLD_ROADS: [[usize; 2]; 17] = [
    [0, 1],
    [0, 2],
    [0, 3],
    [1, 1],
    [1, 2],
    [1, 3],
    [2, 4],
    [2, 3],
    [2, 2],
    [2, 1],
    [3, 5],
    [4, 4],
    [4, 5],
    [3, 3],
    [3, 2],
    [3, 1],
    [4, 2],
];
const OLD_VILLAGES: [[usize; 2]; 6] = [[0, 0], [1, 0], [2, 4], [2, 0], [6, 0], [5, 0]];
const OLD_CITIES: [[usize; 2]; 4] = [[0, 4], [1, 4], [3, 0], [4, 0]];
const OLD_KNIGHTS: [usize; 6] = [0, 1, 2, 3, 4, 5];
const OLD_KNIGHT_RESOURCE: [DiceValue; 6] = [
    DiceValue::Ore,
    DiceValue::Grain,
    DiceValue::Wool,
    DiceValue::Lumber,
    DiceValue::Brick,
    DiceValue::Any,
];
const OLD_ROAD_NEEDS: [i32; 17] = [-1, 0, 1, 1, 3, 4, 4, 6, 7, 8, 9, 10, 11, 8, 13, 14, 15];
const OLD_VILLAGE_NEEDS: [usize; 6] = [0, 3, 6, 8, 10, 12];
const OLD_CITY_NEEDS: [usize; 4] = [2, 5, 14, 16];

fn old_vertex(hex: usize, vertex: usize) -> Pt {
    Pt::new(
        OLD_HEX[hex][0] + OLD_VERTEX[vertex][0],
        OLD_HEX[hex][1] + OLD_VERTEX[vertex][1],
    )
}

#[test]
fn every_map_builds_its_board() {
    assert_eq!(2, MapDef::all().len());
    for map in MapDef::all() {
        let b = Board::for_map(map);
        assert!(same(map, b.map));
        assert!(std::ptr::eq(b, Board::for_map(map)), "cached");
        assert!(same(map, MapDef::by_id(Some(&map.id))));
    }
    assert!(same(tarnholm(), MapDef::default_map()));
    assert!(same(tarnholm(), MapDef::by_id(Some("tarnholm"))));
    assert!(
        same(tarnholm(), MapDef::by_id(Some("nowhere"))),
        "an unknown id is the default map"
    );
    assert!(same(tarnholm(), MapDef::by_id(None)));
}

#[test]
fn tarnholm_tables_are_the_old_tables() {
    let map = tarnholm();
    assert_eq!("tarnholm", map.id);
    assert_eq!(0xFF0D3947, map.sea_fill);
    assert_eq!(OLD_HEX, map.hex);
    assert_eq!(OLD_VERTEX, board::VERTEX);
    assert_eq!(OLD_EDGE, board::EDGE);
    assert_eq!(OLD_ROADS.to_vec(), map.roads);
    assert_eq!(OLD_VILLAGES.to_vec(), map.villages);
    assert_eq!(OLD_CITIES.to_vec(), map.cities);
    assert_eq!(OLD_KNIGHTS.len(), map.traders.len());
    for i in 0..OLD_KNIGHTS.len() {
        assert_eq!(
            TraderSpot::new(OLD_KNIGHTS[i], 0, 0),
            map.traders[i],
            "trader {}",
            i + 1
        );
        // What the view used to read as KNIGHT_RESOURCE[tier - 1]
        assert_eq!(
            OLD_KNIGHT_RESOURCE[i],
            map.bank_resource[map.trader_bank[i]]
        );
        assert_eq!(i as i32 + 1, map.trader_points[i]);
    }
    assert_eq!(vec![3, 4, 5, 7, 9, 11], map.village_points);
    assert_eq!(vec![7, 12, 20, 30], map.city_points);
    assert!(map.road_needs[0].is_empty());
    for (i, &need) in OLD_ROAD_NEEDS.iter().enumerate().skip(1) {
        assert_eq!(vec![need as usize], map.road_needs[i], "road {i}");
    }
    for (i, &need) in OLD_VILLAGE_NEEDS.iter().enumerate() {
        assert_eq!(vec![need], map.village_roads[i]);
    }
    for (i, &need) in OLD_CITY_NEEDS.iter().enumerate() {
        assert_eq!(vec![need], map.city_roads[i]);
    }
    assert_eq!(15, map.turn_cap);
    assert_eq!(0, map.target_score);
    assert_eq!(1, map.road_points);
    assert_eq!(2, map.empty_turn_penalty);
    assert_eq!(None, map.long_road);
    assert_eq!(0, map.trader_bonus_at);
}

/// Every point the old static initialiser computed, by the old formulas.
#[test]
fn tarnholm_points_are_the_old_points() {
    let b = Board::for_map(tarnholm());
    assert_eq!(17, b.road_ends.len());
    for (i, road) in OLD_ROADS.iter().enumerate() {
        let edge = OLD_EDGE[road[1]];
        let a = old_vertex(road[0], edge[0]);
        let c = old_vertex(road[0], edge[1]);
        let (left, right) = if a.x <= c.x { (a, c) } else { (c, a) };
        assert_eq!(left, b.road_ends[i][0], "road {i} left");
        assert_eq!(right, b.road_ends[i][1], "road {i} right");
        assert_eq!(
            Pt::new((a.x + c.x) / 2, (a.y + c.y) / 2),
            b.road_anchor[i],
            "road {i} anchor"
        );
        let (shape, origin) = if left.y == right.y {
            (RoadShape::Horizontal, Pt::new(left.x - 4, left.y - 4))
        } else if left.y > right.y {
            (RoadShape::Slash, Pt::new(left.x - 4, right.y - 4))
        } else {
            (RoadShape::Backslash, Pt::new(left.x - 4, left.y - 4))
        };
        assert_eq!(shape, b.road_shape[i], "road {i} shape");
        assert_eq!(origin, b.road_origin[i], "road {i} origin");
    }
    assert_eq!(6, b.village.len());
    for (i, v) in OLD_VILLAGES.iter().enumerate() {
        assert_eq!(old_vertex(v[0], v[1]), b.village[i], "village {i}");
    }
    assert_eq!(4, b.city.len());
    for (i, c) in OLD_CITIES.iter().enumerate() {
        assert_eq!(old_vertex(c[0], c[1]), b.city[i], "city {i}");
    }
    assert_eq!(6, b.knight.len());
    for (i, &k) in OLD_KNIGHTS.iter().enumerate() {
        assert_eq!(
            Pt::new(OLD_HEX[k][0], OLD_HEX[k][1]),
            b.knight[i],
            "knight {i}"
        );
    }
    assert_eq!(0, RoadShape::Horizontal as i32);
    assert_eq!(1, RoadShape::Slash as i32);
    assert_eq!(2, RoadShape::Backslash as i32);
    assert_eq!(4, board::ROAD_CAP);
}

/// The facts the old check asserted by hand, in literal coordinates.
#[test]
fn tarnholm_hand_facts() {
    let b = Board::for_map(tarnholm());
    let hub = Pt::new(216, 252);
    assert!(b.road_touches(1, hub) && b.road_touches(2, hub) && b.road_touches(3, hub));
    assert_eq!(Pt::new(234, 216), b.village[0]);
    assert!(b.road_touches(0, b.village[0]) && b.road_touches(1, b.village[0]));
    assert_eq!(Pt::new(288, 180), b.village[5]);
    assert!(b.road_touches(12, b.village[5]));
    assert!(b.roads_meet(15, 16));
    assert!(b.road_touches(15, Pt::new(324, 252)) && b.road_touches(16, Pt::new(324, 252)));
    assert_eq!(Pt::new(342, 288), b.city[2]);
    assert_eq!(Pt::new(342, 216), b.city[3]);
    assert_eq!(Pt::new(225, 198), b.road_anchor[0]);
    assert_eq!(Pt::new(216, 180), b.road_ends[0][0]);
    assert!((b.road_distance_squared(0, 225, 198) - 0.0).abs() < 0.001);
    assert!((b.road_distance_squared(2, 198, 262) - 100.0).abs() < 0.001);
}

/// Tarnholm's one-parent road table is a choice among the roads that touch.
#[test]
fn tarnholm_parents_are_among_the_touching_roads() {
    let map = tarnholm();
    let touching = Board::adjacency(&map.hex, &map.roads);
    assert!(touching[0].is_empty());
    for (i, touches) in touching.iter().enumerate().skip(1) {
        assert!(touches.contains(&map.road_needs[i][0]), "road {i}");
    }
}

/// Worked out here from the points, not read from the board's own check: no two anchors
/// within 14 px.
#[test]
fn no_two_anchors_on_any_map_are_within_fourteen_pixels() {
    for map in MapDef::all() {
        let anchors = Board::for_map(map).named_anchors();
        let mut closest = i32::MAX;
        for i in 0..anchors.len() {
            for j in i + 1..anchors.len() {
                let dx = anchors[i].1.x - anchors[j].1.x;
                let dy = anchors[i].1.y - anchors[j].1.y;
                closest = closest.min(dx * dx + dy * dy);
                assert!(
                    dx * dx + dy * dy >= board::MIN_ANCHOR_GAP * board::MIN_ANCHOR_GAP,
                    "{}: {} and {}",
                    map.id,
                    anchors[i].0,
                    anchors[j].0
                );
            }
        }
        assert!(closest >= 14 * 14, "{} closest pair {closest}", map.id);
    }
}

/// Dunholm's close pairs (under 24 px), each with its squared distance. The upper trader of
/// the north desert's pair sits 21 px from the middle of the road on that desert's north west
/// edge; no other trader is near anything. The rest are a road beside the village or castle at
/// its end, which Tarnholm has too.
#[test]
fn dunholms_close_pairs_are_the_known_ones() {
    let anchors = Board::for_map(dunholm()).named_anchors();
    let mut close = Vec::new();
    for i in 0..anchors.len() {
        for j in i + 1..anchors.len() {
            let dx = anchors[i].1.x - anchors[j].1.x;
            let dy = anchors[i].1.y - anchors[j].1.y;
            if dx * dx + dy * dy < 24 * 24 {
                close.push(format!(
                    "{} / {} {}",
                    anchors[i].0,
                    anchors[j].0,
                    dx * dx + dy * dy
                ));
            }
        }
    }
    assert_eq!(
        vec![
            "road 0 / village 2 405",
            "road 1 / village 2 405",
            "road 2 / city 0 324",
            "road 3 / village 4 405",
            "road 4 / village 4 405",
            "road 5 / city 2 324",
            "road 6 / village 6 405",
            "road 7 / village 6 324",
            "road 8 / city 3 405",
            "road 9 / city 3 324",
            "road 10 / village 5 405",
            "road 11 / village 5 405",
            "road 12 / city 1 324",
            "road 13 / village 3 405",
            "road 14 / village 3 405",
            "road 15 / village 1 324",
            "road 16 / village 1 405",
            "road 17 / village 0 324",
            "road 17 / knight 6 441",
        ],
        close
    );
}

fn assert_refused(map: MapDef, because: &str) {
    match map.check() {
        Err(e) => assert!(e.contains(because), "{e}"),
        Ok(map) => match Board::build(leak(map)) {
            Ok(_) => panic!("expected the board to be refused: {because}"),
            Err(e) => assert!(e.contains(because), "{e}"),
        },
    }
}

/// The check has to bite: a table that disagrees with the geometry does not build.
#[test]
fn a_wrong_table_is_refused() {
    let mut off_road = race_builder();
    off_road.village_roads = vec![
        vec![17],
        vec![15, 16],
        vec![0, 1],
        vec![13, 14],
        vec![3, 4],
        vec![10, 11],
        vec![6, 9],
    ];
    assert_refused(off_road, "village 6 is off road 9");

    let mut apart = race_builder();
    apart.road_needs =
        map_def::single(&[-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 2]);
    assert_refused(apart, "road 17 does not meet road 2");

    let mut island = race_builder();
    island.road_needs[6] = vec![];
    island.road_needs[5] = vec![6];
    island.road_needs[7] = vec![6, 8];
    island.road_needs[4] = vec![3];
    // Road 6 now needs nothing it can get, and road 5 only road 6
    assert_refused(island, "cannot be reached");

    let mut crowded = race_builder();
    crowded.traders[5] = TraderSpot::new(board::N, 30, 0);
    assert_refused(crowded, "px apart");
}
