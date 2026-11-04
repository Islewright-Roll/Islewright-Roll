// SPDX-License-Identifier: GPL-3.0-or-later
//! Generates the typed `res::` constants for `resource/`: https://daybrite.dev/docs/how-to/resources
fn main() {
    day_build::prebuild_project().expect("day-build: prebuild");
    day_build::bridge::generate().expect("day-build: bridge codegen");
}
