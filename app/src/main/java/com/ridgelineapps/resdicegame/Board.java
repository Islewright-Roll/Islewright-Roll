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

import java.util.ArrayList;
import java.util.HashMap;

/**
 * One island's geometry, in board pixels (504x504, origin top left). Everything a piece
 * needs to be placed is computed from the map's seven hex centres and its piece tables
 * (see MapDef); nothing is digitised by hand. The numbers come from the bake's geometry
 * file under tools/.
 *
 * Hexes are flat topped, 72 wide and 72 tall. Vertices from a centre: E (+36, 0),
 * NE (+18, -36), NW (-18, -36), W (-36, 0), SW (-18, +36), SE (+18, +36). An edge is
 * named by the neighbour it faces. Anchors: the edge midpoint for a road, the vertex
 * for a village or city, an offset from the hex centre for a knight.
 *
 * Get one with forMap(); a map's board is built and checked once, then cached.
 */
final class Board {
   static final int WIDTH = 504;
   static final int HEIGHT = 504;
   static final int CENTRE_X = 252;
   static final int CENTRE_Y = 252;

   // Hexes, as indices into MapDef.hex
   static final int NW = 0, SW = 1, S = 2, SE = 3, NE = 4, N = 5, CENTRE = 6;

   // Vertices, and edges as the two vertices they join
   static final int V_E = 0, V_NE = 1, V_NW = 2, V_W = 3, V_SW = 4, V_SE = 5;
   static final int[][] VERTEX = { { 36, 0 }, { 18, -36 }, { -18, -36 }, { -36, 0 }, { -18, 36 }, { 18, 36 } };
   static final int E_N = 0, E_NE = 1, E_SE = 2, E_S = 3, E_SW = 4, E_NW = 5;
   static final int[][] EDGE = {
      { V_NW, V_NE }, { V_NE, V_E }, { V_E, V_SE }, { V_SW, V_SE }, { V_W, V_SW }, { V_W, V_NW },
   };

   // Road sprite shapes, chosen from the edge's two vertices and never from its name
   static final int ROAD_H = 0, ROAD_SLASH = 1, ROAD_BACKSLASH = 2;
   /** Round cap the road sprites carry beyond each vertex. */
   static final int ROAD_CAP = 4;

   /** No two anchors may sit closer than this, or a tap could not tell them apart. */
   static final int MIN_ANCHOR_GAP = 14;

   final MapDef map;
   final Pt[][] roadEnds;
   final Pt[] roadAnchor;
   final Pt[] roadOrigin;
   final int[] roadShape;
   final Pt[] village;
   final Pt[] city;
   final Pt[] knight;

   private static final HashMap<String, Board> CACHE = new HashMap<String, Board>();

   /** The board of a map, built and checked the first time it is asked for. */
   static synchronized Board forMap(MapDef map) {
      Board board = CACHE.get(map.id);
      if(board == null || board.map != map) {
         board = new Board(map);
         CACHE.put(map.id, board);
      }
      return board;
   }

   private Board(MapDef map) {
      this.map = map;
      roadEnds = roadEnds(map.hex, map.roads);
      int roads = map.roads.length;
      roadAnchor = new Pt[roads];
      roadOrigin = new Pt[roads];
      roadShape = new int[roads];
      for(int i = 0; i < roads; i++) {
         Pt left = roadEnds[i][0];
         Pt right = roadEnds[i][1];
         roadAnchor[i] = new Pt((left.x + right.x) / 2, (left.y + right.y) / 2);
         if(left.y == right.y) {
            roadShape[i] = ROAD_H;
            roadOrigin[i] = new Pt(left.x - ROAD_CAP, left.y - ROAD_CAP);
         }
         else if(left.y > right.y) {
            // Lower left to upper right
            roadShape[i] = ROAD_SLASH;
            roadOrigin[i] = new Pt(left.x - ROAD_CAP, right.y - ROAD_CAP);
         }
         else {
            roadShape[i] = ROAD_BACKSLASH;
            roadOrigin[i] = new Pt(left.x - ROAD_CAP, left.y - ROAD_CAP);
         }
      }
      village = new Pt[map.villages.length];
      for(int i = 0; i < village.length; i++) {
         village[i] = vertex(map.hex, map.villages[i][0], map.villages[i][1]);
      }
      city = new Pt[map.cities.length];
      for(int i = 0; i < city.length; i++) {
         city[i] = vertex(map.hex, map.cities[i][0], map.cities[i][1]);
      }
      knight = new Pt[map.traders.length];
      for(int i = 0; i < knight.length; i++) {
         int[] t = map.traders[i];
         knight[i] = new Pt(map.hex[t[0]][0] + t[1], map.hex[t[0]][1] + t[2]);
      }
      check();
   }

   static Pt vertex(int[][] hex, int h, int vertex) {
      return new Pt(hex[h][0] + VERTEX[vertex][0], hex[h][1] + VERTEX[vertex][1]);
   }

   /** Each road's two vertices, the left one first (the first of the edge's two on a tie). */
   static Pt[][] roadEnds(int[][] hex, int[][] roads) {
      Pt[][] ends = new Pt[roads.length][];
      for(int i = 0; i < roads.length; i++) {
         int[] edge = EDGE[roads[i][1]];
         Pt a = vertex(hex, roads[i][0], edge[0]);
         Pt b = vertex(hex, roads[i][0], edge[1]);
         ends[i] = a.x <= b.x ? new Pt[] { a, b } : new Pt[] { b, a };
      }
      return ends;
   }

   /**
    * Per road, every other road that shares a vertex with it, in ascending order: the
    * "any touching road" rule as a roadNeeds table. Entry 0 is left empty, because the
    * start road needs nothing. For a map whose roads follow from its geometry; a map
    * whose road order is a rule of its own writes its table by hand.
    */
   static int[][] adjacency(int[][] hex, int[][] roads) {
      Pt[][] ends = roadEnds(hex, roads);
      int[][] needs = new int[roads.length][];
      needs[0] = new int[0];
      for(int i = 1; i < roads.length; i++) {
         ArrayList<Integer> touching = new ArrayList<Integer>();
         for(int j = 0; j < roads.length; j++) {
            if(j != i && meet(ends[i], ends[j])) {
               touching.add(j);
            }
         }
         needs[i] = new int[touching.size()];
         for(int k = 0; k < needs[i].length; k++) {
            needs[i][k] = touching.get(k);
         }
      }
      return needs;
   }

   private static boolean meet(Pt[] a, Pt[] b) {
      return a[0].equals(b[0]) || a[0].equals(b[1]) || a[1].equals(b[0]) || a[1].equals(b[1]);
   }

   boolean roadTouches(int road, Pt p) {
      return roadEnds[road][0].equals(p) || roadEnds[road][1].equals(p);
   }

   boolean roadsMeet(int a, int b) {
      return meet(roadEnds[a], roadEnds[b]);
   }

   /** Squared distance from a board point to the line between a road's two vertices. */
   float roadDistanceSquared(int road, int x, int y) {
      Pt a = roadEnds[road][0];
      Pt b = roadEnds[road][1];
      float dx = b.x - a.x;
      float dy = b.y - a.y;
      float t = ((x - a.x) * dx + (y - a.y) * dy) / (dx * dx + dy * dy);
      t = Math.max(0, Math.min(1, t));
      float px = a.x + t * dx - x;
      float py = a.y + t * dy - y;
      return px * px + py * py;
   }

   /**
    * The map's tables are only right if its rules still hold on its geometry, so this
    * checks them once per map: every road shares a vertex with each road that permits
    * it, every village and city sits on an end of each road that permits it, the long
    * road (if the map has one) can be reached from the start road, and no two anchors
    * are close enough to be confused under a finger.
    */
   private void check() {
      for(int i = 0; i < map.roads.length; i++) {
         for(int need : map.roadNeeds[i]) {
            require(need >= 0 && need < map.roads.length && need != i, "road " + i + " needs road " + need);
            require(roadsMeet(i, need), "road " + i + " does not meet road " + need);
         }
      }
      for(int i = 0; i < village.length; i++) {
         for(int need : map.villageRoads[i]) {
            require(need >= 0 && need < map.roads.length, "village " + i + " needs road " + need);
            require(roadTouches(need, village[i]), "village " + i + " is off road " + need);
         }
      }
      for(int i = 0; i < city.length; i++) {
         for(int need : map.cityRoads[i]) {
            require(need >= 0 && need < map.roads.length, "city " + i + " needs road " + need);
            require(roadTouches(need, city[i]), "city " + i + " is off road " + need);
         }
      }

      // Every road must be buildable eventually, the long road among them
      boolean[] reached = new boolean[map.roads.length];
      reached[0] = true;
      boolean grew = true;
      while(grew) {
         grew = false;
         for(int i = 1; i < reached.length; i++) {
            if(reached[i]) {
               continue;
            }
            for(int need : map.roadNeeds[i]) {
               if(reached[need]) {
                  reached[i] = true;
                  grew = true;
                  break;
               }
            }
         }
      }
      for(int i = 0; i < reached.length; i++) {
         require(reached[i], (i == map.longRoad ? "the long road " : "road ") + i + " cannot be reached from the start road");
      }

      ArrayList<Pt> anchors = new ArrayList<Pt>();
      ArrayList<String> names = new ArrayList<String>();
      for(int i = 0; i < roadAnchor.length; i++) {
         anchors.add(roadAnchor[i]);
         names.add("road " + i);
      }
      for(int i = 0; i < village.length; i++) {
         anchors.add(village[i]);
         names.add("village " + i);
      }
      for(int i = 0; i < city.length; i++) {
         anchors.add(city[i]);
         names.add("city " + i);
      }
      for(int i = 0; i < knight.length; i++) {
         anchors.add(knight[i]);
         names.add("knight " + (i + 1));
      }
      for(int i = 0; i < anchors.size(); i++) {
         for(int j = i + 1; j < anchors.size(); j++) {
            int dx = anchors.get(i).x - anchors.get(j).x;
            int dy = anchors.get(i).y - anchors.get(j).y;
            require(dx * dx + dy * dy >= MIN_ANCHOR_GAP * MIN_ANCHOR_GAP,
                  names.get(i) + " and " + names.get(j) + " are less than " + MIN_ANCHOR_GAP + " px apart");
         }
      }
   }

   private void require(boolean ok, String what) {
      if(!ok) {
         throw new IllegalStateException("Board geometry (" + map.id + "): " + what);
      }
   }
}
