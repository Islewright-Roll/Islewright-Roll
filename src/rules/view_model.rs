// SPDX-License-Identifier: GPL-3.0-or-later
//! The decisions the old game view made on top of the rules, kept with the rules so every UI
//! makes them the same way: which colour a spot is offered in, how a spot glows, which
//! questions to ask before a build, what the roll button says and what a press on it does,
//! what the tutorial sees, and which banked resources show on the tab.

use super::build_plan::Kind;
use super::dice::DiceValue;
use super::game::Game;
use super::glow_fade;
use super::high_scores::EARLY_PROMPT_OFF;
use super::playsheet::Playsheet;
use super::tutorial;

// ---------------------------------------------------------------- offers

/// Affordable right now with the dice on the table (ARGB).
pub const HIGHLIGHT: u32 = argb(100, 35, 180, 15);
/// Buildable, but paying for it spends a trader resource banked from an earlier turn instead
/// of dice in front of the player right now. A deep, saturated blue (rather than the sea's
/// lighter teal) so it reads as a highlight rather than blending into the board art.
pub const HIGHLIGHT_KNIGHT: u32 = argb(85, 40, 70, 230);
/// Buildable, but it spends a "?" rather than an exact match. The same blue, heavier,
/// because it is the more expensive way to pay.
pub const HIGHLIGHT_WILDCARD: u32 = argb(141, 40, 70, 230);

const fn argb(a: u32, r: u32, g: u32, b: u32) -> u32 {
    (a << 24) | (r << 16) | (g << 8) | b
}

/// The same offer when it is reachable only after building something else first: the same
/// colour at a lighter opacity, so "now" and "after another build" read as one idea at two
/// strengths. One rule for every offer colour.
pub const fn chained(argb: u32) -> u32 {
    (((argb >> 24) * 55 / 100) << 24) | (argb & 0xFFFFFF)
}

pub const HIGHLIGHT_CHAIN: u32 = chained(HIGHLIGHT);
pub const HIGHLIGHT_KNIGHT_CHAIN: u32 = chained(HIGHLIGHT_KNIGHT);
pub const HIGHLIGHT_WILDCARD_CHAIN: u32 = chained(HIGHLIGHT_WILDCARD);

/// What paying for an offered spot costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OfferPay {
    /// Dice alone.
    Dice,
    /// A banked resource from an earlier turn.
    Bank,
    /// A "?", the scarcest thing in the bank.
    Wildcard,
}

/// A spot that can be built this turn, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Offer {
    pub pay: OfferPay,
    /// True when it can be built right now; false when another build has to come first (a
    /// tap then builds the whole chain).
    pub now: bool,
}

impl Offer {
    /// The colour the board offers the spot in (ARGB).
    pub fn argb(&self) -> u32 {
        let base = match self.pay {
            OfferPay::Wildcard => HIGHLIGHT_WILDCARD,
            OfferPay::Bank => HIGHLIGHT_KNIGHT,
            OfferPay::Dice => HIGHLIGHT,
        };
        if self.now { base } else { chained(base) }
    }
}

// ---------------------------------------------------------------- the glow

/// Which glow a spot earns, from the map's own rules with the dice left out. Ordered from
/// weakest to strongest, so where two glows meet the stronger owns the shared outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GlowTier {
    None = 0,
    Faint = 1,
    Weak = 2,
    Waiting = 3,
    Full = 4,
}

/// The opacity of a glow's outline, of its ring, and of the halo that replaces the ring.
pub const GLOW_ALPHA: u32 = 166;
pub const GLOW_RING_ALPHA: u32 = 64;
pub const GLOW_HALO_ALPHA: u32 = 64;
/// A glow colour brighter than this gets a black halo, a darker one a white halo.
pub const GLOW_HALO_LIGHT_ABOVE: u32 = 110;
/// A waiting spot's glow is dashed; were it not, it would be drawn at this strength.
pub const WAITING_FADE: f32 = 0.5;
/// The outline of a road is the player's colour drained this much (board_layers::desaturate).
pub const GLOW_ROAD_SATURATION: f32 = 0.55;

/// The strength a glow tier is drawn at.
pub fn glow_strength(tier: GlowTier) -> f32 {
    match tier {
        GlowTier::None => 0.0,
        GlowTier::Faint => glow_fade::strength(Some(2)),
        GlowTier::Weak => glow_fade::strength(Some(1)),
        GlowTier::Waiting | GlowTier::Full => 1.0,
    }
}

/// Whether a pixel of a waiting glow is drawn: the dashes, two on and two off along the
/// diagonals.
pub fn waiting_dash_shows(x: i32, y: i32) -> bool {
    ((x + y) & 3) < 2
}

// ---------------------------------------------------------------- questions before a build

/// A question the player is asked before a tap builds. Declining builds nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildPrompt {
    /// Building now gives up the rolls not yet used. rolls_made is 1 (two rolls would be lost)
    /// or 2 (one roll), for the wording.
    EarlyBuild { rolls_made: i32 },
    /// The build starts a second team of traders while another team is half hired.
    SecondPair,
}

// ---------------------------------------------------------------- the roll button

/// What the roll button reads, which is also what it does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RollLabel {
    FirstRoll,
    SecondRoll,
    ThirdRoll,
    PlayAgain,
    /// Rolls are spent but something is still affordable, so ending the turn is a real choice.
    Done,
    /// Ending this turn ends the game: it is the map's last turn.
    Finish,
    /// Nothing left to build and nothing left to roll: the button ends the turn and rolls.
    NextTurn,
}

/// For this long after the game ends a tap on the roll button is swallowed, so a bump cannot
/// wipe the finished board by starting a new game.
pub const ROLL_LOCKOUT_MS: u64 = 500;

/// What a press on the roll button does (or, from roll_press, would do).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RollPress {
    /// Inside the lockout after the game ended: nothing.
    Locked,
    /// The game is over: a new one.
    NewGame,
    /// The turn would end with something still affordable: the UI asks first, and on a yes
    /// calls Game::roll (which ends the turn).
    ConfirmEndTurn,
    /// Nothing can be rolled or built: the turn ends and the next one's first roll is made.
    EndTurnAndRoll,
    /// A roll.
    Roll,
}

/// What pressing the roll button did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RollOutcome {
    pub press: RollPress,
    /// Whether dice were really thrown (for the firmer haptic): false when the turn that would
    /// have started is the end of the game.
    pub rolled: bool,
}

impl Game {
    /// The colour and cost a position is offered in, or None when it cannot be built this
    /// turn. One path for all four piece types, driven by the build plan.
    pub fn offer(&self, kind: Kind, index: usize) -> Option<Offer> {
        let plan = self.build_plan();
        let chain = plan.chain_for(kind, index)?;
        // What paying costs picks the colour; whether another build has to come first picks
        // its strength
        let pay = if chain.spends_wildcard {
            OfferPay::Wildcard
        } else if chain.spends_bank {
            OfferPay::Bank
        } else {
            OfferPay::Dice
        };
        Some(Offer {
            pay,
            now: chain.steps.len() == 1,
        })
    }

    /// Which glow a spot earns, from the map's own rules with the dice left out. Full: it
    /// could be built next if it could be paid for. Waiting: where the map orders its villages
    /// or castles, a site a built road has reached that is not next in its order. A site no
    /// road has reached, a road that touches nothing built, and anything built get none.
    /// Traders need no road: where the map orders them, the ones after the next fade
    /// (glow_fade): half the glow, then a fifth, then none. The UI draws no glow on a spot
    /// that is on offer (an offer has its own rim).
    pub fn glow_tier(&self, kind: Kind, index: usize) -> GlowTier {
        let sheet = &self.playsheet;
        let map = self.map;
        let reached = |needs: &[usize]| needs.iter().any(|&n| sheet.roads[n]);
        let full_or = |can: bool, other: GlowTier| if can { GlowTier::Full } else { other };
        match kind {
            Kind::Road => full_or(sheet.can_build_road(index), GlowTier::None),
            Kind::Village => full_or(
                sheet.can_build_village(index),
                if index < sheet.villages.len()
                    && !sheet.villages[index]
                    && map.ordered_villages
                    && reached(&map.village_roads[index])
                {
                    GlowTier::Waiting
                } else {
                    GlowTier::None
                },
            ),
            Kind::City => full_or(
                sheet.can_build_city(index),
                if index < sheet.cities.len()
                    && !sheet.cities[index]
                    && map.ordered_cities
                    && reached(&map.city_roads[index])
                {
                    GlowTier::Waiting
                } else {
                    GlowTier::None
                },
            ),
            Kind::Knight => {
                if sheet.can_build_knight(index) {
                    return GlowTier::Full;
                }
                if sheet.is_trader_built(index) || !map.ordered_traders {
                    return GlowTier::None;
                }
                match glow_fade::step(&sheet.traders, index) {
                    Some(0) => GlowTier::Full,
                    Some(1) => GlowTier::Weak,
                    Some(2) => GlowTier::Faint,
                    _ => GlowTier::None,
                }
            }
        }
    }

    /// The questions to ask, in order, before a tap on this position builds it; each must be
    /// answered yes. Empty: build straight away (with Game::build).
    ///
    /// Building ends the roll phase: can_roll goes false the moment anything is built. On the
    /// first or second roll that silently throws away rolls the player has not used, so the
    /// first question asks. It is skipped where the tap plainly means it: the setting is off,
    /// the build wins the race (the rolls it gives up are worth nothing), or the builds the
    /// tap opens up can spend at least the setting's number of dice, measured across the
    /// whole sequence and not just the tapped spot. There is no ceiling on that threshold: an
    /// unspent die can always be rerolled, a lone gold included, since another gold would pair
    /// it.
    ///
    /// The second question: a trader that shares its bank slot with another earns the slot's
    /// resource only when both stand, so hiring the first of one team while another team is
    /// half hired spreads the player thin. Measured over the whole chain the tap would run,
    /// so a chain that reaches such a trader at any step asks too; a build that wins the race
    /// does not ask. Not a setting.
    pub fn build_prompts(&self, kind: Kind, index: usize) -> Vec<BuildPrompt> {
        let plan = self.build_plan();
        let chain = plan.chain_for(kind, index);
        let wins = self.chain_reaches_target(chain);
        let mut prompts = Vec::new();

        if self.would_forfeit_rolls() && self.early_build_prompt != EARLY_PROMPT_OFF && !wins {
            let spends_enough = chain.is_some_and(|c| {
                super::build_plan::BuildPlan::max_dice_spendable_through(
                    &self.dice,
                    &self.playsheet,
                    c,
                ) as i32
                    >= self.early_build_prompt
            });
            if !spends_enough {
                prompts.push(BuildPrompt::EarlyBuild {
                    rolls_made: self.rolls,
                });
            }
        }

        if let Some(chain) = chain
            && !wins
            && Playsheet::chain_starts_second_pair(
                self.map,
                &self.playsheet.traders,
                &chain.trader_tiers(),
            )
        {
            prompts.push(BuildPrompt::SecondPair);
        }
        prompts
    }

    /// What the roll button reads now.
    pub fn roll_label(&self) -> RollLabel {
        let last = self.turns_taken + 1 == self.map.turn_cap;
        if self.can_roll() {
            // The turn is not named here: the Crown's ship in the top bar shows it
            match self.rolls {
                1 => RollLabel::SecondRoll,
                2 => RollLabel::ThirdRoll,
                _ => RollLabel::FirstRoll,
            }
        } else if self.is_game_done() {
            RollLabel::PlayAgain
        } else if self.should_confirm_end_turn() {
            if last {
                RollLabel::Finish
            } else {
                RollLabel::Done
            }
        } else if last {
            RollLabel::Finish
        } else {
            RollLabel::NextTurn
        }
    }

    /// True for ROLL_LOCKOUT_MS after the game ends, on the game's clock. A game with no clock
    /// has no lockout.
    pub fn is_roll_locked(&self) -> bool {
        let Some(clock) = self.clock.as_ref() else {
            return false;
        };
        self.is_game_done() && clock().saturating_sub(self.game_over_at) < ROLL_LOCKOUT_MS
    }

    /// What a press on the roll button would do now.
    pub fn roll_press(&self) -> RollPress {
        if self.is_roll_locked() {
            RollPress::Locked
        } else if self.is_game_done() {
            RollPress::NewGame
        } else if self.should_confirm_end_turn() {
            RollPress::ConfirmEndTurn
        } else if !self.can_roll() {
            RollPress::EndTurnAndRoll
        } else {
            RollPress::Roll
        }
    }

    /// A press (a lift inside it) on the roll button. Does what roll_press says, except
    /// ConfirmEndTurn, which only reports: the UI asks and calls roll on a yes.
    pub fn press_roll_button(&mut self) -> RollOutcome {
        let press = self.roll_press();
        match press {
            RollPress::Locked | RollPress::ConfirmEndTurn => {}
            RollPress::NewGame | RollPress::Roll => self.roll(),
            RollPress::EndTurnAndRoll => self.end_turn_and_roll(),
        }
        let rolled = matches!(press, RollPress::Roll | RollPress::EndTurnAndRoll)
            && self.rolls > 0
            && !self.is_game_done();
        RollOutcome { press, rolled }
    }

    /// Whether a die is drawn as staying: held, or locked in because the rolls are over.
    pub fn die_stays(&self, i: usize) -> bool {
        self.dice.get(i).is_some_and(|d| d.is_held()) || !self.can_roll()
    }

    /// The bank slots whose resource is banked and unused, in slot order: the mini dice on the
    /// tab, so a die never moves but to close up when one before it is spent. Of two traders
    /// that share a slot, the die appears when both stand.
    pub fn banked_slots(&self) -> Vec<usize> {
        (1..=self.playsheet.bank_size())
            .filter(|&slot| self.playsheet.can_use_knight_resource(slot))
            .collect()
    }

    /// What a bank slot holds (the icon of its mini die).
    pub fn bank_resource(&self, slot: usize) -> DiceValue {
        self.map
            .bank_resource
            .get(slot)
            .copied()
            .unwrap_or(DiceValue::None)
    }

    /// Whether every trader behind a bank slot stands.
    pub fn slot_complete(&self, slot: usize) -> bool {
        self.map.bank_traders[slot]
            .iter()
            .all(|&t| self.playsheet.is_trader_built(t))
    }

    /// What the tutorial needs to know about the game right now. flying is how many banked
    /// resources are still on their way to the tab in a hire animation: those are not banked
    /// yet.
    pub fn tutorial_snapshot(&self, flying: i32) -> tutorial::Snapshot {
        let game_over = self.is_game_done();
        tutorial::Snapshot {
            rolls: self.rolls,
            turns_taken: self.turns_taken,
            any_held: self.dice.iter().any(|d| d.is_held()),
            // A spot is on offer exactly when the plan has a chain for it
            offered: !self.build_plan().is_empty(),
            built_this_turn: self.built_resource_this_turn,
            can_undo: self.can_undo(),
            roll_ends_turn: !self.can_roll() && !game_over,
            gold_dice: self
                .dice
                .iter()
                .filter(|d| d.is_usable() && d.value == DiceValue::Gold)
                .count() as i32,
            mini_dice: self.banked_slots().len() as i32 - flying,
            classic_bank: self.trader_classic(),
            game_over,
        }
    }
}
