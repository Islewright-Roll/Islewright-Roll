// SPDX-License-Identifier: GPL-3.0-or-later
//! Which way a phone or tablet was turned, from the accelerometer, so the dice can stay on the
//! side that was the bottom of the screen: on the right after a counterclockwise turn, on the
//! left after a clockwise one.
//!
//! Day reports which way the window is shaped, not which way the device was turned, and
//! both landscape turns give the same shape. Gravity tells them apart. The accelerometer needs
//! no permission on Android or iOS, and no reading is kept: each one is reduced to a side at
//! once.

use day::prelude::*;

use crate::view::DiceSide;

/// How far gravity must lean along the device's x axis, in m/s², before a reading counts as a
/// landscape turn. A phone held upright reads about 9.8 along y and little along x.
const TURNED: f64 = 6.0;

/// The side a reading puts the dice on, or `None` when the device is upright, flat or
/// between turns, where the last side should stand.
///
/// `x` and `y` are the accelerometer's device axes as the platform reports them. Android and
/// the web report the force that holds the device up, which points away from the ground: held
/// upright, y reads about +9.8. iOS reports gravity itself, the opposite sign, which `ios`
/// corrects.
///
/// Turned counterclockwise, the top edge goes to the left and the right edge (+x) points up,
/// so x reads about +9.8: the old bottom is now on the right, and so are the dice. Turned
/// clockwise, the left edge points up and x reads about -9.8: the dice go left.
pub fn side_for(x: f64, y: f64, ios: bool) -> Option<DiceSide> {
    let up = if ios { -x } else { x };
    if up.abs() < TURNED || up.abs() < y.abs() * 1.5 {
        return None;
    }
    Some(if up > 0.0 {
        DiceSide::Right
    } else {
        DiceSide::Left
    })
}

/// A signal of the dice side, driven by the accelerometer where there is one, and Right
/// everywhere else (desktops, the web on a computer). The sensor runs while the returned watch
/// lives; tie it to the page's scope.
pub fn dice_side() -> Signal<DiceSide> {
    let side = Signal::new(DiceSide::Right);
    let sensor = day_part_sensors::SensorKind::Accelerometer;
    if day_part_sensors::is_available(sensor) {
        let setter = side.setter();
        let ios = cfg!(target_os = "ios");
        let watch = day_part_sensors::watch(sensor, move |r| {
            if let Some(s) = side_for(r.x, r.y, ios) {
                setter.set(s);
            }
        });
        Scope::current().on_cleanup(move || drop(watch));
    }
    side
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upright_and_flat_keep_the_last_side() {
        assert_eq!(side_for(0.3, 9.7, false), None);
        assert_eq!(side_for(0.1, 0.2, false), None);
        // Halfway through a turn, neither axis wins clearly.
        assert_eq!(side_for(6.5, 6.0, false), None);
    }

    #[test]
    fn a_counterclockwise_turn_puts_the_dice_on_the_right() {
        assert_eq!(side_for(9.6, 0.5, false), Some(DiceSide::Right));
        // iOS reports gravity, so the same turn reads -1 g along x.
        assert_eq!(side_for(-9.6, 0.5, true), Some(DiceSide::Right));
    }

    #[test]
    fn a_clockwise_turn_puts_the_dice_on_the_left() {
        assert_eq!(side_for(-9.6, -0.4, false), Some(DiceSide::Left));
        assert_eq!(side_for(9.6, -0.4, true), Some(DiceSide::Left));
    }
}
