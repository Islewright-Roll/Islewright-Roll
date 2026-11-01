// SPDX-License-Identifier: GPL-3.0-or-later
//! Islewright, a solitaire dice game of building on a small island, on Day.
//!
//! - `rules`: the game itself, with no UI.
//! - `view`: the game screen, drawn as pixel art.
//! - `app`: the app around it: settings, saved games, top scores, sound, the menu and pages.
//! - `ui`: the pixel-art widget kit the app's dialogs and pages are built from.

pub mod app;
pub mod art;
pub mod clock;
pub mod rules;
pub mod ui;
pub mod view;

// Typed constants for everything under resource/: https://daybrite.dev/docs/how-to/resources
day::resources!();
// The mobile and web entry point; desktop enters through src/main.rs.
day::day_start!(options: res::window_options(), root);

pub use app::root;
