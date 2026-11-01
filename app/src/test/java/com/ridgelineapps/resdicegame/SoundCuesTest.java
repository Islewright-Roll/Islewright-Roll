package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/** What sounds a turn end or a build asks for, and in what order at the end of a game. */
public class SoundCuesTest {

   @Test
   public void anOrdinaryTurnEndIsTheTurnSound() {
      assertEquals("[TURN_END@0]", SoundCues.turnEnd(false, false, false, false, false).toString());
   }

   @Test
   public void anEmptyTurnWithAPenaltyReplacesIt() {
      assertEquals("[EMPTY_TURN@0]", SoundCues.turnEnd(true, false, false, false, false).toString());
   }

   @Test
   public void theShipAppearsAfterTheTurnSound() {
      assertEquals("[TURN_END@0, SHIP_APPEARS@300]", SoundCues.turnEnd(false, true, false, false, false).toString());
   }

   @Test
   public void theLastTurnEndingIsTheTurnThenTheShortFanfareWithNoBell() {
      assertEquals("[TURN_END@0, GAME_OVER@300]", SoundCues.turnEnd(false, false, false, true, false).toString());
   }

   @Test
   public void aNewBestAtTheEndPlaysTheBigFanfareInsteadOfTheShortOne() {
      assertEquals("[TURN_END@0, TOP_SCORE@300]", SoundCues.turnEnd(false, false, false, true, true).toString());
   }

   @Test
   public void anEmptyLastTurnEndingHasTheSameFanfare() {
      assertEquals("[EMPTY_TURN@0, GAME_OVER@300]", SoundCues.turnEnd(true, false, false, true, false).toString());
   }

   @Test
   public void theBellRingsWhenTheShipMoorsForTheLastTurn() {
      assertEquals("[TURN_END@0, SHIP_DOCKS@300]", SoundCues.turnEnd(false, false, true, false, false).toString());
   }

   @Test
   public void theBellNeverComesBeforeTheBuildOrTurnSoundAtZero() {
      for(SoundCues.Cue cue : SoundCues.turnEnd(false, false, true, false, false)) {
         if(cue.sfx == Sfx.SHIP_DOCKS) {
            assertTrue(cue.delayMs >= 300);
         }
      }
   }

   @Test
   public void nothingFollowsABuildThatEarnedAndEndedNothing() {
      assertEquals("[]", SoundCues.afterBuild(false, false, false).toString());
   }

   @Test
   public void aBonusChimesAfterTheBuild() {
      assertEquals("[BONUS@250]", SoundCues.afterBuild(true, false, false).toString());
   }

   @Test
   public void aBuildThatWinsARaceIsFollowedByTheFanfare() {
      assertEquals("[GAME_OVER@300]", SoundCues.afterBuild(false, true, false).toString());
      assertEquals("[TOP_SCORE@300]", SoundCues.afterBuild(false, true, true).toString());
   }

   @Test
   public void aWinningBuildThatAlsoEarnedABonusChimesFirst() {
      assertEquals("[BONUS@250, TOP_SCORE@700]", SoundCues.afterBuild(true, true, true).toString());
   }

   @Test
   public void bonusTransitionIsDetectedOnlyWhenABitIsNewlySet() {
      assertFalse(SoundCues.bonusGained(0, 0));
      assertTrue(SoundCues.bonusGained(0, 1));
      assertTrue(SoundCues.bonusGained(0, 2));
      assertTrue(SoundCues.bonusGained(1, 3));
      assertFalse("already had it", SoundCues.bonusGained(3, 3));
      assertFalse(SoundCues.bonusGained(2, 2));
   }
}
