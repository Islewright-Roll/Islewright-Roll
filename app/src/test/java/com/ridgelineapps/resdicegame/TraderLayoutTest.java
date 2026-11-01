package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import java.io.IOException;

import org.junit.Test;

/**
 * Where Dunholm's desert pairs stand and where the one resource each pair shares rests. The
 * carts and the shared resource sit inside their hex and clear of every road, village and
 * castle, by whole pixels, worked out from the sprites' own pixels.
 */
public class TraderLayoutTest {
   /** Pixels of clear air kept round the pair and what it shares, measured in a square ring. */
   static final int CLEARANCE = 1;

   private static boolean[][] icon() throws IOException {
      // The 12 px wildcard icon is the middle of the atlas sprite: see GameView's iconAny
      boolean[][] big = VoyageTest.mask(VoyageTest.read("icon_any_12"));
      return big;
   }

   private static boolean inHex(int[] centre, int x, int y, int margin) {
      int dx = Math.abs(x - centre[0]), dy = Math.abs(y - centre[1]);
      return 2 * dx + dy <= 72 - 2 * margin && dy <= 36 - margin;
   }

   private static boolean near(boolean[][] sprite, Pt at, int x, int y, int ring) {
      for(int dy = -ring; dy <= ring; dy++) {
         for(int dx = -ring; dx <= ring; dx++) {
            int sx = x + dx - at.x, sy = y + dy - at.y;
            if(sy >= 0 && sy < sprite.length && sx >= 0 && sx < sprite[sy].length && sprite[sy][sx]) {
               return true;
            }
         }
      }
      return false;
   }

   @Test
   public void aPairStandsOneAboveTheOtherWithRoomBetweenForItsSharedResource() {
      Pt[] at = TraderLayout.anchors(MapDef.DUNHOLM);
      for(int[] pair : new int[][] { { 6, 7 }, { 8, 9 } }) {
         Pt upper = at[pair[0] - 1], lower = at[pair[1] - 1];
         assertEquals(upper.x, lower.x);
         assertEquals(30, lower.y - upper.y);
         // the upper cart's 18 opaque rows, then 12 clear, then the lower cart's 18
         assertEquals(12, (lower.y - Art.TRADER_AY) - (upper.y - Art.TRADER_AY + Art.TRADER_BOX_H));
      }
   }

   @Test
   public void onlyASlotWithSeveralTradersHasASharedResourceAndItIsBetweenThem() {
      MapDef d = MapDef.DUNHOLM;
      Pt[] at = TraderLayout.anchors(d);
      for(int slot = 1; slot < d.bankResource.length; slot++) {
         Pt shared = TraderLayout.sharedAt(d, slot);
         if(d.bankTraders[slot].length < 2) {
            assertNull(shared);
            continue;
         }
         Pt a = at[d.bankTraders[slot][0] - 1], b = at[d.bankTraders[slot][1] - 1];
         // centred on the carts, and between their opaque boxes, a row clear above and below
         assertEquals(a.x, shared.x + Art.SHARED_CELL / 2);
         int top = a.y - Art.TRADER_AY + Art.TRADER_BOX_H;
         int bottom = b.y - Art.TRADER_AY;
         assertTrue(shared.y > top);
         assertTrue(shared.y + 10 < bottom);
      }
      for(int slot = 1; slot < MapDef.TARNHOLM.bankResource.length; slot++) {
         assertNull(TraderLayout.sharedAt(MapDef.TARNHOLM, slot));
      }
   }

   @Test
   public void theCartsAndTheSharedResourceAreInsideTheirHexAndClearOfTheRoadsAndPieces() throws IOException {
      MapDef map = MapDef.DUNHOLM;
      Board board = Board.forMap(map);
      boolean[][] village = VoyageTest.mask(VoyageTest.read("piece_village"));
      boolean[][] castle = VoyageTest.mask(VoyageTest.read("piece_castle"));
      boolean[][] trader = VoyageTest.mask(VoyageTest.read("piece_trader"));
      boolean[][][] roads = { VoyageTest.mask(VoyageTest.read("road_h")), VoyageTest.mask(VoyageTest.read("road_slash")),
            VoyageTest.mask(VoyageTest.read("road_backslash")) };
      boolean[][] any = icon();
      Pt[] at = TraderLayout.anchors(map);
      for(int slot = 6; slot <= 7; slot++) {
         int[] centre = map.hex[map.traders[map.bankTraders[slot][0] - 1][0]];
         // every solid pixel of the two carts and of the shared resource
         java.util.ArrayList<int[]> solid = new java.util.ArrayList<int[]>();
         for(int tier : map.bankTraders[slot]) {
            Pt p = at[tier - 1];
            for(int y = 0; y < trader.length; y++) {
               for(int x = 0; x < trader[y].length; x++) {
                  if(trader[y][x]) {
                     solid.add(new int[] { p.x - Art.TRADER_AX + x, p.y - Art.TRADER_AY + y });
                  }
               }
            }
         }
         Pt shared = TraderLayout.sharedAt(map, slot);
         // the icon asset may be bigger than its 12 px cell: its inset is GameView's
         int inset = (any.length - Art.SHARED_CELL) / 2;
         for(int y = 0; y < Art.SHARED_CELL; y++) {
            for(int x = 0; x < Art.SHARED_CELL; x++) {
               if(any[y + inset][x + inset]) {
                  solid.add(new int[] { shared.x + x, shared.y + y });
               }
            }
         }
         for(int[] px : solid) {
            String where = "slot " + slot + " pixel " + px[0] + "," + px[1];
            assertTrue(where + " in the hex", inHex(centre, px[0], px[1], CLEARANCE));
            for(int i = 0; i < board.roadOrigin.length; i++) {
               assertFalse(where + " road " + i, near(roads[board.roadShape[i]], board.roadOrigin[i], px[0], px[1], CLEARANCE));
            }
            for(int i = 0; i < board.village.length; i++) {
               Pt v = board.village[i];
               assertFalse(where + " village " + i, near(village, new Pt(v.x - Art.VILLAGE_AX, v.y - Art.VILLAGE_AY), px[0], px[1], CLEARANCE));
            }
            for(int i = 0; i < board.city.length; i++) {
               Pt c = board.city[i];
               assertFalse(where + " castle " + i, near(castle, new Pt(c.x - Art.CASTLE_AX, c.y - Art.CASTLE_AY), px[0], px[1], CLEARANCE));
            }
         }
      }
   }

   @Test
   public void theSharedResourceTouchesNeitherCart() throws IOException {
      MapDef map = MapDef.DUNHOLM;
      boolean[][] trader = VoyageTest.mask(VoyageTest.read("piece_trader"));
      boolean[][] any = icon();
      int inset = (any.length - Art.SHARED_CELL) / 2;
      Pt[] at = TraderLayout.anchors(map);
      for(int slot = 6; slot <= 7; slot++) {
         Pt shared = TraderLayout.sharedAt(map, slot);
         for(int tier : map.bankTraders[slot]) {
            Pt p = at[tier - 1];
            for(int y = 0; y < Art.SHARED_CELL; y++) {
               for(int x = 0; x < Art.SHARED_CELL; x++) {
                  if(any[y + inset][x + inset]) {
                     assertFalse("slot " + slot + " tier " + tier,
                           near(trader, new Pt(p.x - Art.TRADER_AX, p.y - Art.TRADER_AY), shared.x + x, shared.y + y, 0));
                  }
               }
            }
         }
      }
   }

   @Test
   public void tarnholmTradersAreWhereTheyWere() {
      Pt[] at = TraderLayout.anchors(MapDef.TARNHOLM);
      for(int i = 0; i < at.length; i++) {
         int[] t = MapDef.TARNHOLM.traders[i];
         assertEquals(MapDef.TARNHOLM.hex[t[0]][0] + t[1], at[i].x);
         assertEquals(MapDef.TARNHOLM.hex[t[0]][1] + t[2], at[i].y);
      }
   }
}
