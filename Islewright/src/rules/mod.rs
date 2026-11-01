// SPDX-License-Identifier: GPL-3.0-or-later
//! The game's rules, with no UI in them: the islands and their geometry, the dice, the sheet
//! and the build plan, a game and its turns, the saved game and the scores document, and the
//! pure arithmetic the screens are laid out and animated by. Ported from the Android app's
//! Java classes of the same names, behaviour for behaviour.
//!
//! Start at `Game`: the UI queries it for what to draw and calls its actions. `view_model`
//! adds the decisions the old game view made on top (offers, glows, prompts, the roll
//! button), and `pieces` turns a tap on the board into the spot it means.

pub mod art;
pub mod board;
pub mod board_layers;
pub mod build_plan;
pub mod dice;
pub mod game;
pub mod game_state_store;
pub mod glow_fade;
pub mod high_scores;
pub mod hire_anim;
pub mod json;
pub mod map_def;
pub mod music_gain;
pub mod music_playlist;
pub mod palette;
pub mod pieces;
pub mod player_color;
pub mod playsheet;
pub mod pt;
pub mod rng;
pub mod rules_markup;
pub mod scenario;
pub mod sfx;
pub mod sound_cues;
pub mod sound_playback;
pub mod sound_sink;
pub mod tap_slop;
pub mod team_color;
pub mod trader_layout;
pub mod tutorial;
pub mod tutorial_layout;
pub mod view_model;
pub mod voyage;

pub use board::Board;
pub use build_plan::{BuildPlan, Chain, Kind, Step};
pub use dice::{Dice, DiceValue};
pub use game::{BuildStatus, Game, HireEvent, Spent};
pub use high_scores::{HighScoresAndSettings, ScoreAndDate, Theme};
pub use map_def::MapDef;
pub use pieces::{BoardPieces, Piece};
pub use playsheet::Playsheet;
pub use pt::Pt;
pub use rng::Rng;
pub use sfx::Sfx;
pub use sound_sink::SoundSink;
pub use view_model::{BuildPrompt, GlowTier, Offer, OfferPay, RollLabel, RollOutcome, RollPress};
