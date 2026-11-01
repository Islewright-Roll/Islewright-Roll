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

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.util.Log;

/**
 * Debug builds only: plays the sound effects on request, to audition them on a device or to
 * see their log lines on an emulator with no audio.
 *
 *   adb shell am broadcast -n io.github.islewright_roll.islewright/com.ridgelineapps.resdicegame.DebugSoundReceiver --es sfx ALL
 *   adb shell am broadcast -n io.github.islewright_roll.islewright/com.ridgelineapps.resdicegame.DebugSoundReceiver --es sfx BANKED
 *
 * ALL plays the sixteen moments in order with one second between them; a moment's name (the
 * Sfx constants: CLICK, DIE, ROLL ...) plays that one. They go through the same sink as the
 * game's own, so with the Sound effects setting Off, or the app in the background, nothing plays.
 */
public final class DebugSoundReceiver extends BroadcastReceiver {

   private static final String TAG = "DebugSound";

   @Override
   public void onReceive(Context context, Intent intent) {
      String which = intent.getStringExtra("sfx");
      if(which == null || "ALL".equalsIgnoreCase(which)) {
         int at = 0;
         for(Sfx sfx : Sfx.values()) {
            Sounds.sink.play(sfx, at);
            at += 1000;
         }
         Log.d(TAG, "playing all " + Sfx.values().length + " sounds, one second apart");
         return;
      }
      try {
         Sounds.play(Sfx.valueOf(which.toUpperCase()));
      }
      catch(IllegalArgumentException e) {
         Log.w(TAG, "no such sound: " + which);
      }
   }
}
