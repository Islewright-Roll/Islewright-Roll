package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.util.ArrayList;
import java.util.Random;

import org.junit.Test;

/**
 * Proof that Tarnholm plays exactly as it did before the rules became map driven. The
 * frozen pre-refactor code (LegacyPlaysheet, LegacyRules, LegacyBuildPlan, LegacyGame) and
 * the live code are driven through the same seeded random sequences of legal and illegal
 * actions, and the complete state of both is compared after every single step.
 */
public class TarnholmEquivalenceTest {
   static final int SEEDS = 600;

   static final Dice.Value[] FACES = {
      Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Brick, Dice.Value.Ore, Dice.Value.Lumber, Dice.Value.Gold,
   };
   static final Dice.Value[][] RECIPES = {
      Game.RESOURCES_ROAD, Game.RESOURCES_VILLAGE, Game.RESOURCES_CITY, Game.RESOURCES_KNIGHT,
   };
   static final Dice.Value[][] LEGACY_RECIPES = {
      LegacyRules.RESOURCES_ROAD, LegacyRules.RESOURCES_VILLAGE, LegacyRules.RESOURCES_CITY, LegacyRules.RESOURCES_KNIGHT,
   };

   /** How many sheet comparisons the run made, so a test that compares nothing cannot pass. */
   int compared;
   int chainsCompared;
   int buildsThatLanded;
   int bankSpends;
   int wildcardChains;

   // ------------------------------------------------------------ comparing

   /** Every field and every answer of the two sheets, including indices out of range. */
   void assertSameSheet(String at, LegacyPlaysheet legacy, Playsheet live) {
      compared++;
      assertArrayEquals(at + " roads", legacy.roads, live.roads);
      assertArrayEquals(at + " villages", legacy.villages, live.villages);
      assertArrayEquals(at + " cities", legacy.cities, live.cities);
      assertArrayEquals(at + " resourcesAvail", legacy.resourcesAvail, live.resourcesAvail);
      assertArrayEquals(at + " turnScore", legacy.turnScore, live.turnScore);
      assertEquals(at + " turnsNothingBuilt", legacy.turnsNothingBuilt, live.turnsNothingBuilt);

      // The old count against the new array: the same number, and built as a prefix
      assertEquals(at + " knights", legacy.knights, live.traderCount());
      assertEquals(at + " traders length", 7, live.traders.length);
      assertEquals(at + " traders[0]", false, live.traders[0]);
      for(int tier = -1; tier <= 8; tier++) {
         assertEquals(at + " isTraderBuilt " + tier, tier >= 1 && tier <= 6 && legacy.knights >= tier, live.isTraderBuilt(tier));
      }

      for(int i = -2; i <= 19; i++) {
         assertEquals(at + " canBuildRoad " + i, legacy.canBuildRoad(i), live.canBuildRoad(i));
         assertEquals(at + " canBuildVillage " + i, legacy.canBuildVillage(i), live.canBuildVillage(i));
         assertEquals(at + " canBuildCity " + i, legacy.canBuildCity(i), live.canBuildCity(i));
         assertEquals(at + " canBuildKnight " + i, legacy.canBuildKnight(i), live.canBuildKnight(i));
         assertEquals(at + " canUseKnightResource " + i, legacy.canUseKnightResource(i), live.canUseKnightResource(i));
         assertEquals(at + " isKnightResourceUsed " + i, legacy.isKnightResourceUsed(i), live.isKnightResourceUsed(i));
         assertEquals(at + " getKnightResource " + i, legacy.getKnightResource(i), live.getKnightResource(i));
         assertEquals(at + " getTurnScore " + i, legacy.getTurnScore(i), live.getTurnScore(i));
      }
      assertEquals(at + " getScore", legacy.getScore(), live.getScore());
      assertEquals(at + " bankSize", 6, live.bankSize());

      // The slot the old fixed table named for a resource is the slot the new search finds
      for(Dice.Value v : Dice.Value.values()) {
         int slot = LegacyRules.getKnightResourceIndex(v);
         int expected = legacy.canUseKnightResource(slot) ? slot : 0;
         assertEquals(at + " firstAvailableSlot " + v, expected, live.firstAvailableSlot(v));
      }
      assertEquals(at + " hasWildcard", legacy.canUseKnightResource(6), live.hasWildcard());
   }

   static void assertSameDice(String at, Dice[] legacy, Dice[] live) {
      assertEquals(at + " dice", legacy.length, live.length);
      for(int i = 0; i < legacy.length; i++) {
         assertEquals(at + " die " + i + " value", legacy[i].value, live[i].value);
         assertEquals(at + " die " + i + " used", legacy[i].used, live[i].used);
         assertEquals(at + " die " + i + " held", legacy[i].held, live[i].held);
      }
   }

   /** Every chain the two plans hold, for every kind and every index in and out of range. */
   void assertSamePlan(String at, LegacyBuildPlan legacy, BuildPlan live, Dice[] legacyDice, LegacyPlaysheet legacySheet,
         Dice[] liveDice, Playsheet liveSheet, boolean deep) {
      assertEquals(at + " hasDiceOnlyBuild", legacy.hasDiceOnlyBuild(), live.hasDiceOnlyBuild());
      for(int k = 0; k < 4; k++) {
         for(int i = 0; i < 32; i++) {
            LegacyBuildPlan.Chain a = legacy.chainFor(LegacyBuildPlan.Kind.values()[k], i);
            BuildPlan.Chain b = live.chainFor(BuildPlan.Kind.values()[k], i);
            String what = at + " chain " + BuildPlan.Kind.values()[k] + " " + i;
            assertEquals(what + " exists", a != null, b != null);
            if(a == null) {
               continue;
            }
            chainsCompared++;
            assertEquals(what + " steps", a.steps.size(), b.steps.size());
            for(int s = 0; s < a.steps.size(); s++) {
               assertEquals(what + " step " + s + " kind", a.steps.get(s).kind.ordinal(), b.steps.get(s).kind.ordinal());
               assertEquals(what + " step " + s + " index", a.steps.get(s).index, b.steps.get(s).index);
            }
            assertEquals(what + " spendsBank", a.spendsBank, b.spendsBank);
            assertEquals(what + " spendsWildcard", a.spendsWildcard, b.spendsWildcard);
            assertEquals(what + " diceUsed", a.diceUsed, b.diceUsed);
            if(b.spendsWildcard) {
               wildcardChains++;
            }
            if(deep) {
               assertEquals(what + " maxDiceSpendableThrough",
                     LegacyBuildPlan.maxDiceSpendableThrough(legacyDice, legacySheet, a),
                     BuildPlan.maxDiceSpendableThrough(liveDice, liveSheet, b));
            }
         }
      }
   }

   void assertSameGame(String at, LegacyGame legacy, Game live) {
      assertSameSheet(at, legacy.playsheet, live.playsheet);
      assertSameDice(at, legacy.dice, live.dice);
      assertEquals(at + " rolls", legacy.rolls, live.rolls);
      assertEquals(at + " turnsTaken", legacy.turnsTaken, live.turnsTaken);
      assertEquals(at + " builtResourceThisTurn", legacy.builtResourceThisTurn, live.builtResourceThisTurn);
      assertEquals(at + " knightResources", legacy.knightResources, live.knightResources);
      assertEquals(at + " isGameDone", legacy.isGameDone(), live.isGameDone());
      for(int i = -1; i <= 18; i++) {
         assertEquals(at + " game.canBuildRoad " + i, legacy.canBuildRoad(i), live.canBuildRoad(i));
         assertEquals(at + " game.canBuildVillage " + i, legacy.canBuildVillage(i), live.canBuildVillage(i));
         assertEquals(at + " game.canBuildCity " + i, legacy.canBuildCity(i), live.canBuildCity(i));
         assertEquals(at + " game.canBuildKnight " + i, legacy.canBuildKnight(i), live.canBuildKnight(i));
      }
      // canBuildX rebuilds the knightResources lists as a side effect; they must still agree
      assertEquals(at + " knightResources after asking", legacy.knightResources, live.knightResources);
      for(int r = 0; r < RECIPES.length; r++) {
         assertEquals(at + " canBuildStatus recipe " + r, legacy.canBuildStatus(LEGACY_RECIPES[r]), live.canBuildStatus(RECIPES[r]));
      }
   }

   // ------------------------------------------------------------ driving

   /** One random position kind and index, often out of range or not buildable. */
   static void buildOnSheets(Random random, LegacyPlaysheet legacy, Playsheet live, boolean wantLegal) {
      for(int attempt = 0; attempt < (wantLegal ? 40 : 1); attempt++) {
         int kind = random.nextInt(4);
         int index = random.nextInt(21) - 2;
         boolean legal;
         switch(kind) {
            case 0:
               legal = legacy.canBuildRoad(index);
               break;
            case 1:
               legal = legacy.canBuildVillage(index);
               break;
            case 2:
               legal = legacy.canBuildCity(index);
               break;
            default:
               legal = legacy.canBuildKnight(index);
         }
         if(wantLegal && !legal) {
            continue;
         }
         switch(kind) {
            case 0:
               legacy.buildRoad(index);
               live.buildRoad(index);
               break;
            case 1:
               legacy.buildVillage(index);
               live.buildVillage(index);
               break;
            case 2:
               legacy.buildCity(index);
               live.buildCity(index);
               break;
            default:
               legacy.buildKnight(index);
               live.buildKnight(index);
         }
         return;
      }
   }

   static Dice[] randomDice(Random random) {
      Dice[] dice = new Dice[6];
      for(int i = 0; i < 6; i++) {
         dice[i] = new Dice();
         // Mostly rolled faces, sometimes an unrolled die, sometimes one already spent
         dice[i].value = random.nextInt(12) == 0 ? Dice.Value.None : FACES[random.nextInt(6)];
         dice[i].used = random.nextInt(6) == 0;
         dice[i].held = random.nextBoolean();
      }
      return dice;
   }

   static Dice[] copy(Dice[] dice) {
      Dice[] copy = new Dice[dice.length];
      for(int i = 0; i < dice.length; i++) {
         copy[i] = dice[i].copy();
      }
      return copy;
   }

   @Test
   public void freshSheetsAreTheSame() {
      assertSameSheet("fresh", new LegacyPlaysheet(), new Playsheet(MapDef.TARNHOLM));
      assertSameGame("fresh game", new LegacyGame(), new Game(MapDef.TARNHOLM));
      assertTrue(compared > 0);
   }

   /**
    * Ending a turn hands back whatever the turn had set aside from the bank. In play the
    * list only ever names slots that are still available, so nothing moves; a save file
    * can say otherwise, and then the old code reopened the slot. So does the new.
    */
   @Test
   public void endingATurnReturnsWhatWasSetAside() {
      LegacyGame legacy = new LegacyGame();
      Game live = new Game(MapDef.TARNHOLM);
      legacy.playsheet.knights = 2;
      live.playsheet.setTraderCount(2);
      for(int slot : new int[] { 2, 4, 6 }) {
         legacy.playsheet.resourcesAvail[slot] = false;
         live.playsheet.resourcesAvail[slot] = false;
      }
      for(Dice.Value v : new Dice.Value[] { Dice.Value.Grain, Dice.Value.Any, Dice.Value.None, Dice.Value.Gold, Dice.Value.Ore }) {
         legacy.knightResources.add(v);
         live.knightResources.add(v);
      }
      legacy.newTurn(false);
      live.newTurn(false);
      assertSameGame("after the turn", legacy, live);
      assertTrue(live.playsheet.resourcesAvail[2]);
      assertTrue(live.playsheet.resourcesAvail[6]);
      assertEquals(false, live.playsheet.resourcesAvail[4]);
   }

   /**
    * The sheets on their own: builds in random order (legal ones, refused ones, indices
    * off either end), bank slots spent directly, empty turns, turn scoring, and against
    * random dice the static rules and the whole build plan.
    */
   @Test
   public void sheetsStayIdenticalUnderRandomActions() {
      for(int seed = 0; seed < SEEDS; seed++) {
         Random random = new Random(seed);
         LegacyPlaysheet legacy = new LegacyPlaysheet();
         Playsheet live = new Playsheet(MapDef.TARNHOLM);
         int turn = 0;
         for(int step = 0; step < 120; step++) {
            String at = "seed " + seed + " step " + step;
            int action = random.nextInt(20);
            if(action < 7) {
               buildOnSheets(random, legacy, live, true);
            }
            else if(action < 11) {
               buildOnSheets(random, legacy, live, false);
            }
            else if(action < 13) {
               int slot = random.nextInt(10) - 1;
               assertEquals(at + " useKnightResource " + slot, legacy.useKnightResource(slot), live.useKnightResource(slot));
            }
            else if(action < 14) {
               legacy.turnsNothingBuilt++;
               live.turnsNothingBuilt++;
            }
            else if(action < 16) {
               // Mostly the next turn in order, sometimes a turn that does not exist
               int t = random.nextInt(5) == 0 ? random.nextInt(20) - 2 : ++turn;
               legacy.scoreTurn(t);
               live.scoreTurn(t);
            }
            else if(action < 18) {
               // Pay for a random recipe with random dice, affordable or not
               int r = random.nextInt(4);
               Dice[] legacyDice = randomDice(random);
               Dice[] liveDice = copy(legacyDice);
               ArrayList<Dice.Value> legacyBank = new ArrayList<Dice.Value>();
               ArrayList<Dice.Value> liveBank = new ArrayList<Dice.Value>();
               assertEquals(at + " canBuildStatus",
                     LegacyRules.canBuildStatus(LEGACY_RECIPES[r], legacyDice, legacy, legacyBank),
                     Game.canBuildStatus(RECIPES[r], liveDice, live, liveBank));
               assertEquals(at + " bank list after canBuildStatus", legacyBank, liveBank);
               boolean before6 = legacy.resourcesAvail[6];
               LegacyRules.useResources(LEGACY_RECIPES[r], legacyDice, legacy, legacyBank);
               Game.useResources(RECIPES[r], liveDice, live, liveBank);
               assertSameDice(at + " after useResources", legacyDice, liveDice);
               assertEquals(at + " bank list after useResources", legacyBank, liveBank);
               if(before6 && !legacy.resourcesAvail[6]) {
                  bankSpends++;
               }
            }
            else {
               Dice[] legacyDice = randomDice(random);
               Dice[] liveDice = copy(legacyDice);
               int rolls = random.nextInt(8) == 0 ? 0 : 1 + random.nextInt(3);
               assertSamePlan(at, LegacyBuildPlan.compute(legacyDice, legacy, rolls), BuildPlan.compute(liveDice, live, rolls),
                     legacyDice, legacy, liveDice, live, step % 3 == 0);
               // Planning must not have touched what it was given
               assertSameDice(at + " after planning", legacyDice, liveDice);
            }
            assertSameSheet(at, legacy, live);
            assertSameSheet(at + " copy", legacy.copy(), live.copy());
         }
      }
      assertTrue("sheets were compared", compared >= SEEDS * 240);
      assertTrue("plans held chains to compare, had " + chainsCompared, chainsCompared > 1000);
      assertTrue("the wildcard was spent, " + bankSpends + " times", bankSpends > 10);
   }

   /**
    * Whole games: the live Game against the frozen game flow, turn by turn. Each turn
    * rolls the same dice into both, compares the plans, then builds through Game.build
    * (targets the plan offers, targets it does not, and indices that do not exist), pays
    * for recipes it cannot afford, spends bank slots, and ends the turn, which scores it.
    * Half the games start from a board already partly built so the late positions,
    * traders and the bank all come into play.
    */
   @Test
   public void gamesStayIdenticalTurnByTurn() {
      int finished = 0;
      for(int seed = 0; seed < SEEDS; seed++) {
         Random random = new Random(1000003L * seed + 17);
         LegacyGame legacy = new LegacyGame();
         Game live = new Game(MapDef.TARNHOLM);
         if(seed % 2 == 1) {
            int free = random.nextInt(25);
            for(int i = 0; i < free; i++) {
               buildOnSheets(random, legacy.playsheet, live.playsheet, true);
            }
         }
         assertSameGame("seed " + seed + " start", legacy, live);

         for(int turn = 0; turn < 17; turn++) {
            int rollsThisTurn = 1 + random.nextInt(3);
            for(int roll = 1; roll <= rollsThisTurn && !legacy.builtResourceThisTurn; roll++) {
               String at = "seed " + seed + " turn " + turn + " roll " + roll;
               if(!legacy.isGameDone()) {
                  for(int i = 0; i < 6; i++) {
                     if(!legacy.dice[i].held) {
                        Dice.Value face = FACES[random.nextInt(6)];
                        legacy.dice[i].value = face;
                        live.dice[i].value = face;
                     }
                     if(random.nextInt(4) == 0) {
                        legacy.dice[i].held = !legacy.dice[i].held;
                        live.dice[i].held = legacy.dice[i].held;
                     }
                  }
                  legacy.rolls++;
                  live.rolls++;
               }
               assertSameGame(at, legacy, live);
               LegacyBuildPlan legacyPlan = legacy.getBuildPlan();
               assertSamePlan(at, legacyPlan, live.getBuildPlan(), legacy.dice, legacy.playsheet, live.dice, live.playsheet,
                     (seed + turn) % 4 == 0);

               int actions = random.nextInt(4);
               for(int a = 0; a < actions; a++) {
                  String act = at + " action " + a;
                  int pick = random.nextInt(10);
                  if(pick < 6) {
                     // A target the plan offers, if it offers any
                     ArrayList<int[]> offered = new ArrayList<int[]>();
                     LegacyBuildPlan plan = legacy.getBuildPlan();
                     for(int k = 0; k < 4; k++) {
                        for(int i = 0; i < 32; i++) {
                           if(plan.chainFor(LegacyBuildPlan.Kind.values()[k], i) != null) {
                              offered.add(new int[] { k, i });
                           }
                        }
                     }
                     if(!offered.isEmpty()) {
                        int[] target = offered.get(random.nextInt(offered.size()));
                        int scoreBefore = legacy.playsheet.getScore();
                        legacy.build(LegacyBuildPlan.Kind.values()[target[0]], target[1]);
                        live.build(BuildPlan.Kind.values()[target[0]], target[1]);
                        if(legacy.playsheet.getScore() != scoreBefore) {
                           buildsThatLanded++;
                        }
                     }
                  }
                  else if(pick < 8) {
                     // Any position at all, mostly ones that will be refused
                     int k = random.nextInt(4);
                     int i = random.nextInt(21) - 2;
                     legacy.build(LegacyBuildPlan.Kind.values()[k], i);
                     live.build(BuildPlan.Kind.values()[k], i);
                  }
                  else if(pick < 9) {
                     int r = random.nextInt(4);
                     legacy.useResources(LEGACY_RECIPES[r]);
                     live.useResources(RECIPES[r]);
                  }
                  else {
                     int slot = random.nextInt(8);
                     assertEquals(act + " useKnightResource", legacy.playsheet.useKnightResource(slot), live.playsheet.useKnightResource(slot));
                  }
                  assertSameGame(act, legacy, live);
               }
            }
            legacy.newTurn(false);
            live.newTurn(false);
            assertSameGame("seed " + seed + " after turn " + turn, legacy, live);
         }
         assertTrue("seed " + seed + " finished", live.isGameDone());
         assertEquals("seed " + seed + " turns", 15, live.turnsTaken);
         finished++;
      }
      assertEquals(SEEDS, finished);
      assertTrue("builds landed, " + buildsThatLanded, buildsThatLanded > SEEDS * 5);
      assertTrue("chains spending the wildcard were compared, " + wildcardChains, wildcardChains > 10);
   }
}
