package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;

import org.junit.Test;

/**
 * Hand worked cases for paying from the bank on Tarnholm. Each expected value was worked
 * out by reading the pre-refactor Game.canBuildStatus and Game.useResources, and each
 * case also runs the frozen copy of that code to confirm the reading.
 */
public class TarnholmBankTest {
   static Dice[] dice(Dice.Value... values) {
      Dice[] dice = new Dice[6];
      for(int i = 0; i < 6; i++) {
         dice[i] = new Dice();
         dice[i].value = i < values.length ? values[i] : Dice.Value.None;
      }
      return dice;
   }

   static Playsheet sheet(int traders, int... spentSlots) {
      Playsheet sheet = new Playsheet(MapDef.TARNHOLM);
      sheet.setTraderCount(traders);
      for(int slot : spentSlots) {
         sheet.resourcesAvail[slot] = false;
      }
      return sheet;
   }

   static LegacyPlaysheet legacySheet(int traders, int... spentSlots) {
      LegacyPlaysheet sheet = new LegacyPlaysheet();
      sheet.knights = traders;
      for(int slot : spentSlots) {
         sheet.resourcesAvail[slot] = false;
      }
      return sheet;
   }

   /** Every trader built, a full bank, no dice: a road comes wholly from the bank. */
   @Test
   public void roadFromAFullBank() {
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice(), sheet(6), new ArrayList<Dice.Value>()));
      assertEquals(LegacyRules.BUILD_WITH_KNIGHT,
            LegacyRules.canBuildStatus(LegacyRules.RESOURCES_ROAD, dice(), legacySheet(6), new ArrayList<Dice.Value>()));
   }

   /**
    * The same bank cannot pay for a castle: it holds one grain, one ore and one "?",
    * which is three of the five resources needed.
    */
   @Test
   public void castleIsBeyondAFullBank() {
      assertEquals(Game.BUILD_NONE,
            Game.canBuildStatus(Game.RESOURCES_CITY, dice(), sheet(6), new ArrayList<Dice.Value>()));
      assertEquals(LegacyRules.BUILD_NONE,
            LegacyRules.canBuildStatus(LegacyRules.RESOURCES_CITY, dice(), legacySheet(6), new ArrayList<Dice.Value>()));
   }

   /** Brick and lumber on the dice: a road from dice alone, bank untouched. */
   @Test
   public void roadFromDice() {
      Dice[] dice = dice(Dice.Value.Brick, Dice.Value.Lumber);
      assertEquals(Game.BUILD_WITH_DICE,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet(0), new ArrayList<Dice.Value>()));
      assertEquals(Game.BUILD_WITH_DICE,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet(6), new ArrayList<Dice.Value>()));
      assertEquals(LegacyRules.BUILD_WITH_DICE,
            LegacyRules.canBuildStatus(LegacyRules.RESOURCES_ROAD, dice, legacySheet(6), new ArrayList<Dice.Value>()));
   }

   /**
    * A village with brick, lumber and wool on the dice and traders 1 and 2 built: the
    * missing grain comes from slot 2, and nothing else in the bank moves.
    */
   @Test
   public void villageTakesItsGrainFromSlotTwo() {
      Dice[] dice = dice(Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool);
      Playsheet sheet = sheet(2);
      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());

      assertFalse("slot 2 spent", sheet.resourcesAvail[2]);
      assertTrue("slot 2 reads as used", sheet.isKnightResourceUsed(2));
      for(int slot : new int[] { 1, 3, 4, 5, 6 }) {
         assertTrue("slot " + slot + " untouched", sheet.resourcesAvail[slot]);
      }
      assertTrue(dice[0].used && dice[1].used && dice[2].used);
      assertFalse(dice[3].used || dice[4].used || dice[5].used);

      Dice[] legacyDice = dice(Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool);
      LegacyPlaysheet legacy = legacySheet(2);
      LegacyRules.useResources(LegacyRules.RESOURCES_VILLAGE, legacyDice, legacy, new ArrayList<Dice.Value>());
      org.junit.Assert.assertArrayEquals(legacy.resourcesAvail, sheet.resourcesAvail);
   }

   /**
    * The same village when the only thing left in the bank is the "?": it stands in for
    * the grain, slot 6 is spent, and the build plan says the chain spends the wildcard.
    */
   @Test
   public void villageTakesTheWildcardWhenNothingElseFits() {
      Dice[] dice = dice(Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool);
      Playsheet sheet = sheet(6, 1, 2, 3, 4, 5);

      BuildPlan.Chain chain = BuildPlan.compute(dice, sheet, 1).chainFor(BuildPlan.Kind.VILLAGE, 0);
      assertNotNull("village 0 is on offer", chain);
      assertEquals(1, chain.steps.size());
      assertTrue(chain.spendsBank);
      assertTrue(chain.spendsWildcard);
      assertEquals(3, chain.diceUsed);
      assertTrue("planning left the live bank alone", sheet.resourcesAvail[6]);

      LegacyBuildPlan.Chain legacyChain = LegacyBuildPlan.compute(dice, legacySheet(6, 1, 2, 3, 4, 5), 1)
            .chainFor(LegacyBuildPlan.Kind.VILLAGE, 0);
      assertTrue(legacyChain.spendsBank && legacyChain.spendsWildcard);
      assertEquals(3, legacyChain.diceUsed);

      assertTrue(sheet.hasWildcard());
      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse("slot 6 spent", sheet.resourcesAvail[6]);
      assertFalse(sheet.hasWildcard());
      assertEquals(0, sheet.firstAvailableSlot(Dice.Value.Any));
   }

   /** A grain in the bank is preferred to the "?": the exact match goes first. */
   @Test
   public void exactMatchIsSpentBeforeTheWildcard() {
      Dice[] dice = dice(Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool);
      Playsheet sheet = sheet(6);
      BuildPlan.Chain chain = BuildPlan.compute(dice, sheet, 1).chainFor(BuildPlan.Kind.VILLAGE, 0);
      assertTrue(chain.spendsBank);
      assertFalse(chain.spendsWildcard);
      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse(sheet.resourcesAvail[2]);
      assertTrue(sheet.resourcesAvail[6]);
   }

   /** A game with no view can run to its end: the top score call is skipped, not fatal. */
   @Test
   public void aGameWithNoViewEndsAfterFifteenTurns() {
      Game game = new Game(MapDef.TARNHOLM);
      assertEquals(MapDef.TARNHOLM, game.map);
      assertEquals(Board.forMap(MapDef.TARNHOLM), game.board);
      game.playsheet.buildRoad(1);
      for(int turn = 1; turn <= 15; turn++) {
         assertFalse(game.isGameDone());
         game.newTurn(false);
         assertEquals(turn, game.turnsTaken);
      }
      assertTrue(game.isGameDone());
      // One road, less two for each of fifteen empty turns
      assertEquals(1 - 30, game.playsheet.getScore());
      game.newTurn(false);
      assertEquals(15, game.turnsTaken);

      game.startNewGame(MapDef.TARNHOLM);
      assertFalse(game.isGameDone());
      assertEquals(0, game.turnsTaken);
      assertEquals(0, game.playsheet.getScore());
   }
}
