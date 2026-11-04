// SPDX-License-Identifier: GPL-3.0-or-later
//! The rules for playing a sound, apart from any audio API so they can be tested: delayed
//! requests, the machine-gun guard and the retrigger of Sfx::Banked. The audio itself sits
//! behind `SoundBackend` and the clock behind `SoundClock`.
//!
//! Guard. The same sound requested twice within GUARD_MS plays once: a double delivery of
//! one tap, or two builds landing in the same frame, is one sound.
//!
//! Retrigger. For a sound with a retrigger window, a request inside that window of the last
//! play first stops the stream that is still sounding. Several banked resources spent in one
//! build therefore restart the sample each time, stuttering its opening, rather than
//! stacking. The stream id kept for this is only the last one.
//!
//! Delayed requests wait in a queue inside this struct rather than on a platform timer, so
//! the UI calls `poll` from a frame or timer callback (`next_due` says when one is wanted).

use super::sfx::Sfx;
use super::sound_sink::SoundSink;

/// What actually makes the noise.
pub trait SoundBackend {
    /// Starts a sound and returns its stream id, or 0 when nothing played (not loaded yet).
    fn play(&mut self, sfx: Sfx) -> u32;

    fn stop(&mut self, sfx: Sfx, stream_id: u32);
}

/// Milliseconds on a monotonic clock.
pub trait SoundClock {
    fn now(&self) -> u64;
}

impl<F: Fn() -> u64> SoundClock for F {
    fn now(&self) -> u64 {
        self()
    }
}

pub const GUARD_MS: u64 = 40;

pub struct SoundPlayback<B: SoundBackend, C: SoundClock> {
    pub backend: B,
    pub clock: C,
    last_played: [Option<u64>; Sfx::ALL.len()],
    last_stream: [u32; Sfx::ALL.len()],
    /// Waiting requests: when each is due, and in what order they were asked for.
    pending: Vec<(u64, u64, Sfx)>,
    sequence: u64,
    closed: bool,
}

impl<B: SoundBackend, C: SoundClock> SoundPlayback<B, C> {
    pub fn new(backend: B, clock: C) -> Self {
        SoundPlayback {
            backend,
            clock,
            last_played: [None; Sfx::ALL.len()],
            last_stream: [0; Sfx::ALL.len()],
            pending: Vec::new(),
            sequence: 0,
            closed: false,
        }
    }

    /// When the next waiting request is due, on the clock, or None if nothing waits.
    pub fn next_due(&self) -> Option<u64> {
        self.pending.iter().map(|p| p.0).min()
    }

    /// Plays every waiting request that has come due, earliest first (and in the order asked
    /// among those due at once).
    pub fn poll(&mut self) {
        loop {
            let now = self.clock.now();
            let next = self
                .pending
                .iter()
                .enumerate()
                .filter(|(_, p)| p.0 <= now)
                .min_by_key(|(_, p)| (p.0, p.1))
                .map(|(i, _)| i);
            let Some(i) = next else {
                break;
            };
            let (_, _, sfx) = self.pending.remove(i);
            self.play_now(sfx);
        }
    }

    /// After this nothing plays, including requests already waiting on their delay.
    pub fn close(&mut self) {
        self.closed = true;
        self.pending.clear();
    }

    fn play_now(&mut self, sfx: Sfx) {
        if self.closed {
            return;
        }
        let i = sfx.ordinal();
        let now = self.clock.now();
        let since = self.last_played[i].map(|t| now.saturating_sub(t));
        if since.is_some_and(|s| s < GUARD_MS) {
            return;
        }
        if sfx.retrigger_ms() > 0
            && self.last_stream[i] != 0
            && since.is_some_and(|s| s < sfx.retrigger_ms())
        {
            self.backend.stop(sfx, self.last_stream[i]);
            self.last_stream[i] = 0;
        }
        let stream = self.backend.play(sfx);
        if stream == 0 {
            // Dropped (still loading): it does not count as having played
            return;
        }
        self.last_played[i] = Some(now);
        self.last_stream[i] = stream;
    }
}

impl<B: SoundBackend, C: SoundClock> SoundSink for SoundPlayback<B, C> {
    fn play(&mut self, sfx: Sfx, delay_ms: i32) {
        if self.closed {
            return;
        }
        if delay_ms <= 0 {
            self.play_now(sfx);
        } else {
            let due = self.clock.now() + delay_ms as u64;
            self.sequence += 1;
            self.pending.push((due, self.sequence, sfx));
        }
    }
}
