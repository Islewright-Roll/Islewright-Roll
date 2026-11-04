// SPDX-License-Identifier: GPL-3.0-or-later
//! One island as data: where its pieces sit, what each needs before it can be built, what
//! the bank holds, how the sheet scores and when the game ends. Playsheet, BuildPlan, Game
//! and Board read everything map specific from here, so a second island is a second table
//! and no second rules module.
//!
//! Conventions, which every reader relies on:
//! - roads, villages and cities are 0 based; road 0 is the start road and is prebuilt.
//! - trader tiers and bank slots are 1 based. `traders`, `trader_bank` and `trader_points`
//!   are indexed by tier - 1; `bank_resource` and `bank_traders` are indexed by slot and
//!   carry an unused entry at index 0.
//! - a "needs" list is ANY ONE OF for roads (road_needs, village_roads, city_roads) and
//!   ALL OF for the traders behind a bank slot (bank_traders).
//!
//! Geometry names (hexes, vertices, edges) are the constants in the board module.
//!
//! The same struct serves as the builder: the table functions return one unchecked, a test
//! may break a table on purpose, and `check` validates it into a map the rules can trust.

use std::sync::OnceLock;

use super::board;
use super::dice::DiceValue;

/// Where a trader stands: a hex, and its anchor as an offset from that hex's centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraderSpot {
    pub hex: usize,
    pub dx: i32,
    pub dy: i32,
}

impl TraderSpot {
    pub const fn new(hex: usize, dx: i32, dy: i32) -> TraderSpot {
        TraderSpot { hex, dx, dy }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapDef {
    /// Save and preference keys, never shown. The UI names the island from it.
    pub id: String,
    /// The board's own sea colour (ARGB), painted wherever the board art does not reach.
    pub sea_fill: u32,
    /// 7 centres, in the board's order NW, SW, S, SE, NE, N, CENTRE.
    pub hex: [[i32; 2]; 7],
    /// { hex, edge } per road.
    pub roads: Vec<[usize; 2]>,
    /// Per road, roads of which any one built permits it; empty for road 0.
    pub road_needs: Vec<Vec<usize>>,
    /// { hex, vertex } per village.
    pub villages: Vec<[usize; 2]>,
    /// Per village, roads of which any one built permits it.
    pub village_roads: Vec<Vec<usize>>,
    /// True: village i also needs village i - 1.
    pub ordered_villages: bool,
    pub cities: Vec<[usize; 2]>,
    pub city_roads: Vec<Vec<usize>>,
    pub ordered_cities: bool,
    /// Per tier, the anchor as an offset from a hex centre.
    pub traders: Vec<TraderSpot>,
    /// True: tier t needs tier t - 1.
    pub ordered_traders: bool,
    /// Per tier: the bank slot it feeds, 0 for none.
    pub trader_bank: Vec<usize>,
    /// Per bank slot.
    pub bank_resource: Vec<DiceValue>,
    /// Per bank slot: the tiers that must all be built.
    pub bank_traders: Vec<Vec<usize>>,
    /// True: one banked resource pays for one needed; false: Tarnholm's old rule, see
    /// game::use_resources.
    pub bank_matches_once: bool,
    /// Per built road, the start road excepted.
    pub road_points: i32,
    pub trader_points: Vec<i32>,
    pub village_points: Vec<i32>,
    pub city_points: Vec<i32>,
    /// Per turn in which nothing was built.
    pub empty_turn_penalty: i32,
    /// The road that earns long_road_bonus, if any.
    pub long_road: Option<usize>,
    pub long_road_bonus: i32,
    /// Traders built that earn trader_bonus, 0 for never.
    pub trader_bonus_at: usize,
    pub trader_bonus: i32,
    /// The game ends after this many turns; 0 for no cap.
    pub turn_cap: i32,
    /// Reaching it ends the game; 0 for none.
    pub target_score: i32,
    /// Top scores: fewest turns rather than most points.
    pub lower_is_better: bool,
}

/// Wraps a one-parent table as "any one of" lists; -1 becomes the empty list.
pub fn single(needs: &[i32]) -> Vec<Vec<usize>> {
    needs
        .iter()
        .map(|&n| if n < 0 { Vec::new() } else { vec![n as usize] })
        .collect()
}

fn lists(rows: &[&[usize]]) -> Vec<Vec<usize>> {
    rows.iter().map(|r| r.to_vec()).collect()
}

/// The hex centres both islands share.
const GRID: [[i32; 2]; 7] = [
    [198, 216],
    [198, 288],
    [252, 324],
    [306, 288],
    [306, 216],
    [252, 180],
    [252, 252],
];

impl MapDef {
    /// The tables must agree with each other on their lengths before anything indexes them.
    /// Returns the map once they do, or what is wrong.
    pub fn check(self) -> Result<MapDef, String> {
        let fail = |what: String| Err(format!("Map {}: {what}", self.id));
        if self.id.is_empty() {
            return fail("no id".into());
        }
        if self.roads.is_empty() || self.road_needs.len() != self.roads.len() {
            return fail("roadNeeds must have one entry per road".into());
        }
        if self.roads.len() > 32 {
            return fail("BuildPlan keys allow at most 32 roads".into());
        }
        if self.village_roads.len() != self.villages.len()
            || self.village_points.len() != self.villages.len()
        {
            return fail("village tables differ in length".into());
        }
        if self.city_roads.len() != self.cities.len() || self.city_points.len() != self.cities.len()
        {
            return fail("city tables differ in length".into());
        }
        if self.trader_bank.len() != self.traders.len()
            || self.trader_points.len() != self.traders.len()
        {
            return fail("trader tables differ in length".into());
        }
        if self.bank_traders.len() != self.bank_resource.len() || self.bank_resource.is_empty() {
            return fail("bank tables differ in length".into());
        }
        for tier in 1..=self.traders.len() {
            let slot = self.trader_bank[tier - 1];
            if slot >= self.bank_resource.len() {
                return fail(format!("trader {tier} feeds bank slot {slot}"));
            }
            if slot > 0 && !self.bank_traders[slot].contains(&tier) {
                return fail(format!("bank slot {slot} does not list trader {tier}"));
            }
        }
        for slot in 1..self.bank_resource.len() {
            if self.bank_traders[slot].is_empty() {
                return fail(format!("bank slot {slot} has no trader"));
            }
            for &tier in &self.bank_traders[slot] {
                if tier < 1 || tier > self.traders.len() || self.trader_bank[tier - 1] != slot {
                    return fail(format!("bank slot {slot} lists trader {tier}"));
                }
            }
        }
        if let Some(long) = self.long_road
            && long >= self.roads.len()
        {
            return fail(format!("longRoad {long}"));
        }
        if self.turn_cap < 0 {
            return fail(format!("turnCap {}", self.turn_cap));
        }
        // A game must end somehow: after so many turns, or on reaching a score
        if self.turn_cap <= 0 && self.target_score <= 0 {
            return fail("no turn cap and no target score".into());
        }
        Ok(self)
    }

    /// The number of trader tiers.
    pub fn trader_count(&self) -> usize {
        self.traders.len()
    }

    /// The number of bank slots; they are numbered 1 to this.
    pub fn bank_size(&self) -> usize {
        self.bank_resource.len() - 1
    }

    /// The most a game on this map can score: every piece built, both bonuses earned and no
    /// empty turn. Read from the same tables as Playsheet::score, so it follows them. On a
    /// race it is the highest target that can still be reached.
    pub fn max_score(&self) -> i32 {
        let mut score = self.road_points * (self.roads.len() as i32 - 1);
        score += self.trader_points.iter().sum::<i32>();
        score += self.village_points.iter().sum::<i32>();
        score += self.city_points.iter().sum::<i32>();
        if self.long_road.is_some() {
            score += self.long_road_bonus;
        }
        if self.trader_bonus_at > 0 && self.trader_bonus_at <= self.traders.len() {
            score += self.trader_bonus;
        }
        score
    }

    /// The first island: a lake at its heart, fifteen turns, highest score wins. Its roads
    /// are a tree with one named parent each, which is a rule of the sheet and not something
    /// the geometry implies, so the table is explicit. Unchecked; see `tarnholm`.
    pub fn tarnholm_tables() -> MapDef {
        use board::*;
        MapDef {
            id: "tarnholm".into(),
            sea_fill: 0xFF0D3947,
            hex: GRID,
            roads: vec![
                [NW, E_NE],
                [NW, E_SE],
                [NW, E_S],
                [SW, E_NE],
                [SW, E_SE],
                [SW, E_S],
                [S, E_SW],
                [S, E_S],
                [S, E_SE],
                [S, E_NE],
                [SE, E_NW],
                [NE, E_SW],
                [NE, E_NW],
                [SE, E_S],
                [SE, E_SE],
                [SE, E_NE],
                [NE, E_SE],
            ],
            road_needs: single(&[-1, 0, 1, 1, 3, 4, 4, 6, 7, 8, 9, 10, 11, 8, 13, 14, 15]),
            villages: vec![
                [NW, V_E],
                [SW, V_E],
                [S, V_SW],
                [S, V_E],
                [CENTRE, V_E],
                [N, V_E],
            ],
            village_roads: single(&[0, 3, 6, 8, 10, 12]),
            ordered_villages: true,
            cities: vec![[NW, V_SW], [SW, V_SW], [SE, V_E], [NE, V_E]],
            city_roads: single(&[2, 5, 14, 16]),
            ordered_cities: true,
            traders: vec![
                TraderSpot::new(NW, 0, 0),
                TraderSpot::new(SW, 0, 0),
                TraderSpot::new(S, 0, 0),
                TraderSpot::new(SE, 0, 0),
                TraderSpot::new(NE, 0, 0),
                TraderSpot::new(N, 0, 0),
            ],
            ordered_traders: true,
            trader_bank: vec![1, 2, 3, 4, 5, 6],
            bank_resource: vec![
                DiceValue::None,
                DiceValue::Ore,
                DiceValue::Grain,
                DiceValue::Wool,
                DiceValue::Lumber,
                DiceValue::Brick,
                DiceValue::Any,
            ],
            bank_traders: lists(&[&[], &[1], &[2], &[3], &[4], &[5], &[6]]),
            // Tarnholm keeps the sheet's old habit: when a build needs a resource twice and the
            // bank holds it once, both count as found and the second is free. By design.
            bank_matches_once: false,
            road_points: 1,
            trader_points: vec![1, 2, 3, 4, 5, 6],
            village_points: vec![3, 4, 5, 7, 9, 11],
            city_points: vec![7, 12, 20, 30],
            empty_turn_penalty: 2,
            long_road: None,
            long_road_bonus: 0,
            trader_bonus_at: 0,
            trader_bonus: 0,
            turn_cap: 15,
            target_score: 0,
            lower_is_better: false,
        }
    }

    /// The second island: no lake, a race to ten points with no turn cap, fewest turns wins.
    /// Roads follow from the geometry (any road that touches), villages and castles go up
    /// in any order, and the two desert hexes each hold a pair of traders that must both
    /// stand before their "?" can be spent. Unchecked, so a test can break one table on
    /// purpose; see `dunholm`.
    pub fn dunholm_tables() -> MapDef {
        use board::*;
        let roads = vec![
            [NW, E_NE],
            [NW, E_SE],
            [NW, E_S],
            [SW, E_NE],
            [SW, E_SE],
            [SW, E_S],
            [S, E_SW],
            [S, E_S],
            [S, E_SE],
            [SE, E_S],
            [SE, E_SE],
            [SE, E_NE],
            [NE, E_S],
            [NE, E_SE],
            [NE, E_NE],
            [NE, E_N],
            [N, E_NE],
            [N, E_N],
        ];
        // Any road that touches, worked out from the shared vertices
        let road_needs = Board::adjacency(&GRID, &roads);
        MapDef {
            id: "dunholm".into(),
            sea_fill: 0xFF0D3947,
            // The same grid as Tarnholm
            hex: GRID,
            roads,
            road_needs,
            villages: vec![
                [N, V_NW],
                [N, V_E],
                [NW, V_E],
                [NE, V_E],
                [SW, V_E],
                [SE, V_E],
                [S, V_SW],
            ],
            village_roads: lists(&[
                &[17],
                &[15, 16],
                &[0, 1],
                &[13, 14],
                &[3, 4],
                &[10, 11],
                &[6, 7],
            ]),
            ordered_villages: false,
            cities: vec![[NW, V_SW], [CENTRE, V_E], [SW, V_SW], [SE, V_SW]],
            city_roads: lists(&[&[2], &[12], &[5], &[8, 9]]),
            ordered_cities: false,
            traders: vec![
                TraderSpot::new(NW, 0, 0),
                TraderSpot::new(SW, 0, 0),
                TraderSpot::new(S, 0, 0),
                TraderSpot::new(SE, 0, 0),
                TraderSpot::new(NE, 0, 0),
                // The two traders of a desert stand one above the other, 30 px apart: a cart is
                // 28 px long and a desert holds no room for two abreast beside the roads and the
                // village or castle on its east vertex. The 12 px of room between them holds
                // the one resource the pair shares (trader_layout::shared_at), a row clear
                // above and below it.
                TraderSpot::new(N, 0, -15),
                TraderSpot::new(N, 0, 15),
                TraderSpot::new(CENTRE, 0, -15),
                TraderSpot::new(CENTRE, 0, 15),
            ],
            ordered_traders: false,
            trader_bank: vec![1, 2, 3, 4, 5, 6, 6, 7, 7],
            bank_resource: vec![
                DiceValue::None,
                DiceValue::Ore,
                DiceValue::Grain,
                DiceValue::Wool,
                DiceValue::Lumber,
                DiceValue::Brick,
                DiceValue::Any,
                DiceValue::Any,
            ],
            bank_traders: lists(&[&[], &[1], &[2], &[3], &[4], &[5], &[6, 7], &[8, 9]]),
            // A banked resource pays for one needed resource, never two
            bank_matches_once: true,
            road_points: 0,
            trader_points: vec![0; 9],
            village_points: vec![1; 7],
            city_points: vec![2; 4],
            empty_turn_penalty: 0,
            long_road: Some(6),
            long_road_bonus: 2,
            trader_bonus_at: 3,
            trader_bonus: 2,
            // No cap: the game ends only when the target is reached, as in the source game
            turn_cap: 0,
            target_score: 10,
            lower_is_better: true,
        }
    }

    /// The first island, checked once.
    pub fn tarnholm() -> &'static MapDef {
        static MAP: OnceLock<MapDef> = OnceLock::new();
        MAP.get_or_init(|| {
            MapDef::tarnholm_tables()
                .check()
                .expect("Tarnholm's tables")
        })
    }

    /// The second island, checked once.
    pub fn dunholm() -> &'static MapDef {
        static MAP: OnceLock<MapDef> = OnceLock::new();
        MAP.get_or_init(|| MapDef::dunholm_tables().check().expect("Dunholm's tables"))
    }

    /// The island a new player starts on, whose scores keep the original key.
    pub fn default_map() -> &'static MapDef {
        MapDef::tarnholm()
    }

    /// Every island, the default first.
    pub fn all() -> [&'static MapDef; 2] {
        [MapDef::tarnholm(), MapDef::dunholm()]
    }

    /// Whether this is the default island (compared by id, which is what saves name).
    pub fn is_default(&self) -> bool {
        self.id == MapDef::default_map().id
    }

    /// The map with this id; an id nobody knows (or none) is the default map.
    pub fn by_id(id: Option<&str>) -> &'static MapDef {
        MapDef::find(id.unwrap_or("")).unwrap_or(MapDef::default_map())
    }

    /// The map with this id, or None: unlike `by_id`, an unknown id is not the default.
    pub fn find(id: &str) -> Option<&'static MapDef> {
        MapDef::all().into_iter().find(|m| m.id == id)
    }

    /// The map the app opens on: the one the Map setting names. Each map keeps its own saved
    /// game, so a save cannot decide; a setting nobody knows (or none at all) is the default.
    pub fn for_launch(settings_map_id: Option<&str>) -> &'static MapDef {
        MapDef::by_id(settings_map_id)
    }
}
