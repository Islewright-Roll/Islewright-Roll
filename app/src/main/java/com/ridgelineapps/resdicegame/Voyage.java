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
 * How the game shows the turn limit: the King's ship sails toward a pier. The pure
 * numbers of that, apart from any drawing, so they can be tested: which turn the ship has
 * reached, where it is on the board, where the dock is, and how the HUD strip lays out
 * its cells. GameView draws from these.
 *
 * The ship's position is a "cell": 0 is the first turn, turnCap is the pier. It moves one
 * cell on from the turn in progress as each turn is taken. It is on the board for the last
 * three turns, sailing in on the first and moving up on the second, and is moored at the
 * dock for the whole last turn; the game ending by the turn limit puts a flag up on the dock.
 *
 * A map with no turn cap (Dunholm, which ends only when the target is reached) has no
 * voyage on the board at all. Its HUD strip is a window onto the latest turns instead,
 * see FLAG_WINDOW.
 */
final class Voyage {
   // The ship is on the island from this many turns before the end, the one being played included
   static final int APPROACH_TURNS = 3;
   // It sails this many times on the board: from the far stop to the middle one, and on to the dock
   static final int SAIL_STEPS = 2;

   // Where the ship is on the board. Moored is the last turn, played with the ship at the
   // dock; done is the game over by the turn limit: moored, and the flag is up.
   static final int STAGE_NONE = 0;
   static final int STAGE_FAR = 1;
   static final int STAGE_MID = 2;
   static final int STAGE_MOORED = 3;
   static final int STAGE_DONE = 4;

   /** The ship sails in from off the screen, and between its stops, in this many milliseconds. */
   static final int SAIL_IN_MS = 1100;
   static final int SAIL_STEP_MS = 650;

   /**
    * At the far stop this much of the ship's sprite, half of it, lies beyond the left edge
    * of what the screen shows of the board: the ship comes in bow first, the way it heads.
    */
   static final int FAR_OUT_X = 30;
   /** The ship sails at least this far between one stop and the next. */
   static final int MIN_STEP = 20;
   /** However much of the board shows, a stop is never further than this from the next, each way. */
   static final int MAX_STEP = 24;
   /**
    * The dock is part of the baked board (tools/bake_board.py, the same on every map): a
    * jetty running up the screen from the vertex where the start road meets the sea. These
    * are that vertex, the point the ship's bowsprit comes to (two columns onto the deck's west
    * edge), and the top left of dock.png (the dock's above water art alone, which the game
    * draws over the roads, so the deck lies over the start road's round end), in board pixels;
    * they are dock_foot, dock_mooring and dock_sprite in tools/board_geometry_*.txt, and a
    * test holds them to it.
    */
   static final Pt DOCK_FOOT = new Pt(216, 180);
   static final Pt DOCK_MOORING = new Pt(214, 152);
   static final Pt DOCK_SPRITE = new Pt(210, 134);
   /**
    * The board row that has to show under the top bar for the moored ship to be seen whole
    * (its masthead flag is one row below), and the row just under the island's lowest
    * building. See boardOrigin.
    */
   static final int HARBOUR_TOP = 94;
   static final int ISLAND_FOOT = 366;
   /** How far below the exact middle the map sits: the eye weighs the island, not the masthead. */
   static final int MAP_DROP = 15;

   private Voyage() {
   }

   // ---------------------------------------------------------------- where the ship is

   /**
    * The cell the ship has reached. Not over: the turn being played. Over by running out
    * of turns: the pier. Over by reaching a race's target: the turn it was reached in, so
    * the King has not arrived yet. A map with no turn cap (turnCap 0) has no pier to count
    * toward: the cell is just the turn being played, or the last one taken once it is over.
    */
   static int cell(int turnCap, int turnsTaken, boolean done, boolean reachedTarget) {
      if(turnCap == 0) {
         return Math.max(0, done ? turnsTaken - 1 : turnsTaken);
      }
      if(!done) {
         return Math.max(0, Math.min(turnsTaken, turnCap - 1));
      }
      if(reachedTarget) {
         return Math.max(0, Math.min(turnsTaken - 1, turnCap - 1));
      }
      return turnCap;
   }

   /**
    * Where on the board a ship at this cell is: none until three turns remain, then the far
    * stop, the middle one, moored for the turn that is left, and done once the turns have run out.
    * A map with no turn cap never runs out of turns, so its ship never comes onto the board.
    */
   static int stage(int turnCap, int cell) {
      if(turnCap == 0) {
         return STAGE_NONE;
      }
      int remaining = turnCap - cell;
      if(remaining <= 0) {
         return STAGE_DONE;
      }
      if(remaining > APPROACH_TURNS) {
         return STAGE_NONE;
      }
      // 3 turns remaining is the far stop, 1 the dock
      return STAGE_DONE - remaining;
   }

   /** The same two steps from a game's own numbers. */
   static int stage(MapDef map, int turnsTaken, boolean done, int score) {
      boolean reached = map.targetScore > 0 && score >= map.targetScore;
      return stage(map.turnCap, cell(map.turnCap, turnsTaken, done, reached));
   }

   /**
    * Board position of the ship sprite's top left for a stage. Moored (and done), its contact
    * pixel is on the dock's mooring point. The two stops before that are evenly spaced on a
    * straight line over open sea from the far stop to the mooring. The far stop is set by
    * what the screen shows: visibleLeft is the board coordinate of the screen's left edge,
    * and the far stop hangs half off it. Where the screen is wide enough for two good
    * steps the ship sails in level; where it is not, it starts higher up instead, and comes
    * down from under the top bar.
    */
   static Pt shipOrigin(int stage, Pt mooring, int visibleLeft) {
      int mooredX = mooring.x - Art.SHIP_CONTACT_X;
      int mooredY = mooring.y - Art.SHIP_CONTACT_Y;
      if(stage >= STAGE_MOORED || stage == STAGE_NONE) {
         return new Pt(mooredX, mooredY);
      }
      int reach = SAIL_STEPS * MAX_STEP;
      int across = mooredX - Math.max(mooredX - reach, Math.min(mooredX, visibleLeft - FAR_OUT_X));
      // The whole way is a little over two least steps, so each half still rounds to one
      int way = SAIL_STEPS * MIN_STEP + 2;
      int down = 0;
      while(down < reach && across * across + down * down < way * way) {
         down++;
      }
      int stepsOut = STAGE_MOORED - stage;
      return new Pt(mooredX - part(across, stepsOut), mooredY - part(down, stepsOut));
   }

   /** stepsOut halves of a distance, rounded to the nearest pixel. */
   private static int part(int distance, int stepsOut) {
      return (2 * distance * stepsOut + SAIL_STEPS) / (2 * SAIL_STEPS);
   }

   // ---------------------------------------------------------------- the ship under sail

   /**
    * How long the ship takes to sail from one stage to the next, or 0 when it is not a sail
    * and the ship is just there: the stage before the first (from is -1), going back (an undo,
    * a new game), skipping a stage, or the flag going up. From none to the far stop it sails
    * in from off the screen, which is further.
    */
   static int sailMs(int from, int to) {
      if(from < STAGE_NONE || to != from + 1 || to > STAGE_MOORED) {
         return 0;
      }
      return from == STAGE_NONE ? SAIL_IN_MS : SAIL_STEP_MS;
   }

   /**
    * Where a sail from this stage starts. From a stop, that stop. From none, the ship starts
    * wholly off the left edge of what the screen shows (visibleLeft, as for shipOrigin),
    * level with the far stop: so it comes in on the line it has to follow, bow first.
    */
   static Pt sailStart(int from, Pt mooring, int visibleLeft) {
      if(from == STAGE_NONE) {
         return new Pt(visibleLeft - Art.SHIP_W, shipOrigin(STAGE_FAR, mooring, visibleLeft).y);
      }
      return shipOrigin(from, mooring, visibleLeft);
   }

   /**
    * Where the ship is elapsedMs into a sail of durationMs from one place to another: at
    * the start before it, at the end after it, and between them eased in and out (smoothstep),
    * rounded to a whole board pixel so it moves on the art grid.
    */
   static Pt sailAt(Pt from, Pt to, int elapsedMs, int durationMs) {
      if(durationMs <= 0 || elapsedMs >= durationMs) {
         return to;
      }
      if(elapsedMs <= 0) {
         return from;
      }
      double t = (double)elapsedMs / durationMs;
      double eased = t * t * (3 - 2 * t);
      return new Pt(from.x + (int)Math.round((to.x - from.x) * eased), from.y + (int)Math.round((to.y - from.y) * eased));
   }

   /**
    * Where the board's top left goes on the art canvas. Across, the board is centred. Down,
    * what the map shows at its fullest, from the moored ship's masthead to the island's foot,
    * is centred between the top bar (ending at barBottom) and the bottom bar (starting at
    * trayTop), then dropped by MAP_DROP, on any screen. The island's foot always stays above
    * the bottom bar, which on a short screen takes up some or all of the drop.
    */
   static Pt boardOrigin(int artWidth, int barBottom, int trayTop) {
      int y = (barBottom + trayTop - HARBOUR_TOP - ISLAND_FOOT) / 2 + MAP_DROP;
      return new Pt(artWidth / 2 - Board.CENTRE_X, Math.min(y, trayTop - ISLAND_FOOT));
   }

   // ---------------------------------------------------------------- the dock

   /**
    * The end of the start road that touches the sea: the one vertex that belongs to no land
    * hex beyond the two the road runs between. The road has to have exactly one, and it
    * has to be DOCK_FOOT, where the board has its dock; a map whose start road does not
    * meet the sea there is a bug.
    */
   static Pt coastalEnd(int[][] hex, Pt[] roadEnds) {
      Pt found = null;
      for(Pt end : roadEnds) {
         int hexes = 0;
         for(int[] centre : hex) {
            for(int[] v : Board.VERTEX) {
               if(centre[0] + v[0] == end.x && centre[1] + v[1] == end.y) {
                  hexes++;
                  break;
               }
            }
         }
         if(hexes == 2) {
            if(found != null) {
               throw new IllegalStateException("Both ends of the start road touch the sea");
            }
            found = end;
         }
      }
      if(found == null) {
         throw new IllegalStateException("Neither end of the start road touches the sea");
      }
      return found;
   }

   // ---------------------------------------------------------------- the HUD strip

   /** The strip as the widest margin lets the island's own turn count fit in 216 px. */
   static final int STRIP_MARGIN = 3;
   static final int STRIP_LEAD = 2;
   static final int MAX_PITCH = 14;
   /** Where the ship lies once the turns are over, from the pier's left edge: in harbour, its stern behind the pier. */
   static final int PIER_SHIP_IN = 4;

   /**
    * How many cells the strip of a map with no turn cap has. The same as Tarnholm's fifteen
    * turns, so both islands' strips look alike. There is no last turn to count down to, so
    * the strip is a window onto the latest turns: once it is full the oldest flag falls off
    * the left as each new one goes up.
    */
   static final int FLAG_WINDOW = 15;

   /**
    * The first turn a window strip shows, counted from 0: its flag is in cell 0. While the
    * game is on, the ship sails in the cell after the last flag and is never allowed into
    * the last cell, whose bare water lies beside the pier (see Strip.shipX), so the window
    * holds FLAG_WINDOW - 2 flags. The turn that reaches the target puts its flag up in the
    * cell the ship was in, and the ship sails on into harbour, so no flag moves on the win
    * and the cell beside the pier stays bare water.
    */
   static int windowStart(int turnsTaken, boolean done) {
      return Math.max(0, turnsTaken - (done ? FLAG_WINDOW - 1 : FLAG_WINDOW - 2));
   }

   /** How many flags a window strip shows, in cells 0 onward. */
   static int windowFlags(int turnsTaken, boolean done) {
      return turnsTaken - windowStart(turnsTaken, done);
   }

   /**
    * The cell of a window strip the ship is in: the one after the last flag while the game
    * is on, and FLAG_WINDOW, in harbour behind the pier, once the target is reached.
    */
   static int windowShipCell(int turnsTaken, boolean done) {
      return done ? FLAG_WINDOW : windowFlags(turnsTaken, false);
   }

   /**
    * One row of sea across the HUD: a cell per turn, then the pier. Two kinds. A strip
    * with flags (one island) puts a flag in the cell of each finished turn, so a cell is
    * at least as wide as the flag's pole and field, and the flags overlap by the width of
    * the pole, the next flag's pole standing where the last one's edge was. A plain strip
    * has only buoys and the ship, centred in cells as narrow as the fit allows.
    *
    * All x values are relative to the strip's left edge; the strip itself is centred
    * in the HUD. The water is as wide as it needs to be, ending at the pier.
    */
   static final class Strip {
      final int turns;
      final boolean flags;
      final int pitch;
      final int pad;
      /** Water width, from the left end to the pier's right edge. */
      final int width;
      /** Where the strip's left edge is within a HUD of the given width. */
      final int left;

      Strip(int turns, boolean flags, int hudWidth) {
         this.turns = turns;
         this.flags = flags;
         int room = hudWidth - 2 * STRIP_MARGIN;
         int p = MAX_PITCH;
         while(p > 1 && measure(turns, flags, p).width > room) {
            p--;
         }
         pitch = p;
         Strip m = measure(turns, flags, p);
         pad = m.pad;
         width = m.width;
         left = (hudWidth - width) / 2;
      }

      /** At a given pitch, before it is centred: the way the fit is measured. */
      private Strip(int turns, boolean flags, int pitch, boolean measured) {
         this.turns = turns;
         this.flags = flags;
         this.pitch = pitch;
         // A plain strip's first ship, centred in its cell, starts STRIP_LEAD in
         this.pad = flags ? STRIP_LEAD : STRIP_LEAD - (pitch - Art.TRACK_SHIP_W) / 2;
         this.left = 0;
         this.width = Math.max(pierX(this) + Art.TRACK_PIER_W, shipX(this, turns) + Art.TRACK_SHIP_W);
      }

      /**
       * The strip a map shows. Every finished turn gets its flag. A map with no turn cap
       * shows the latest of them in a window of FLAG_WINDOW cells; a race with a cap has
       * no per-turn scores to show, so its strip is the narrow kind.
       */
      static Strip forMap(MapDef map, int hudWidth) {
         if(map.turnCap == 0) {
            return new Strip(FLAG_WINDOW, true, hudWidth);
         }
         return new Strip(map.turnCap, map.targetScore == 0, hudWidth);
      }

      private static Strip measure(int turns, boolean flags, int pitch) {
         return new Strip(turns, flags, pitch, true);
      }

      /** Left edge of cell i's flag, or of its buoy's room. */
      int cellX(int i) {
         return pad + pitch * i;
      }

      /** Left edge of a buoy centred in cell i. */
      int buoyX(int i) {
         return cellX(i) + (pitch - Art.TRACK_BUOY_W) / 2;
      }

      /**
       * Whether cell i has a buoy while its turn is still to come. A strip with flags
       * marks every cell, because each becomes a flag. A plain strip marks every second
       * one, the even turns (cells 1, 3, 5 and so on), so the last buoy is the last turn
       * before the pier and none of them moves as the ship advances.
       */
      boolean marked(int i) {
         return flags || i % 2 == 1;
      }

      /**
       * Left edge of the ship at a cell, 0 to turns. In a strip with flags the ship sits
       * where a flag will; in a plain one it is centred on the cell. In the last turn's cell
       * (the last turn is played with the ship at the dock) it lies alongside the pier, which
       * directly follows the last cell; once the turns are over it is in harbour behind it.
       */
      int shipX(int cell) {
         return shipX(this, cell);
      }

      int pierX() {
         return pierX(this);
      }

      private static int shipX(Strip s, int cell) {
         if(cell >= s.turns) {
            // Over: in harbour, its stern behind the pier, so the last flag shows whole
            return pierX(s) + PIER_SHIP_IN;
         }
         if(cell == s.turns - 1) {
            // The last turn: alongside the pier, in the last cell's bare water
            return pierX(s) - Art.TRACK_SHIP_W + 1;
         }
         if(s.flags) {
            return s.cellX(cell);
         }
         return s.pad + s.pitch * cell + (s.pitch - Art.TRACK_SHIP_W) / 2;
      }

      /**
       * The pier directly follows the last cell: with flags, one column past the last flag;
       * without, where the last cell's room ends less the half of it round a buoy, but
       * never touching the last buoy.
       */
      private static int pierX(Strip s) {
         if(s.flags) {
            return s.cellX(s.turns - 1) + Art.TRACK_FLAG_W + 1;
         }
         int beside = s.cellX(s.turns) - (s.pitch - Art.TRACK_BUOY_W) / 2;
         return Math.max(beside, s.buoyX(s.turns - 1) + Art.TRACK_BUOY_W + 1);
      }
   }
}
