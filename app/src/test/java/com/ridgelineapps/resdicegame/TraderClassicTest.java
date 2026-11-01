package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;

import org.junit.Test;

/**
 * Trader resource classic mode: the bank pays for no build by itself, and a banked
 * resource is spent only by dragging it onto a rolled die (Game.overwriteDie), which turns
 * to show it and stays for the rest of the turn.
 */
public class TraderClassicTest {

   /** Tarnholm with every trader hired and the whole bank open, after a first roll of these dice. */
   private static Game game(boolean classic, Dice.Value... values) {
      Game game = new Game(MapDef.TARNHOLM);
      game.setTraderClassic(classic);
      game.playsheet.setTraderCount(6);
      game.rolls = 1;
      for(int i = 0; i < 6; i++) {
         game.dice[i].value = values[i];
      }
      return game;
   }

   private static Game wools(boolean classic) {
      return game(classic, Dice.Value.Wool, Dice.Value.Wool, Dice.Value.Wool, Dice.Value.Wool, Dice.Value.Wool, Dice.Value.Wool);
   }

   @Test
   public void withTheModeOffTheBankStillPaysForARoad() {
      assertEquals(Game.BUILD_WITH_KNIGHT, wools(false).canBuildStatus(Game.RESOURCES_ROAD));
   }

   @Test
   public void inClassicModeTheBankPaysForNothingByItself() {
      Game game = wools(true);
      assertEquals(Game.BUILD_NONE, game.canBuildStatus(Game.RESOURCES_ROAD));
      assertFalse(game.canBuildRoad(1));
      // The plan's copies of the sheet carry the rule, so it offers nothing from the bank either
      assertFalse(game.playsheet.copy().bankPaysBuilds);
      assertEquals(null, game.getBuildPlan().chainFor(BuildPlan.Kind.ROAD, 1));
      // And building anyway spends nothing from the bank
      game.buildRoad(1);
      assertFalse(game.playsheet.roads[1]);
      for(int slot = 1; slot <= 6; slot++) {
         assertTrue(game.playsheet.canUseKnightResource(slot));
      }
   }

   @Test
   public void turningTheModeOffLetsTheBankPayAgain() {
      Game game = wools(true);
      game.setTraderClassic(false);
      assertEquals(Game.BUILD_WITH_KNIGHT, game.canBuildStatus(Game.RESOURCES_ROAD));
   }

   @Test
   public void overwritingADieSpendsTheSlotAndLocksTheDie() {
      Game game = wools(true);
      game.overwriteDie(0, 5, Dice.Value.Brick);
      assertEquals(Dice.Value.Brick, game.dice[0].value);
      assertTrue(game.dice[0].held);
      assertTrue(game.dice[0].locked);
      assertFalse(game.playsheet.canUseKnightResource(5));
      // A tap does not let it go, and a roll leaves it as it is
      game.diceTouched(0);
      assertTrue(game.dice[0].isHeld());
      game.roll();
      assertEquals(Dice.Value.Brick, game.dice[0].value);
      // A locked die is not a target a second time
      assertFalse(game.canOverwriteDie(0, 4));
   }

   @Test
   public void theDieThenPaysForABuildLikeAnyOther() {
      Game game = wools(true);
      game.overwriteDie(0, 5, Dice.Value.Brick);
      game.overwriteDie(1, 4, Dice.Value.Lumber);
      assertTrue(game.canBuildRoad(1));
      game.buildRoad(1);
      assertTrue(game.playsheet.roads[1]);
      assertFalse(game.dice[0].isUsable());
      assertFalse(game.dice[1].isUsable());
   }

   @Test
   public void undoPutsTheDieAndTheSlotBack() {
      Game game = wools(true);
      game.overwriteDie(2, 1, Dice.Value.Ore);
      assertTrue(game.canUndo());
      game.undo();
      assertEquals(Dice.Value.Wool, game.dice[2].value);
      assertFalse(game.dice[2].held);
      assertFalse(game.dice[2].locked);
      assertTrue(game.playsheet.canUseKnightResource(1));
      // Still in classic mode after the undo
      assertEquals(Game.BUILD_NONE, game.canBuildStatus(Game.RESOURCES_ROAD));
   }

   @Test
   public void aQuestionMarkBecomesTheResourceChosenAndOnlyARealOne() {
      Game game = wools(true);
      game.overwriteDie(0, 6, Dice.Value.Any);
      game.overwriteDie(0, 6, Dice.Value.Gold);
      game.overwriteDie(0, 6, Dice.Value.None);
      assertTrue(game.playsheet.canUseKnightResource(6));
      assertEquals(Dice.Value.Wool, game.dice[0].value);
      game.overwriteDie(0, 6, Dice.Value.Grain);
      assertEquals(Dice.Value.Grain, game.dice[0].value);
      assertFalse(game.playsheet.canUseKnightResource(6));
   }

   @Test
   public void aSlotOnlyEverBecomesItsOwnResource() {
      Game game = wools(true);
      game.overwriteDie(0, 1, Dice.Value.Grain);
      assertTrue(game.playsheet.canUseKnightResource(1));
      assertEquals(Dice.Value.Wool, game.dice[0].value);
   }

   @Test
   public void nothingIsOverwrittenOutsideClassicModeBeforeARollOrOntoASpentDie() {
      Game usual = wools(false);
      usual.overwriteDie(0, 1, Dice.Value.Ore);
      assertEquals(Dice.Value.Wool, usual.dice[0].value);
      assertTrue(usual.playsheet.canUseKnightResource(1));

      Game unrolled = wools(true);
      unrolled.rolls = 0;
      assertFalse(unrolled.canOverwriteDie(0, 1));

      Game spent = wools(true);
      spent.dice[3].use();
      assertFalse(spent.canOverwriteDie(3, 1));
      // Nor from a slot whose trader is not hired
      spent.playsheet.setTraderCount(2);
      assertFalse(spent.canOverwriteDie(0, 3));
   }

   /** Q2 of the plan: a slot spent by a drag is not handed back when the turn ends. */
   @Test
   public void theTurnEndDoesNotReturnASlotADragSpent() {
      Game game = wools(false);
      // In the usual mode the list fills with what the bank offers
      game.canBuildStatus(Game.RESOURCES_ROAD);
      assertFalse(game.knightResources.isEmpty());
      game.setTraderClassic(true);
      assertTrue(game.knightResources.isEmpty());
      game.overwriteDie(0, 1, Dice.Value.Ore);
      game.canBuildStatus(Game.RESOURCES_CITY);
      assertTrue(game.knightResources.isEmpty());
      game.newTurn(false);
      assertFalse(game.playsheet.canUseKnightResource(1));
      assertFalse(game.dice[0].locked);
      assertFalse(game.dice[0].held);
      // And the next turn the other slots are all still there, each spent once
      for(int slot = 2; slot <= 6; slot++) {
         assertTrue(game.playsheet.canUseKnightResource(slot));
      }
   }

   @Test
   public void aLockedDieIsSavedAndAnOldSaveLoadsUnlocked() throws Exception {
      Game game = wools(true);
      game.overwriteDie(4, 2, Dice.Value.Grain);
      Game back = GameStateStore.fromJson(GameStateStore.toJson(game));
      assertTrue(back.dice[4].locked);
      assertTrue(back.dice[4].held);
      assertFalse(back.dice[3].locked);
      // A save from before the flag existed has no "locked" at all
      Game old = SaveCompatTest.load(SaveCompatTest.SAVE);
      for(Dice die : old.dice) {
         assertFalse(die.locked);
      }
   }

   @Test
   public void theStaticEngineHonoursTheSheetsRule() {
      Playsheet sheet = new Playsheet(MapDef.TARNHOLM);
      sheet.setTraderCount(6);
      sheet.bankPaysBuilds = false;
      Dice[] dice = TarnholmBankTest.dice();
      assertEquals(Game.BUILD_NONE, Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>()));
      Game.Spent spent = Game.useResources(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>());
      assertEquals(0, spent.banked);
   }
}
