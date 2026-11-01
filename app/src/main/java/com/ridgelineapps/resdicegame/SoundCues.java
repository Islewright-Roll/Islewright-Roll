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

import java.util.ArrayList;
import java.util.List;

/**
 * Which sounds a game event asks for and when, with no Android in it so it can be tested.
 * Delays are milliseconds from the moment of the event.
 *
 * At the end of a game the order is: the sound of what ended it (the turn ending, or the
 * build that reached a race's target), then 300 ms later the fanfare: the bigger one if the
 * result is a new best for the island, else the short one. The King's ship docks a turn
 * earlier, at the turn end that starts the last turn, so its bell is not part of the ending.
 */
final class SoundCues {

   static final class Cue {
      final Sfx sfx;
      final int delayMs;

      Cue(Sfx sfx, int delayMs) {
         this.sfx = sfx;
         this.delayMs = delayMs;
      }

      @Override
      public String toString() {
         return sfx + "@" + delayMs;
      }
   }

   /** Between the steps of a chain build, so they are heard as a sequence and not a smear. */
   static final int CHAIN_STAGGER_MS = 90;
   /** Between banked resources spent in one build: each cuts the one before off at about this point. */
   static final int BANKED_GAP_MS = 110;
   /** The ship's horn, the dock bell and the build's own fanfare follow what caused them by this much. */
   static final int FOLLOW_MS = 300;
   /** A bonus chime follows the build that earned it. */
   static final int BONUS_DELAY_MS = 250;
   /** A roll that starts right after the turn ended (the button reading "1st Roll") waits for the turn end sound. */
   static final int ROLL_AFTER_TURN_END_MS = 250;

   private SoundCues() {
   }

   /**
    * A turn ending. emptyPenalised is a turn with nothing built on an island that charges
    * for it (it replaces the plain turn end sound); shipAppeared is the turn end that leaves
    * three turns; docked is the turn end that leaves one, the ship mooring for the last
    * turn; gameOver is the end by the turn limit (the ship docked already, so no bell).
    */
   static List<Cue> turnEnd(boolean emptyPenalised, boolean shipAppeared, boolean docked, boolean gameOver, boolean newBest) {
      List<Cue> cues = new ArrayList<Cue>();
      cues.add(new Cue(emptyPenalised ? Sfx.EMPTY_TURN : Sfx.TURN_END, 0));
      int at = FOLLOW_MS;
      if(shipAppeared && !gameOver) {
         cues.add(new Cue(Sfx.SHIP_APPEARS, at));
      }
      if(docked && !gameOver) {
         cues.add(new Cue(Sfx.SHIP_DOCKS, at));
      }
      if(gameOver) {
         cues.add(new Cue(newBest ? Sfx.TOP_SCORE : Sfx.GAME_OVER, at));
      }
      return cues;
   }

   /** What follows a build: a bonus chime, and the fanfare if the build ended a race. */
   static List<Cue> afterBuild(boolean bonusGained, boolean gameEnded, boolean newBest) {
      List<Cue> cues = new ArrayList<Cue>();
      int at = FOLLOW_MS;
      if(bonusGained) {
         cues.add(new Cue(Sfx.BONUS, BONUS_DELAY_MS));
         at = BONUS_DELAY_MS + FOLLOW_MS + 150;
      }
      if(gameEnded) {
         cues.add(new Cue(newBest ? Sfx.TOP_SCORE : Sfx.GAME_OVER, at));
      }
      return cues;
   }

   /** The build's own sound. */
   static Sfx forBuild(BuildPlan.Kind kind) {
      switch(kind) {
         case ROAD:
            return Sfx.ROAD;
         case VILLAGE:
            return Sfx.VILLAGE;
         case CITY:
            return Sfx.CASTLE;
         default:
            return Sfx.TRADER;
      }
   }

   /** True when a bonus bit is set after a build that was not set before (see Playsheet.bonusState). */
   static boolean bonusGained(int before, int after) {
      return (after & ~before) != 0;
   }
}
