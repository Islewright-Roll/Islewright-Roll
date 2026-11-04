// SPDX-License-Identifier: GPL-3.0-or-later
//! A small seedable random number generator (PCG32, O'Neill's XSH RR variant).
//!
//! The dice and the music shuffle take one of these rather than reaching for a global
//! source, so a test can seed it and get the same game every time, and no crate is needed.
//! It is not for anything that has to be unpredictable to an adversary.

/// PCG32: 64 bits of state, 32 bits out per step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
    inc: u64,
}

const MULTIPLIER: u64 = 6364136223846793005;

impl Rng {
    /// A generator whose whole sequence follows from the seed.
    pub fn new(seed: u64) -> Rng {
        // The stream is fixed and the seed goes through one step, as the reference seeding does
        let mut rng = Rng {
            state: 0,
            inc: (0xda3e_39cb_94b9_5bdb_u64 << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// A number from 0 to bound - 1, every one equally likely. 0 for a bound of 0.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        // Rejection of the top slice that would favour the low numbers
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }

    pub fn next_bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A number in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new(0x5EED)
    }
}
