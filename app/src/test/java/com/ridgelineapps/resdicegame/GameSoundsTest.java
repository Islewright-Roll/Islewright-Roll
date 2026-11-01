package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;

import java.util.ArrayList;
import java.util.List;

import org.junit.Test;

/** Which sounds the game asks for, and when, for what the player does. Games here have no view. */
public class GameSoundsTest {

   private static final class Recorder implements SoundSink {
      final List<String> log = new ArrayList<String>();

      @Override
      public void play(Sfx sfx, int delayMs) {
         log.add(sfx + "@" + delayMs);
      }

      String take() {
         String s = log.toString();
         log.clear();
         return s;
      }
   }

   private static Game game(MapDef map, Recorder recorder) {
      Game game = new Game(map);
      game.sounds = recorder;
      return game;
   }

   private static void dice(Game game, Dice.Value... values) {
      GameRaceTest.giveDice(game, values);
   }

   @Test
   public void aGameStartsInSilence() {
      Recorder r = new Recorder();
      game(MapDef.TARNHOLM, r);
      assertEquals("[]", r.take());
   }

   @Test
   public void rollingSoundsTheRattleAndHoldingADieTheTick() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      game.diceTouched(0);
      assertEquals("before the first roll a tap does nothing, silently", "[]", r.take());
      game.roll();
      assertEquals("[ROLL@0]", r.take());
      game.diceTouched(2);
      assertEquals("[DIE@0]", r.take());
      game.diceTouched(2);
      assertEquals("releasing sounds too", "[DIE@0]", r.take());
   }

   @Test
   public void aTapOutOfRangeSoundsNothing() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      game.roll();
      r.take();
      game.diceTouched(9);
      assertEquals("[]", r.take());
   }

   @Test
   public void endingTheTurnAndRollingInOneTapSoundsBothApart() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      game.builtResourceThisTurn = true;
      game.rolls = 3;
      game.endTurnAndRoll();
      assertEquals("[TURN_END@0, ROLL@250]", r.take());
   }

   @Test
   public void eachKindOfBuildHasItsOwnSound() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      java.util.Arrays.fill(game.playsheet.roads, true);
      game.playsheet.roads[1] = false;

      dice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.buildRoad(1);
      assertEquals("[ROAD@0]", r.take());

      dice(game, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Wool, Dice.Value.Grain);
      game.buildVillage(0);
      assertEquals("[VILLAGE@0]", r.take());

      dice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(0);
      assertEquals("[CASTLE@0]", r.take());

      dice(game, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.buildKnight(1);
      assertEquals("[TRADER@0]", r.take());
   }

   @Test
   public void aBuildThatCannotBeDoneIsSilent() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      dice(game, Dice.Value.Brick);
      game.buildRoad(1);
      game.buildCity(0);
      assertEquals("[]", r.take());
   }

   /** Dunholm's bank: slot 2 is grain, slot 3 wool. A village from lumber and brick on the dice takes both. */
   @Test
   public void twoBankedResourcesInOneBuildAreTwoRetriggeredTicksNotOneStack() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      game.playsheet.setTraderCount(3);
      game.playsheet.roads[17] = true;
      dice(game, Dice.Value.Lumber, Dice.Value.Brick);
      game.buildVillage(0);
      assertEquals("[VILLAGE@0, BANKED@0, BANKED@110]", r.take());
      assertEquals("both really were spent", false, game.playsheet.canUseKnightResource(2) || game.playsheet.canUseKnightResource(3));
   }

   @Test
   public void aPairOfGoldPayingForSomethingIsOneCoinSound() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      dice(game, Dice.Value.Brick, Dice.Value.Gold, Dice.Value.Gold);
      game.buildRoad(1);
      assertEquals("[ROAD@0, GOLD@0]", r.take());
   }

   @Test
   public void goldThatPaysNothingIsSilent() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      dice(game, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Gold, Dice.Value.Gold);
      game.buildRoad(1);
      assertEquals("[ROAD@0]", r.take());
   }

   @Test
   public void aChainBuildSoundsEachStepNinetyMillisecondsAfterTheLast() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      dice(game, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Lumber);
      BuildPlan.Chain chain = game.getBuildPlan().chainFor(BuildPlan.Kind.ROAD, 4);
      assertEquals("roads 1, 3 and 4", 3, chain.steps.size());
      game.build(BuildPlan.Kind.ROAD, 4);
      assertEquals("[ROAD@0, ROAD@90, ROAD@180]", r.take());
      // The stagger does not leak into the next build
      dice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.buildRoad(5);
      assertEquals("[ROAD@0]", r.take());
   }

   @Test
   public void aSingleStepBuildThroughTheChainEntryIsNotStaggered() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      dice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.build(BuildPlan.Kind.ROAD, 1);
      assertEquals("[ROAD@0]", r.take());
   }

   /** The undo is one click; the build's own sounds are not repeated or reversed. */
   @Test
   public void anUndoSoundsOneClick() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      dice(game, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Lumber);
      game.undo();
      assertEquals("nothing to undo, silently", "[]", r.take());
      game.build(BuildPlan.Kind.ROAD, 4);
      assertEquals("[ROAD@0, ROAD@90, ROAD@180]", r.take());
      game.undo();
      assertEquals("[CLICK@0]", r.take());
      game.undo();
      assertEquals("one level only", "[]", r.take());
   }

   @Test
   public void theThirdTraderOnTheSecondIslandEarnsTheChime() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      game.playsheet.setTraderCount(2);
      dice(game, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.buildKnight(3);
      assertEquals("[TRADER@0, BONUS@250]", r.take());
      // A fourth trader earns nothing more
      dice(game, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.buildKnight(4);
      assertEquals("[TRADER@0]", r.take());
   }

   @Test
   public void theLongRoadOnTheSecondIslandEarnsTheChime() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      game.playsheet.roads[4] = true;
      dice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.buildRoad(6);
      assertEquals("[ROAD@0, BONUS@250]", r.take());
   }

   @Test
   public void theFirstIslandHasNoBonusToChime() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      java.util.Arrays.fill(game.playsheet.roads, true);
      game.playsheet.roads[MapDef.DUNHOLM.longRoad] = false;
      dice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.buildRoad(MapDef.DUNHOLM.longRoad);
      assertEquals("[ROAD@0]", r.take());
   }

   @Test
   public void anEmptyTurnOnTheFirstIslandIsRueful() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      game.newTurn(false);
      assertEquals("[EMPTY_TURN@0]", r.take());
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals("a turn with a build is a plain turn end", "[TURN_END@0]", r.take());
   }

   @Test
   public void anEmptyTurnOnTheSecondIslandCostsNothingSoItIsAPlainTurnEnd() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      game.newTurn(false);
      assertEquals("[TURN_END@0]", r.take());
   }

   @Test
   public void theShipAppearsOnTheTurnThatLeavesThreeAndNotBefore() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      int cap = MapDef.TARNHOLM.turnCap;
      for(int turn = 1; turn <= cap - 4; turn++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
         assertEquals("turn " + turn, "[TURN_END@0]", r.take());
      }
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals("three turns remain", "[TURN_END@0, SHIP_APPEARS@300]", r.take());
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals("two remain: no second horn", "[TURN_END@0]", r.take());
   }

   @Test
   public void theBellRingsOnTheTurnEndThatStartsTheLastTurn() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      int cap = MapDef.TARNHOLM.turnCap;
      // Play until two turns remain; the next turn end leaves one
      for(int turn = 1; turn <= cap - 2; turn++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
      }
      r.take();
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals(false, game.isGameDone());
      assertEquals("one remains: the ship has moored", "[TURN_END@0, SHIP_DOCKS@300]", r.take());
   }

   @Test
   public void theLastTurnEndsWithTheShortFanfareAndNoBell() {
      Recorder r = new Recorder();
      Game game = game(MapDef.TARNHOLM, r);
      int cap = MapDef.TARNHOLM.turnCap;
      for(int turn = 1; turn < cap; turn++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
      }
      r.take();
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals(true, game.isGameDone());
      assertEquals("[TURN_END@0, GAME_OVER@300]", r.take());
      // A finished game is silent when poked
      game.newTurn(false);
      assertEquals("[]", r.take());
   }

   @Test
   public void aNewBestPlaysTheBiggerFanfareInsteadOfTheShortOne() {
      Recorder r = new Recorder();
      Game game = new Game(MapDef.TARNHOLM) {
         @Override
         int recordScore(int result) {
            return 1;
         }
      };
      game.sounds = r;
      game.playsheet.villages[0] = true;
      int cap = MapDef.TARNHOLM.turnCap;
      for(int turn = 1; turn < cap; turn++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
      }
      r.take();
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals("[TURN_END@0, TOP_SCORE@300]", r.take());
   }

   @Test
   public void aGoodButNotBestResultStillGetsTheShortFanfare() {
      Recorder r = new Recorder();
      Game game = new Game(MapDef.TARNHOLM) {
         @Override
         int recordScore(int result) {
            return 3;
         }
      };
      game.sounds = r;
      game.playsheet.villages[0] = true;
      for(int turn = 1; turn < MapDef.TARNHOLM.turnCap; turn++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
      }
      r.take();
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertEquals("[TURN_END@0, GAME_OVER@300]", r.take());
   }

   @Test
   public void aRaceWonByABuildIsTheBuildSoundThenTheFanfareAndNoDock() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      GameRaceTest.scoreEight(game);
      GameRaceTest.emptyTurns(game, 6);
      r.take();
      GameRaceTest.giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      assertEquals(true, game.isGameDone());
      assertEquals("[CASTLE@0, GAME_OVER@300]", r.take());
   }

   @Test
   public void aRaceWonWithANewBestIsTheBigFanfare() {
      Recorder r = new Recorder();
      Game game = new Game(MapDef.DUNHOLM) {
         @Override
         int recordScore(int result) {
            return 1;
         }
      };
      game.sounds = r;
      GameRaceTest.scoreEight(game);
      GameRaceTest.emptyTurns(game, 6);
      r.take();
      GameRaceTest.giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      assertEquals("[CASTLE@0, TOP_SCORE@300]", r.take());
   }

   @Test
   public void aRaceThatRunsOutOfTurnsSoundsTheShortFanfareWithNoBell() {
      Recorder r = new Recorder();
      Game game = game(RaceFixture.CAPPED, r);
      int cap = RaceFixture.CAPPED.turnCap;
      for(int turn = 1; turn < cap; turn++) {
         game.newTurn(false);
      }
      r.take();
      game.newTurn(false);
      assertEquals("[TURN_END@0, GAME_OVER@300]", r.take());
   }

   /** Dunholm has no cap: however long the race runs, a turn end is only a turn end, with no ship or bell. */
   @Test
   public void aRaceWithNoCapNeverSoundsTheShipOrTheBell() {
      Recorder r = new Recorder();
      Game game = game(MapDef.DUNHOLM, r);
      for(int turn = 1; turn <= 40; turn++) {
         r.take();
         game.newTurn(false);
         assertEquals("turn " + turn, "[TURN_END@0]", r.take());
      }
   }
}
