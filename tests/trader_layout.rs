// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TraderLayoutTest: Dunholm's pairs stand one above the other with their shared
//! resource between them. The tests that check the sprites' pixels against the roads and
//! pieces are left out: they read the PNG art.

mod common;

use common::*;
use dayapp::rules::art;
use dayapp::rules::trader_layout;

#[test]
fn a_pair_stands_one_above_the_other_with_room_between_for_its_shared_resource() {
    let at = trader_layout::anchors(dunholm());
    for [a, b] in [[6, 7], [8, 9]] {
        let (upper, lower) = (at[a - 1], at[b - 1]);
        assert_eq!(upper.x, lower.x);
        assert_eq!(30, lower.y - upper.y);
        // the upper cart's 18 opaque rows, then 12 clear, then the lower cart's 18
        assert_eq!(
            12,
            (lower.y - art::TRADER_AY) - (upper.y - art::TRADER_AY + art::TRADER_BOX_H)
        );
    }
}

#[test]
fn only_a_slot_with_several_traders_has_a_shared_resource_and_it_is_between_them() {
    let d = dunholm();
    let at = trader_layout::anchors(d);
    for slot in 1..d.bank_resource.len() {
        let shared = trader_layout::shared_at(d, slot);
        if d.bank_traders[slot].len() < 2 {
            assert!(shared.is_none());
            continue;
        }
        let shared = shared.unwrap();
        let (a, b) = (
            at[d.bank_traders[slot][0] - 1],
            at[d.bank_traders[slot][1] - 1],
        );
        // centred on the carts, and between their opaque boxes, a row clear above and below
        assert_eq!(a.x, shared.x + art::SHARED_CELL / 2);
        let top = a.y - art::TRADER_AY + art::TRADER_BOX_H;
        let bottom = b.y - art::TRADER_AY;
        assert!(shared.y > top);
        assert!(shared.y + 10 < bottom);
    }
    for slot in 1..tarnholm().bank_resource.len() {
        assert!(trader_layout::shared_at(tarnholm(), slot).is_none());
    }
}

#[test]
fn tarnholm_traders_are_where_they_were() {
    let map = tarnholm();
    let at = trader_layout::anchors(map);
    for (i, t) in map.traders.iter().enumerate() {
        assert_eq!(map.hex[t.hex][0] + t.dx, at[i].x);
        assert_eq!(map.hex[t.hex][1] + t.dy, at[i].y);
    }
}
