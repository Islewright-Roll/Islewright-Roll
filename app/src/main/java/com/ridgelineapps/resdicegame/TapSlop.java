/*
 * Copyright (C) 2012 Resource Dice Game (http://code.google.com/p/android-dice-game)
 *
 * This program is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as published
 * by the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This source code is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
package com.ridgelineapps.resdicegame;

/**
 * Whether a touch was a tap or a swipe. A finger that lands on one spot and lifts on
 * another is crossing the screen, not choosing the second spot, so it must never build.
 */
final class TapSlop {
   /**
    * The system's touch slop is the travel Android itself forgives in a tap. The board's
    * targets are small, so a tap wobbles more than a list item's; twice the system figure
    * keeps those taps and still refuses a swipe.
    */
   static final int SLOP_FACTOR = 2;

   private TapSlop() {
   }

   /** True when the touch ended no further from where it began than slopPx (screen pixels). */
   static boolean isTap(float downX, float downY, float upX, float upY, float slopPx) {
      float dx = upX - downX;
      float dy = upY - downY;
      return dx * dx + dy * dy <= slopPx * slopPx;
   }
}
