package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Random;

import org.junit.Test;

/**
 * Dunholm, the second island, pinned against the plan's tables (section 2), its rules
 * played on a Playsheet, and the bank rules that only it can reach: two "?" slots that
 * must both be spent when a build needs both, and a banked resource that pays once.
 */
public class DunholmRulesTest {
   static final MapDef MAP = MapDef.DUNHOLM;

   static Dice[] dice(Dice.Value... values) {
      return TarnholmBankTest.dice(values);
   }

   /** A Dunholm sheet with these traders standing. */
   static Playsheet sheet(int... traders) {
      Playsheet sheet = new Playsheet(MAP);
      for(int tier : traders) {
         sheet.buildKnight(tier);
      }
      return sheet;
   }

   static int usable(Dice[] dice) {
      int n = 0;
      for(Dice die : dice) {
         if(die.isUsable()) {
            n++;
         }
      }
      return n;
   }

   // ---- the tables, as written in the plan -------------------------------------------

   @Test
   public void theMapIsRegistered() {
      assertSame(MAP, MapDef.byId("dunholm"));
      assertEquals("dunholm", MAP.id);
      assertEquals(2, MapDef.ALL.length);
      assertSame(MapDef.TARNHOLM, MapDef.ALL[0]);
      assertSame(MAP, MapDef.ALL[1]);
      assertNotEquals(0, MAP.nameRes);
      assertNotEquals(0, MapDef.TARNHOLM.nameRes);
      assertNotEquals(MapDef.TARNHOLM.nameRes, MAP.nameRes);
      assertEquals(R.drawable.board_dunholm, MAP.boardRes);
      assertEquals(R.drawable.board_tarnholm, MapDef.TARNHOLM.boardRes);
      assertEquals(0xFF0D3947, MAP.seaFill);
      assertEquals(MapDef.TARNHOLM.seaFill, MAP.seaFill);
   }

   @Test
   public void theTablesAreThePlansTables() {
      assertArrayEquals(MapDef.TARNHOLM.hex, MAP.hex);
      assertArrayEquals(new int[][] {
         { 0, 1 }, { 0, 2 }, { 0, 3 }, { 1, 1 }, { 1, 2 }, { 1, 3 }, { 2, 4 }, { 2, 3 }, { 2, 2 },
         { 3, 3 }, { 3, 2 }, { 3, 1 }, { 4, 3 }, { 4, 2 }, { 4, 1 }, { 4, 0 }, { 5, 1 }, { 5, 0 },
      }, MAP.roads);
      assertEquals(RaceFixture.EXPECTED_ROAD_NEEDS.length, MAP.roadNeeds.length);
      for(int i = 0; i < MAP.roadNeeds.length; i++) {
         assertArrayEquals("road " + i, RaceFixture.EXPECTED_ROAD_NEEDS[i], MAP.roadNeeds[i]);
      }
      assertArrayEquals(new int[][] {
         { 5, 2 }, { 5, 0 }, { 0, 0 }, { 4, 0 }, { 1, 0 }, { 3, 0 }, { 2, 4 },
      }, MAP.villages);
      assertArrayEquals(new int[][] { { 17 }, { 15, 16 }, { 0, 1 }, { 13, 14 }, { 3, 4 }, { 10, 11 }, { 6, 7 } }, MAP.villageRoads);
      assertFalse(MAP.orderedVillages);
      assertArrayEquals(new int[][] { { 0, 4 }, { 6, 0 }, { 1, 4 }, { 3, 4 } }, MAP.cities);
      assertArrayEquals(new int[][] { { 2 }, { 12 }, { 5 }, { 8, 9 } }, MAP.cityRoads);
      assertFalse(MAP.orderedCities);
      assertArrayEquals(new int[][] {
         { 0, 0, 0 }, { 1, 0, 0 }, { 2, 0, 0 }, { 3, 0, 0 }, { 4, 0, 0 },
         { 5, 0, -15 }, { 5, 0, 15 }, { 6, 0, -15 }, { 6, 0, 15 },
      }, MAP.traders);
      assertFalse(MAP.orderedTraders);
      assertArrayEquals(new int[] { 1, 2, 3, 4, 5, 6, 6, 7, 7 }, MAP.traderBank);
      assertArrayEquals(new Dice.Value[] {
         Dice.Value.None, Dice.Value.Ore, Dice.Value.Grain, Dice.Value.Wool, Dice.Value.Lumber,
         Dice.Value.Brick, Dice.Value.Any, Dice.Value.Any,
      }, MAP.bankResource);
      assertArrayEquals(new int[][] { {}, { 1 }, { 2 }, { 3 }, { 4 }, { 5 }, { 6, 7 }, { 8, 9 } }, MAP.bankTraders);
      assertEquals(0, MAP.roadPoints);
      assertArrayEquals(new int[9], MAP.traderPoints);
      assertArrayEquals(new int[] { 1, 1, 1, 1, 1, 1, 1 }, MAP.villagePoints);
      assertArrayEquals(new int[] { 2, 2, 2, 2 }, MAP.cityPoints);
      assertEquals(0, MAP.emptyTurnPenalty);
      assertEquals(6, MAP.longRoad);
      assertEquals(2, MAP.longRoadBonus);
      assertEquals(3, MAP.traderBonusAt);
      assertEquals(2, MAP.traderBonus);
      assertEquals("no turn cap: the race ends only at the target", 0, MAP.turnCap);
      assertEquals(10, MAP.targetScore);
      assertTrue(MAP.lowerIsBetter);
      assertTrue(MAP.bankMatchesOnce);
      assertFalse("Tarnholm keeps its old bank habit", MapDef.TARNHOLM.bankMatchesOnce);
   }

   @Test
   public void theBoardPointsAreWherePlanPutThem() {
      Board board = Board.forMap(MAP);
      assertEquals(new Pt(225, 198), board.roadAnchor[0]);
      assertEquals(new Pt(225, 342), board.roadAnchor[6]);
      assertEquals(new Pt(252, 144), board.roadAnchor[17]);
      assertEquals(new Pt(234, 144), board.village[0]);
      assertEquals(new Pt(342, 288), board.village[5]);
      assertEquals(new Pt(180, 252), board.city[0]);
      assertEquals(new Pt(288, 252), board.city[1]);
      assertEquals(new Pt(288, 324), board.city[3]);
      assertEquals(new Pt(198, 216), board.knight[0]);
      assertEquals(new Pt(252, 165), board.knight[5]);
      assertEquals(new Pt(252, 195), board.knight[6]);
      assertEquals(new Pt(252, 237), board.knight[7]);
      assertEquals(new Pt(252, 267), board.knight[8]);
   }

   // ---- placement rules --------------------------------------------------------------

   @Test
   public void aFreshSheetOffersVillageTwoAndNoCastle() {
      Playsheet sheet = new Playsheet(MAP);
      for(int i = 0; i < 7; i++) {
         assertEquals("village " + i, i == 2, sheet.canBuildVillage(i));
      }
      for(int i = 0; i < 4; i++) {
         assertFalse("castle " + i, sheet.canBuildCity(i));
      }
      // The start road touches only road 1
      for(int i = 1; i < 18; i++) {
         assertEquals("road " + i, i == 1, sheet.canBuildRoad(i));
      }
      assertEquals(0, sheet.getScore());
   }

   @Test
   public void villagesGoUpInAnyOrder() {
      Playsheet sheet = new Playsheet(MAP);
      for(int road : new int[] { 1, 3, 4, 6, 7, 8, 9, 10, 11 }) {
         sheet.buildRoad(road);
      }
      // Last village first, then the one beside the start, then a middle one
      assertTrue(sheet.canBuildVillage(6));
      sheet.buildVillage(6);
      assertTrue(sheet.canBuildVillage(2));
      sheet.buildVillage(2);
      assertTrue(sheet.canBuildVillage(5));
      sheet.buildVillage(5);
      assertTrue(sheet.villages[6] && sheet.villages[2] && sheet.villages[5]);
      assertFalse(sheet.villages[0] || sheet.villages[1]);
      assertFalse("one village once", sheet.canBuildVillage(6));
      // Still not reachable: no road touches it
      assertFalse(sheet.canBuildVillage(0));
      assertFalse(sheet.canBuildVillage(1));
      assertEquals("three villages and the long road", 5, sheet.getScore());
   }

   @Test
   public void castlesGoUpInAnyOrder() {
      Playsheet sheet = new Playsheet(MAP);
      for(int road : new int[] { 1, 3, 4, 5, 6, 7, 8 }) {
         sheet.buildRoad(road);
      }
      assertTrue(sheet.canBuildCity(3));
      sheet.buildCity(3);
      assertTrue(sheet.canBuildCity(2));
      sheet.buildCity(2);
      assertFalse(sheet.canBuildCity(0));
      assertFalse(sheet.canBuildCity(1));
      sheet.buildRoad(2);
      assertTrue(sheet.canBuildCity(0));
      assertEquals("two castles and the long road", 6, sheet.getScore());
   }

   @Test
   public void tradersGoUpInAnyOrderOnce() {
      Playsheet sheet = new Playsheet(MAP);
      for(int tier = 9; tier >= 1; tier--) {
         assertTrue("trader " + tier, sheet.canBuildKnight(tier));
         sheet.buildKnight(tier);
         assertFalse(sheet.canBuildKnight(tier));
      }
      assertEquals(9, sheet.traderCount());
      assertFalse(sheet.canBuildKnight(10));
      assertFalse(sheet.canBuildKnight(0));
   }

   /** Q2 of the brief: road 6 cannot be reached except along roads 1, 3 and 4. */
   @Test
   public void roadSixNeedsRoadsOneThreeAndFour() {
      int[][] withheld = { { 1, 3, 4 }, { 1 }, { 3 }, { 4 }, { 1, 3 } };
      for(int[] cut : withheld) {
         Playsheet sheet = new Playsheet(MAP);
         // Build every road that can be built, except the ones held back and road 6 itself
         boolean grew = true;
         while(grew) {
            grew = false;
            for(int road = 1; road < 18; road++) {
               boolean skip = road == 6;
               for(int c : cut) {
                  skip |= c == road;
               }
               if(!skip && sheet.canBuildRoad(road)) {
                  sheet.buildRoad(road);
                  grew = true;
               }
            }
         }
         assertFalse("road 6 with " + Arrays.toString(cut) + " held back", sheet.canBuildRoad(6));
         assertFalse(sheet.roads[6]);
      }
      // And road 6 does open the moment road 4 is down
      Playsheet sheet = new Playsheet(MAP);
      sheet.buildRoad(1);
      sheet.buildRoad(3);
      assertFalse(sheet.canBuildRoad(6));
      sheet.buildRoad(4);
      assertTrue(sheet.canBuildRoad(6));
   }

   // ---- bank slots and scoring -------------------------------------------------------

   @Test
   public void aWildcardSlotNeedsBothItsTraders() {
      Playsheet sheet = sheet(6);
      assertFalse(sheet.canUseKnightResource(6));
      assertFalse(sheet.hasWildcard());
      sheet.buildKnight(7);
      assertTrue(sheet.canUseKnightResource(6));
      assertTrue(sheet.hasWildcard());
      assertFalse("slot 7 still waits on the centre pair", sheet.canUseKnightResource(7));

      Playsheet centre = sheet(8);
      assertFalse(centre.canUseKnightResource(7));
      centre.buildKnight(9);
      assertTrue(centre.canUseKnightResource(7));
      assertFalse(centre.canUseKnightResource(6));

      // A single trader opens a plain slot at once
      for(int tier = 1; tier <= 5; tier++) {
         assertTrue("slot " + tier, sheet(tier).canUseKnightResource(tier));
      }
   }

   @Test
   public void oneCastleScoresTwo() {
      Playsheet sheet = new Playsheet(MAP);
      sheet.buildRoad(1);
      sheet.buildRoad(2);
      sheet.buildCity(0);
      assertEquals(2, sheet.getScore());
   }

   @Test
   public void theLongRoadBonusIsRoadSixAndSurvivesTheBank() {
      Playsheet sheet = sheet(6, 7);
      for(int road : new int[] { 1, 3, 4, 5, 2 }) {
         sheet.buildRoad(road);
      }
      assertEquals("roads score nothing", 0, sheet.getScore());
      sheet.buildRoad(6);
      assertEquals(2, sheet.getScore());
      sheet.useKnightResource(6);
      assertTrue(sheet.isKnightResourceUsed(6));
      assertEquals("spending slot 6 costs no points", 2, sheet.getScore());
      sheet.buildRoad(7);
      assertEquals("and only road 6 earns it", 2, sheet.getScore());
   }

   @Test
   public void theTraderBonusComesAtTheThirdTraderWhicheverTheyAre() {
      int[][] sets = { { 1, 2, 3 }, { 9, 1, 5 }, { 6, 7, 8 }, { 7, 8, 9 }, { 4, 9, 2 } };
      for(int[] set : sets) {
         Playsheet sheet = new Playsheet(MAP);
         sheet.buildKnight(set[0]);
         sheet.buildKnight(set[1]);
         assertEquals(Arrays.toString(set) + " two traders", 0, sheet.getScore());
         sheet.buildKnight(set[2]);
         assertEquals(Arrays.toString(set) + " three traders", 2, sheet.getScore());
         for(int tier = 1; tier <= 9; tier++) {
            sheet.buildKnight(tier);
         }
         assertEquals(Arrays.toString(set) + " nine traders", 2, sheet.getScore());
      }
   }

   @Test
   public void emptyTurnsCostNothing() {
      Playsheet sheet = new Playsheet(MAP);
      sheet.turnsNothingBuilt = 12;
      assertEquals(0, sheet.getScore());
   }

   @Test
   public void theMostAnyoneCanScoreIsNineteen() {
      Playsheet sheet = new Playsheet(MAP);
      for(int road = 1; road < 18; road++) {
         sheet.buildRoad(road);
      }
      boolean grew = true;
      while(grew) {
         grew = false;
         for(int road = 1; road < 18; road++) {
            if(sheet.canBuildRoad(road)) {
               sheet.buildRoad(road);
               grew = true;
            }
         }
      }
      for(int i = 0; i < 7; i++) {
         sheet.buildVillage(i);
      }
      for(int i = 0; i < 4; i++) {
         sheet.buildCity(i);
      }
      for(int tier = 1; tier <= 9; tier++) {
         sheet.buildKnight(tier);
      }
      assertEquals(7 + 8 + 2 + 2, sheet.getScore());
      // The same sum read from the tables, which is what caps the target setting
      assertEquals(19, MAP.maxScore());
   }

   // ---- the early build prompts stand aside for the winning build --------------------

   /** A Dunholm game on its first roll with these dice, every castle up (8 points). */
   static Game castlesUp(Dice.Value... values) {
      Game game = new Game(MAP);
      for(int i = 0; i < 4; i++) {
         game.playsheet.cities[i] = true;
      }
      game.rolls = 1;
      for(int i = 0; i < values.length; i++) {
         game.dice[i].value = values[i];
      }
      return game;
   }

   @Test
   public void aVillageThatReachesTenWinsTheRace() {
      Game game = castlesUp(Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Wool, Dice.Value.Grain);
      game.playsheet.villages[6] = true;
      assertEquals(9, game.playsheet.getScore());
      BuildPlan plan = BuildPlan.compute(game.dice, game.playsheet, game.rolls);
      BuildPlan.Chain chain = plan.chainFor(BuildPlan.Kind.VILLAGE, 2);
      // Q1 of the brief: a position affordable right now has a chain of one step, not null
      assertEquals(1, chain.steps.size());
      assertTrue(game.wouldForfeitRolls());
      assertTrue(game.chainReachesTarget(chain));
      assertEquals("and the check built nothing", 9, game.playsheet.getScore());

      game.playsheet.villages[6] = false;
      assertFalse("one short of the target", game.chainReachesTarget(chain));
      assertFalse("nothing reaches it", game.chainReachesTarget(null));
   }

   @Test
   public void theThirdTradersBonusCanWinTheRace() {
      Game game = castlesUp(Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Ore);
      game.playsheet.buildKnight(1);
      game.playsheet.buildKnight(2);
      assertEquals(8, game.playsheet.getScore());
      BuildPlan plan = BuildPlan.compute(game.dice, game.playsheet, game.rolls);
      assertTrue(game.chainReachesTarget(plan.chainFor(BuildPlan.Kind.KNIGHT, 6)));

      // A chain of several steps counts every one of them
      game.playsheet.traders = new boolean[MAP.traders.length + 1];
      ArrayList<BuildPlan.Step> steps = new ArrayList<BuildPlan.Step>();
      steps.add(new BuildPlan.Step(BuildPlan.Kind.KNIGHT, 3));
      steps.add(new BuildPlan.Step(BuildPlan.Kind.KNIGHT, 4));
      steps.add(new BuildPlan.Step(BuildPlan.Kind.KNIGHT, 5));
      assertTrue(game.chainReachesTarget(new BuildPlan.Chain(steps, false, false, 0)));
      steps.remove(2);
      assertFalse(game.chainReachesTarget(new BuildPlan.Chain(steps, false, false, 0)));
   }

   @Test
   public void theWinningBuildIsJudgedAgainstTheGamesOwnTarget() {
      Game game = castlesUp(Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Wool, Dice.Value.Grain);
      game.playsheet.villages[6] = true;
      BuildPlan.Chain chain = BuildPlan.compute(game.dice, game.playsheet, game.rolls).chainFor(BuildPlan.Kind.VILLAGE, 2);
      assertTrue(game.chainReachesTarget(chain));
      game.setRaceTarget(11);
      assertFalse("ten is no longer enough", game.chainReachesTarget(chain));
   }

   // ---- the target setting ------------------------------------------------------------

   @Test
   public void aNewTargetMovesTheGameInProgressOnlyWhenItIsStillAhead() {
      Game game = new Game(MAP);
      assertEquals(10, game.targetScore);
      game.playsheet.cities[0] = true;
      game.playsheet.cities[1] = true;
      game.playsheet.cities[2] = true;
      assertEquals(6, game.playsheet.getScore());

      game.setRaceTarget(15);
      assertEquals(15, game.targetScore);
      game.setRaceTarget(7);
      assertEquals("lowered, but still ahead of the score", 7, game.targetScore);
      game.setRaceTarget(4);
      assertEquals("already passed: the game would end on the spot, so it keeps its target", 7, game.targetScore);
      assertFalse(game.isGameDone());

      // The next game starts with the setting, clamped to what the island can give
      game.reset();
      assertEquals(4, game.targetScore);
      game.setRaceTarget(40);
      assertEquals(19, game.targetScore);
      game.reset();
      assertEquals(19, game.targetScore);
   }

   @Test
   public void aFinishedGameKeepsItsTarget() {
      Game game = new Game(MAP);
      game.setRaceTarget(8);
      for(int i = 0; i < 4; i++) {
         game.playsheet.cities[i] = true;
      }
      assertTrue(game.isGameDone());
      game.setRaceTarget(12);
      assertEquals(8, game.targetScore);
      assertTrue(game.isGameDone());
   }

   @Test
   public void onlyARaceToTheIslandsOwnTargetGoesOnTheTopScores() {
      Game game = new Game(MAP);
      for(int i = 0; i < 4; i++) {
         game.playsheet.cities[i] = true;
      }
      game.playsheet.villages[0] = true;
      game.playsheet.villages[1] = true;
      game.turnsTaken = 9;
      assertEquals(10, game.playsheet.getScore());
      assertEquals(9, game.resultToRecord());

      Game other = new Game(MAP);
      other.setRaceTarget(8);
      for(int i = 0; i < 4; i++) {
         other.playsheet.cities[i] = true;
      }
      other.turnsTaken = 6;
      assertTrue(other.isGameDone());
      assertEquals(0, other.resultToRecord());
   }

   @Test
   public void tarnholmIsNoRaceSoNoBuildWinsIt() {
      Game game = new Game(MapDef.TARNHOLM);
      ArrayList<BuildPlan.Step> steps = new ArrayList<BuildPlan.Step>();
      steps.add(new BuildPlan.Step(BuildPlan.Kind.ROAD, 1));
      assertFalse(game.chainReachesTarget(new BuildPlan.Chain(steps, false, false, 0)));
   }

   // ---- paying from the bank: wildcards ----------------------------------------------

   @Test
   public void aBuildMissingOneResourceSpendsOneWildcardSlot() {
      // Traders 6 and 7 open slot 6; slot 7 stays shut. A road is Brick and Lumber.
      Playsheet sheet = sheet(6, 7);
      Dice[] dice = dice(Dice.Value.Brick);
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse(sheet.resourcesAvail[6]);
      assertTrue("the shut slot is untouched", sheet.resourcesAvail[7]);
      assertEquals(0, usable(new Dice[] { dice[0] }));
   }

   @Test
   public void aBuildMissingOneResourceWithTwoWildcardsOpenSpendsOnlyOne() {
      Playsheet sheet = sheet(6, 7, 8, 9);
      Dice[] dice = dice(Dice.Value.Brick);
      Game.useResources(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse("the lowest slot pays", sheet.resourcesAvail[6]);
      assertTrue(sheet.resourcesAvail[7]);
   }

   @Test
   public void aBuildMissingTwoResourcesSpendsBothWildcardSlots() {
      Playsheet sheet = sheet(6, 7, 8, 9);
      Dice[] dice = dice();
      assertEquals("offered", Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse("paid", sheet.resourcesAvail[6]);
      assertFalse("paid", sheet.resourcesAvail[7]);
      // Nothing is left to pay a second road
      assertEquals(Game.BUILD_NONE,
            Game.canBuildStatus(Game.RESOURCES_ROAD, dice, sheet, new ArrayList<Dice.Value>()));
   }

   @Test
   public void aBuildMissingThreeIsNotOfferedOnTwoWildcards() {
      Playsheet sheet = sheet(6, 7, 8, 9);
      assertEquals(Game.BUILD_NONE,
            Game.canBuildStatus(Game.RESOURCES_VILLAGE, dice(Dice.Value.Lumber), sheet, new ArrayList<Dice.Value>()));
   }

   @Test
   public void wildcardsAndExactSlotsAndDiceShareAVillage() {
      // Village: Lumber, Brick, Wool, Grain. Lumber from the dice, Wool from its slot,
      // Brick and Grain from the two wildcards.
      Playsheet sheet = sheet(3, 6, 7, 8, 9);
      Dice[] dice = dice(Dice.Value.Lumber);
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse(dice[0].isUsable());
      assertFalse("wool slot", sheet.resourcesAvail[3]);
      assertFalse("first wildcard", sheet.resourcesAvail[6]);
      assertFalse("second wildcard", sheet.resourcesAvail[7]);
   }

   @Test
   public void goldPaysWhatWildcardsLeave() {
      // Lumber on the dice and a pair of gold covers one more; two wildcards cover the rest
      Playsheet sheet = sheet(6, 7, 8, 9);
      Dice[] dice = dice(Dice.Value.Lumber, Dice.Value.Gold, Dice.Value.Gold);
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_VILLAGE, dice, sheet, new ArrayList<Dice.Value>());
      assertEquals("every die spent", 3, 6 - usable(dice));
      assertFalse(sheet.resourcesAvail[6]);
      assertFalse(sheet.resourcesAvail[7]);
   }

   /**
    * The turn end hands back resources the turn set aside. With two "?" slots, a spent
    * one must not come back just because the other is still open.
    */
   @Test
   public void aSpentWildcardStaysSpentAtTheNextTurn() {
      Game game = new Game(MAP);
      for(int tier = 6; tier <= 9; tier++) {
         game.playsheet.buildKnight(tier);
      }
      game.rolls = 1;
      game.dice[0].value = Dice.Value.Brick;
      game.buildRoad(1);
      assertTrue(game.playsheet.roads[1]);
      assertFalse("slot 6 paid for the lumber", game.playsheet.resourcesAvail[6]);
      assertTrue(game.playsheet.resourcesAvail[7]);
      game.newTurn(false);
      assertFalse("and is still spent", game.playsheet.resourcesAvail[6]);
      assertTrue(game.playsheet.resourcesAvail[7]);
   }

   // ---- paying from the bank: one banked resource pays once --------------------------

   @Test
   public void aBankedGrainPaysForOneGrainOnDunholm() {
      // Castle: Grain, Grain, Ore, Ore, Ore. Ore on three dice, one grain on a die, one in
      // the bank. Dunholm spends the grain slot for the second grain.
      Playsheet sheet = sheet(2);
      Dice[] dice = dice(Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Grain);
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse(sheet.resourcesAvail[2]);
      assertEquals("three ore and a grain gone", 2, usable(dice));
   }

   @Test
   public void twoGrainNeededOneBankedIsNotAffordableWithoutAWildcard() {
      // Both grain from a single banked grain: not offered, not paid
      Playsheet sheet = sheet(2);
      Dice[] dice = dice(Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      assertEquals(Game.BUILD_NONE,
            Game.canBuildStatus(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>()));
   }

   @Test
   public void theSecondGrainOfACastleComesFromAWildcardNotFromNothing() {
      Playsheet sheet = sheet(2, 6, 7);
      Dice[] dice = dice(Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore);
      assertEquals(Game.BUILD_WITH_KNIGHT,
            Game.canBuildStatus(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>()));
      Game.useResources(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse("the grain slot", sheet.resourcesAvail[2]);
      assertFalse("and the wildcard for the second grain", sheet.resourcesAvail[6]);
   }

   @Test
   public void tarnholmStillGetsItsFreeSecondGrain() {
      // The old quirk, kept for Tarnholm: one banked grain pays both grain of a castle
      Playsheet sheet = new Playsheet(MapDef.TARNHOLM);
      sheet.setTraderCount(2);
      Dice[] dice = dice(Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Ore, Dice.Value.Grain);
      Game.useResources(Game.RESOURCES_CITY, dice, sheet, new ArrayList<Dice.Value>());
      assertFalse(sheet.resourcesAvail[2]);
   }

   // ---- offered is paid --------------------------------------------------------------

   static final Dice.Value[] FACES = {
      Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Brick, Dice.Value.Ore, Dice.Value.Lumber, Dice.Value.Gold,
   };

   /**
    * On random dice and random banks, whenever a build is offered it is paid in full:
    * the resources it needs equal the dice spent, plus one per pair of gold, plus the
    * bank slots spent. Dunholm only, since Tarnholm's free grain is a deliberate quirk.
    */
   @Test
   public void whateverIsOfferedIsExactlyPaid() {
      Dice.Value[][] recipes = {
         Game.RESOURCES_ROAD, Game.RESOURCES_VILLAGE, Game.RESOURCES_KNIGHT, Game.RESOURCES_CITY,
      };
      int offered = 0;
      int bankPaid = 0;
      for(int seed = 0; seed < 3000; seed++) {
         Random random = new Random(seed);
         Playsheet sheet = new Playsheet(MAP);
         for(int tier = 1; tier <= 9; tier++) {
            if(random.nextBoolean()) {
               sheet.buildKnight(tier);
            }
         }
         for(int slot = 1; slot <= 7; slot++) {
            if(random.nextInt(4) == 0) {
               sheet.resourcesAvail[slot] = false;
            }
         }
         Dice[] dice = new Dice[6];
         for(int i = 0; i < 6; i++) {
            dice[i] = new Dice();
            dice[i].value = random.nextInt(8) == 0 ? Dice.Value.None : FACES[random.nextInt(6)];
            dice[i].used = random.nextInt(8) == 0;
         }
         Dice.Value[] recipe = recipes[random.nextInt(4)];
         int status = Game.canBuildStatus(recipe, dice, sheet, new ArrayList<Dice.Value>());
         if(status == Game.BUILD_NONE) {
            continue;
         }
         offered++;
         int bankBefore = available(sheet);
         int diceBefore = usable(dice);
         int goldBefore = usableGold(dice);
         Game.useResources(recipe, dice, sheet, new ArrayList<Dice.Value>());
         int bankSpent = bankBefore - available(sheet);
         int goldSpent = goldBefore - usableGold(dice);
         int plainSpent = diceBefore - usable(dice) - goldSpent;
         assertEquals("seed " + seed + " gold is spent in pairs", 0, goldSpent % 2);
         assertEquals("seed " + seed, recipe.length, plainSpent + goldSpent / 2 + bankSpent);
         if(bankSpent > 0) {
            bankPaid++;
            assertEquals("seed " + seed + " a bank spend was announced", Game.BUILD_WITH_KNIGHT, status);
         }
      }
      assertTrue("the run offered builds: " + offered, offered > 500);
      assertTrue("and some paid from the bank: " + bankPaid, bankPaid > 200);
   }

   private static int available(Playsheet sheet) {
      int n = 0;
      for(int slot = 1; slot <= sheet.bankSize(); slot++) {
         if(sheet.resourcesAvail[slot]) {
            n++;
         }
      }
      return n;
   }

   private static int usableGold(Dice[] dice) {
      int n = 0;
      for(Dice die : dice) {
         if(die.isUsable() && die.value == Dice.Value.Gold) {
            n++;
         }
      }
      return n;
   }
}
