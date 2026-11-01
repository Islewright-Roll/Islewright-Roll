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

import android.os.Build;
import android.util.Log;
import android.view.HapticFeedbackConstants;
import android.view.View;

/**
 * Every haptic in the game goes through here, so there are exactly two feels and one
 * switch. Both use View.performHapticFeedback with no flags: no vibrate permission, and
 * the device's own touch feedback setting still decides whether anything is felt.
 */
final class Haptics {
   private static final String TAG = "Haptics";

   /** The "Haptic feedback" setting. See HighScoresAndSettings. */
   static boolean enabled = true;

   private Haptics() {
   }

   /** A light tick: something was pressed. */
   static void tap(View view) {
      perform(view, HapticFeedbackConstants.CLOCK_TICK, "tap CLOCK_TICK");
   }

   /** A firmer, distinct pulse: something actually happened (a build, a roll). */
   static void confirm(View view) {
      if(Build.VERSION.SDK_INT >= 30) {
         perform(view, HapticFeedbackConstants.CONFIRM, "confirm CONFIRM");
      }
      else {
         perform(view, HapticFeedbackConstants.VIRTUAL_KEY, "confirm VIRTUAL_KEY");
      }
   }

   private static void perform(View view, int constant, String what) {
      if(!enabled || view == null) {
         return;
      }
      // Emulators do not vibrate; this line is how a haptic is seen in logcat
      Log.d(TAG, what);
      view.performHapticFeedback(constant);
   }
}
