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

/**
 * One island as data: where its pieces sit, what each needs before it can be built, what
 * the bank holds, how the sheet scores and when the game ends. Playsheet, BuildPlan, Game
 * and Board read everything map specific from here, so a second island is a second table
 * and no second rules class.
 *
 * Conventions, which every reader relies on:
 *   roads, villages and cities are 0 based; road 0 is the start road and is prebuilt.
 *   trader tiers and bank slots are 1 based. `traders`, `traderBank` and `traderPoints`
 *   are indexed by tier - 1; `bankResource` and `bankTraders` are indexed by slot and
 *   carry an unused entry at index 0.
 *   a "needs" list is ANY ONE OF for roads (roadNeeds, villageRoads, cityRoads) and
 *   ALL OF for the traders behind a bank slot (bankTraders).
 *
 * Geometry names (hexes, vertices, edges) are the constants in Board.
 */
final class MapDef {
   final String id;                 // file keys, never shown
   final int nameRes;               // string resource of the map's name
   final int boardRes;              // the baked board drawable
   final int seaFill;               // the board's own sea colour, painted wherever it does not reach
   final int[][] hex;               // 7 centres, in Board's order NW, SW, S, SE, NE, N, CENTRE
   final int[][] roads;             // { hex, edge } per road
   final int[][] roadNeeds;         // per road, roads of which any one built permits it; {} for road 0
   final int[][] villages;          // { hex, vertex }
   final int[][] villageRoads;      // per village, roads of which any one built permits it
   final boolean orderedVillages;   // true: village i also needs village i - 1
   final int[][] cities;            // { hex, vertex }
   final int[][] cityRoads;
   final boolean orderedCities;
   final int[][] traders;           // per tier: { hex, dx, dy }, the anchor as an offset from the hex centre
   final boolean orderedTraders;    // true: tier t needs tier t - 1
   final int[] traderBank;          // per tier: the bank slot it feeds, 0 for none
   final Dice.Value[] bankResource; // per bank slot
   final int[][] bankTraders;       // per bank slot: the tiers that must all be built
   final boolean bankMatchesOnce;   // true: one banked resource pays for one needed; false: Tarnholm's old rule, see Game.useResources
   final int roadPoints;            // per built road, the start road excepted
   final int[] traderPoints;        // per tier
   final int[] villagePoints;
   final int[] cityPoints;
   final int emptyTurnPenalty;      // per turn in which nothing was built
   final int longRoad;              // the road that earns longRoadBonus, -1 for none
   final int longRoadBonus;
   final int traderBonusAt;         // traders built that earn traderBonus, 0 for never
   final int traderBonus;
   final int turnCap;               // the game ends after this many turns; 0 for no cap
   final int targetScore;           // reaching it ends the game; 0 for none
   final boolean lowerIsBetter;     // top scores: fewest turns rather than most points

   /** Mutable twin of the fields above, filled in and handed to the constructor. */
   static final class Builder {
      String id;
      int nameRes;
      int boardRes;
      int seaFill;
      int[][] hex;
      int[][] roads;
      int[][] roadNeeds;
      int[][] villages;
      int[][] villageRoads;
      boolean orderedVillages;
      int[][] cities;
      int[][] cityRoads;
      boolean orderedCities;
      int[][] traders;
      boolean orderedTraders;
      int[] traderBank;
      Dice.Value[] bankResource;
      int[][] bankTraders;
      boolean bankMatchesOnce;
      int roadPoints;
      int[] traderPoints;
      int[] villagePoints;
      int[] cityPoints;
      int emptyTurnPenalty;
      int longRoad = -1;
      int longRoadBonus;
      int traderBonusAt;
      int traderBonus;
      int turnCap;
      int targetScore;
      boolean lowerIsBetter;

      MapDef build() {
         return new MapDef(this);
      }
   }

   private MapDef(Builder b) {
      id = b.id;
      nameRes = b.nameRes;
      boardRes = b.boardRes;
      seaFill = b.seaFill;
      hex = b.hex;
      roads = b.roads;
      roadNeeds = b.roadNeeds;
      villages = b.villages;
      villageRoads = b.villageRoads;
      orderedVillages = b.orderedVillages;
      cities = b.cities;
      cityRoads = b.cityRoads;
      orderedCities = b.orderedCities;
      traders = b.traders;
      orderedTraders = b.orderedTraders;
      traderBank = b.traderBank;
      bankResource = b.bankResource;
      bankTraders = b.bankTraders;
      bankMatchesOnce = b.bankMatchesOnce;
      roadPoints = b.roadPoints;
      traderPoints = b.traderPoints;
      villagePoints = b.villagePoints;
      cityPoints = b.cityPoints;
      emptyTurnPenalty = b.emptyTurnPenalty;
      longRoad = b.longRoad;
      longRoadBonus = b.longRoadBonus;
      traderBonusAt = b.traderBonusAt;
      traderBonus = b.traderBonus;
      turnCap = b.turnCap;
      targetScore = b.targetScore;
      lowerIsBetter = b.lowerIsBetter;
      validate();
   }

   /** The tables must agree with each other on their lengths before anything indexes them. */
   private void validate() {
      require(id != null && id.length() > 0, "no id");
      require(hex.length == 7, "hex must have 7 centres");
      require(roads.length > 0 && roadNeeds.length == roads.length, "roadNeeds must have one entry per road");
      require(roads.length <= 32, "BuildPlan keys allow at most 32 roads");
      require(villageRoads.length == villages.length && villagePoints.length == villages.length, "village tables differ in length");
      require(cityRoads.length == cities.length && cityPoints.length == cities.length, "city tables differ in length");
      require(traderBank.length == traders.length && traderPoints.length == traders.length, "trader tables differ in length");
      require(bankTraders.length == bankResource.length && bankResource.length >= 1, "bank tables differ in length");
      for(int tier = 1; tier <= traders.length; tier++) {
         int slot = traderBank[tier - 1];
         require(slot >= 0 && slot < bankResource.length, "trader " + tier + " feeds bank slot " + slot);
         if(slot > 0) {
            boolean listed = false;
            for(int t : bankTraders[slot]) {
               listed |= t == tier;
            }
            require(listed, "bank slot " + slot + " does not list trader " + tier);
         }
      }
      for(int slot = 1; slot < bankResource.length; slot++) {
         require(bankTraders[slot].length > 0, "bank slot " + slot + " has no trader");
         for(int tier : bankTraders[slot]) {
            require(tier >= 1 && tier <= traders.length && traderBank[tier - 1] == slot, "bank slot " + slot + " lists trader " + tier);
         }
      }
      require(longRoad >= -1 && longRoad < roads.length, "longRoad " + longRoad);
      require(turnCap >= 0, "turnCap " + turnCap);
      // A game must end somehow: after so many turns, or on reaching a score
      require(turnCap > 0 || targetScore > 0, "no turn cap and no target score");
   }

   /**
    * The most a game on this map can score: every piece built, both bonuses earned and no
    * empty turn. Read from the same tables as Playsheet.getScore, so it follows them. On a
    * race it is the highest target that can still be reached.
    */
   int maxScore() {
      int score = roadPoints * (roads.length - 1);
      for(int points : traderPoints) {
         score += points;
      }
      for(int points : villagePoints) {
         score += points;
      }
      for(int points : cityPoints) {
         score += points;
      }
      if(longRoad >= 0) {
         score += longRoadBonus;
      }
      if(traderBonusAt > 0 && traderBonusAt <= traders.length) {
         score += traderBonus;
      }
      return score;
   }

   private void require(boolean ok, String what) {
      if(!ok) {
         throw new IllegalStateException("Map " + id + ": " + what);
      }
   }

   /** Wraps a one-parent table as "any one of" lists; -1 becomes the empty list. */
   static int[][] single(int... needs) {
      int[][] lists = new int[needs.length][];
      for(int i = 0; i < needs.length; i++) {
         lists[i] = needs[i] < 0 ? new int[0] : new int[] { needs[i] };
      }
      return lists;
   }

   /**
    * The first island: a lake at its heart, fifteen turns, highest score wins. Its
    * roads are a tree with one named parent each, which is a rule of the sheet and not
    * something the geometry implies, so the table is explicit.
    */
   static final MapDef TARNHOLM = tarnholm();

   private static MapDef tarnholm() {
      Builder b = new Builder();
      b.id = "tarnholm";
      b.nameRes = R.string.map_tarnholm;
      b.boardRes = R.drawable.board_tarnholm;
      b.seaFill = 0xFF0D3947;
      b.hex = new int[][] {
         { 198, 216 }, { 198, 288 }, { 252, 324 }, { 306, 288 }, { 306, 216 }, { 252, 180 }, { 252, 252 },
      };
      b.roads = new int[][] {
         { Board.NW, Board.E_NE }, { Board.NW, Board.E_SE }, { Board.NW, Board.E_S },
         { Board.SW, Board.E_NE }, { Board.SW, Board.E_SE }, { Board.SW, Board.E_S },
         { Board.S, Board.E_SW }, { Board.S, Board.E_S }, { Board.S, Board.E_SE },
         { Board.S, Board.E_NE }, { Board.SE, Board.E_NW }, { Board.NE, Board.E_SW },
         { Board.NE, Board.E_NW }, { Board.SE, Board.E_S }, { Board.SE, Board.E_SE },
         { Board.SE, Board.E_NE }, { Board.NE, Board.E_SE },
      };
      b.roadNeeds = single(-1, 0, 1, 1, 3, 4, 4, 6, 7, 8, 9, 10, 11, 8, 13, 14, 15);
      b.villages = new int[][] {
         { Board.NW, Board.V_E }, { Board.SW, Board.V_E }, { Board.S, Board.V_SW },
         { Board.S, Board.V_E }, { Board.CENTRE, Board.V_E }, { Board.N, Board.V_E },
      };
      b.villageRoads = single(0, 3, 6, 8, 10, 12);
      b.orderedVillages = true;
      b.cities = new int[][] {
         { Board.NW, Board.V_SW }, { Board.SW, Board.V_SW }, { Board.SE, Board.V_E }, { Board.NE, Board.V_E },
      };
      b.cityRoads = single(2, 5, 14, 16);
      b.orderedCities = true;
      b.traders = new int[][] {
         { Board.NW, 0, 0 }, { Board.SW, 0, 0 }, { Board.S, 0, 0 },
         { Board.SE, 0, 0 }, { Board.NE, 0, 0 }, { Board.N, 0, 0 },
      };
      b.orderedTraders = true;
      b.traderBank = new int[] { 1, 2, 3, 4, 5, 6 };
      b.bankResource = new Dice.Value[] {
         Dice.Value.None,
         Dice.Value.Ore, Dice.Value.Grain, Dice.Value.Wool, Dice.Value.Lumber, Dice.Value.Brick, Dice.Value.Any,
      };
      b.bankTraders = new int[][] { {}, { 1 }, { 2 }, { 3 }, { 4 }, { 5 }, { 6 } };
      // Tarnholm keeps the sheet's old habit: when a build needs a resource twice and the bank
      // holds it once, both count as found and the second is free. Byte for byte, by design.
      b.bankMatchesOnce = false;
      b.roadPoints = 1;
      b.traderPoints = new int[] { 1, 2, 3, 4, 5, 6 };
      b.villagePoints = new int[] { 3, 4, 5, 7, 9, 11 };
      b.cityPoints = new int[] { 7, 12, 20, 30 };
      b.emptyTurnPenalty = 2;
      b.longRoad = -1;
      b.longRoadBonus = 0;
      b.traderBonusAt = 0;
      b.traderBonus = 0;
      b.turnCap = 15;
      b.targetScore = 0;
      b.lowerIsBetter = false;
      return b.build();
   }

   /**
    * The second island: no lake, a race to ten points with no turn cap, fewest turns
    * wins. Roads follow from the geometry (any road that touches), villages and
    * castles go up in any order, and the two desert hexes each hold a pair of traders
    * that must both stand before their "?" can be spent. Tables as set out in the plan
    * (section 2); DunholmRulesTest pins every one of them.
    */
   static final MapDef DUNHOLM = dunholmBuilder().build();

   /** The Dunholm tables as a builder, for tests that need to break one on purpose. */
   static Builder dunholmBuilder() {
      Builder b = new Builder();
      b.id = "dunholm";
      b.nameRes = R.string.map_dunholm;
      b.boardRes = R.drawable.board_dunholm;
      b.seaFill = 0xFF0D3947;
      // The same grid as Tarnholm
      b.hex = new int[][] {
         { 198, 216 }, { 198, 288 }, { 252, 324 }, { 306, 288 }, { 306, 216 }, { 252, 180 }, { 252, 252 },
      };
      b.roads = new int[][] {
         { Board.NW, Board.E_NE }, { Board.NW, Board.E_SE }, { Board.NW, Board.E_S },
         { Board.SW, Board.E_NE }, { Board.SW, Board.E_SE }, { Board.SW, Board.E_S },
         { Board.S, Board.E_SW }, { Board.S, Board.E_S }, { Board.S, Board.E_SE },
         { Board.SE, Board.E_S }, { Board.SE, Board.E_SE }, { Board.SE, Board.E_NE },
         { Board.NE, Board.E_S }, { Board.NE, Board.E_SE }, { Board.NE, Board.E_NE },
         { Board.NE, Board.E_N }, { Board.N, Board.E_NE }, { Board.N, Board.E_N },
      };
      // Any road that touches, worked out from the shared vertices
      b.roadNeeds = Board.adjacency(b.hex, b.roads);
      b.villages = new int[][] {
         { Board.N, Board.V_NW }, { Board.N, Board.V_E }, { Board.NW, Board.V_E }, { Board.NE, Board.V_E },
         { Board.SW, Board.V_E }, { Board.SE, Board.V_E }, { Board.S, Board.V_SW },
      };
      b.villageRoads = new int[][] { { 17 }, { 15, 16 }, { 0, 1 }, { 13, 14 }, { 3, 4 }, { 10, 11 }, { 6, 7 } };
      b.orderedVillages = false;
      b.cities = new int[][] {
         { Board.NW, Board.V_SW }, { Board.CENTRE, Board.V_E }, { Board.SW, Board.V_SW }, { Board.SE, Board.V_SW },
      };
      b.cityRoads = new int[][] { { 2 }, { 12 }, { 5 }, { 8, 9 } };
      b.orderedCities = false;
      b.traders = new int[][] {
         { Board.NW, 0, 0 }, { Board.SW, 0, 0 }, { Board.S, 0, 0 }, { Board.SE, 0, 0 }, { Board.NE, 0, 0 },
         // The two traders of a desert stand one above the other, 30 px apart: a cart is 28 px
         // long and a desert holds no room for two abreast beside the roads and the village or
         // castle on its east vertex. The 12 px of room between them holds the one resource the
         // pair shares (TraderLayout.sharedAt), a row clear above and below it.
         { Board.N, 0, -15 }, { Board.N, 0, 15 }, { Board.CENTRE, 0, -15 }, { Board.CENTRE, 0, 15 },
      };
      b.orderedTraders = false;
      b.traderBank = new int[] { 1, 2, 3, 4, 5, 6, 6, 7, 7 };
      b.bankResource = new Dice.Value[] {
         Dice.Value.None, Dice.Value.Ore, Dice.Value.Grain, Dice.Value.Wool, Dice.Value.Lumber, Dice.Value.Brick,
         Dice.Value.Any, Dice.Value.Any,
      };
      b.bankTraders = new int[][] { {}, { 1 }, { 2 }, { 3 }, { 4 }, { 5 }, { 6, 7 }, { 8, 9 } };
      // A banked resource pays for one needed resource, never two
      b.bankMatchesOnce = true;
      b.roadPoints = 0;
      b.traderPoints = new int[9];
      b.villagePoints = new int[] { 1, 1, 1, 1, 1, 1, 1 };
      b.cityPoints = new int[] { 2, 2, 2, 2 };
      b.emptyTurnPenalty = 0;
      b.longRoad = 6;
      b.longRoadBonus = 2;
      b.traderBonusAt = 3;
      b.traderBonus = 2;
      // No cap: the game ends only when the target is reached, as in the source game
      b.turnCap = 0;
      b.targetScore = 10;
      b.lowerIsBetter = true;
      return b;
   }

   static final MapDef DEFAULT = TARNHOLM;
   static final MapDef[] ALL = { TARNHOLM, DUNHOLM };

   /**
    * The map the app opens on: the one the Map setting names. Each map keeps its own saved
    * game, so a save cannot decide; a setting nobody knows (or none at all, as in a file
    * from before there were two maps) is the default map.
    */
   static MapDef forLaunch(String settingsMapId) {
      return byId(settingsMapId);
   }

   /** The map with this id; an id nobody knows is the default map. */
   static MapDef byId(String id) {
      for(MapDef map : ALL) {
         if(map.id.equals(id)) {
            return map;
         }
      }
      return DEFAULT;
   }
}
