package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/** Whether a touch counts as a tap on the spot where it began. */
public class TapSlopTest {
   @Test
   public void aTouchThatNeverMovedIsATap() {
      assertTrue(TapSlop.isTap(100, 200, 100, 200, 42));
   }

   @Test
   public void aWobbleInsideTheSlopIsATap() {
      assertTrue(TapSlop.isTap(100, 200, 110, 215, 42));
      assertTrue(TapSlop.isTap(100, 200, 80, 190, 42));
   }

   @Test
   public void exactlyTheSlopIsStillATap() {
      assertTrue(TapSlop.isTap(0, 0, 30, 40, 50));
      assertTrue(TapSlop.isTap(0, 0, 0, -50, 50));
   }

   @Test
   public void anythingFurtherIsASwipe() {
      assertFalse(TapSlop.isTap(0, 0, 31, 40, 50));
      assertFalse(TapSlop.isTap(100, 200, 100, 400, 42));
      assertFalse(TapSlop.isTap(500, 500, 100, 500, 42));
   }

   @Test
   public void distanceIsStraightLineNotPerAxis() {
      // 35 across and 35 down is 49.5 away: past 42 though neither axis is
      assertFalse(TapSlop.isTap(0, 0, 35, 35, 42));
      assertTrue(TapSlop.isTap(0, 0, 29, 29, 42));
   }
}
