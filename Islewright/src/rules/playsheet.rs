// SPDX-License-Identifier: GPL-3.0-or-later
//! What the player has built and banked, and the rules for building more. Every count,
//! prerequisite and score comes from the map (see MapDef); nothing here names an island.
//!
//! Road, village and city indices are 0 based. Trader ("knight") tiers and bank slots are
//! 1 based, and their vectors carry an unused entry at index 0.

use super::dice::DiceValue;
use super::map_def::MapDef;

#[derive(Clone, Debug)]
pub struct Playsheet {
    pub map: &'static MapDef,
    /// 1 based, by tier.
    pub traders: Vec<bool>,
    /// 1 based, by bank slot.
    pub resources_avail: Vec<bool>,
    pub roads: Vec<bool>,
    pub villages: Vec<bool>,
    pub cities: Vec<bool>,
    pub turns_nothing_built: i32,
    /// The running score at the end of each turn, entry 0 unused.
    pub turn_score: Vec<i32>,
    /// Whether a build may pay with a banked resource by itself. False in trader resource
    /// classic mode, where a banked resource is only ever spent by dragging it onto a die
    /// (Game::overwrite_die). A rule of play, not of the sheet, so it is not saved; it lives
    /// here so the build plan's copies of the sheet carry it.
    pub bank_pays_builds: bool,
}

fn any_built(needs: &[usize], roads: &[bool]) -> bool {
    needs.iter().any(|&n| roads[n])
}

impl Playsheet {
    pub fn new(map: &'static MapDef) -> Playsheet {
        let mut sheet = Playsheet {
            map,
            traders: Vec::new(),
            resources_avail: Vec::new(),
            roads: Vec::new(),
            villages: Vec::new(),
            cities: Vec::new(),
            turns_nothing_built: 0,
            turn_score: Vec::new(),
            bank_pays_builds: true,
        };
        sheet.reset();
        sheet
    }

    pub fn can_build_village(&self, i: usize) -> bool {
        if i >= self.villages.len() || self.villages[i] {
            return false;
        }
        if self.map.ordered_villages && i > 0 && !self.villages[i - 1] {
            return false;
        }
        any_built(&self.map.village_roads[i], &self.roads)
    }

    pub fn build_village(&mut self, i: usize) {
        if self.can_build_village(i) {
            self.villages[i] = true;
        }
    }

    pub fn can_build_city(&self, i: usize) -> bool {
        if i >= self.cities.len() || self.cities[i] {
            return false;
        }
        if self.map.ordered_cities && i > 0 && !self.cities[i - 1] {
            return false;
        }
        any_built(&self.map.city_roads[i], &self.roads)
    }

    pub fn build_city(&mut self, i: usize) {
        if self.can_build_city(i) {
            self.cities[i] = true;
        }
    }

    pub fn can_build_road(&self, i: usize) -> bool {
        if i >= self.roads.len() || self.roads[i] {
            return false;
        }
        // The start road is prebuilt, so this should never be hit, but just in case
        if i == 0 {
            return true;
        }
        any_built(&self.map.road_needs[i], &self.roads)
    }

    pub fn build_road(&mut self, i: usize) {
        if self.can_build_road(i) {
            self.roads[i] = true;
        }
    }

    pub fn can_build_knight(&self, i: usize) -> bool {
        if i < 1 || i >= self.traders.len() || self.traders[i] {
            return false;
        }
        !(self.map.ordered_traders && i > 1 && !self.traders[i - 1])
    }

    pub fn build_knight(&mut self, i: usize) {
        if self.can_build_knight(i) {
            self.traders[i] = true;
        }
    }

    /// Whether hiring this tier would start a bank slot's team of traders while another
    /// team is half hired: the slot behind this tier needs more than one trader, none of
    /// them stands yet, and some other multi trader slot has at least one but not all of
    /// its traders. The player is asked before that, because the "?" of the first team
    /// comes only once it is whole. A single trader, a team already begun, and a map with
    /// no teams (Tarnholm) never qualify. `hired` is Playsheet::traders (1 based, entry 0
    /// unused); nothing here changes it.
    pub fn starts_second_pair(map: &MapDef, hired: &[bool], tier: usize) -> bool {
        if tier < 1 || tier > map.trader_bank.len() || tier >= hired.len() || hired[tier] {
            return false;
        }
        let slot = map.trader_bank[tier - 1];
        if slot == 0 || map.bank_traders[slot].len() < 2 || team_hired(map, hired, slot) > 0 {
            return false;
        }
        (1..map.bank_traders.len()).any(|other| {
            let size = map.bank_traders[other].len();
            let count = if other == slot || size < 2 {
                0
            } else {
                team_hired(map, hired, other)
            };
            count > 0 && count < size
        })
    }

    /// As starts_second_pair for a whole chain of hires in the order they happen: true when
    /// any one of them would, counting the hires of the chain before it as done.
    pub fn chain_starts_second_pair(map: &MapDef, hired: &[bool], tiers: &[usize]) -> bool {
        let mut sim = hired.to_vec();
        for &tier in tiers {
            if Playsheet::starts_second_pair(map, &sim, tier) {
                return true;
            }
            if tier >= 1 && tier < sim.len() {
                sim[tier] = true;
            }
        }
        false
    }

    /// Whether the trader of this tier (1 based) stands.
    pub fn is_trader_built(&self, tier: usize) -> bool {
        tier >= 1 && tier < self.traders.len() && self.traders[tier]
    }

    pub fn trader_count(&self) -> usize {
        self.traders.iter().skip(1).filter(|&&b| b).count()
    }

    /// Builds tiers 1 to count and clears the rest: how a scenario that records traders as
    /// a count (written for a map with ordered traders) is applied.
    pub fn set_trader_count(&mut self, count: usize) {
        for tier in 1..self.traders.len() {
            self.traders[tier] = tier <= count;
        }
    }

    /// The number of bank slots; they are numbered from 1 to this.
    pub fn bank_size(&self) -> usize {
        self.resources_avail.len() - 1
    }

    /// Whether every trader behind this bank slot stands.
    fn bank_open(&self, slot: usize) -> bool {
        self.map.bank_traders[slot]
            .iter()
            .all(|&tier| self.traders[tier])
    }

    pub fn can_use_knight_resource(&self, i: usize) -> bool {
        i >= 1 && i <= self.bank_size() && self.bank_open(i) && self.resources_avail[i]
    }

    pub fn is_knight_resource_used(&self, i: usize) -> bool {
        i >= 1 && i <= self.bank_size() && self.bank_open(i) && !self.resources_avail[i]
    }

    /// Spends a bank slot if it can be used. The return value is None either way and
    /// always has been (the slot is spent before it is read back, and a spent slot reads
    /// as None); nothing uses it. Ask knight_resource first for what the slot holds.
    pub fn use_knight_resource(&mut self, i: usize) -> DiceValue {
        if self.can_use_knight_resource(i) {
            self.resources_avail[i] = false;
            return self.knight_resource(i);
        }
        DiceValue::None
    }

    /// What a usable bank slot holds, or None.
    pub fn knight_resource(&self, i: usize) -> DiceValue {
        if self.can_use_knight_resource(i) {
            self.map.bank_resource[i]
        } else {
            DiceValue::None
        }
    }

    /// The lowest bank slot holding this resource that can be used right now, or 0.
    pub fn first_available_slot(&self, v: DiceValue) -> usize {
        (1..=self.bank_size())
            .find(|&slot| self.map.bank_resource[slot] == v && self.can_use_knight_resource(slot))
            .unwrap_or(0)
    }

    /// Whether a slot that stands for any resource can be used right now.
    pub fn has_wildcard(&self) -> bool {
        self.first_available_slot(DiceValue::Any) != 0
    }

    /// Hands a resource the turn set aside but did not spend back to the bank: the lowest
    /// spent slot holding it becomes available again. Nothing happens if no slot holding it
    /// is spent.
    pub fn return_to_bank(&mut self, v: DiceValue) {
        if let Some(slot) = (1..=self.bank_size())
            .find(|&slot| self.map.bank_resource[slot] == v && !self.resources_avail[slot])
        {
            self.resources_avail[slot] = true;
        }
    }

    pub fn reset(&mut self) {
        let map = self.map;
        self.traders = vec![false; map.traders.len() + 1];
        self.resources_avail = vec![true; map.bank_resource.len()];
        self.resources_avail[0] = false;
        self.roads = vec![false; map.roads.len()];
        self.roads[0] = true;
        self.villages = vec![false; map.villages.len()];
        self.cities = vec![false; map.cities.len()];
        self.turns_nothing_built = 0;
        // A map with no turn cap grows the vector as turns are scored (see score_turn)
        self.turn_score = vec![0; map.turn_cap as usize + 1];
    }

    pub fn score_turn(&mut self, turn: i32) {
        if turn < 1 || (self.map.turn_cap > 0 && turn > self.map.turn_cap) {
            return;
        }
        let turn = turn as usize;
        if turn >= self.turn_score.len() {
            self.turn_score.resize(turn + 1, 0);
        }
        self.turn_score[turn] = self.score();
    }

    /// The points gained in one turn.
    pub fn turn_score_of(&self, turn: i32) -> i32 {
        if turn < 1 || turn as usize >= self.turn_score.len() {
            return 0;
        }
        self.turn_score[turn as usize] - self.turn_score[turn as usize - 1]
    }

    /// Which of the island's bonuses stand, one bit each: 1 the long road, 2 the trader
    /// bonus. The score is derived from the tables, so the sound for earning a bonus compares
    /// this before and after a build (see sound_cues::bonus_gained).
    pub fn bonus_state(&self) -> u32 {
        let mut state = 0;
        if self.map.long_road.is_some_and(|r| self.roads[r]) {
            state |= 1;
        }
        if self.map.trader_bonus_at > 0 && self.trader_count() >= self.map.trader_bonus_at {
            state |= 2;
        }
        state
    }

    /// The bonuses are worked out from what stands, never stored, so nothing that happens
    /// to the bank can take one away.
    pub fn score(&self) -> i32 {
        let map = self.map;
        let mut score = 0;
        score += self.roads.iter().skip(1).filter(|&&b| b).count() as i32 * map.road_points;
        for tier in 1..self.traders.len() {
            if self.traders[tier] {
                score += map.trader_points[tier - 1];
            }
        }
        for (i, &built) in self.villages.iter().enumerate() {
            if built {
                score += map.village_points[i];
            }
        }
        for (i, &built) in self.cities.iter().enumerate() {
            if built {
                score += map.city_points[i];
            }
        }
        if map.long_road.is_some_and(|r| self.roads[r]) {
            score += map.long_road_bonus;
        }
        if map.trader_bonus_at > 0 && self.trader_count() >= map.trader_bonus_at {
            score += map.trader_bonus;
        }
        score - self.turns_nothing_built * map.empty_turn_penalty
    }
}

/// How many of a bank slot's traders stand.
fn team_hired(map: &MapDef, hired: &[bool], slot: usize) -> usize {
    map.bank_traders[slot]
        .iter()
        .filter(|&&tier| hired[tier])
        .count()
}
