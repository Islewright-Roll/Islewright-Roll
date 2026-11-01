package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/**
 * The undo button: one build tap taken back, within the turn, until anything moves the game
 * on. The proof of "exactly as it was" is the save file's JSON, which lists every field the
 * game persists. Games here have no view, so nothing is written to disk.
 */
public class GameUndoTest {
   private static final Dice.Value B = Dice.Value.Brick;
   private static final Dice.Value L = Dice.Value.Lumber;

   private static String json(Game game) {
      return GameStateStore.toJson(game);
   }

   @Test
   public void aFreshGameHasNothingToUndo() {
      Game game = new Game(MapDef.TARNHOLM);
      assertFalse(game.canUndo());
      String before = json(game);
      game.undo();
      assertEquals(before, json(game));
   }

   @Test
   public void undoTakesBackARoadAndTheDiceItSpent() {
      Game game = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(game, B, L, Dice.Value.Wool, Dice.Value.Grain);
      game.dice[2].held = true;
      String before = json(game);
      Dice[] diceObjects = game.dice.clone();
      boolean[] roadsObject = game.playsheet.roads;

      game.build(BuildPlan.Kind.ROAD, 1);
      assertTrue(game.playsheet.roads[1]);
      assertTrue(game.builtResourceThisTurn);
      assertTrue(game.canUndo());

      game.undo();

      assertEquals(before, json(game));
      assertFalse(game.playsheet.roads[1]);
      assertFalse(game.canUndo());
      // The view's dice and the sheet's arrays are the same objects, holding the old values
      for(int i = 0; i < diceObjects.length; i++) {
         assertSame(diceObjects[i], game.dice[i]);
      }
      assertSame(roadsObject, game.playsheet.roads);
      assertTrue(game.dice[2].held);
      assertTrue(game.dice[0].isUsable());
   }

   /** Q2: a build on roll one forfeits the other rolls; undoing it gives them back. */
   @Test
   public void undoAfterAnEarlyBuildGivesTheForfeitedRollsBack() {
      Game game = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(game, B, L);
      assertEquals(1, game.rolls);
      assertTrue(game.wouldForfeitRolls());
      assertTrue(game.canRoll());

      game.build(BuildPlan.Kind.ROAD, 1);
      assertFalse("building ends the roll phase", game.canRoll());

      game.undo();
      assertEquals(1, game.rolls);
      assertTrue("the player can roll again", game.canRoll());
      assertTrue(game.wouldForfeitRolls());
   }

   @Test
   public void aChainBuildIsTakenBackWholeByOneUndo() {
      Game game = new Game(MapDef.DUNHOLM);
      GameRaceTest.giveDice(game, B, L, B, L, B, L);
      String before = json(game);

      game.build(BuildPlan.Kind.ROAD, 4);
      assertTrue("roads 1, 3 and 4", game.playsheet.roads[1] && game.playsheet.roads[3] && game.playsheet.roads[4]);

      game.undo();

      assertEquals(before, json(game));
      assertFalse(game.playsheet.roads[1] || game.playsheet.roads[3] || game.playsheet.roads[4]);
      assertFalse(game.canUndo());
   }

   @Test
   public void aBuildThatChangesNothingKeepsTheEarlierSnapshot() {
      Game game = new Game(MapDef.DUNHOLM);
      GameRaceTest.giveDice(game, B, L);
      String before = json(game);
      game.build(BuildPlan.Kind.ROAD, 1);
      // Nothing is left to pay for road 5, and no chain reaches it
      game.build(BuildPlan.Kind.ROAD, 5);
      assertFalse(game.playsheet.roads[5]);
      assertTrue(game.canUndo());
      game.undo();
      assertEquals(before, json(game));
   }

   @Test
   public void twoBuildsThenOneUndoRevertsOnlyTheSecond() {
      Game game = new Game(MapDef.DUNHOLM);
      GameRaceTest.giveDice(game, B, L, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      String afterFirst = json(game);
      game.build(BuildPlan.Kind.ROAD, 3);
      assertTrue(game.playsheet.roads[1] && game.playsheet.roads[3]);

      game.undo();

      assertEquals(afterFirst, json(game));
      assertTrue(game.playsheet.roads[1]);
      assertFalse(game.playsheet.roads[3]);
      assertFalse("one level only", game.canUndo());
   }

   @Test
   public void endingTheTurnClearsTheUndo() {
      Game game = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(game, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      assertTrue(game.canUndo());
      game.newTurn(false);
      assertFalse(game.canUndo());
      String after = json(game);
      game.undo();
      assertEquals(after, json(game));
      assertTrue(game.playsheet.roads[1]);
   }

   /** A roll replaces the dice the snapshot holds; undoing across one would be a free re-roll. */
   @Test
   public void aRollClearsTheUndo() {
      Game game = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(game, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      assertTrue(game.canUndo());
      game.roll();
      assertFalse(game.canUndo());

      Game other = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(other, B, L);
      other.build(BuildPlan.Kind.ROAD, 1);
      other.endTurnAndRoll();
      assertFalse(other.canUndo());
   }

   @Test
   public void aNewGameClearsTheUndo() {
      Game game = new Game(MapDef.TARNHOLM);
      GameRaceTest.giveDice(game, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      game.reset();
      assertFalse(game.canUndo());

      GameRaceTest.giveDice(game, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      assertTrue(game.canUndo());
      game.startNewGame(MapDef.DUNHOLM);
      assertFalse(game.canUndo());
   }

   @Test
   public void aBuildThatWinsARaceCannotBeUndone() {
      Game game = new Game(MapDef.DUNHOLM);
      GameRaceTest.scoreEight(game);
      GameRaceTest.emptyTurns(game, 6);
      GameRaceTest.giveDice(game, Dice.Value.Grain, Dice.Value.Grain, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);

      game.build(BuildPlan.Kind.CITY, 1);

      assertTrue(game.isGameDone());
      assertTrue(game.scored);
      assertFalse(game.canUndo());
      String done = json(game);
      game.undo();
      assertEquals("the recorded result stands", done, json(game));
      assertTrue(game.playsheet.cities[1]);
      assertEquals(7, game.resultToRecord());
   }

   @Test
   public void aChainThatWinsARaceCannotBeUndoneEither() {
      Game game = new Game(MapDef.DUNHOLM);
      Playsheet sheet = game.playsheet;
      for(int i = 0; i < 6; i++) {
         sheet.villages[i] = true;
      }
      sheet.cities[0] = true;
      sheet.roads[1] = true;
      GameRaceTest.giveDice(game, B, L, B, L, B, L);
      game.build(BuildPlan.Kind.ROAD, 6);
      assertTrue(game.isGameDone());
      assertFalse(game.canUndo());
   }

   @Test
   public void aBuildThatDoesNotWinARaceCanBeUndone() {
      Game game = new Game(MapDef.DUNHOLM);
      GameRaceTest.scoreEight(game);
      GameRaceTest.giveDice(game, B, L);
      game.build(BuildPlan.Kind.ROAD, 1);
      assertFalse(game.isGameDone());
      assertTrue(game.canUndo());
   }
}
