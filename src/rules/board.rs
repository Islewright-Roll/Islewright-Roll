// SPDX-License-Identifier: GPL-3.0-or-later
//! One island's geometry, in board pixels (504x504, origin top left). Everything a piece
//! needs to be placed is computed from the map's seven hex centres and its piece tables
//! (see MapDef); nothing is digitised by hand. The numbers come from the bake's geometry
//! file under tools/.
//!
//! Hexes are flat topped, 72 wide and 72 tall. Vertices from a centre: E (+36, 0),
//! NE (+18, -36), NW (-18, -36), W (-36, 0), SW (-18, +36), SE (+18, +36). An edge is
//! named by the neighbour it faces. Anchors: the edge midpoint for a road, the vertex
//! for a village or city, an offset from the hex centre for a knight.
//!
//! Get one with `Board::for_map`; a map's board is built and checked once, then cached.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::map_def::MapDef;
use super::pt::Pt;

pub const WIDTH: i32 = 504;
pub const HEIGHT: i32 = 504;
pub const CENTRE_X: i32 = 252;
pub const CENTRE_Y: i32 = 252;

// Hexes, as indices into MapDef::hex
pub const NW: usize = 0;
pub const SW: usize = 1;
pub const S: usize = 2;
pub const SE: usize = 3;
pub const NE: usize = 4;
pub const N: usize = 5;
pub const CENTRE: usize = 6;

// Vertices, and edges as the two vertices they join
pub const V_E: usize = 0;
pub const V_NE: usize = 1;
pub const V_NW: usize = 2;
pub const V_W: usize = 3;
pub const V_SW: usize = 4;
pub const V_SE: usize = 5;
pub const VERTEX: [[i32; 2]; 6] = [
    [36, 0],
    [18, -36],
    [-18, -36],
    [-36, 0],
    [-18, 36],
    [18, 36],
];
pub const E_N: usize = 0;
pub const E_NE: usize = 1;
pub const E_SE: usize = 2;
pub const E_S: usize = 3;
pub const E_SW: usize = 4;
pub const E_NW: usize = 5;
pub const EDGE: [[usize; 2]; 6] = [
    [V_NW, V_NE],
    [V_NE, V_E],
    [V_E, V_SE],
    [V_SW, V_SE],
    [V_W, V_SW],
    [V_W, V_NW],
];

/// Road sprite shapes, chosen from the edge's two vertices and never from its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoadShape {
    /// Level.
    Horizontal = 0,
    /// Lower left to upper right.
    Slash = 1,
    /// Upper left to lower right.
    Backslash = 2,
}

/// Round cap the road sprites carry beyond each vertex.
pub const ROAD_CAP: i32 = 4;

/// No two anchors may sit closer than this, or a tap could not tell them apart.
pub const MIN_ANCHOR_GAP: i32 = 14;

#[derive(Debug)]
pub struct Board {
    pub map: &'static MapDef,
    /// Each road's two vertices, the left one first.
    pub road_ends: Vec<[Pt; 2]>,
    pub road_anchor: Vec<Pt>,
    /// Where the road's sprite goes (its top left).
    pub road_origin: Vec<Pt>,
    pub road_shape: Vec<RoadShape>,
    pub village: Vec<Pt>,
    pub city: Vec<Pt>,
    /// Per tier - 1, the trader's anchor before trader_layout centres a pair.
    pub knight: Vec<Pt>,
}

impl Board {
    /// The board of a map, built and checked the first time it is asked for. A map whose
    /// tables disagree with its geometry is a bug, so it panics; `Board::build` says why
    /// without panicking.
    pub fn for_map(map: &'static MapDef) -> &'static Board {
        static CACHE: OnceLock<Mutex<HashMap<String, &'static Board>>> = OnceLock::new();
        let mut cache = CACHE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(board) = cache.get(&map.id)
            && std::ptr::eq(board.map, map)
        {
            return board;
        }
        let board: &'static Board = Box::leak(Box::new(
            Board::build(map).unwrap_or_else(|e| panic!("{e}")),
        ));
        cache.insert(map.id.clone(), board);
        board
    }

    /// Builds and checks a map's board, uncached.
    pub fn build(map: &'static MapDef) -> Result<Board, String> {
        let road_ends = Board::road_ends(&map.hex, &map.roads);
        let mut road_anchor = Vec::new();
        let mut road_origin = Vec::new();
        let mut road_shape = Vec::new();
        for [left, right] in &road_ends {
            road_anchor.push(Pt::new((left.x + right.x) / 2, (left.y + right.y) / 2));
            if left.y == right.y {
                road_shape.push(RoadShape::Horizontal);
                road_origin.push(Pt::new(left.x - ROAD_CAP, left.y - ROAD_CAP));
            } else if left.y > right.y {
                // Lower left to upper right
                road_shape.push(RoadShape::Slash);
                road_origin.push(Pt::new(left.x - ROAD_CAP, right.y - ROAD_CAP));
            } else {
                road_shape.push(RoadShape::Backslash);
                road_origin.push(Pt::new(left.x - ROAD_CAP, left.y - ROAD_CAP));
            }
        }
        let village = map
            .villages
            .iter()
            .map(|v| Board::vertex(&map.hex, v[0], v[1]))
            .collect();
        let city = map
            .cities
            .iter()
            .map(|c| Board::vertex(&map.hex, c[0], c[1]))
            .collect();
        let knight = map
            .traders
            .iter()
            .map(|t| Pt::new(map.hex[t.hex][0] + t.dx, map.hex[t.hex][1] + t.dy))
            .collect();
        let board = Board {
            map,
            road_ends,
            road_anchor,
            road_origin,
            road_shape,
            village,
            city,
            knight,
        };
        board.check()?;
        Ok(board)
    }

    pub fn vertex(hex: &[[i32; 2]], h: usize, vertex: usize) -> Pt {
        Pt::new(hex[h][0] + VERTEX[vertex][0], hex[h][1] + VERTEX[vertex][1])
    }

    /// Each road's two vertices, the left one first (the first of the edge's two on a tie).
    pub fn road_ends(hex: &[[i32; 2]], roads: &[[usize; 2]]) -> Vec<[Pt; 2]> {
        roads
            .iter()
            .map(|road| {
                let edge = EDGE[road[1]];
                let a = Board::vertex(hex, road[0], edge[0]);
                let b = Board::vertex(hex, road[0], edge[1]);
                if a.x <= b.x { [a, b] } else { [b, a] }
            })
            .collect()
    }

    /// Per road, every other road that shares a vertex with it, in ascending order: the
    /// "any touching road" rule as a road_needs table. Entry 0 is left empty, because the
    /// start road needs nothing. For a map whose roads follow from its geometry; a map
    /// whose road order is a rule of its own writes its table by hand.
    pub fn adjacency(hex: &[[i32; 2]], roads: &[[usize; 2]]) -> Vec<Vec<usize>> {
        let ends = Board::road_ends(hex, roads);
        let mut needs = vec![Vec::new()];
        for i in 1..roads.len() {
            needs.push(
                (0..roads.len())
                    .filter(|&j| j != i && meet(&ends[i], &ends[j]))
                    .collect(),
            );
        }
        needs
    }

    pub fn road_touches(&self, road: usize, p: Pt) -> bool {
        self.road_ends[road][0] == p || self.road_ends[road][1] == p
    }

    pub fn roads_meet(&self, a: usize, b: usize) -> bool {
        meet(&self.road_ends[a], &self.road_ends[b])
    }

    /// Squared distance from a board point to the line between a road's two vertices.
    pub fn road_distance_squared(&self, road: usize, x: i32, y: i32) -> f32 {
        let [a, b] = self.road_ends[road];
        let dx = (b.x - a.x) as f32;
        let dy = (b.y - a.y) as f32;
        let t = ((x - a.x) as f32 * dx + (y - a.y) as f32 * dy) / (dx * dx + dy * dy);
        let t = t.clamp(0.0, 1.0);
        let px = a.x as f32 + t * dx - x as f32;
        let py = a.y as f32 + t * dy - y as f32;
        px * px + py * py
    }

    /// The anchors with their names, in the order the spacing check reads them.
    pub fn named_anchors(&self) -> Vec<(String, Pt)> {
        let mut out = Vec::new();
        for (i, p) in self.road_anchor.iter().enumerate() {
            out.push((format!("road {i}"), *p));
        }
        for (i, p) in self.village.iter().enumerate() {
            out.push((format!("village {i}"), *p));
        }
        for (i, p) in self.city.iter().enumerate() {
            out.push((format!("city {i}"), *p));
        }
        for (i, p) in self.knight.iter().enumerate() {
            out.push((format!("knight {}", i + 1), *p));
        }
        out
    }

    /// The map's tables are only right if its rules still hold on its geometry, so this
    /// checks them once per map: every road shares a vertex with each road that permits
    /// it, every village and city sits on an end of each road that permits it, the long
    /// road (if the map has one) can be reached from the start road, and no two anchors
    /// are close enough to be confused under a finger.
    fn check(&self) -> Result<(), String> {
        let map = self.map;
        let require = |ok: bool, what: String| {
            if ok {
                Ok(())
            } else {
                Err(format!("Board geometry ({}): {what}", map.id))
            }
        };
        let roads = map.roads.len();
        for i in 0..roads {
            for &need in &map.road_needs[i] {
                require(
                    need < roads && need != i,
                    format!("road {i} needs road {need}"),
                )?;
                require(
                    self.roads_meet(i, need),
                    format!("road {i} does not meet road {need}"),
                )?;
            }
        }
        for (i, needs) in map.village_roads.iter().enumerate() {
            for &need in needs {
                require(need < roads, format!("village {i} needs road {need}"))?;
                require(
                    self.road_touches(need, self.village[i]),
                    format!("village {i} is off road {need}"),
                )?;
            }
        }
        for (i, needs) in map.city_roads.iter().enumerate() {
            for &need in needs {
                require(need < roads, format!("city {i} needs road {need}"))?;
                require(
                    self.road_touches(need, self.city[i]),
                    format!("city {i} is off road {need}"),
                )?;
            }
        }

        // Every road must be buildable eventually, the long road among them
        let mut reached = vec![false; roads];
        reached[0] = true;
        let mut grew = true;
        while grew {
            grew = false;
            for i in 1..roads {
                if !reached[i] && map.road_needs[i].iter().any(|&n| reached[n]) {
                    reached[i] = true;
                    grew = true;
                }
            }
        }
        for (i, ok) in reached.iter().enumerate() {
            let name = if Some(i) == map.long_road {
                "the long road "
            } else {
                "road "
            };
            require(
                *ok,
                format!("{name}{i} cannot be reached from the start road"),
            )?;
        }

        let anchors = self.named_anchors();
        for i in 0..anchors.len() {
            for j in i + 1..anchors.len() {
                let dx = anchors[i].1.x - anchors[j].1.x;
                let dy = anchors[i].1.y - anchors[j].1.y;
                require(
                    dx * dx + dy * dy >= MIN_ANCHOR_GAP * MIN_ANCHOR_GAP,
                    format!(
                        "{} and {} are less than {MIN_ANCHOR_GAP} px apart",
                        anchors[i].0, anchors[j].0
                    ),
                )?;
            }
        }
        Ok(())
    }
}

fn meet(a: &[Pt; 2], b: &[Pt; 2]) -> bool {
    a[0] == b[0] || a[0] == b[1] || a[1] == b[0] || a[1] == b[1]
}
