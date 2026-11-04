// SPDX-License-Identifier: GPL-3.0-or-later
//! A breadth-first search over copies of the live dice and playsheet, answering "what can
//! this position be built by, and in how many steps": a chain of one step is affordable
//! right now (the green highlight); a chain of two or more steps needs an earlier build
//! first (the lighter tier). Each step runs the real `use_resources` and the real
//! `Playsheet::build_x` on a copy, so board position gates (a village needing a road beside
//! it, say) apply for free with no separate "position tier" logic.
//!
//! Never mutates the live game. The search runs entirely on copies.

use std::collections::{HashMap, HashSet, VecDeque};

use super::dice::Dice;
use super::game::{self, BuildStatus};
use super::playsheet::Playsheet;

/// The four things that can be built. A knight is a trader; its index is its 1 based tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Road,
    Village,
    City,
    Knight,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Road, Kind::Village, Kind::City, Kind::Knight];

    pub fn ordinal(self) -> usize {
        self as usize
    }

    /// What building one of these costs.
    pub fn recipe(self) -> &'static [super::dice::DiceValue] {
        match self {
            Kind::Road => game::RESOURCES_ROAD,
            Kind::Village => game::RESOURCES_VILLAGE,
            Kind::City => game::RESOURCES_CITY,
            Kind::Knight => game::RESOURCES_KNIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Step {
    pub kind: Kind,
    pub index: usize,
}

impl Step {
    pub const fn new(kind: Kind, index: usize) -> Step {
        Step { kind, index }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    pub steps: Vec<Step>,
    pub spends_bank: bool,
    /// Whether the chain spends a bank slot holding the "?" that stands for any resource.
    /// It is the scarcest thing in the bank, so spending it costs more than spending an
    /// exact match and the board says so.
    pub spends_wildcard: bool,
    /// How many rolled dice the whole chain consumes. Counted by diffing the usable dice
    /// before and after, so resources paid from the bank correctly do not add to it: those
    /// are not dice.
    pub dice_used: usize,
}

impl Chain {
    pub fn new(
        steps: Vec<Step>,
        spends_bank: bool,
        spends_wildcard: bool,
        dice_used: usize,
    ) -> Chain {
        Chain {
            steps,
            spends_bank,
            spends_wildcard,
            dice_used,
        }
    }

    /// The trader tiers the chain hires, in order.
    pub fn trader_tiers(&self) -> Vec<usize> {
        self.steps
            .iter()
            .filter(|s| s.kind == Kind::Knight)
            .map(|s| s.index)
            .collect()
    }
}

/// Defensive only: a bug in the search can never hang the UI thread. On the cap, positions
/// past it simply stay unhighlighted, which fails safe.
pub const MAX_STATES: usize = 512;

#[derive(Clone, Debug, Default)]
pub struct BuildPlan {
    chains: HashMap<usize, Chain>,
}

/// kind * 32 + index; 32 is above the largest index (MapDef allows at most 32 roads).
fn key(kind: Kind, index: usize) -> usize {
    kind.ordinal() * 32 + index
}

struct Node {
    dice: Vec<Dice>,
    playsheet: Playsheet,
    chain: Vec<Step>,
    spends_bank: bool,
    spends_wildcard: bool,
}

impl BuildPlan {
    pub fn chain_for(&self, kind: Kind, index: usize) -> Option<&Chain> {
        self.chains.get(&key(kind, index))
    }

    /// True when no position can be built this turn, now or after another build.
    pub fn is_empty(&self) -> bool {
        self.chains.is_empty()
    }

    /// True when some position is buildable right now without dipping into the bank.
    pub fn has_dice_only_build(&self) -> bool {
        self.chains
            .values()
            .any(|c| c.steps.len() == 1 && !c.spends_bank)
    }

    /// The most dice the turn can still spend if the player builds this position now,
    /// counting the chain that reaches it plus everything reachable afterwards.
    ///
    /// This is the question that matters for the early-build prompt. A village costing four
    /// dice looks modest alone, but if the two it leaves behind are exactly a road, the
    /// player can spend all six and plainly does not want another roll. Asking there is the
    /// prompt being wrong, not the player being careless.
    ///
    /// It cannot be read off the recorded chains: the search stores only the shortest chain
    /// to each position, so "village, then road" is never recorded when the road is already
    /// reachable on its own. This walks forward from the state the build leaves behind. It
    /// runs on a tap, not on every draw, so the extra search is affordable.
    pub fn max_dice_spendable_through(
        live_dice: &[Dice],
        live_playsheet: &Playsheet,
        chain: &Chain,
    ) -> usize {
        let mut dice = live_dice.to_vec();
        let mut playsheet = live_playsheet.clone();
        let root_usable = usable_count(&dice);
        for step in &chain.steps {
            game::use_resources(
                step.kind.recipe(),
                &mut dice,
                &mut playsheet,
                &mut Vec::new(),
            );
            apply_build(step.kind, step.index, &mut playsheet);
        }
        (root_usable - usable_count(&dice)) + spendable_from(&dice, &playsheet, &mut HashSet::new())
    }

    pub fn compute(live_dice: &[Dice], live_playsheet: &Playsheet, rolls: i32) -> BuildPlan {
        let mut plan = BuildPlan::default();
        if rolls == 0 {
            return plan;
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<Node> = VecDeque::new();
        let root_dice = live_dice.to_vec();
        let root_playsheet = live_playsheet.clone();
        let root_usable = usable_count(&root_dice);
        visited.insert(fingerprint(&root_dice, &root_playsheet));
        queue.push_back(Node {
            dice: root_dice,
            playsheet: root_playsheet,
            chain: Vec::new(),
            spends_bank: false,
            spends_wildcard: false,
        });

        let map = live_playsheet.map;
        while let Some(node) = queue.pop_front() {
            // Road 0 is the start road, never built; knight tiers are 1 based
            for i in 1..map.roads.len() {
                plan.try_step(Kind::Road, i, &node, &mut visited, &mut queue, root_usable);
            }
            for i in 0..map.villages.len() {
                plan.try_step(
                    Kind::Village,
                    i,
                    &node,
                    &mut visited,
                    &mut queue,
                    root_usable,
                );
            }
            for i in 0..map.cities.len() {
                plan.try_step(Kind::City, i, &node, &mut visited, &mut queue, root_usable);
            }
            for i in 1..=map.traders.len() {
                plan.try_step(
                    Kind::Knight,
                    i,
                    &node,
                    &mut visited,
                    &mut queue,
                    root_usable,
                );
            }
        }
        plan
    }

    fn try_step(
        &mut self,
        kind: Kind,
        index: usize,
        node: &Node,
        visited: &mut HashSet<String>,
        queue: &mut VecDeque<Node>,
        root_usable: usize,
    ) {
        if !is_legal(kind, index, &node.dice, &node.playsheet) {
            return;
        }
        let mut child_dice = node.dice.clone();
        let mut child_playsheet = node.playsheet.clone();
        game::use_resources(
            kind.recipe(),
            &mut child_dice,
            &mut child_playsheet,
            &mut Vec::new(),
        );
        let spends_bank_step = spends_bank(&node.playsheet, &child_playsheet);
        let spends_wildcard_step = spends_wildcard(&node.playsheet, &child_playsheet);
        apply_build(kind, index, &mut child_playsheet);

        let chain_spends_bank = node.spends_bank || spends_bank_step;
        let chain_spends_wildcard = node.spends_wildcard || spends_wildcard_step;
        let mut steps = node.chain.clone();
        steps.push(Step::new(kind, index));
        self.chains.entry(key(kind, index)).or_insert_with(|| {
            Chain::new(
                steps.clone(),
                chain_spends_bank,
                chain_spends_wildcard,
                root_usable - usable_count(&child_dice),
            )
        });

        if visited.len() >= MAX_STATES {
            return;
        }
        if !visited.insert(fingerprint(&child_dice, &child_playsheet)) {
            return;
        }
        queue.push_back(Node {
            dice: child_dice,
            playsheet: child_playsheet,
            chain: steps,
            spends_bank: chain_spends_bank,
            spends_wildcard: chain_spends_wildcard,
        });
    }
}

/// Depth first, returning the most additional dice any sequence of builds consumes.
fn spendable_from(dice: &[Dice], playsheet: &Playsheet, seen: &mut HashSet<String>) -> usize {
    if seen.len() >= MAX_STATES || !seen.insert(fingerprint(dice, playsheet)) {
        return 0;
    }
    let before = usable_count(dice);
    let mut best = 0;
    // One bound for every kind: an index a kind does not have is simply not legal
    let map = playsheet.map;
    let top = map
        .roads
        .len()
        .max(map.villages.len())
        .max(map.cities.len().max(map.traders.len()));
    for kind in Kind::ALL {
        for i in 0..=top {
            if !is_legal(kind, i, dice, playsheet) {
                continue;
            }
            let mut child_dice = dice.to_vec();
            let mut child_playsheet = playsheet.clone();
            game::use_resources(
                kind.recipe(),
                &mut child_dice,
                &mut child_playsheet,
                &mut Vec::new(),
            );
            apply_build(kind, i, &mut child_playsheet);
            let spent = (before - usable_count(&child_dice))
                + spendable_from(&child_dice, &child_playsheet, seen);
            best = best.max(spent);
        }
    }
    best
}

/// Dice still available to spend this turn.
fn usable_count(dice: &[Dice]) -> usize {
    dice.iter().filter(|d| d.is_usable()).count()
}

fn is_legal(kind: Kind, index: usize, dice: &[Dice], playsheet: &Playsheet) -> bool {
    let position_ok = match kind {
        Kind::Road => playsheet.can_build_road(index),
        Kind::Village => playsheet.can_build_village(index),
        Kind::City => playsheet.can_build_city(index),
        Kind::Knight => playsheet.can_build_knight(index),
    };
    position_ok
        && game::can_build_status(kind.recipe(), dice, playsheet, &mut Vec::new())
            != BuildStatus::None
}

/// Builds one position on a sheet (no paying), as the plan's steps do.
pub fn apply_build(kind: Kind, index: usize, playsheet: &mut Playsheet) {
    match kind {
        Kind::Road => playsheet.build_road(index),
        Kind::Village => playsheet.build_village(index),
        Kind::City => playsheet.build_city(index),
        Kind::Knight => playsheet.build_knight(index),
    }
}

/// True when building the step took at least one resource out of the bank.
fn spends_bank(before: &Playsheet, after: &Playsheet) -> bool {
    (1..=before.bank_size()).any(|i| before.resources_avail[i] && !after.resources_avail[i])
}

/// True when paying for the step took a "?" out of the bank. Only paying is compared (the
/// build itself is applied afterwards), and a slot can only be spent while it is usable, so
/// a slot that was available and no longer is was usable and got spent.
fn spends_wildcard(before: &Playsheet, after: &Playsheet) -> bool {
    (1..=before.bank_size()).any(|i| {
        before.map.bank_resource[i] == super::dice::DiceValue::Any
            && before.resources_avail[i]
            && !after.resources_avail[i]
    })
}

/// A textual key for a (dice, playsheet) state: each die's value ordinal and used flag, then
/// the bank's resources_avail, roads, villages, cities and knight tiers. Held is excluded
/// because it does not affect affordability. Field order and every length are fixed for a
/// map, so no separators are needed between fields.
pub fn fingerprint(dice: &[Dice], playsheet: &Playsheet) -> String {
    let mut sb = String::new();
    let bit = |b: bool| if b { '1' } else { '0' };
    for die in dice {
        sb.push(char::from(b'0' + die.value.ordinal() as u8));
        sb.push(bit(die.used));
    }
    for i in 1..=playsheet.bank_size() {
        sb.push(bit(playsheet.resources_avail[i]));
    }
    for &b in playsheet
        .roads
        .iter()
        .chain(&playsheet.villages)
        .chain(&playsheet.cities)
    {
        sb.push(bit(b));
    }
    for &b in playsheet.traders.iter().skip(1) {
        sb.push(bit(b));
    }
    sb
}
