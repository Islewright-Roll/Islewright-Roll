// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the game asks for a sound. The rules know only this trait, so they can be tested
//! with a fake; the UI implements it over the platform's audio (usually by wrapping a
//! `SoundPlayback` around its own backend).
//!
//! There is also one sink per thread for everything that has no game to hand, such as a
//! button click: the UI installs its sink with `set_sink`, and until it does (a unit test,
//! or no audio at all) the sounds go nowhere.

use std::cell::RefCell;

use super::sfx::Sfx;

pub trait SoundSink {
    /// Plays the sound after the delay (0 for now). Nothing happens if sound is off or not
    /// loaded yet.
    fn play(&mut self, sfx: Sfx, delay_ms: i32);
}

/// The silent sink.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSound;

impl SoundSink for NoSound {
    fn play(&mut self, _sfx: Sfx, _delay_ms: i32) {}
}

thread_local! {
    static SINK: RefCell<Option<Box<dyn SoundSink>>> = const { RefCell::new(None) };
}

/// Installs the sink that sounds without a game of their own go to, and that a game with no
/// sink of its own uses. None goes back to silence.
pub fn set_sink(sink: Option<Box<dyn SoundSink>>) {
    SINK.with(|s| *s.borrow_mut() = sink);
}

/// Plays a sound on the installed sink now.
pub fn play(sfx: Sfx) {
    play_after(sfx, 0);
}

/// Plays a sound on the installed sink after a delay. A sink that asks for another sound
/// from inside its own play is not reentered; that request is dropped.
pub fn play_after(sfx: Sfx, delay_ms: i32) {
    SINK.with(|s| {
        if let Ok(mut sink) = s.try_borrow_mut()
            && let Some(sink) = sink.as_mut()
        {
            sink.play(sfx, delay_ms);
        }
    });
}
