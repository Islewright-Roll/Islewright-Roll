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

/** Where the game asks for a sound. The game logic knows only this, so it can be tested with a fake. */
interface SoundSink {
   /** Plays the sound after the delay (0 for now). Nothing happens if sound is off or not loaded yet. */
   void play(Sfx sfx, int delayMs);

   SoundSink NONE = new SoundSink() {
      @Override
      public void play(Sfx sfx, int delayMs) {
      }
   };
}
