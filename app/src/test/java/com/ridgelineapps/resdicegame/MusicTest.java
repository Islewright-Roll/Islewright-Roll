/*
 * Copyright (C) 2012 Resource Dice Game (http://code.google.com/p/android-dice-game)
 * 
 * This program is free software: you can redistribute it and/or 
 * modify it under the terms of the GNU General Public License as published 
 * by the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *   
 * This source code is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.util.HashSet;
import java.util.Random;
import java.util.Set;

import org.junit.Test;

/** The pure parts of the soundtrack: the play order and the volume curve. */
public class MusicTest {

   @Test
   public void gainIsTheSettingAsAFractionSquared() {
      assertEquals(0f, MusicGain.gainFor(0), 0f);
      assertEquals(1f, MusicGain.gainFor(100), 0f);
      assertEquals(0.36f, MusicGain.gainFor(60), 1e-6f);
      assertEquals(0.25f, MusicGain.gainFor(50), 1e-6f);
      assertEquals(0.0001f, MusicGain.gainFor(1), 1e-8f);
   }

   @Test
   public void gainRisesWithTheSettingAndStaysInRange() {
      float previous = -1f;
      for(int setting = 0; setting <= 100; setting++) {
         float gain = MusicGain.gainFor(setting);
         assertTrue(gain > previous);
         assertTrue(gain >= 0f && gain <= 1f);
         previous = gain;
      }
      assertEquals(0f, MusicGain.gainFor(-20), 0f);
      assertEquals(1f, MusicGain.gainFor(400), 0f);
   }

   @Test
   public void everyTrackPlaysOncePerPassInAShuffledOrder() {
      for(int seed = 0; seed < 50; seed++) {
         MusicPlaylist list = new MusicPlaylist(10, new Random(seed));
         for(int pass = 0; pass < 3; pass++) {
            Set<Integer> seen = new HashSet<Integer>();
            for(int i = 0; i < 10; i++) {
               int track = list.next();
               assertTrue(track >= 0 && track < 10);
               assertTrue("repeat inside a pass", seen.add(track));
            }
         }
      }
   }

   @Test
   public void noTrackRepeatsStraightAwayAcrossTheLoopBoundary() {
      for(int count = 2; count <= 10; count++) {
         for(int seed = 0; seed < 300; seed++) {
            MusicPlaylist list = new MusicPlaylist(count, new Random(seed));
            int previous = list.next();
            for(int i = 1; i < count * 6; i++) {
               int track = list.next();
               assertTrue("count " + count + " seed " + seed + " step " + i, track != previous);
               previous = track;
            }
         }
      }
   }

   @Test
   public void theOrderChangesWithTheSeed() {
      Set<Integer> firsts = new HashSet<Integer>();
      for(int seed = 0; seed < 40; seed++) {
         firsts.add(new MusicPlaylist(10, new Random(seed)).next());
      }
      assertTrue(firsts.size() > 3);
   }

   @Test
   public void oneTrackRepeatsAndNoTracksGiveMinusOne() {
      MusicPlaylist one = new MusicPlaylist(1, new Random(1));
      assertEquals(0, one.next());
      assertEquals(0, one.next());
      assertEquals(-1, new MusicPlaylist(0, new Random(1)).next());
   }
}
