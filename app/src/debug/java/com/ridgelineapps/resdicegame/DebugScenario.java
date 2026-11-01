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

import java.io.File;
import java.util.Arrays;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import android.content.Context;

/**
 * A test scenario for the debug harness: dice, and enough of Playsheet's board state
 * (roads, villages, cities, traders, spent bank slots) to set up situations that would
 * be impractical to reach by playing, on either island.
 *
 * fromJson() parses AND fully validates before returning: every field is checked for
 * shape and range while still local to this object, against the MapDef the scenario
 * names (array lengths, tier ranges and the turn range all come from that map), so a
 * scenario either comes back whole or throws and changes nothing. applyTo() only ever
 * runs on an object that already passed that gate, and only on a game already running
 * that same map (the loader checks), so it has nothing left to reject.
 *
 * This does not validate board legality (whether the pieces given could be reached by
 * legal play, e.g. adjacency). That is the scenario author's job; the point of this
 * harness is precision for testing, not re-deriving the rules engine.
 *
 * JSON schema, all fields optional:
 * {
 *   "map": "tarnholm",       // a MapDef id; default is MapDef.DEFAULT. Unknown ids are rejected.
 *   "dice": ["Brick", "Brick", "Lumber", "Lumber", "Wool", "Ore"],   // exactly 6, from
 *                            // Dice.Value: None, Wool, Grain, Brick, Ore, Lumber, Gold, Any
 *   "turnsTaken": 0,         // 0 .. turnCap-1 of the map (any on a map with no cap), completed turns, default 0
 *   "turnScores": [2, 3, 1], // optional; the points gained in each passed turn, exactly one
 *                            // whole number >= 0 per completed turn, so as many entries as
 *                            // turnsTaken. Without it the running totals the HUD strip reads are
 *                            // spread evenly so the scenario's score is reached by the last passed
 *                            // turn and the turn in progress shows +0. With it the totals are the
 *                            // running sum of these gains, and the turn in progress shows whatever
 *                            // the scenario's score has over their sum.
 *   "rolls": 1,              // 0-3, default 1
 *   "playsheet": {
 *     "roads": [true, false, ...],     // one boolean per road of the map, index 0 is the
 *                                      // fixed starting road
 *     "villages": [false, ...],        // one boolean per village of the map
 *     "cities": [false, ...],          // one boolean per castle of the map
 *
 *     // Traders, EITHER form (never an inconsistent mix of the two):
 *     "knights": 3,                    // maps whose traders are built in order only: the
 *                                      // highest tier built, 0..number of traders
 *     "traders": [true, false, ...],   // one boolean per trader in tier order (index 0 is
 *                                      // tier 1). Any map; on an ordered map the flags must
 *                                      // form a prefix. If "knights" is also given, the
 *                                      // flags must be exactly tiers 1..knights.
 *
 *     // Spent bank slots, EITHER key (not both): 1-based bank slots, 1..bankSize of the
 *     // map, whose banked resource has already been used
 *     "bankSpent": [6],
 *     "knightResourcesSpent": [1]      // older alias of bankSpent
 *   }
 * }
 *
 * Any field left out keeps the value a fresh Game.reset() already gives it: dice at
 * Dice.Value.None, rolls defaulting to 1 regardless, roads all false except index 0,
 * villages and cities all false, no traders, and every bank slot still banked. Existing
 * scenario files, which name no map, keep meaning the default map.
 */
public final class DebugScenario {

   public static final String FILE_NAME = "debug-scenario.json";

   MapDef map = MapDef.DEFAULT;
   Dice.Value[] dice;
   Integer turnsTaken;
   int[] turnScores;  // per passed turn gain, or null to spread the score evenly
   Integer rolls;
   boolean[] roads;
   boolean[] villages;
   boolean[] cities;
   boolean[] traders; // one per tier, already merged from "knights" or "traders"
   int[] bankSpent;

   /**
    * The file this harness reads, in the same external, no-root-needed directory
    * HighScoresAndSettings already uses, so "adb push" can write it directly.
    */
   public static File file(Context context) {
      File dir = context.getExternalFilesDir(null);
      if(dir == null) {
         dir = context.getFilesDir();
      }
      return new File(dir, FILE_NAME);
   }

   /** The map with this id, or null; unlike MapDef.byId, an unknown id is not the default. */
   static MapDef findMap(String id) {
      for(MapDef candidate : MapDef.ALL) {
         if(candidate.id.equals(id)) {
            return candidate;
         }
      }
      return null;
   }

   public static DebugScenario fromJson(String json) throws JSONException {
      JSONObject root = new JSONObject(json);
      DebugScenario scenario = new DebugScenario();

      if(root.has("map")) {
         String id = root.getString("map");
         MapDef found = findMap(id);
         if(found == null) {
            throw new JSONException("unknown map \"" + id + "\"");
         }
         scenario.map = found;
      }
      MapDef map = scenario.map;

      if(root.has("dice")) {
         JSONArray array = root.getJSONArray("dice");
         if(array.length() != 6) {
            throw new JSONException("dice must have exactly 6 entries, had " + array.length());
         }
         scenario.dice = new Dice.Value[6];
         for(int i = 0; i < 6; i++) {
            scenario.dice[i] = parseDiceValue(array.getString(i));
         }
      }

      if(root.has("turnsTaken")) {
         int turnsTaken = root.getInt("turnsTaken");
         // A map with no turn cap takes any number of completed turns
         if(turnsTaken < 0 || (map.turnCap > 0 && turnsTaken > map.turnCap - 1)) {
            throw new JSONException("turnsTaken must be between 0 and " + (map.turnCap - 1) + " on " + map.id + ", was " + turnsTaken);
         }
         scenario.turnsTaken = turnsTaken;
      }

      if(root.has("turnScores")) {
         JSONArray array = root.getJSONArray("turnScores");
         int passed = (scenario.turnsTaken != null) ? scenario.turnsTaken : 0;
         if(array.length() != passed) {
            throw new JSONException("turnScores must have exactly one entry per completed turn (" + passed + "), had " + array.length());
         }
         scenario.turnScores = new int[passed];
         for(int i = 0; i < passed; i++) {
            int gain = array.getInt(i);
            if(gain < 0) {
               throw new JSONException("turnScores entries must be 0 or more, was " + gain);
            }
            scenario.turnScores[i] = gain;
         }
      }

      if(root.has("rolls")) {
         int rolls = root.getInt("rolls");
         if(rolls < 0 || rolls > 3) {
            throw new JSONException("rolls must be between 0 and 3, was " + rolls);
         }
         scenario.rolls = rolls;
      }

      if(root.has("playsheet")) {
         JSONObject playsheet = root.getJSONObject("playsheet");
         int traderTiers = map.traders.length;

         if(playsheet.has("roads")) {
            scenario.roads = parseBooleanArray(playsheet.getJSONArray("roads"), map.roads.length, "playsheet.roads");
         }
         if(playsheet.has("villages")) {
            scenario.villages = parseBooleanArray(playsheet.getJSONArray("villages"), map.villages.length, "playsheet.villages");
         }
         if(playsheet.has("cities")) {
            scenario.cities = parseBooleanArray(playsheet.getJSONArray("cities"), map.cities.length, "playsheet.cities");
         }

         Integer knights = null;
         if(playsheet.has("knights")) {
            if(!map.orderedTraders) {
               throw new JSONException("playsheet.knights needs a map whose traders are built in order; " + map.id + " is not, use playsheet.traders");
            }
            int count = playsheet.getInt("knights");
            if(count < 0 || count > traderTiers) {
               throw new JSONException("playsheet.knights must be between 0 and " + traderTiers + " on " + map.id + ", was " + count);
            }
            knights = count;
         }
         boolean[] flags = null;
         if(playsheet.has("traders")) {
            flags = parseBooleanArray(playsheet.getJSONArray("traders"), traderTiers, "playsheet.traders");
         }
         if(flags != null && knights != null) {
            for(int i = 0; i < traderTiers; i++) {
               if(flags[i] != (i < knights)) {
                  throw new JSONException("playsheet.traders and playsheet.knights disagree at trader " + (i + 1));
               }
            }
         }
         if(flags == null && knights != null) {
            flags = new boolean[traderTiers];
            for(int i = 0; i < traderTiers; i++) {
               flags[i] = i < knights;
            }
         }
         if(flags != null && map.orderedTraders) {
            for(int i = 1; i < traderTiers; i++) {
               if(flags[i] && !flags[i - 1]) {
                  throw new JSONException("playsheet.traders must be a prefix on " + map.id + ": trader " + (i + 1) + " needs trader " + i);
               }
            }
         }
         scenario.traders = flags;

         if(playsheet.has("bankSpent") && playsheet.has("knightResourcesSpent")) {
            throw new JSONException("give playsheet.bankSpent or its alias playsheet.knightResourcesSpent, not both");
         }
         String spentKey = playsheet.has("bankSpent") ? "bankSpent" : "knightResourcesSpent";
         if(playsheet.has(spentKey)) {
            int bankSize = map.bankResource.length - 1;
            JSONArray array = playsheet.getJSONArray(spentKey);
            scenario.bankSpent = new int[array.length()];
            for(int i = 0; i < array.length(); i++) {
               int slot = array.getInt(i);
               if(slot < 1 || slot > bankSize) {
                  throw new JSONException("playsheet." + spentKey + " entries must be between 1 and " + bankSize + " on " + map.id + ", was " + slot);
               }
               scenario.bankSpent[i] = slot;
            }
         }
      }

      return scenario;
   }

   /**
    * Mutates the live game to match this scenario. Always starts from Game.reset(), so
    * the result never depends on whatever state the game happened to be in before this
    * was applied; every field this scenario did not mention keeps reset()'s default.
    *
    * Only call this on a scenario that came back from fromJson() without throwing, and
    * only when game.map is this scenario's map: every value here has already been range
    * checked against that map, so there is nothing left that could fail partway through.
    * Switching a running game to another map needs MainActivity to rebuild the GameView,
    * so the loader calls MainActivity.switchMap first when the maps differ and applies this to
    * the game that returns (see DebugScenarioLoader).
    */
   public void applyTo(Game game) {
      game.reset();

      if(dice != null) {
         for(int i = 0; i < 6; i++) {
            game.dice[i].reset();
            game.dice[i].setValue(dice[i]);
         }
      }

      if(turnsTaken != null) {
         game.turnsTaken = turnsTaken;
      }

      game.rolls = (rolls != null) ? rolls : 1;

      if(roads != null) {
         System.arraycopy(roads, 0, game.playsheet.roads, 0, roads.length);
      }
      if(villages != null) {
         System.arraycopy(villages, 0, game.playsheet.villages, 0, villages.length);
      }
      if(cities != null) {
         System.arraycopy(cities, 0, game.playsheet.cities, 0, cities.length);
      }
      if(traders != null) {
         for(int i = 0; i < traders.length; i++) {
            game.playsheet.traders[i + 1] = traders[i];
         }
      }
      if(bankSpent != null) {
         for(int slot : bankSpent) {
            game.playsheet.resourcesAvail[slot] = false;
         }
      }

      fillTurnScores(game.playsheet, (turnsTaken != null) ? turnsTaken : 0);

      // The tutorial would read this setup as a player's moves and draw over it
      game.tutorialPaused = true;

      if(game.gameView != null) {
         game.gameView.postInvalidate();
      }
   }

   /**
    * Fills the running totals for the turns already passed (Playsheet.turnScore, which the HUD
    * strip's per turn flags read), since the pieces above are set directly and no turn was
    * ever scored. Default: the final score rises evenly across the passed turns, whole numbers
    * that never decrease, ending on the current score so the turn in progress shows +0. With
    * "turnScores" the totals are the running sum of the given gains instead. Real play and the
    * save format are untouched: this only writes the same array scoreTurn() writes.
    */
   private void fillTurnScores(Playsheet sheet, int passed) {
      if(passed < 1) {
         return;
      }
      if(sheet.turnScore.length <= passed) {
         // A map with no turn cap grows the array as turns are scored; do the same here
         sheet.turnScore = Arrays.copyOf(sheet.turnScore, passed + 1);
      }
      int total = sheet.getScore();
      int running = 0;
      for(int turn = 1; turn <= passed; turn++) {
         if(turnScores != null) {
            running += turnScores[turn - 1];
            sheet.turnScore[turn] = running;
         }
         else {
            sheet.turnScore[turn] = Math.floorDiv(total * turn, passed);
         }
      }
   }

   private static boolean[] parseBooleanArray(JSONArray array, int length, String field) throws JSONException {
      if(array.length() != length) {
         throw new JSONException(field + " must have exactly " + length + " entries, had " + array.length());
      }
      boolean[] result = new boolean[length];
      for(int i = 0; i < length; i++) {
         result[i] = array.getBoolean(i);
      }
      return result;
   }

   private static Dice.Value parseDiceValue(String name) throws JSONException {
      try {
         return Dice.Value.valueOf(name);
      }
      catch(IllegalArgumentException e) {
         throw new JSONException("unrecognised dice value \"" + name + "\"");
      }
   }
}
