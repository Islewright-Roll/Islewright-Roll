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
 * The sixteen moments that have a sound, and a second take of one of them. Each is a res/raw file named sfx_<name> (see
 * SoundEffects, and tools/sfx-sources.txt for where the sounds come from).
 */
enum Sfx {
   CLICK("click"),
   DIE("die"),
   ROLL("roll"),
   ROAD("road"),
   VILLAGE("village"),
   CASTLE("castle"),
   TRADER("trader"),
   /** Spending a banked resource. Retriggers: each use cuts off the one still playing. */
   BANKED("banked", 260),
   GOLD("gold"),
   TURN_END("turn_end"),
   EMPTY_TURN("empty_turn"),
   SHIP_APPEARS("ship_appears"),
   SHIP_DOCKS("ship_docks"),
   GAME_OVER("game_over"),
   TOP_SCORE("top_score"),
   BONUS("bonus"),
   /** The second roll sound. Never asked for by the game: SoundEffects plays it for half the ROLLs. */
   ROLL_B("roll_b");

   /** The res/raw name is "sfx_" + this. */
   final String file;
   /**
    * Zero for a sound that plays out and may overlap itself. Above zero the sound retriggers:
    * a new request within this many milliseconds of the last one (about the sample's own
    * length) stops the stream still playing first, so a run of requests stutters the opening
    * of the sample instead of stacking copies.
    */
   final int retriggerMs;

   Sfx(String file) {
      this(file, 0);
   }

   Sfx(String file, int retriggerMs) {
      this.file = file;
      this.retriggerMs = retriggerMs;
   }
}
