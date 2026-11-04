// SPDX-License-Identifier: GPL-3.0-or-later
//! The spots on an island a tap can mean, and which one a tap on the board means. This was
//! the old game view's own logic (its piece lists and pieceAt); it is here so the UI only has
//! to turn a touch into board pixels and back.
//!
//! A board point is in board pixels (see the board module), so the UI subtracts where it drew
//! the board before asking.

use super::art;
use super::board::Board;
use super::build_plan::Kind;
use super::game::Game;
use super::map_def::MapDef;
use super::pt::Pt;
use super::trader_layout;

/// How far from a village or castle's vertex a tap still counts.
pub const REACH_EDGE: i32 = 14;
/// How far from the line of a road a tap still counts.
pub const REACH_ROAD: i32 = 7;
/// How far from a trader's anchor a tap still counts.
pub const REACH_TRADER: i32 = 20;

/// One spot: a road, village or castle (0 based) or a trader (1 based tier), with its anchor
/// in board pixels and how far from it a tap still counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub kind: Kind,
    pub index: usize,
    pub anchor: Pt,
    pub reach: i32,
}

impl Piece {
    /// Squared distance from a board point to the anchor, or None when it is beyond reach.
    pub fn distance_squared(&self, x: i32, y: i32) -> Option<i32> {
        let dx = x - self.anchor.x;
        let dy = y - self.anchor.y;
        let d = dx * dx + dy * dy;
        (d <= self.reach * self.reach).then_some(d)
    }

    /// The row the piece stands on, which the painter's order sorts by.
    pub fn foot_y(&self) -> i32 {
        if self.kind == Kind::Knight {
            self.anchor.y + art::TRADER_FEET
        } else {
            self.anchor.y
        }
    }

    /// How far above its anchor the middle of the piece's sprite is.
    pub fn body_rise(&self) -> i32 {
        match self.kind {
            Kind::Village => art::VILLAGE_RISE,
            Kind::City => art::CASTLE_RISE,
            _ => art::TRADER_RISE,
        }
    }

    /// How far the sprite reaches round that middle: a tap inside that circle is on the piece.
    pub fn body_radius(&self) -> i32 {
        match self.kind {
            Kind::Village => art::VILLAGE_RADIUS,
            Kind::City => art::CASTLE_RADIUS,
            _ => art::TRADER_RADIUS,
        }
    }
}

/// Every spot of one island.
#[derive(Clone, Debug)]
pub struct BoardPieces {
    pub map: &'static MapDef,
    /// Every road, the start road included, in road order.
    pub roads: Vec<Piece>,
    /// Villages, castles and traders, in the painter's order: whatever stands further down
    /// the island comes later.
    pub standing: Vec<Piece>,
    /// Per bank slot (entry 0 unused), where the resource its several traders share rests
    /// (trader_layout::shared_at), or None.
    pub shared_at: Vec<Option<Pt>>,
}

impl BoardPieces {
    pub fn for_map(map: &'static MapDef) -> BoardPieces {
        let board = Board::for_map(map);
        let roads = board
            .road_anchor
            .iter()
            .enumerate()
            .map(|(i, &p)| Piece {
                kind: Kind::Road,
                index: i,
                anchor: p,
                reach: REACH_ROAD,
            })
            .collect();
        let mut standing: Vec<Piece> = Vec::new();
        for (i, &p) in board.village.iter().enumerate() {
            standing.push(Piece {
                kind: Kind::Village,
                index: i,
                anchor: p,
                reach: REACH_EDGE,
            });
        }
        for (i, &p) in board.city.iter().enumerate() {
            standing.push(Piece {
                kind: Kind::City,
                index: i,
                anchor: p,
                reach: REACH_EDGE,
            });
        }
        for (i, p) in trader_layout::anchors(map).into_iter().enumerate() {
            // Trader tiers and their bank slots are 1 based, as in the playsheet
            standing.push(Piece {
                kind: Kind::Knight,
                index: i + 1,
                anchor: p,
                reach: REACH_TRADER,
            });
        }
        // Painter's algorithm, and a stable sort, so equal rows keep the order above
        standing.sort_by_key(Piece::foot_y);
        let shared_at = (0..map.bank_resource.len())
            .map(|slot| {
                if slot == 0 {
                    None
                } else {
                    trader_layout::shared_at(map, slot)
                }
            })
            .collect();
        BoardPieces {
            map,
            roads,
            standing,
            shared_at,
        }
    }

    /// Every piece, roads first.
    pub fn all(&self) -> impl Iterator<Item = &Piece> {
        self.roads.iter().chain(&self.standing)
    }

    /// The shared resources still resting on the board: (bank slot, top left of its 12 px
    /// cell). Each rests between its carts from the start until the last of them is hired;
    /// after that it is in the tray or spent.
    pub fn resting_shared(&self, game: &Game) -> Vec<(usize, Pt)> {
        (1..self.shared_at.len())
            .filter_map(|slot| {
                self.shared_at[slot]
                    .filter(|_| !game.slot_complete(slot))
                    .map(|at| (slot, at))
            })
            .collect()
    }

    /// Whether a point of the board lies on a resting shared resource (a tap there builds nothing).
    pub fn on_shared_resource(&self, game: &Game, x: i32, y: i32) -> bool {
        self.resting_shared(game).iter().any(|(_, at)| {
            x >= at.x - 1
                && x < at.x + art::SHARED_CELL + 1
                && y >= at.y - 1
                && y < at.y + art::SHARED_CELL + 1
        })
    }

    /// The piece a tap on the island means, among the positions on offer only: a position
    /// that cannot be built has nothing to do with a tap, so it never takes one away from a
    /// buildable neighbour. A tap on a building or a trader itself wins first, since a house
    /// stands over the ends of its roads. Then the nearest road, measured to the line between
    /// its two vertices so a tap near either end still counts. Then the nearest spot within
    /// the wider reach of its anchor.
    pub fn piece_at(&self, game: &Game, x: i32, y: i32) -> Option<Piece> {
        // The resource two traders share lies between their carts, close to both: a tap on it
        // is on neither, and does not hire whichever happens to be nearer
        if self.on_shared_resource(game, x, y) {
            return None;
        }
        let offered = |p: &Piece| game.offer(p.kind, p.index).is_some();
        let mut best: Option<Piece> = None;
        let mut best_distance = i32::MAX;
        for e in &self.standing {
            let dx = x - e.anchor.x;
            let dy = y - (e.anchor.y - e.body_rise());
            let d = dx * dx + dy * dy;
            let body = e.body_radius();
            if d <= body * body && d < best_distance && offered(e) {
                best = Some(*e);
                best_distance = d;
            }
        }
        if best.is_some() {
            return best;
        }
        let mut best_road = (REACH_ROAD * REACH_ROAD) as f32;
        for e in &self.roads {
            let d = game.board.road_distance_squared(e.index, x, y);
            if d <= best_road && offered(e) {
                best = Some(*e);
                best_road = d;
            }
        }
        if best.is_some() {
            return best;
        }
        for e in &self.standing {
            if let Some(d) = e.distance_squared(x, y)
                && d < best_distance
                && offered(e)
            {
                best = Some(*e);
                best_distance = d;
            }
        }
        best
    }

    /// One spot for the tutorial's Build callout to point at: a building that could be built
    /// right now, else a road that could, else anything on offer (it needs another build
    /// first). The same spot every frame for the same game, so the callout does not hop.
    pub fn offered_spot(&self, game: &Game) -> Option<Piece> {
        let rank = |p: &Piece, kind_rank: i32| match game.offer(p.kind, p.index) {
            None => 0,
            Some(offer) => (if offer.now { 10 } else { 0 }) + kind_rank,
        };
        let mut best = None;
        let mut best_rank = 0;
        for e in &self.standing {
            let r = rank(e, 3);
            if r > best_rank {
                best = Some(*e);
                best_rank = r;
            }
        }
        for e in &self.roads {
            let r = rank(e, 2);
            if r > best_rank {
                best = Some(*e);
                best_rank = r;
            }
        }
        best
    }
}
