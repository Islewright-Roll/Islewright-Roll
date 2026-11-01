package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/**
 * When hiring a trader should ask "Start a second pair?": the rule alone, on Tarnholm (one
 * trader to a bank slot, so never) and Dunholm (the desert hexes' traders 6 and 7, and 8 and
 * 9, share a slot each and earn one "?" between them).
 */
public class SecondPairTest {
   static boolean[] hired(MapDef map, int... tiers) {
      boolean[] hired = new boolean[map.traders.length + 1];
      for(int tier : tiers) {
         hired[tier] = true;
      }
      return hired;
   }

   static boolean asks(MapDef map, int tier, int... alreadyHired) {
      return Playsheet.startsSecondPair(map, hired(map, alreadyHired), tier);
   }

   @Test
   public void tarnholmNeverAsks() {
      for(int tier = 1; tier <= 6; tier++) {
         assertFalse(asks(MapDef.TARNHOLM, tier));
         assertFalse(asks(MapDef.TARNHOLM, tier, 1, 2, 3, 4, 5, 6));
      }
   }

   @Test
   public void theFirstTraderOfTheFirstPairDoesNotAsk() {
      assertFalse(asks(MapDef.DUNHOLM, 6));
      assertFalse(asks(MapDef.DUNHOLM, 7));
      assertFalse(asks(MapDef.DUNHOLM, 8, 1, 2, 3));
   }

   @Test
   public void theSecondTraderOfTheSamePairDoesNotAsk() {
      assertFalse(asks(MapDef.DUNHOLM, 7, 6));
      assertFalse(asks(MapDef.DUNHOLM, 6, 7));
      assertFalse(asks(MapDef.DUNHOLM, 9, 8));
   }

   @Test
   public void theFirstTraderOfTheOtherPairAsksWhileTheFirstPairIsHalfDone() {
      assertTrue(asks(MapDef.DUNHOLM, 8, 6));
      assertTrue(asks(MapDef.DUNHOLM, 9, 6));
      assertTrue(asks(MapDef.DUNHOLM, 8, 7));
      assertTrue(asks(MapDef.DUNHOLM, 6, 8));
      assertTrue(asks(MapDef.DUNHOLM, 7, 9));
      // single traders hired besides change nothing
      assertTrue(asks(MapDef.DUNHOLM, 9, 1, 2, 6));
   }

   @Test
   public void itDoesNotAskOnceTheFirstPairIsComplete() {
      assertFalse(asks(MapDef.DUNHOLM, 8, 6, 7));
      assertFalse(asks(MapDef.DUNHOLM, 9, 6, 7));
      assertFalse(asks(MapDef.DUNHOLM, 6, 8, 9));
   }

   @Test
   public void itDoesNotAskForTheSecondTraderOfTheOtherPairEither() {
      // both pairs half done: finishing either one is no new pair
      assertFalse(asks(MapDef.DUNHOLM, 7, 6, 8));
      assertFalse(asks(MapDef.DUNHOLM, 9, 6, 8));
   }

   @Test
   public void aPlainSingleTraderNeverAsksEvenWithAPairHalfDone() {
      for(int tier = 1; tier <= 5; tier++) {
         assertFalse(asks(MapDef.DUNHOLM, tier, 6));
         assertFalse(asks(MapDef.DUNHOLM, tier, 8));
      }
   }

   @Test
   public void aTraderAlreadyHiredAndANonsenseTierNeverAsk() {
      assertFalse(asks(MapDef.DUNHOLM, 6, 6, 8));
      assertFalse(asks(MapDef.DUNHOLM, 0, 6));
      assertFalse(asks(MapDef.DUNHOLM, 10, 6));
   }

   @Test
   public void aChainAsksWhenAnyStepWould() {
      MapDef d = MapDef.DUNHOLM;
      assertTrue(Playsheet.chainStartsSecondPair(d, hired(d, 6), new int[] { 1, 8 }));
      assertTrue(Playsheet.chainStartsSecondPair(d, hired(d, 6), new int[] { 8 }));
      assertFalse(Playsheet.chainStartsSecondPair(d, hired(d, 6), new int[] { 7, 8 }));
      assertFalse(Playsheet.chainStartsSecondPair(d, hired(d), new int[] { 1, 2 }));
      assertFalse(Playsheet.chainStartsSecondPair(d, hired(d), new int[0]));
   }

   @Test
   public void aChainThatItselfLeavesAPairHalfDoneBeforeStartingAnotherAsks() {
      MapDef d = MapDef.DUNHOLM;
      // 6 then 8 with nothing hired: the first step starts the first pair (fine), the second starts another
      assertTrue(Playsheet.chainStartsSecondPair(d, hired(d), new int[] { 6, 8 }));
      // 6, 7 then 8: the first pair is whole by then
      assertFalse(Playsheet.chainStartsSecondPair(d, hired(d), new int[] { 6, 7, 8 }));
   }

   @Test
   public void theRuleDoesNotChangeWhatItIsGiven() {
      MapDef d = MapDef.DUNHOLM;
      boolean[] h = hired(d, 6);
      Playsheet.chainStartsSecondPair(d, h, new int[] { 8, 9 });
      assertFalse(h[8] || h[9]);
   }

   @Test
   public void thePlaysheetAsksTheSameAboutItsOwnTraders() {
      Playsheet sheet = new Playsheet(MapDef.DUNHOLM);
      assertFalse(sheet.traders[6]);
      sheet.traders[6] = true;
      assertTrue(Playsheet.startsSecondPair(sheet.map, sheet.traders, 8));
   }
}
