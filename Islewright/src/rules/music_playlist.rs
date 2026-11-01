// SPDX-License-Identifier: GPL-3.0-or-later
//! The order the tracks play in: every track once in a shuffled order, then a new shuffle,
//! forever. A fresh shuffle never starts with the track that just ended, so there is no
//! immediate repeat across the loop boundary (unless there is only one track, when a repeat
//! is all there is).

use super::rng::Rng;

#[derive(Clone, Debug)]
pub struct MusicPlaylist {
    count: usize,
    rng: Rng,
    order: Vec<usize>,
    position: usize,
    last: Option<usize>,
}

impl MusicPlaylist {
    /// A playlist over the indexes 0 to count - 1.
    pub fn new(count: usize, rng: Rng) -> MusicPlaylist {
        MusicPlaylist {
            count,
            rng,
            order: Vec::new(),
            position: 0,
            last: None,
        }
    }

    pub fn size(&self) -> usize {
        self.count
    }

    /// The index of the next track to play, or None when there are no tracks.
    pub fn next_track(&mut self) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        if self.position >= self.order.len() {
            self.reshuffle();
        }
        let track = self.order[self.position];
        self.position += 1;
        self.last = Some(track);
        Some(track)
    }

    fn reshuffle(&mut self) {
        self.order = (0..self.count).collect();
        for i in (1..self.count).rev() {
            let j = self.rng.below(i as u32 + 1) as usize;
            self.order.swap(i, j);
        }
        if self.count > 1 && Some(self.order[0]) == self.last {
            // Swap the track that just played away from the front, with a random later one
            let j = 1 + self.rng.below(self.count as u32 - 1) as usize;
            self.order.swap(0, j);
        }
        self.position = 0;
    }
}
