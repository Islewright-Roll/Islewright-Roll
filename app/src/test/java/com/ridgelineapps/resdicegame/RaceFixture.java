package com.ridgelineapps.resdicegame;

/**
 * A race map for the seam tests: now simply Dunholm's tables under another id, so that a
 * test can break one table (see BoardGeometryTest.aWrongTableIsRefused) or build a map
 * that is not in MapDef.ALL without disturbing the shipped map or its cached board.
 */
final class RaceFixture {
   static final int[][] EXPECTED_ROAD_NEEDS = {
      {}, { 0, 2, 3 }, { 1, 3 }, { 1, 2, 4 }, { 3, 5, 6 }, { 4, 6 }, { 4, 5, 7 }, { 6, 8 }, { 7, 9 },
      { 8, 10 }, { 9, 11 }, { 10, 12, 13 }, { 11, 13 }, { 11, 12, 14 }, { 13, 15 }, { 14, 16 }, { 15, 17 }, { 16 },
   };

   static MapDef.Builder builder() {
      MapDef.Builder b = MapDef.dunholmBuilder();
      b.id = "race-fixture";
      return b;
   }

   static final MapDef MAP = builder().build();

   /**
    * Dunholm's tables with the twenty turn cap the island once had. No shipped map is a race
    * with a cap any more, but the code still handles one (the ship on the board, a race that
    * runs out of turns with nothing to record), and these keep that path pinned.
    */
   static final MapDef CAPPED = capped();

   private static MapDef capped() {
      MapDef.Builder b = MapDef.dunholmBuilder();
      b.id = "race-capped";
      b.turnCap = 20;
      return b.build();
   }
}
