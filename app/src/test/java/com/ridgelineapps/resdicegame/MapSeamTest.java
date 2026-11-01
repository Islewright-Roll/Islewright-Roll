package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;

import org.junit.Test;

/**
 * The rules on a map that is nothing like Tarnholm (see RaceFixture): proof that the
 * tables in MapDef carry every rule, so a new island is data and not code.
 */
public class MapSeamTest {
   static final MapDef MAP = RaceFixture.MAP;

   static Dice[] dice(Dice.Value... values) {
      return TarnholmBankTest.dice(values);
   }

   @Test
   public void geometryPassesAndRoadsFollowFromIt() {
      Board board = Board.forMap(MAP);
      assertEquals(18, board.roadAnchor.length);
      assertEquals(RaceFixture.EXPECTED_ROAD_NEEDS.length, MAP.roadNeeds.length);
      for(int i = 0; i < MAP.roadNeeds.length; i++) {
         assertArrayEquals("road " + i, RaceFixture.EXPECTED_ROAD_NEEDS[i], MAP.roadNeeds[i]);
      }
      assertEquals(new Pt(252, 165), board.knight[5]);
      assertEquals(new Pt(252, 267), board.knight[8]);
   }

   @Test
   public void sheetIsSizedByTheMap() {
      Playsheet sheet = new Playsheet(MAP);
      assertEquals(18, sheet.roads.length);
      assertEquals(7, sheet.villages.length);
      assertEquals(4, sheet.cities.length);
      assertEquals(10, sheet.traders.length);
      assertEquals(7, sheet.bankSize());
      // No turn cap: the turn scores grow as turns are taken
      assertEquals(1, sheet.turnScore.length);
      sheet.scoreTurn(30);
      assertEquals(31, sheet.turnScore.length);
      assertEquals(0, sheet.getScore());
   }

   @Test
   public void anyTouchingRoadPermitsAndNothingIsOrdered() {
      Playsheet sheet = new Playsheet(MAP);
      // The start road touches village 2 and road 1, and nothing else
      assertTrue(sheet.canBuildVillage(2));
      assertFalse(sheet.canBuildVillage(0));
      assertTrue(sheet.canBuildRoad(1));
      assertFalse(sheet.canBuildRoad(2));
      assertFalse(sheet.canBuildRoad(17));
      assertFalse(sheet.canBuildCity(0));

      sheet.buildRoad(1);
      assertTrue(sheet.canBuildRoad(2));
      assertTrue(sheet.canBuildRoad(3));
      sheet.buildRoad(3);
      // Village 4 without villages 0 to 3, by either of its roads
      assertTrue(sheet.canBuildVillage(4));
      sheet.buildRoad(2);
      assertTrue(sheet.canBuildCity(0));
      sheet.buildCity(0);
      assertFalse(sheet.canBuildCity(0));
      assertFalse(sheet.canBuildCity(1));

      // Any trader, any time, once
      assertTrue(sheet.canBuildKnight(9));
      assertTrue(sheet.canBuildKnight(4));
      assertFalse(sheet.canBuildKnight(10));
      assertFalse(sheet.canBuildKnight(0));
      sheet.buildKnight(9);
      assertTrue(sheet.isTraderBuilt(9));
      assertFalse(sheet.isTraderBuilt(8));
      assertFalse(sheet.canBuildKnight(9));
      assertEquals(1, sheet.traderCount());
   }

   @Test
   public void aBankSlotBehindTwoTradersNeedsBoth() {
      Playsheet sheet = new Playsheet(MAP);
      sheet.buildKnight(6);
      assertFalse(sheet.canUseKnightResource(6));
      assertFalse(sheet.isKnightResourceUsed(6));
      assertFalse(sheet.hasWildcard());
      assertEquals(Dice.Value.None, sheet.useKnightResource(6));
      assertTrue("a refused use spends nothing", sheet.resourcesAvail[6]);

      sheet.buildKnight(7);
      assertTrue(sheet.canUseKnightResource(6));
      assertEquals(6, sheet.firstAvailableSlot(Dice.Value.Any));
      assertEquals(Dice.Value.Any, sheet.getKnightResource(6));
      sheet.useKnightResource(6);
      assertTrue(sheet.isKnightResourceUsed(6));
      assertEquals(0, sheet.firstAvailableSlot(Dice.Value.Any));

      sheet.buildKnight(9);
      sheet.buildKnight(8);
      assertEquals(7, sheet.firstAvailableSlot(Dice.Value.Any));
      sheet.buildKnight(2);
      assertEquals(2, sheet.firstAvailableSlot(Dice.Value.Grain));
      assertEquals(Dice.Value.Grain, sheet.getKnightResource(2));

      sheet.returnToBank(Dice.Value.Any);
      assertEquals(6, sheet.firstAvailableSlot(Dice.Value.Any));
   }

   @Test
   public void scoreIsPiecesPlusTwoDerivedBonuses() {
      Playsheet sheet = new Playsheet(MAP);
      for(int road : new int[] { 1, 3, 4 }) {
         sheet.buildRoad(road);
      }
      sheet.turnsNothingBuilt = 5;
      assertEquals("roads and empty turns score nothing", 0, sheet.getScore());
      sheet.buildVillage(2);
      sheet.buildVillage(4);
      assertEquals(2, sheet.getScore());
      sheet.buildRoad(6);
      assertEquals("the long road", 4, sheet.getScore());
      sheet.buildRoad(5);
      sheet.buildCity(2);
      assertEquals(6, sheet.getScore());
      sheet.buildKnight(8);
      sheet.buildKnight(9);
      assertEquals(6, sheet.getScore());
      sheet.buildKnight(1);
      assertEquals("three traders", 8, sheet.getScore());
      sheet.buildKnight(2);
      assertEquals("and only once", 8, sheet.getScore());
      sheet.useKnightResource(7);
      sheet.useKnightResource(1);
      assertEquals("spending the bank costs no points", 8, sheet.getScore());
   }

   @Test
   public void theGameEndsAtTheTargetOrTheCap() {
      Game game = new Game(MAP);
      assertFalse(game.isGameDone());
      for(int road : new int[] { 1, 3, 4, 6, 2, 5 }) {
         game.playsheet.buildRoad(road);
      }
      game.playsheet.buildVillage(2);
      game.playsheet.buildVillage(4);
      game.playsheet.buildVillage(6);
      game.playsheet.buildCity(0);
      assertEquals(7, game.playsheet.getScore());
      assertFalse(game.isGameDone());
      game.playsheet.buildCity(2);
      assertEquals(9, game.playsheet.getScore());
      assertFalse(game.isGameDone());
      game.playsheet.buildKnight(3);
      game.playsheet.buildKnight(5);
      game.playsheet.buildKnight(7);
      assertEquals(11, game.playsheet.getScore());
      assertTrue(game.isGameDone());

      // With no cap, no number of turns ends it
      Game slow = new Game(MAP);
      for(int turn = 1; turn <= 40; turn++) {
         assertFalse(slow.isGameDone());
         slow.newTurn(false);
      }
      assertFalse(slow.isGameDone());
      assertEquals(40, slow.turnsTaken);
      assertEquals(0, slow.playsheet.getScore());

      // A race with a cap still ends there
      Game capped = new Game(RaceFixture.CAPPED);
      for(int turn = 1; turn <= 20; turn++) {
         assertFalse(capped.isGameDone());
         capped.newTurn(false);
      }
      assertTrue(capped.isGameDone());

      // And a game can move from one map to the other
      slow.startNewGame(MapDef.TARNHOLM);
      assertEquals(17, slow.playsheet.roads.length);
      assertEquals(MapDef.TARNHOLM, slow.board.map);
      assertFalse(slow.isGameDone());
   }

   @Test
   public void thePlanWalksTheMapAndSeesEitherWildcard() {
      // Brick, lumber and wool on the dice; the grain for village 2 has to be a "?".
      // Slot 6 is shut (one trader of its pair), slot 7 is open.
      Playsheet sheet = new Playsheet(MAP);
      sheet.buildKnight(6);
      sheet.buildKnight(8);
      sheet.buildKnight(9);
      Dice[] dice = dice(Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool);
      BuildPlan plan = BuildPlan.compute(dice, sheet, 1);
      BuildPlan.Chain village = plan.chainFor(BuildPlan.Kind.VILLAGE, 2);
      assertNotNull(village);
      assertTrue(village.spendsBank);
      assertTrue(village.spendsWildcard);
      assertNotNull("a road from dice alone", plan.chainFor(BuildPlan.Kind.ROAD, 1));
      assertFalse(plan.chainFor(BuildPlan.Kind.ROAD, 1).spendsBank);
      assertEquals(null, plan.chainFor(BuildPlan.Kind.ROAD, 17));
      assertTrue(BuildPlan.maxDiceSpendableThrough(dice, sheet, village) >= 3);

      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());
      assertTrue("slot 6 was never open", sheet.resourcesAvail[6]);
      assertFalse("slot 7 paid", sheet.resourcesAvail[7]);

      // A trader on the last tier is reachable by the plan
      Dice[] traderDice = dice(Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      assertNotNull(BuildPlan.compute(traderDice, new Playsheet(MAP), 1).chainFor(BuildPlan.Kind.KNIGHT, 9));
   }

   @Test
   public void theFingerprintTellsTraderSetsApart() {
      Playsheet a = new Playsheet(MAP);
      Playsheet b = new Playsheet(MAP);
      a.buildKnight(1);
      b.buildKnight(2);
      assertEquals(a.traderCount(), b.traderCount());
      assertNotEquals(BuildPlan.fingerprint(dice(), a), BuildPlan.fingerprint(dice(), b));
   }
}
