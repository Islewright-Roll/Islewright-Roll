package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;

import org.junit.Test;

/** The fading glow round the traders still to be hired on a map that hires them in order. */
public class GlowFadeTest {
   private static boolean[] hired(int count) {
      boolean[] hired = new boolean[7];
      for(int tier = 1; tier <= count; tier++) {
         hired[tier] = true;
      }
      return hired;
   }

   @Test
   public void theNextTraderGlowsInFullAndTheRestFade() {
      boolean[] hired = hired(0);
      assertEquals(0, GlowFade.step(hired, 1));
      assertEquals(1, GlowFade.step(hired, 2));
      assertEquals(2, GlowFade.step(hired, 3));
      assertEquals(3, GlowFade.step(hired, 4));
      assertEquals(5, GlowFade.step(hired, 6));
   }

   @Test
   public void theFadeMovesUpAsTradersAreHired() {
      boolean[] hired = hired(2);
      assertEquals(-1, GlowFade.step(hired, 1));
      assertEquals(-1, GlowFade.step(hired, 2));
      assertEquals(0, GlowFade.step(hired, 3));
      assertEquals(1, GlowFade.step(hired, 4));
      assertEquals(2, GlowFade.step(hired, 5));
      assertEquals(3, GlowFade.step(hired, 6));
   }

   @Test
   public void strengthIsFullThenHalfThenAFifthThenNone() {
      assertEquals(1.0f, GlowFade.strength(0), 0f);
      assertEquals(0.5f, GlowFade.strength(1), 0f);
      assertEquals(0.2f, GlowFade.strength(2), 0f);
      assertEquals(0f, GlowFade.strength(3), 0f);
      assertEquals(0f, GlowFade.strength(5), 0f);
      assertEquals(0f, GlowFade.strength(-1), 0f);
   }

   @Test
   public void aHiredOrMissingTierHasNoStep() {
      boolean[] hired = hired(3);
      assertEquals(-1, GlowFade.step(hired, 3));
      assertEquals(-1, GlowFade.step(hired, 0));
      assertEquals(-1, GlowFade.step(hired, 7));
   }

   @Test
   public void aSkippedTraderStillCountsAsAheadOfTheOnesAfterIt() {
      // Not reachable on Tarnholm, where traders are hired in order, but the rule is only
      // "how many unhired ones come before this one"
      boolean[] hired = new boolean[7];
      hired[2] = true;
      assertEquals(0, GlowFade.step(hired, 1));
      assertEquals(1, GlowFade.step(hired, 3));
   }
}
