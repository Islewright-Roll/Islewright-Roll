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
 * The in-context tutorial's decisions, and nothing else: which callout (a "coach mark")
 * is due now, and which have been seen. No Android types, so it is unit tested. GameView
 * gathers a {@link Snapshot} of the game after every change, asks {@link #update}, and
 * draws what comes back, where TutorialLayout says. The tutorial never blocks play; it only
 * decides what to say.
 *
 * Ten steps. 1 to 7 are met in order, and 8 to 10 whenever their moment first comes. Each
 * step has a trigger (the state in which its callout shows) and is done when the player
 * does the thing it describes, or taps Got it. Of the two banked resource steps a game
 * shows the one for the trader mode it is played in, so a player who never turns classic
 * mode on keeps the tutorial on with nothing left to say, and is shown how to drag the
 * first time they bank a resource in classic mode. A step that has been done is never shown
 * again; the set of them is a bitmask the caller saves, so the tutorial resumes where it
 * left off. Only one callout is up at a time, and the one up stays until it is done or its
 * trigger no longer holds, so a callout never changes under the player's eyes.
 *
 * The "player did X" events (rolled, held, built, undone, turnEnded) are public so a test
 * can speak them directly; update() derives them itself by comparing each snapshot with the
 * one before, and from the first snapshot alone (a resumed game) it catches up from the
 * state.
 */
final class Tutorial {

   /** The steps, in the order 1 to 7 are met and then the three that wait for their moment. */
   enum Step {
      /** Roll button: a new game, before the first roll. */
      ROLL,
      /** Dice tray: after the first roll of the game. */
      KEEP,
      /** The costs key in the top bar: after KEEP is done. */
      COSTS,
      /** One spot on offer: the first time anything is, after COSTS. */
      BUILD,
      /** The undo button: right after the first build. */
      UNDO,
      /** The roll button: the first time it would end the turn. */
      END,
      /** The turn strip: from the second turn on. */
      SHIP,
      /** One gold die: the first time two or more dice show gold. */
      GOLD,
      /** The banked mini dice: the first time one appears, in the usual trader mode. */
      BANK,
      /**
       * The banked mini dice: the first time one appears in trader resource classic mode.
       * A standing hint rather than a tour stop: it is not part of ALL, so finishing the
       * tour does not wait for it, and it still shows after the tour is finished or skipped,
       * because a player who turns classic mode on later has never been shown the drag.
       */
      BANK_CLASSIC;

      int bit() {
         return 1 << ordinal();
      }
   }

   /** Every step there is, the standing hint included: the bits worth keeping. */
   static final int KNOWN = (1 << Step.values().length) - 1;

   /** Every step of the tour seen. The standing hint is left out (see BANK_CLASSIC). */
   static final int ALL = KNOWN & ~Step.BANK_CLASSIC.bit();

   /** What the tutorial needs to know about the game, as plain numbers and flags. */
   static final class Snapshot {
      /** Rolls made this turn, 0 to 3. */
      int rolls;
      /** Turns finished so far: 0 on the first turn. */
      int turnsTaken;
      /** Whether any die is held. */
      boolean anyHeld;
      /** Whether any spot on the island is on offer right now. */
      boolean offered;
      /** Whether something has been built this turn. */
      boolean builtThisTurn;
      /** Whether the undo button is showing. */
      boolean canUndo;
      /** Whether the roll button would end the turn: no rolls left or a build made, and the game not over. */
      boolean rollEndsTurn;
      /** How many of the dice showing are gold. */
      int goldDice;
      /** How many banked resources show as mini dice. */
      int miniDice;
      /** Whether the game is in trader resource classic mode: a banked resource is dragged onto a die. */
      boolean classicBank;
      boolean gameOver;

      Snapshot copy() {
         Snapshot c = new Snapshot();
         c.rolls = rolls;
         c.turnsTaken = turnsTaken;
         c.anyHeld = anyHeld;
         c.offered = offered;
         c.builtThisTurn = builtThisTurn;
         c.canUndo = canUndo;
         c.rollEndsTurn = rollEndsTurn;
         c.goldDice = goldDice;
         c.miniDice = miniDice;
         c.classicBank = classicBank;
         c.gameOver = gameOver;
         return c;
      }
   }

   private boolean active;
   private int seen;
   private Step current;
   private Snapshot prev;
   private boolean dirty;

   Tutorial(boolean active, int seen) {
      this.active = active;
      this.seen = seen & KNOWN;
   }

   /** Whether the tutorial is on: false once skipped or finished. */
   boolean isActive() {
      return active;
   }

   /** The steps done, one bit each (Step.bit), for the settings file. */
   int seenMask() {
      return seen;
   }

   boolean isSeen(Step step) {
      return (seen & step.bit()) != 0;
   }

   /** The step the last update() asked to show, or null. */
   Step current() {
      return current;
   }

   /**
    * Takes the stored settings over, when they differ from what is held (the Settings
    * dialog and a backup import can change them). A change starts the watching afresh.
    */
   void configure(boolean active, int seen) {
      seen &= KNOWN;
      if(active == this.active && seen == this.seen) {
         return;
      }
      this.active = active;
      this.seen = seen;
      current = null;
      prev = null;
   }

   /**
    * Stops comparing with the snapshot before, and drops the callout up: the game under the
    * tutorial was swapped for another, or was set aside for a while. The steps seen are kept.
    */
   void forget() {
      current = null;
      prev = null;
   }

   /** True once after the active flag or the seen steps changed: the caller saves them. */
   boolean takeDirty() {
      boolean d = dirty;
      dirty = false;
      return d;
   }

   // ---------------------------------------------------------------- what to show

   /**
    * Looks at the game as it is now and returns the step whose callout is due, or null.
    * Any "player did X" that happened since the last call is applied first. Nothing is due
    * once the game is over, or the tutorial is off.
    */
   Step update(Snapshot s) {
      if(!active) {
         return standingHint(s);
      }
      if(prev != null && isNewGame(prev, s)) {
         prev = null;
         current = null;
      }
      if(prev == null) {
         catchUp(s);
      }
      else {
         transitions(prev, s);
      }
      prev = s.copy();

      if((seen & ALL) == ALL) {
         finish();
         return standingHint(s);
      }
      if(s.gameOver) {
         current = null;
         return null;
      }
      if(current != null && isSeen(current)) {
         current = null;
      }
      if(current != null && !triggered(current, s)) {
         // The two moment steps are done when their moment passes; the rest wait to come back
         if(current == Step.GOLD || current == Step.BANK || current == Step.BANK_CLASSIC) {
            markSeen(current);
         }
         current = null;
      }
      if(current == null) {
         for(Step step : Step.values()) {
            if(!isSeen(step) && triggered(step, s)) {
               current = step;
               break;
            }
         }
      }
      return current;
   }

   /** Whether this step's callout shows in this state, if it has not been done. */
   private boolean triggered(Step step, Snapshot s) {
      switch(step) {
         case ROLL:
            return s.turnsTaken == 0 && s.rolls == 0 && !s.builtThisTurn;
         case KEEP:
            return isSeen(Step.ROLL) && s.rolls >= 1;
         case COSTS:
            return isSeen(Step.KEEP);
         case BUILD:
            return isSeen(Step.COSTS) && s.offered && !s.builtThisTurn;
         case UNDO:
            return s.canUndo;
         case END:
            return s.rollEndsTurn;
         case SHIP:
            return s.turnsTaken >= 1;
         case GOLD:
            return s.goldDice >= 2;
         case BANK:
            return s.miniDice >= 1 && !s.classicBank;
         default:
            return s.miniDice >= 1 && s.classicBank;
      }
   }

   /** A restart or a new game puts the turn count or the roll count back. */
   private static boolean isNewGame(Snapshot before, Snapshot now) {
      return now.turnsTaken < before.turnsTaken || (now.turnsTaken == before.turnsTaken && now.rolls < before.rolls);
   }

   /** The events between two snapshots. */
   private void transitions(Snapshot before, Snapshot now) {
      if(now.rolls > before.rolls || (now.turnsTaken > before.turnsTaken && now.rolls > 0)) {
         rolled(now.rolls);
      }
      if(now.anyHeld && !before.anyHeld) {
         held();
      }
      if(now.builtThisTurn && !before.builtThisTurn) {
         built();
      }
      if(before.canUndo && !now.canUndo) {
         undone();
      }
      if(now.turnsTaken > before.turnsTaken) {
         turnEnded();
      }
   }

   /**
    * The first look at a game (the app was reopened, the island changed, a new game began):
    * whatever the state shows has already been done is not shown. A game with no progress
    * marks nothing.
    */
   private void catchUp(Snapshot s) {
      if(s.rolls >= 1 || s.turnsTaken >= 1 || s.builtThisTurn) {
         markSeen(Step.ROLL);
      }
      if(s.anyHeld || s.rolls >= 2 || s.builtThisTurn || s.turnsTaken >= 1) {
         markSeen(Step.KEEP);
      }
      if(s.builtThisTurn) {
         markSeen(Step.BUILD);
      }
   }

   // ---------------------------------------------------------------- the player did X

   /** A roll was made; rollsNow is the count this turn after it (1 to 3). */
   void rolled(int rollsNow) {
      markSeen(Step.ROLL);
      if(rollsNow >= 2) {
         markSeen(Step.KEEP);
      }
      markIfCurrent(Step.COSTS);
   }

   /** A die was held. */
   void held() {
      markSeen(Step.ROLL);
      markSeen(Step.KEEP);
   }

   /** Something was built. */
   void built() {
      markSeen(Step.ROLL);
      markSeen(Step.KEEP);
      markSeen(Step.BUILD);
      markIfCurrent(Step.COSTS);
   }

   /** The last build was taken back, or the chance to was lost to a roll or the turn ending. */
   void undone() {
      markIfCurrent(Step.UNDO);
   }

   /** The turn ended (by the roll button). */
   void turnEnded() {
      markSeen(Step.ROLL);
      markSeen(Step.KEEP);
      markSeen(Step.END);
      markIfCurrent(Step.COSTS);
      markIfCurrent(Step.UNDO);
      markIfCurrent(Step.SHIP);
   }

   /** The Got it button: the callout up is done. */
   void gotIt() {
      if(current != null) {
         markSeen(current);
      }
   }

   /**
    * The Skip tutorial button: no more tour callouts, now or in later games. The standing
    * hint is only dismissed when it is the one up, so skipping the tour does not hide it.
    */
   void skip() {
      markIfCurrent(Step.BANK_CLASSIC);
      current = null;
      finish();
   }

   /**
    * The standing hint on its own, once the tour is off: up while its moment lasts, and
    * done when that moment passes or the player taps Got it.
    */
   private Step standingHint(Snapshot s) {
      Step hint = Step.BANK_CLASSIC;
      if(isSeen(hint)) {
         current = null;
         return null;
      }
      if(!s.gameOver && triggered(hint, s)) {
         current = hint;
         return hint;
      }
      if(current == hint) {
         markSeen(hint);
      }
      current = null;
      return null;
   }

   private void finish() {
      if(active) {
         active = false;
         dirty = true;
      }
   }

   private void markIfCurrent(Step step) {
      if(current == step) {
         markSeen(step);
      }
   }

   private void markSeen(Step step) {
      if(!isSeen(step)) {
         seen |= step.bit();
         dirty = true;
      }
      if(current == step) {
         current = null;
      }
   }
}
