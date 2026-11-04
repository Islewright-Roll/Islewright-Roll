// SPDX-License-Identifier: GPL-3.0-or-later
//! Where each trader stands on the board: the map's anchor (the hex centre plus the map's own
//! offset), moved so that the traders of one hex are centred on that hex. A hex with one
//! trader already is (see art::TRADER_AX and TRADER_AY), so it is not moved. A hex with
//! several (Dunholm's desert pairs, one cart above the other with the room between them for
//! the one resource they share) is moved so that the combined opaque box of its sprites is
//! centred on the hex centre, both ways.

use super::art;
use super::map_def::MapDef;
use super::pt::Pt;

/// The anchors, in the map's trader order (tier - 1).
pub fn anchors(map: &MapDef) -> Vec<Pt> {
    map.traders
        .iter()
        .map(|t| {
            let [sx, sy] = pair_shift(map, t.hex);
            Pt::new(map.hex[t.hex][0] + t.dx + sx, map.hex[t.hex][1] + t.dy + sy)
        })
        .collect()
}

/// How far the traders of a hex move to centre them; none for a hex with one trader.
pub fn pair_shift(map: &MapDef, hex: usize) -> [i32; 2] {
    let mut count = 0;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for t in map.traders.iter().filter(|t| t.hex == hex) {
        count += 1;
        // The sprite's opaque box, relative to the hex centre
        let left = t.dx - art::TRADER_AX;
        let top = t.dy - art::TRADER_AY;
        min_x = min_x.min(left);
        max_x = max_x.max(left + art::TRADER_BOX_W);
        min_y = min_y.min(top);
        max_y = max_y.max(top + art::TRADER_BOX_H);
    }
    if count < 2 {
        return [0, 0];
    }
    let centre = |lo: i32, hi: i32| (-(lo + hi) as f64 / 2.0 + 0.5).floor() as i32;
    [centre(min_x, max_x), centre(min_y, max_y)]
}

/// Where the one resource of a bank slot that more than one trader stands behind rests, as
/// the top left of its 12 px cell in board pixels: centred between the traders' carts, and as
/// far down as the middle of the carts' height less half the cell (see
/// art::TRADER_SHARED_DROP), so that it lies on the ground between them. None for a slot with
/// a single trader, whose resource rides on its own cart.
pub fn shared_at(map: &MapDef, slot: usize) -> Option<Pt> {
    let tiers = &map.bank_traders[slot];
    if tiers.len() < 2 {
        return None;
    }
    let at = anchors(map);
    let n = tiers.len() as i32;
    let sum_x: i32 = tiers.iter().map(|&t| at[t - 1].x).sum();
    let sum_top: i32 = tiers.iter().map(|&t| at[t - 1].y - art::TRADER_AY).sum();
    Some(Pt::new(
        sum_x.div_euclid(n) - art::SHARED_CELL / 2,
        sum_top.div_euclid(n) + art::TRADER_SHARED_DROP,
    ))
}
