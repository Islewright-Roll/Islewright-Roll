package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/**
 * How a game ends. Tarnholm after fifteen turns with its score; Dunholm the moment the
 * score reaches ten, with the turns taken as the result, and never by running out of
 * turns. A race with a cap (RaceFixture.CAPPED) ends after twenty turns with nothing to
 * record. Games here have no view, so nothing is written to disk; what would be
 * recorded is read from resultToRecord() and whether it was is read from `scored`.
 */
public class GameRaceTest {
   /** Six villages and a castle: eight points, short of the target, with road 12 down for castle 1. */
   static void scoreEight(Game game) {
      Playsheet sheet = game.playsheet;
      for(int i = 0; i < 6; i++) {
         sheet.villages[i] = true;
      }
      sheet.cities[0] = true;
      sheet.roads[12] = true;
      assertEquals(8, sheet.getScore());
   }

   static void giveDice(Game game, Dice.Value... values) {
      for(int i = 0; i < game.dice.length; i++) {
         game.dice[i].used = false;
         game.dice[i].value = i < values.length ? values[i] : Dice.Value.None;
      }
      game.rolls = 1;
   }

   static void emptyTurns(Game game, int n) {
      for(int i = 0; i < n; i++) {
         game.newTurn(false);
      }
   }

   @Test
   public void reachingTenOnTurnSevenEndsTheGameWithSeven() {
      Game game = new Game(MapDef.DUNHOLM);
      scoreEight(game);
      emptyTurns(game, 6);
      assertEquals(6, game.turnsTaken);
      assertFalse(game.isGameDone());

      giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);

      assertEquals(10, game.playsheet.getScore());
      assertTrue(game.isGameDone());
      assertEquals(7, game.turnsTaken);
      assertTrue(game.scored);
      assertEquals(7, game.resultToRecord());
      assertEquals("turn 7 is scored as a turn", 2, game.playsheet.getTurnScore(7));
      assertFalse(game.canRoll());
   }

   @Test
   public void nineIsNotEnough() {
      Game game = new Game(MapDef.DUNHOLM);
      scoreEight(game);
      game.playsheet.villages[6] = true;
      assertEquals(9, game.playsheet.getScore());
      assertFalse(game.isGameDone());
      assertFalse(game.scored);
      assertEquals("no result until the target is met", 0, game.resultToRecord());
   }

   @Test
   public void aFinishedGameDoesNotMoveOnAndIsNotRecordedTwice() {
      Game game = new Game(MapDef.DUNHOLM);
      scoreEight(game);
      emptyTurns(game, 6);
      giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      long stamp = game.gameOverAt;
      game.newTurn(false);
      game.newTurn(false);
      assertEquals(7, game.turnsTaken);
      assertEquals(7, game.resultToRecord());
      assertEquals(stamp, game.gameOverAt);
      // Nothing more can be built on the finished board, though dice remain
      giveDice(game, Dice.Value.Brick, Dice.Value.Lumber);
      assertFalse(game.canBuildRoad());
      assertFalse(game.canBuildRoad(2));
      assertEquals(Game.BUILD_NONE, game.getRoadBuildStatus());
      game.buildRoad(1);
      assertFalse(game.playsheet.roads[1]);
      assertEquals("the plan is empty on a finished game", null, game.getBuildPlan().chainFor(BuildPlan.Kind.ROAD, 1));
   }

   /** Q1: the target is reached on the first step of a two step chain; the second is not built. */
   @Test
   public void aChainStopsCleanlyAfterTheStepThatWins() {
      Game game = new Game(MapDef.DUNHOLM);
      Playsheet sheet = game.playsheet;
      for(int i = 0; i < 6; i++) {
         sheet.villages[i] = true;
      }
      sheet.cities[0] = true;
      sheet.roads[1] = true;
      sheet.roads[3] = true;
      sheet.roads[4] = true;
      assertEquals(8, sheet.getScore());
      // Road 6 (brick, lumber) is the long road and wins; village 6 (lumber, brick, wool,
      // grain) would follow it and would score an eleventh point
      giveDice(game, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Lumber, Dice.Value.Brick,
            Dice.Value.Wool, Dice.Value.Grain);
      assertFalse(game.canBuildVillage(6));
      BuildPlan.Chain chain = game.getBuildPlan().chainFor(BuildPlan.Kind.VILLAGE, 6);
      assertEquals("road 6 then village 6", 2, chain.steps.size());
      assertEquals(BuildPlan.Kind.ROAD, chain.steps.get(0).kind);
      assertEquals(6, chain.steps.get(0).index);

      game.build(BuildPlan.Kind.VILLAGE, 6);

      assertTrue(sheet.roads[6]);
      assertEquals(10, sheet.getScore());
      assertFalse("nothing is built after the game is won", sheet.villages[6]);
      assertEquals("the second step's dice are untouched", 4, usableDice(game));
      assertTrue(game.isGameDone());
      assertEquals(1, game.turnsTaken);
      assertEquals(1, game.resultToRecord());
      assertTrue(game.scored);
   }

   /** A three step chain that wins on its last step runs all three. */
   @Test
   public void aChainThatWinsOnItsLastStepRunsWhole() {
      Game game = new Game(MapDef.DUNHOLM);
      Playsheet sheet = game.playsheet;
      for(int i = 0; i < 6; i++) {
         sheet.villages[i] = true;
      }
      sheet.cities[0] = true;
      sheet.roads[1] = true;
      giveDice(game, Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Lumber,
            Dice.Value.Brick, Dice.Value.Lumber);
      game.build(BuildPlan.Kind.ROAD, 6);
      assertTrue(sheet.roads[3] && sheet.roads[4] && sheet.roads[6]);
      assertTrue(game.isGameDone());
      assertEquals(1, game.turnsTaken);
   }

   private static int usableDice(Game game) {
      int n = 0;
      for(Dice die : game.dice) {
         if(die.isUsable() && die.value != Dice.Value.None) {
            n++;
         }
      }
      return n;
   }

   @Test
   public void theCapEndsTheRaceWithNothingToRecord() {
      Game game = new Game(RaceFixture.CAPPED);
      scoreEight(game);
      emptyTurns(game, 19);
      assertFalse(game.isGameDone());
      assertEquals(19, game.turnsTaken);
      game.newTurn(false);
      assertTrue(game.isGameDone());
      assertEquals(20, game.turnsTaken);
      assertFalse("nothing recorded", game.scored);
      assertEquals(0, game.resultToRecord());
      emptyTurns(game, 3);
      assertEquals("never past the cap", 20, game.turnsTaken);
      assertFalse(game.scored);
   }

   @Test
   public void reachingTenOnTheLastTurnStillCounts() {
      Game game = new Game(RaceFixture.CAPPED);
      scoreEight(game);
      emptyTurns(game, 19);
      giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      assertTrue(game.isGameDone());
      assertEquals(20, game.turnsTaken);
      assertEquals(20, game.resultToRecord());
      assertTrue(game.scored);
   }

   @Test
   public void aNewGameStartsFresh() {
      Game game = new Game(MapDef.DUNHOLM);
      scoreEight(game);
      emptyTurns(game, 6);
      giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      assertTrue(game.isGameDone());
      game.reset();
      assertFalse(game.isGameDone());
      assertFalse(game.scored);
      assertEquals(0, game.turnsTaken);
      assertEquals(0, game.playsheet.getScore());
   }

   @Test
   public void tarnholmStillEndsAtFifteenWithItsScore() {
      Game game = new Game(MapDef.TARNHOLM);
      game.playsheet.roads[1] = true;
      game.playsheet.roads[2] = true;
      game.playsheet.villages[0] = true;
      assertTrue(game.playsheet.getScore() > 0);
      // Turns in which something was built, so no empty turn is charged
      for(int i = 0; i < 14; i++) {
         game.builtResourceThisTurn = true;
         game.newTurn(false);
      }
      assertFalse(game.isGameDone());
      assertFalse(game.scored);
      game.builtResourceThisTurn = true;
      game.newTurn(false);
      assertTrue(game.isGameDone());
      assertEquals(15, game.turnsTaken);
      assertTrue("the record path was taken", game.scored);
      assertEquals(game.playsheet.getScore(), game.resultToRecord());
      assertTrue(game.resultToRecord() > 0);
   }

   @Test
   public void tarnholmNeverEndsMidTurnOnScore() {
      Game game = new Game(MapDef.TARNHOLM);
      for(int i = 0; i < game.playsheet.villages.length; i++) {
         game.playsheet.villages[i] = true;
      }
      for(int i = 0; i < game.playsheet.cities.length; i++) {
         game.playsheet.cities[i] = true;
      }
      assertTrue(game.playsheet.getScore() > 10);
      assertFalse(game.isGameDone());
      giveDice(game, Dice.Value.Brick, Dice.Value.Lumber);
      game.playsheet.roads[1] = true;
      game.buildRoad(2);
      assertTrue(game.playsheet.roads[2]);
      assertFalse(game.isGameDone());
      assertFalse(game.scored);
      assertEquals(0, game.turnsTaken);
   }

   @Test
   public void aTarnholmGameWithNoPointsStillCountsAsScored() {
      Game game = new Game(MapDef.TARNHOLM);
      emptyTurns(game, 15);
      assertTrue(game.isGameDone());
      assertTrue(game.scored);
      assertEquals(0, game.resultToRecord());
   }

   @Test
   public void theLastTurnComesFromTheMap() {
      // What UIDiceRoll tests to show "Finish": turnsTaken + 1 == map.turnCap
      assertEquals(15, MapDef.TARNHOLM.turnCap);
      assertEquals(20, RaceFixture.CAPPED.turnCap);
      Game game = new Game(RaceFixture.CAPPED);
      emptyTurns(game, 14);
      assertFalse(game.turnsTaken + 1 == game.map.turnCap);
      emptyTurns(game, 5);
      assertTrue(game.turnsTaken + 1 == game.map.turnCap);

      // Dunholm has no cap, so no turn is ever its last
      Game dunholm = new Game(MapDef.DUNHOLM);
      for(int turn = 0; turn < 60; turn++) {
         assertFalse(dunholm.turnsTaken + 1 == dunholm.map.turnCap);
         dunholm.newTurn(false);
      }
   }

   @Test
   public void dunholmNeverRunsOutOfTurns() {
      Game game = new Game(MapDef.DUNHOLM);
      scoreEight(game);
      emptyTurns(game, 60);
      assertFalse(game.isGameDone());
      assertEquals(60, game.turnsTaken);
      assertFalse(game.scored);
      assertEquals("every turn is scored, past the old cap too", 0, game.playsheet.getTurnScore(60));
      giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      game.buildCity(1);
      assertTrue(game.isGameDone());
      assertEquals(61, game.turnsTaken);
      assertEquals(61, game.resultToRecord());
      assertEquals(2, game.playsheet.getTurnScore(61));
   }

   /** Q2: the chain's trader step reads the sheet through isTraderBuilt. */
   @Test
   public void aTraderChainBuildsEachTraderInOrderOnTarnholm() {
      Game game = new Game(MapDef.TARNHOLM);
      giveDice(game, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.build(BuildPlan.Kind.KNIGHT, 2);
      assertTrue(game.playsheet.isTraderBuilt(1));
      assertTrue(game.playsheet.isTraderBuilt(2));
      assertFalse(game.playsheet.isTraderBuilt(3));
   }

   @Test
   public void aDunholmTraderIsOneStepWhicheverTier() {
      Game game = new Game(MapDef.DUNHOLM);
      giveDice(game, Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.build(BuildPlan.Kind.KNIGHT, 9);
      assertTrue(game.playsheet.isTraderBuilt(9));
      assertFalse(game.playsheet.isTraderBuilt(1));
      assertEquals(1, game.playsheet.traderCount());
   }
}
