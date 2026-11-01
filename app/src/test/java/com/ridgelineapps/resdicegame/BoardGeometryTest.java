package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import java.util.ArrayList;
import java.util.Arrays;

import org.junit.Test;

/**
 * The geometry of every map, and Tarnholm's against the tables and the hand facts the
 * old static Board carried before its tables moved into MapDef.
 */
public class BoardGeometryTest {
   // FROZEN: the old Board's tables, as plain numbers. Hexes NW 0, SW 1, S 2, SE 3, NE 4,
   // N 5, CENTRE 6. Edges N 0, NE 1, SE 2, S 3, SW 4, NW 5. Vertices E 0, NE 1, NW 2, W 3,
   // SW 4, SE 5.
   static final int[][] OLD_HEX = {
      { 198, 216 }, { 198, 288 }, { 252, 324 }, { 306, 288 }, { 306, 216 }, { 252, 180 }, { 252, 252 },
   };
   static final int[][] OLD_VERTEX = { { 36, 0 }, { 18, -36 }, { -18, -36 }, { -36, 0 }, { -18, 36 }, { 18, 36 } };
   static final int[][] OLD_EDGE = { { 2, 1 }, { 1, 0 }, { 0, 5 }, { 4, 5 }, { 3, 4 }, { 3, 2 } };
   static final int[][] OLD_ROADS = {
      { 0, 1 }, { 0, 2 }, { 0, 3 }, { 1, 1 }, { 1, 2 }, { 1, 3 }, { 2, 4 }, { 2, 3 }, { 2, 2 }, { 2, 1 },
      { 3, 5 }, { 4, 4 }, { 4, 5 }, { 3, 3 }, { 3, 2 }, { 3, 1 }, { 4, 2 },
   };
   static final int[][] OLD_VILLAGES = { { 0, 0 }, { 1, 0 }, { 2, 4 }, { 2, 0 }, { 6, 0 }, { 5, 0 } };
   static final int[][] OLD_CITIES = { { 0, 4 }, { 1, 4 }, { 3, 0 }, { 4, 0 } };
   static final int[] OLD_KNIGHTS = { 0, 1, 2, 3, 4, 5 };
   static final Dice.Value[] OLD_KNIGHT_RESOURCE = {
      Dice.Value.Ore, Dice.Value.Grain, Dice.Value.Wool, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Any,
   };
   static final int[] OLD_VILLAGE_POINTS = { 3, 4, 5, 7, 9, 11 };
   static final int[] OLD_CITY_POINTS = { 7, 12, 20, 30 };
   static final int[] OLD_ROAD_NEEDS = { -1, 0, 1, 1, 3, 4, 4, 6, 7, 8, 9, 10, 11, 8, 13, 14, 15 };
   static final int[] OLD_VILLAGE_NEEDS = { 0, 3, 6, 8, 10, 12 };
   static final int[] OLD_CITY_NEEDS = { 2, 5, 14, 16 };

   static Pt oldVertex(int hex, int vertex) {
      return new Pt(OLD_HEX[hex][0] + OLD_VERTEX[vertex][0], OLD_HEX[hex][1] + OLD_VERTEX[vertex][1]);
   }

   @Test
   public void everyMapBuildsItsBoard() {
      assertEquals(2, MapDef.ALL.length);
      for(MapDef map : MapDef.ALL) {
         Board board = Board.forMap(map);
         assertSame(map, board.map);
         assertSame("cached", board, Board.forMap(map));
         assertSame(map, MapDef.byId(map.id));
      }
      assertSame(MapDef.TARNHOLM, MapDef.DEFAULT);
      assertSame(MapDef.TARNHOLM, MapDef.byId("tarnholm"));
      assertSame("an unknown id is the default map", MapDef.TARNHOLM, MapDef.byId("nowhere"));
      assertSame(MapDef.TARNHOLM, MapDef.byId(null));
   }

   @Test
   public void tarnholmTablesAreTheOldTables() {
      MapDef map = MapDef.TARNHOLM;
      assertEquals("tarnholm", map.id);
      assertEquals(0xFF0D3947, map.seaFill);
      assertArrayEquals(OLD_HEX, map.hex);
      assertArrayEquals(OLD_VERTEX, Board.VERTEX);
      assertArrayEquals(OLD_EDGE, Board.EDGE);
      assertArrayEquals(OLD_ROADS, map.roads);
      assertArrayEquals(OLD_VILLAGES, map.villages);
      assertArrayEquals(OLD_CITIES, map.cities);
      assertEquals(OLD_KNIGHTS.length, map.traders.length);
      for(int i = 0; i < OLD_KNIGHTS.length; i++) {
         assertArrayEquals("trader " + (i + 1), new int[] { OLD_KNIGHTS[i], 0, 0 }, map.traders[i]);
         // What the view used to read as KNIGHT_RESOURCE[tier - 1]
         assertEquals(OLD_KNIGHT_RESOURCE[i], map.bankResource[map.traderBank[i]]);
         assertEquals(i + 1, map.traderPoints[i]);
      }
      assertArrayEquals(OLD_VILLAGE_POINTS, map.villagePoints);
      assertArrayEquals(OLD_CITY_POINTS, map.cityPoints);
      assertArrayEquals(new int[0], map.roadNeeds[0]);
      for(int i = 1; i < OLD_ROAD_NEEDS.length; i++) {
         assertArrayEquals("road " + i, new int[] { OLD_ROAD_NEEDS[i] }, map.roadNeeds[i]);
      }
      for(int i = 0; i < OLD_VILLAGE_NEEDS.length; i++) {
         assertArrayEquals(new int[] { OLD_VILLAGE_NEEDS[i] }, map.villageRoads[i]);
      }
      for(int i = 0; i < OLD_CITY_NEEDS.length; i++) {
         assertArrayEquals(new int[] { OLD_CITY_NEEDS[i] }, map.cityRoads[i]);
      }
      assertEquals(15, map.turnCap);
      assertEquals(0, map.targetScore);
      assertEquals(1, map.roadPoints);
      assertEquals(2, map.emptyTurnPenalty);
      assertEquals(-1, map.longRoad);
      assertEquals(0, map.traderBonusAt);
      // Tarnholm shows no point ranges in its key: they read as noise beside the costs
   }

   /** Every point the old static initialiser computed, by the old formulas. */
   @Test
   public void tarnholmPointsAreTheOldPoints() {
      Board board = Board.forMap(MapDef.TARNHOLM);
      assertEquals(17, board.roadEnds.length);
      for(int i = 0; i < OLD_ROADS.length; i++) {
         int[] edge = OLD_EDGE[OLD_ROADS[i][1]];
         Pt a = oldVertex(OLD_ROADS[i][0], edge[0]);
         Pt b = oldVertex(OLD_ROADS[i][0], edge[1]);
         Pt left = a.x <= b.x ? a : b;
         Pt right = a.x <= b.x ? b : a;
         assertEquals("road " + i + " left", left, board.roadEnds[i][0]);
         assertEquals("road " + i + " right", right, board.roadEnds[i][1]);
         assertEquals("road " + i + " anchor", new Pt((a.x + b.x) / 2, (a.y + b.y) / 2), board.roadAnchor[i]);
         int shape;
         Pt origin;
         if(left.y == right.y) {
            shape = 0;
            origin = new Pt(left.x - 4, left.y - 4);
         }
         else if(left.y > right.y) {
            shape = 1;
            origin = new Pt(left.x - 4, right.y - 4);
         }
         else {
            shape = 2;
            origin = new Pt(left.x - 4, left.y - 4);
         }
         assertEquals("road " + i + " shape", shape, board.roadShape[i]);
         assertEquals("road " + i + " origin", origin, board.roadOrigin[i]);
      }
      assertEquals(6, board.village.length);
      for(int i = 0; i < OLD_VILLAGES.length; i++) {
         assertEquals("village " + i, oldVertex(OLD_VILLAGES[i][0], OLD_VILLAGES[i][1]), board.village[i]);
      }
      assertEquals(4, board.city.length);
      for(int i = 0; i < OLD_CITIES.length; i++) {
         assertEquals("city " + i, oldVertex(OLD_CITIES[i][0], OLD_CITIES[i][1]), board.city[i]);
      }
      assertEquals(6, board.knight.length);
      for(int i = 0; i < OLD_KNIGHTS.length; i++) {
         assertEquals("knight " + i, new Pt(OLD_HEX[OLD_KNIGHTS[i]][0], OLD_HEX[OLD_KNIGHTS[i]][1]), board.knight[i]);
      }
      assertEquals(0, Board.ROAD_H);
      assertEquals(1, Board.ROAD_SLASH);
      assertEquals(2, Board.ROAD_BACKSLASH);
      assertEquals(4, Board.ROAD_CAP);
   }

   /** The facts the old check() asserted by hand, in literal coordinates. */
   @Test
   public void tarnholmHandFacts() {
      Board board = Board.forMap(MapDef.TARNHOLM);
      Pt hub = new Pt(216, 252);
      assertTrue(board.roadTouches(1, hub) && board.roadTouches(2, hub) && board.roadTouches(3, hub));
      assertEquals(new Pt(234, 216), board.village[0]);
      assertTrue(board.roadTouches(0, board.village[0]) && board.roadTouches(1, board.village[0]));
      assertEquals(new Pt(288, 180), board.village[5]);
      assertTrue(board.roadTouches(12, board.village[5]));
      assertTrue(board.roadsMeet(15, 16));
      assertTrue(board.roadTouches(15, new Pt(324, 252)) && board.roadTouches(16, new Pt(324, 252)));
      assertEquals(new Pt(342, 288), board.city[2]);
      assertEquals(new Pt(342, 216), board.city[3]);
      assertEquals(new Pt(225, 198), board.roadAnchor[0]);
      assertEquals(new Pt(216, 180), board.roadEnds[0][0]);
      assertEquals(0f, board.roadDistanceSquared(0, 225, 198), 0.001f);
      assertEquals(100f, board.roadDistanceSquared(2, 198, 262), 0.001f);
   }

   /** Tarnholm's one-parent road table is a choice among the roads that touch. */
   @Test
   public void tarnholmParentsAreAmongTheTouchingRoads() {
      MapDef map = MapDef.TARNHOLM;
      int[][] touching = Board.adjacency(map.hex, map.roads);
      assertEquals(0, touching[0].length);
      for(int i = 1; i < map.roads.length; i++) {
         boolean found = false;
         for(int j : touching[i]) {
            found |= j == map.roadNeeds[i][0];
         }
         assertTrue("road " + i, found);
      }
   }

   /** The anchors of a board, named, for the spacing tests. */
   static ArrayList<String> anchorNames(Board board) {
      ArrayList<String> names = new ArrayList<String>();
      for(int i = 0; i < board.roadAnchor.length; i++) {
         names.add("road " + i);
      }
      for(int i = 0; i < board.village.length; i++) {
         names.add("village " + i);
      }
      for(int i = 0; i < board.city.length; i++) {
         names.add("city " + i);
      }
      for(int i = 0; i < board.knight.length; i++) {
         names.add("knight " + (i + 1));
      }
      return names;
   }

   static ArrayList<Pt> anchorPoints(Board board) {
      ArrayList<Pt> points = new ArrayList<Pt>();
      points.addAll(Arrays.asList(board.roadAnchor));
      points.addAll(Arrays.asList(board.village));
      points.addAll(Arrays.asList(board.city));
      points.addAll(Arrays.asList(board.knight));
      return points;
   }

   /** Worked out here from the points, not read from Board.check(): no two anchors within 14 px. */
   @Test
   public void noTwoAnchorsOnAnyMapAreWithinFourteenPixels() {
      for(MapDef map : MapDef.ALL) {
         Board board = Board.forMap(map);
         ArrayList<Pt> points = anchorPoints(board);
         ArrayList<String> names = anchorNames(board);
         assertEquals(map.id, points.size(), names.size());
         int closest = Integer.MAX_VALUE;
         for(int i = 0; i < points.size(); i++) {
            for(int j = i + 1; j < points.size(); j++) {
               int dx = points.get(i).x - points.get(j).x;
               int dy = points.get(i).y - points.get(j).y;
               closest = Math.min(closest, dx * dx + dy * dy);
               assertTrue(map.id + ": " + names.get(i) + " and " + names.get(j),
                     dx * dx + dy * dy >= Board.MIN_ANCHOR_GAP * Board.MIN_ANCHOR_GAP);
            }
         }
         assertTrue(map.id + " closest pair " + closest, closest >= 14 * 14);
      }
   }

   /**
    * Dunholm's close pairs (under 24 px), each with its squared distance. The upper
    * trader of the north desert's pair sits 21 px from the middle of the road on that
    * desert's north west edge; no other trader is near anything. The rest
    * are a road beside the village or castle at its end, which Tarnholm has too.
    */
   @Test
   public void dunholmsClosePairsAreTheKnownOnes() {
      Board board = Board.forMap(MapDef.DUNHOLM);
      ArrayList<Pt> points = anchorPoints(board);
      ArrayList<String> names = anchorNames(board);
      ArrayList<String> close = new ArrayList<String>();
      for(int i = 0; i < points.size(); i++) {
         for(int j = i + 1; j < points.size(); j++) {
            int dx = points.get(i).x - points.get(j).x;
            int dy = points.get(i).y - points.get(j).y;
            if(dx * dx + dy * dy < 24 * 24) {
               close.add(names.get(i) + " / " + names.get(j) + " " + (dx * dx + dy * dy));
            }
         }
      }
      assertEquals(Arrays.asList(
            "road 0 / village 2 405",
            "road 1 / village 2 405",
            "road 2 / city 0 324",
            "road 3 / village 4 405",
            "road 4 / village 4 405",
            "road 5 / city 2 324",
            "road 6 / village 6 405", "road 7 / village 6 324",
            "road 8 / city 3 405", "road 9 / city 3 324",
            "road 10 / village 5 405", "road 11 / village 5 405",
            "road 12 / city 1 324",
            "road 13 / village 3 405", "road 14 / village 3 405",
            "road 15 / village 1 324", "road 16 / village 1 405",
            "road 17 / village 0 324", "road 17 / knight 6 441"), close);
   }

   /** check() has to bite: a table that disagrees with the geometry does not build. */
   @Test
   public void aWrongTableIsRefused() {
      MapDef.Builder offRoad = RaceFixture.builder();
      offRoad.villageRoads = new int[][] { { 17 }, { 15, 16 }, { 0, 1 }, { 13, 14 }, { 3, 4 }, { 10, 11 }, { 6, 9 } };
      assertRefused(offRoad, "village 6 is off road 9");

      MapDef.Builder apart = RaceFixture.builder();
      apart.roadNeeds = MapDef.single(-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 2);
      assertRefused(apart, "road 17 does not meet road 2");

      MapDef.Builder island = RaceFixture.builder();
      island.roadNeeds = RaceFixture.builder().roadNeeds.clone();
      island.roadNeeds[6] = new int[0];
      island.roadNeeds[5] = new int[] { 6 };
      island.roadNeeds[7] = new int[] { 6, 8 };
      island.roadNeeds[4] = new int[] { 3 };
      // Road 6 now needs nothing it can get, and road 5 only road 6
      assertRefused(island, "cannot be reached");

      MapDef.Builder crowded = RaceFixture.builder();
      crowded.traders = crowded.traders.clone();
      crowded.traders[5] = new int[] { Board.N, 30, 0 };
      assertRefused(crowded, "px apart");
   }

   static void assertRefused(MapDef.Builder builder, String because) {
      try {
         Board.forMap(builder.build());
         fail("expected the board to be refused: " + because);
      }
      catch(IllegalStateException e) {
         assertTrue(e.getMessage(), e.getMessage().contains(because));
      }
   }
}
