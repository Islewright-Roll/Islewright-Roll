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

import android.os.SystemClock;
import android.widget.Toast;

//TODO: 
//  Confirm "Use knight resource(s)"
//  Center vertically
//  High score menu & toast when beating after 1st play

public class Game {
   /** The island being played. Everything map specific in the rules is read from it. */
   MapDef map;
   /** That island's geometry, for the view. */
   Board board;
   Playsheet playsheet;
   Dice[] dice;
   int rolls;
   ArrayList<Dice.Value> knightResources;
   boolean builtResourceThisTurn;
   int turnsTaken;
   boolean scored = false;
   /**
    * The score that wins this race, 0 on a map that is not one. The game's own and saved
    * with it, so a change of setting cannot end or lengthen a finished game, and a saved
    * game goes on with the target it was played to (see setRaceTarget).
    */
   int targetScore;
   // When the game became done, in SystemClock.uptimeMillis(). The roll button ignores
   // taps for a moment after this so a bump cannot start a new game over the final board.
   // Not persisted: a done game is never saved.
   long gameOverAt;
   private BuildPlan buildPlan;
   private String buildPlanFingerprint;

   /**
    * Where this game's sounds go. Null means the process-wide sink (SoundEffects in the
    * app, silence in a unit test); a test sets its own to see what was asked for.
    */
   SoundSink sounds;
   /** Milliseconds added to every sound of the build in progress: the stagger of a chain build. */
   private int soundBase;
   /** The roll that follows a turn end in one tap waits for the turn end sound. */
   private int rollSoundDelay;

   /**
    * The game as it stood before the latest build tap, for the undo button, or null when
    * there is nothing to undo. The same JSON as the save file, kept in memory only: an
    * undo does not survive the process. Cleared by anything that moves the game on past
    * the point where taking a build back would be fair: a roll, a turn ending, a new game,
    * and a build that ended the game.
    */
   private String undoJson;

   /**
    * Counts the moments the game is replaced under the view rather than played: a restart, a
    * new island, a debug scenario (all through reset()) and an undo. The hire animation notes
    * the number when it begins and ends at once if it has moved on.
    */
   int stateEpoch;
   /** Traders hired so far in the build tap in progress, to stagger their animations. */
   private int hiresThisTap;

   public static Dice.Value[] RESOURCES_ROAD = new Dice.Value[]{ Dice.Value.Brick, Dice.Value.Lumber };
   public static Dice.Value[] RESOURCES_VILLAGE = new Dice.Value[]{ Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Wool, Dice.Value.Grain };
   public static Dice.Value[] RESOURCES_KNIGHT = new Dice.Value[]{ Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore };
   public static Dice.Value[] RESOURCES_CITY = new Dice.Value[]{ Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore };
   
   GameView gameView;

   /**
    * True while a debug scenario holds this game (see DebugScenario.applyTo): the
    * tutorial neither watches nor draws over it. Cleared by reset(), so a restart from the
    * menu ends it.
    */
   boolean tutorialPaused;
   
   /**
    * The player's chosen colour, used to tint everything they have built. Held here
    * rather than looked up per draw because draw() runs constantly and reading the
    * settings file from it would be absurd. MainActivity sets it at startup and again
    * whenever the picker saves.
    */
   int playerColor = PlayerColor.DEFAULT;

   /** See HighScoresAndSettings. Held here for the same reason as playerColor: draw()
    *  and touch() run constantly and must not read a settings file. */
   int earlyBuildPrompt = HighScoresAndSettings.EARLY_PROMPT_SMART;
   boolean endTurnPrompt = true;
   /** The Dunholm target setting, held for the same reason. A race starts with it. */
   int raceTarget = HighScoresAndSettings.RACE_TARGET_DEFAULT;
   /**
    * Trader resource classic mode (see HighScoresAndSettings.traderClassic), held for the
    * same reason. Set through setTraderClassic, which passes it on to the playsheet.
    */
   boolean traderClassic = false;
   
   public Game(MapDef map) {
      this.map = map;
      targetScore = map.targetScore;
      board = Board.forMap(map);
      playsheet = new Playsheet(map);
      playsheet.bankPaysBuilds = !traderClassic;
      knightResources = new ArrayList<Dice.Value>();
      dice = new Dice[6];
      for(int i=0; i < dice.length; i++) {
         dice[i] = new Dice();
      }
      
      turnsTaken = 0;
      newTurn(true);
   }
   
   public String getString(int id) {
      return gameView.activity.getResources().getString(id);
   }
   
   public void setGameView(GameView gameView) {
      this.gameView = gameView;
   }
   
   /** A new game on the same map. */
   public void reset() {
      stateEpoch++;
      tutorialPaused = false;
      undoJson = null;
      turnsTaken = 0;
      playsheet.reset();
      scored = false;
      targetScore = isRace() ? Math.max(1, Math.min(map.maxScore(), raceTarget)) : map.targetScore;
      newTurn(true);
      changed();
   }

   /**
    * A new game on the given map. The view holds piece lists and layers sized for the
    * map it was built with, so the caller must build a new GameView when the map changes.
    */
   public void startNewGame(MapDef map) {
      this.map = map;
      board = Board.forMap(map);
      playsheet = new Playsheet(map);
      playsheet.bankPaysBuilds = !traderClassic;
      // Anything set aside from the old map's bank means nothing on this one
      knightResources.clear();
      buildPlan = null;
      buildPlanFingerprint = null;
      undoJson = null;
      reset();
   }
   
   /**
    * Turns trader resource classic mode on or off for this game from now on. In classic
    * mode a build never pays with a banked resource by itself; the player spends one by
    * dragging it onto a die (overwriteDie).
    */
   void setTraderClassic(boolean on) {
      boolean was = traderClassic;
      traderClassic = on;
      playsheet.bankPaysBuilds = !on;
      if(was == on) {
         return;
      }
      if(on) {
         // The list is what the bank offered at the last check, for newTurn to hand back
         // what a turn set aside. Classic mode sets nothing aside, and a stale entry could
         // return a slot a drag has since spent, so it starts empty and stays so.
         knightResources.clear();
      }
      changed();
   }

   private SoundSink sink() {
      return sounds != null ? sounds : Sounds.sink;
   }

   private void sound(Sfx sfx, int delayMs) {
      sink().play(sfx, soundBase + delayMs);
   }

   public void newTurn(boolean firstTurn) {
       if(isGameDone()) {
           return;
       }
       
      undoJson = null;

      // For the sounds: the ship's stage and whether this turn was empty, before anything changes
      int stageBefore = Voyage.stage(map, turnsTaken, false, playsheet.getScore());
      boolean emptyTurn = !firstTurn && !builtResourceThisTurn;
      int rank = 0;

      rolls = 0;
      
      //Return unused knightResources
      for(int i=0; i < knightResources.size(); i++) {
          // The list is the bank slots that were open at the last check, so a value that
          // still has an open slot is not something set aside. Returning it would hand a
          // spent "?" back whenever the other "?" is open, which only a map with two can
          // do; on Tarnholm every value has just one slot and this changes nothing.
          if(playsheet.firstAvailableSlot(knightResources.get(i)) == 0) {
             playsheet.returnToBank(knightResources.get(i));
          }
      }
      
      knightResources.clear();
      for(int i=0; i < dice.length; i++) {
         dice[i].reset();
      }
      if(!firstTurn && !builtResourceThisTurn) {
         playsheet.turnsNothingBuilt++;
      }
      builtResourceThisTurn = false;
      
      if(!firstTurn) {
         turnsTaken++;
         playsheet.scoreTurn(turnsTaken);
         if(isGameDone()) {
             gameOverAt = SystemClock.uptimeMillis();
             // A race that reaches its target ends in finishIfTargetReached, so a race that
             // is done here ran out of turns and has no result to record. Nothing is
             // written for it, and `scored` stays false. A race with no turn cap (Dunholm)
             // never runs out, so it never comes here.
             if(!scored && !isRace()) {
                 scored = true;
                 rank = recordResult();
             }
         }
         boolean done = isGameDone();
         int stageAfter = Voyage.stage(map, turnsTaken, done, playsheet.getScore());
         for(SoundCues.Cue cue : SoundCues.turnEnd(emptyTurn && map.emptyTurnPenalty > 0,
               stageBefore == Voyage.STAGE_NONE && stageAfter == Voyage.STAGE_FAR,
               stageBefore != Voyage.STAGE_MOORED && stageAfter == Voyage.STAGE_MOORED && !done, done, rank == 1)) {
            sound(cue.sfx, cue.delayMs);
         }
      }
      changed();
   }
   
   /**
    * True when starting another game would throw something away: a game that is not over
    * and in which a turn has passed, the dice have been rolled or anything has been built.
    * A finished game has nothing left to lose, and neither has one nobody has touched.
    */
   boolean hasProgress() {
      if(isGameDone()) {
         return false;
      }
      if(turnsTaken > 0 || rolls > 0 || playsheet.traderCount() > 0) {
         return true;
      }
      // Road 0 is the start road, which every game begins with
      for(int i = 1; i < playsheet.roads.length; i++) {
         if(playsheet.roads[i]) {
            return true;
         }
      }
      for(boolean built : playsheet.villages) {
         if(built) {
            return true;
         }
      }
      for(boolean built : playsheet.cities) {
         if(built) {
            return true;
         }
      }
      return false;
   }

   /** True on a map that is a race to a target score, not a fixed number of turns. */
   boolean isRace() {
      return map.targetScore > 0;
   }

   /**
    * Takes a new Dunholm target setting. It moves the game in progress too when that game
    * is a race, is not over and has not already passed the new target: a player who sets
    * a higher target expects to play on, and one already passed would end the game on the
    * spot. Otherwise it waits for the next new game, which reset() starts with it.
    */
   void setRaceTarget(int target) {
      raceTarget = target;
      if(isRace() && !isGameDone()) {
         int clamped = Math.max(1, Math.min(map.maxScore(), target));
         if(playsheet.getScore() < clamped) {
            targetScore = clamped;
         }
      }
   }

   /**
    * What this game puts on the top scores, or 0 for nothing. On a points map it is the
    * score, once there is one. On a race it is the turns taken, and only once the target
    * has been reached; a race that ran out of turns has no result. Nor has a race to any
    * target but the island's own: fewer turns to another number is not the same feat, and
    * the list would mix them.
    */
   int resultToRecord() {
      if(isRace()) {
         return targetScore == map.targetScore && playsheet.getScore() >= targetScore ? turnsTaken : 0;
      }
      return Math.max(0, playsheet.getScore());
   }

   /** Records the result and returns its rank on the island's list (1 is a new best), or 0 for none. */
   private int recordResult() {
      int result = resultToRecord();
      return result > 0 ? recordScore(result) : 0;
   }

   /** Writes a result to the island's top scores. A game with no view has no player to tell: it writes nothing. */
   int recordScore(int result) {
      if(gameView != null && gameView.activity != null) {
         return HighScores.score(gameView.activity, map.id, result);
      }
      return 0;
   }

   /**
    * Called at the end of every build. On a race map the game is over the moment the
    * score reaches the target, in the middle of the turn: that turn counts as taken and
    * is the result. The same four things as the end of newTurn: count the turn, score it,
    * stamp the time, record. Does not call changed(); the build that called this does,
    * and with the game done that clears the save.
    *
    * Returns -1 if the game did not end here, otherwise the rank of the result (0 if none).
    */
   private int finishIfTargetReached() {
      if(!isRace() || scored || playsheet.getScore() < targetScore) {
         return -1;
      }
      turnsTaken++;
      playsheet.scoreTurn(turnsTaken);
      gameOverAt = SystemClock.uptimeMillis();
      scored = true;
      return recordResult();
   }

   /**
    * Whether running this chain of builds would end a race. The early build prompts ask
    * the player to think twice about giving up rolls or spreading their traders thin, and
    * neither matters when the game ends on that build. Pure: works on a copy of the
    * playsheet and touches nothing live. A null chain (nothing reaches the position) never
    * wins.
    */
   boolean chainReachesTarget(BuildPlan.Chain chain) {
      if(!isRace() || chain == null) {
         return false;
      }
      Playsheet copy = playsheet.copy();
      for(BuildPlan.Step step : chain.steps) {
         switch(step.kind) {
            case ROAD:
               copy.roads[step.index] = true;
               break;
            case VILLAGE:
               copy.villages[step.index] = true;
               break;
            case CITY:
               copy.cities[step.index] = true;
               break;
            case KNIGHT:
               copy.traders[step.index] = true;
               break;
         }
      }
      return copy.getScore() >= targetScore;
   }

   public boolean canUseKnightResource(int i) {
      return playsheet.canUseKnightResource(i);
   }
   
   public void consumeKnightResource(int i) {
      Dice.Value val = playsheet.getKnightResource(i);
      if(val != Dice.Value.None) {
         knightResources.add(val);
      }
      changed();
   }
   
   public void diceTouched(int i) {
      if(rolls == 0 || i < 0 || i >= dice.length) {
         return;
      }
      // A die a banked resource overwrote stays for the rest of the turn (see overwriteDie)
      if(dice[i].locked) {
         return;
      }
      
      dice[i].hold(rolls, !dice[i].isHeld());
      sound(Sfx.DIE, 0);
      changed();
   }
   
   public void roll() {
       // A new roll replaces the dice the snapshot holds: restoring them would be a free re-roll
       undoJson = null;
       if(isGameDone()) {
           reset();
           return;
       }
       
      if(!canRoll()) {
         newTurn(false);
         return;
      }
      
      int len = dice.length;
      Dice.Value[] old = new Dice.Value[len];
      for(int i=0; i < len; i++) {
         Dice die = dice[i];
         if(!die.isHeld()) {
            old[i] = die.getValue();
            die.roll();
         }         
      }
      
      // People think the dice are not random when they don't change, 
      // so keep the same dice, but switch order if possible to help
      // remove that disconcerting "I rolled, but some of the dice
      // didn't change" effect.
      for(int i=0; i < len; i++) {
         Dice die = dice[i];
         if(!die.isHeld()) {
            if(old[i] == die.getValue()) {
               for(int j=0; j < len; j++) {
                  Dice die2 = dice[j];
                  if(i == j || die2.isHeld()) {
                     continue;
                  }
                  if(old[j] != die.getValue() && old[i] != die2.getValue()) {
                     die.swap(die2);
                     break;
                  }
               }
            }
         }         
      }
      
      rolls++;
      sound(Sfx.ROLL, rollSoundDelay);
      changed();
   }
   
   /**
    * Called at the end of every action that changes the game: a roll, a build, a turn
    * ending, a die held, a knight resource taken.
    *
    * Redraws, and writes the game to disk. Saving only in onPause protected against
    * Android reclaiming a backgrounded app, which is the common case, but not against
    * the process dying while the player is looking at it. The state is about 1.6KB of
    * JSON, so there is no reason to be clever about when to write it: after every action
    * means at most one action can ever be lost.
    */
   void changed() {
      if(gameView == null) {
         return;
      }
      gameView.postInvalidate();
      if(gameView.activity != null) {
         GameStateStore.persist(gameView.activity, this);
      }
   }

   public boolean canRoll() {
      return (rolls < 3 && !builtResourceThisTurn && !isGameDone());
   }

   /**
    * Ends the turn and immediately rolls, for the case where the roll button reads
    * "1st Roll" because nothing can be built and nothing can be rolled. Plain roll()
    * only advances the turn there, which would leave the player tapping a button
    * labelled "1st Roll" and watching nothing roll.
    */
   public void endTurnAndRoll() {
      newTurn(false);
      if(canRoll()) {
         rollSoundDelay = SoundCues.ROLL_AFTER_TURN_END_MS;
         try {
            roll();
         }
         finally {
            rollSoundDelay = 0;
         }
      }
   }

   /**
    * True when the roll button would end the turn while the player can still afford to
    * build something. Tapping it then is nearly always a misclick, since the button reads
    * "Done" in the same spot the player has just tapped three times to roll, so the UI
    * confirms first. When nothing is affordable, ending the turn is the only legal move
    * left and confirming would just be a nag, so this returns false.
    */
   public boolean shouldConfirmEndTurn() {
      if(!endTurnPrompt || isGameDone() || canRoll()) {
         return false;
      }

      // Only dice are lost when the turn ends. newTurn() hands unused knight resources
      // back to playsheet.resourcesAvail, so a build that would have been paid from the
      // bank is still there next turn and there is nothing to warn about. Warning on it
      // anyway would be a nag on every turn that a banked resource exists.
      return getBuildPlan().hasDiceOnlyBuild();
   }

   /**
    * True when building right now would forfeit rolls the player has not used yet.
    * canRoll() goes false as soon as anything is built, so building on the first or
    * second roll silently throws the rest of the turn away.
    */
   public boolean wouldForfeitRolls() {
      return rolls > 0 && rolls < 3 && !builtResourceThisTurn && !isGameDone();
   }
   
   public void buildVillage(int i) {
      if(!canBuildVillage(i)) {
         return;
      }
      
      int bonuses = playsheet.bonusState();
      Spent spent = useResources(RESOURCES_VILLAGE);
      playsheet.buildVillage(i);
      builtResourceThisTurn = true;
      builtSounds(BuildPlan.Kind.VILLAGE, spent, bonuses, finishIfTargetReached());
      changed();
   }
   
   public boolean canBuildVillage(int i) {
      if(!canBuildVillage()) {
         return false;
      }
      
      return playsheet.canBuildVillage(i);
   }
   
   public boolean canBuildVillage() {
      if(rolls == 0 || isGameDone()) {
         return false;
      }
      
      return canBuild(RESOURCES_VILLAGE);
   }
   
   public void buildKnight(int i) {
      if(!canBuildKnight(i)) {
         return;
      }
      
      int bonuses = playsheet.bonusState();
      Spent spent = useResources(RESOURCES_KNIGHT);
      playsheet.buildKnight(i);
      if(gameView != null) {
         gameView.traderHired(i, hiresThisTap * HireAnim.CHAIN_STAGGER_MS);
      }
      hiresThisTap++;
      builtResourceThisTurn = true;
      builtSounds(BuildPlan.Kind.KNIGHT, spent, bonuses, finishIfTargetReached());
      changed();
   }
   
   public boolean canBuildKnight(int i) {
      if(!canBuildKnight()) {
         return false;
      }
      
      return playsheet.canBuildKnight(i);
   }
   
   public boolean canBuildKnight() {
      if(rolls == 0 || isGameDone()) {
         return false;
      }
      
      return canBuild(RESOURCES_KNIGHT);
   }
   
   public void buildRoad(int i) {
      if(!canBuildRoad(i)) {
         return;
      }
      
      int bonuses = playsheet.bonusState();
      Spent spent = useResources(RESOURCES_ROAD);
      playsheet.buildRoad(i);
      builtResourceThisTurn = true;
      builtSounds(BuildPlan.Kind.ROAD, spent, bonuses, finishIfTargetReached());
      changed();
   }
   
   public boolean canBuildRoad(int i) {
      if(!canBuildRoad()) {
         return false;
      }
      
      return playsheet.canBuildRoad(i);
   }   
   
   public boolean canBuildRoad() {
      if(rolls == 0 || isGameDone()) {
         return false;
      }

      return canBuild(RESOURCES_ROAD);
   }
   
   public void buildCity(int i) {
      if(!canBuildCity(i)) {
         return;
      }
      
      int bonuses = playsheet.bonusState();
      Spent spent = useResources(RESOURCES_CITY);
      playsheet.buildCity(i);
      builtResourceThisTurn = true;
      builtSounds(BuildPlan.Kind.CITY, spent, bonuses, finishIfTargetReached());
      changed();
   }
   
   public boolean canBuildCity(int i) {
      if(!canBuildCity()) {
         return false;
      }
      
      return playsheet.canBuildCity(i);
   }   
   
   public boolean canBuildCity() {
      if(rolls == 0 || isGameDone()) {
         return false;
      }

      return canBuild(RESOURCES_CITY);
   }

   /**
    * The current build plan: which positions are reachable right now (a chain of one
    * step) or after building something else first (a chain of two or more), and whether
    * paying for the chain dips into the resource bank. Recomputed only when the state
    * fingerprint changes, so repeated calls between rolls, builds and taps are cheap.
    */
   public BuildPlan getBuildPlan() {
      // A finished game offers nothing, even mid-turn (a race ends the moment it is won)
      int planRolls = isGameDone() ? 0 : rolls;
      String fp = playsheet.bankPaysBuilds + ":" + planRolls + ":" + BuildPlan.fingerprint(dice, playsheet);
      if(buildPlan == null || !fp.equals(buildPlanFingerprint)) {
         buildPlan = BuildPlan.compute(dice, playsheet, planRolls);
         buildPlanFingerprint = fp;
      }
      return buildPlan;
   }

   /**
    * The sounds of one build step: its own sound, a banked resource's tick for each one spent
    * (a gap apart, so each cuts the last off; see SoundPlayback), one coin sound if a pair of
    * gold paid, a chime if it earned a bonus, and the fanfare if it ended a race. endedRank
    * is -1 if the game did not end, otherwise the result's rank.
    */
   private void builtSounds(BuildPlan.Kind kind, Spent spent, int bonusesBefore, int endedRank) {
      sound(SoundCues.forBuild(kind), 0);
      for(int n = 0; n < spent.banked; n++) {
         sound(Sfx.BANKED, n * SoundCues.BANKED_GAP_MS);
      }
      if(spent.goldDice > 0) {
         sound(Sfx.GOLD, 0);
      }
      for(SoundCues.Cue cue : SoundCues.afterBuild(SoundCues.bonusGained(bonusesBefore, playsheet.bonusState()),
            endedRank >= 0, endedRank == 1)) {
         sound(cue.sfx, cue.delayMs);
      }
   }

   /**
    * Builds a position directly if it is affordable right now, otherwise executes the
    * recorded chain that reaches it (each step spending resources and applying its
    * playsheet effect before the next step is attempted), so a single tap on a
    * yellow-green or indigo position builds everything on the way to it.
    */
   public void build(BuildPlan.Kind kind, int index) {
      hiresThisTap = 0;
      String before = isGameDone() ? null : GameStateStore.toJson(this);
      switch(kind) {
         case ROAD:
            if(canBuildRoad(index)) {
               undoJson = before;
               buildRoad(index);
               afterBuild();
               return;
            }
            break;
         case VILLAGE:
            if(canBuildVillage(index)) {
               undoJson = before;
               buildVillage(index);
               afterBuild();
               return;
            }
            break;
         case CITY:
            if(canBuildCity(index)) {
               undoJson = before;
               buildCity(index);
               afterBuild();
               return;
            }
            break;
         case KNIGHT:
            if(canBuildKnight(index)) {
               undoJson = before;
               buildKnight(index);
               afterBuild();
               return;
            }
            break;
      }

      BuildPlan.Chain chain = getBuildPlan().chainFor(kind, index);
      if(chain == null) {
         return;
      }

      // One snapshot covers the whole chain, so one undo takes the whole tap back
      undoJson = before;

      // Each step sounds a little after the one before, so a chain is heard as a sequence
      soundBase = 0;
      try {
         buildChain(chain);
      }
      finally {
         soundBase = 0;
      }
      afterBuild();
   }

   /**
    * Trader resource classic mode: whether the banked resource in this slot may be dragged
    * onto this die right now. After a roll, before the game is over, onto a die not spent
    * on a build and not already overwritten, from a slot that holds a resource.
    */
   public boolean canOverwriteDie(int dieIndex, int slot) {
      return traderClassic && rolls >= 1 && !isGameDone()
            && dieIndex >= 0 && dieIndex < dice.length
            && dice[dieIndex].isUsable() && !dice[dieIndex].locked
            && playsheet.canUseKnightResource(slot);
   }

   /**
    * Trader resource classic mode: spends the banked resource in this slot by turning a
    * rolled die to show it. value is what the die becomes: the slot's own resource, or for
    * a "?" slot the one the player chose. The die is held and locked for the rest of the
    * turn, since the resource cost a trader to earn and one stray tap and a roll would lose
    * it. One undo takes it back, as it does a build.
    */
   public void overwriteDie(int dieIndex, int slot, Dice.Value value) {
      if(!canOverwriteDie(dieIndex, slot)) {
         return;
      }
      Dice.Value banked = playsheet.getKnightResource(slot);
      if(banked != Dice.Value.Any && value != banked) {
         return;
      }
      if(value == null || value == Dice.Value.None || value == Dice.Value.Any || value == Dice.Value.Gold) {
         return;
      }
      undoJson = GameStateStore.toJson(this);
      Dice die = dice[dieIndex];
      die.value = value;
      die.held = true;
      die.locked = true;
      playsheet.useKnightResource(slot);
      knightResources.clear();
      sound(Sfx.BANKED, 0);
      changed();
   }

   /** A build that ended the game has been recorded as its result, and stays. */
   private void afterBuild() {
      if(isGameDone()) {
         undoJson = null;
      }
   }

   /** True when the latest build tap of this turn can be taken back. */
   public boolean canUndo() {
      return undoJson != null && !isGameDone();
   }

   /**
    * Takes back the latest build tap: the pieces, the dice it spent, the rolls it
    * forfeited. One level only; the snapshot is gone afterwards.
    */
   public void undo() {
      if(!canUndo()) {
         return;
      }
      Game saved;
      try {
         saved = GameStateStore.fromJson(undoJson);
      }
      catch(org.json.JSONException e) {
         // The snapshot is our own output, so this does not happen; if it somehow did, do nothing
         undoJson = null;
         return;
      }
      undoJson = null;
      restore(saved);
      sound(Sfx.CLICK, 0);
      if(gameView != null) {
         Haptics.tap(gameView);
      }
      changed();
   }

   /**
    * Copies every field the save file holds from another game of the same map into this
    * one. Into the objects this game already has, not over them, because the view's dice
    * and the build plan hold references to them.
    */
   private void restore(Game other) {
      stateEpoch++;
      // All but the target: that is a setting the player may have changed since the
      // snapshot, and an undo takes back a build, not a choice made in Settings.
      rolls = other.rolls;
      turnsTaken = other.turnsTaken;
      builtResourceThisTurn = other.builtResourceThisTurn;
      scored = other.scored;
      knightResources.clear();
      knightResources.addAll(other.knightResources);
      for(int i = 0; i < dice.length; i++) {
         dice[i].value = other.dice[i].value;
         dice[i].held = other.dice[i].held;
         dice[i].used = other.dice[i].used;
         dice[i].locked = other.dice[i].locked;
      }
      copy(other.playsheet.traders, playsheet.traders);
      copy(other.playsheet.resourcesAvail, playsheet.resourcesAvail);
      copy(other.playsheet.roads, playsheet.roads);
      copy(other.playsheet.villages, playsheet.villages);
      copy(other.playsheet.cities, playsheet.cities);
      playsheet.turnsNothingBuilt = other.playsheet.turnsNothingBuilt;
      System.arraycopy(other.playsheet.turnScore, 0, playsheet.turnScore, 0, playsheet.turnScore.length);
   }

   private static void copy(boolean[] from, boolean[] to) {
      System.arraycopy(from, 0, to, 0, to.length);
   }

   private void buildChain(BuildPlan.Chain chain) {
      for(BuildPlan.Step step : chain.steps) {
         // A race can end partway along a chain; nothing is built after it does
         if(isGameDone()) {
            return;
         }
         switch(step.kind) {
            case ROAD:
               buildRoad(step.index);
               if(!playsheet.roads[step.index]) {
                  return;
               }
               break;
            case VILLAGE:
               buildVillage(step.index);
               if(!playsheet.villages[step.index]) {
                  return;
               }
               break;
            case CITY:
               buildCity(step.index);
               if(!playsheet.cities[step.index]) {
                  return;
               }
               break;
            case KNIGHT:
               buildKnight(step.index);
               if(!playsheet.isTraderBuilt(step.index)) {
                  return;
               }
               break;
         }
         soundBase += SoundCues.CHAIN_STAGGER_MS;
      }
   }

   /** What paying for a build took: banked resources spent, and gold dice (always in pairs). */
   static final class Spent {
      final int banked;
      final int goldDice;

      Spent(int banked, int goldDice) {
         this.banked = banked;
         this.goldDice = goldDice;
      }
   }

   public Spent useResources(Dice.Value[] resources) {
      return useResources(resources, dice, playsheet, knightResources);
   }

   static Spent useResources(Dice.Value[] resources, Dice[] dice, Playsheet playsheet, ArrayList<Dice.Value> knightResources) {
      // TODO: Cleanup how knightResources are tracked
      knightResources.clear();
      // In trader resource classic mode the bank pays for nothing by itself
      for(int i=1; i <= playsheet.bankSize() && playsheet.bankPaysBuilds; i++) {
          if(playsheet.canUseKnightResource(i)) {
              knightResources.add(playsheet.getKnightResource(i));
          }
      }

      int gold = 0;
      for(Dice die : dice) {
         if(die.isUsable() && die.getValue().equals(Dice.Value.Gold)) {
            gold++;
         }
      }

      if(gold % 2 != 0) {
         gold--;
      }

      ArrayList<Dice.Value> resList = new ArrayList<Dice.Value>();
      for(Dice.Value val : resources) {
          resList.add(val);
      }

      for(Dice.Value val : resources) {
         for(int i=0; i < dice.length; i++) {
            if(dice[i].isUsable() && dice[i].value.equals(val)) {
               dice[i].use();
               resList.remove(val);
               break;
            }
         }
      }

      ArrayList<Dice.Value> resListFound = new ArrayList<Dice.Value>();

      // Where the map says a banked resource pays for one needed resource, a slot that has
      // matched is not offered to the next one. Tarnholm keeps its old behaviour, in which
      // a castle's two grain both match a single banked grain and the second is free.
      boolean[] matched = new boolean[knightResources.size()];
      for(Dice.Value val : resList) {
          for(int i = knightResources.size() - 1; i >=0 ; i--) {
             if(knightResources.get(i).equals(val) && !(playsheet.map.bankMatchesOnce && matched[i])) {
                matched[i] = true;
                resListFound.add(val);
                break;
             }
          }
      }

      for(Dice.Value val : resListFound) {
         resList.remove(val);
      }

      int goldUsed = 0;
      // How many bank slots standing for any resource pay for what the dice and the exact
      // bank slots did not cover and gold cannot
      int wildcardsNeeded = 0;

      if(gold / 2 == resList.size()) {
         goldUsed = gold;
      }
      else if(gold / 2 > resList.size()) {
         goldUsed = resList.size() * 2;
         for(int i=0; i < gold / 2 - resList.size() && resListFound.size() > 0; i++) {
            resListFound.remove(0);
            goldUsed += 2;
         }
      }
      else if(gold / 2 < resList.size()) {
         wildcardsNeeded = resList.size() - gold / 2;
         goldUsed = gold;
      }


      int goldPaid = goldUsed;
      int bankedSpent = 0;
      for(int i=0; i < dice.length && goldUsed > 0; i++) {
         if(dice[i].isUsable() && dice[i].value.equals(Dice.Value.Gold)) {
            dice[i].use();
            goldUsed--;
         }
      }

      for(Dice.Value val : resListFound) {
         for(int i = knightResources.size() - 1; i >=0 ; i--) {
            if(knightResources.get(i).equals(val)) {
               int index = playsheet.firstAvailableSlot(knightResources.get(i));
               playsheet.useKnightResource(index);
               knightResources.remove(i);
               bankedSpent++;
               break;
            }
         }
      }
      // Each open wildcard slot is spent once, as many as the build needs. canBuildStatus
      // offers a build on the same count, so what is offered is what is paid. Tarnholm has
      // one such slot, so for it this is the single spend it always was.
      for(int n = 0; n < wildcardsNeeded; n++) {
          for(int i = knightResources.size() - 1; i >=0 ; i--) {
              if(knightResources.get(i).equals(Dice.Value.Any)) {
                  int index = playsheet.firstAvailableSlot(knightResources.get(i));
                  playsheet.useKnightResource(index);
                  knightResources.remove(i);
                  bankedSpent++;
                  break;
              }
          }
      }
      return new Spent(bankedSpent, goldPaid);
   }

   // Build status returned by canBuildStatus(): whether a recipe is affordable at all,
   // and if so, whether paying for it would spend a banked knight resource rather than
   // just the dice sitting in front of the player this turn.
   public static final int BUILD_NONE = 0;
   public static final int BUILD_WITH_DICE = 1;
   public static final int BUILD_WITH_KNIGHT = 2;

   public boolean canBuild(Dice.Value[] resourcesRequired) {
      return canBuildStatus(resourcesRequired) != BUILD_NONE;
   }

   public int canBuildStatus(Dice.Value[] resourcesRequired) {
      return canBuildStatus(resourcesRequired, dice, playsheet, knightResources);
   }

   static int canBuildStatus(Dice.Value[] resourcesRequired, Dice[] dice, Playsheet playsheet, ArrayList<Dice.Value> knightResources) {
      boolean[] diceUsed = new boolean[dice.length];

      // TODO: Cleanup how knightResources are tracked
      knightResources.clear();
      // In trader resource classic mode the bank pays for nothing by itself
      for(int i=1; i <= playsheet.bankSize() && playsheet.bankPaysBuilds; i++) {
          if(playsheet.canUseKnightResource(i)) {
              knightResources.add(playsheet.getKnightResource(i));
          }
      }

      boolean[] knightResourcesUsed = new boolean[knightResources.size()];
      boolean usedKnightResource = false;
      int gold = 0;
      for(Dice die : dice) {
         if(die.isUsable() && die.getValue().equals(Dice.Value.Gold)) {
            gold++;
         }
      }
      int unfound = 0;
      for(Dice.Value val : resourcesRequired) {
         boolean found = false;
         for(int i=0; i < dice.length; i++) {
            if(dice[i].isUsable() && !diceUsed[i] && dice[i].value.equals(val)) {
               diceUsed[i] = true;
               found = true;
               break;
            }
         }
         if(!found) {
            for(int i=0; i < knightResources.size(); i++) {
               if(!knightResourcesUsed[i] && knightResources.get(i).equals(val)) {
                  knightResourcesUsed[i] = true;
                  found = true;
                  usedKnightResource = true;
                  break;
               }
            }
         }
         if(!found) {
             for(int i=0; i < knightResources.size(); i++) {
                if(!knightResourcesUsed[i] && knightResources.get(i).equals(Dice.Value.Any)) {
                   knightResourcesUsed[i] = true;
                   found = true;
                   usedKnightResource = true;
                   break;
                }
             }
          }

         if(!found) {
            unfound++;
         }
      }

      if(unfound > gold / 2) {
          return BUILD_NONE;
      }
      return usedKnightResource ? BUILD_WITH_KNIGHT : BUILD_WITH_DICE;
   }

   // Per-recipe status, mirroring the rolls==0 guard each canBuildX() already applies.
   // The board highlighting reads these to decide green (dice) vs blue (knight resource)
   // without re-deriving the affordability logic itself; canBuildStatus() above is the
   // single source of truth both this and canBuild() delegate to.
   public int getRoadBuildStatus() {
      if(rolls == 0 || isGameDone()) {
         return BUILD_NONE;
      }
      return canBuildStatus(RESOURCES_ROAD);
   }

   public int getVillageBuildStatus() {
      if(rolls == 0 || isGameDone()) {
         return BUILD_NONE;
      }
      return canBuildStatus(RESOURCES_VILLAGE);
   }

   public int getCityBuildStatus() {
      if(rolls == 0 || isGameDone()) {
         return BUILD_NONE;
      }
      return canBuildStatus(RESOURCES_CITY);
   }

   public int getKnightBuildStatus() {
      if(rolls == 0 || isGameDone()) {
         return BUILD_NONE;
      }
      return canBuildStatus(RESOURCES_KNIGHT);
   }
   
   /** Out of turns (on a map with a cap), or (on a map that has one) at the target score. */
   public boolean isGameDone() {
       return (map.turnCap > 0 && turnsTaken >= map.turnCap)
             || (targetScore > 0 && playsheet.getScore() >= targetScore);
   }
}
