package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Iterator;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import org.junit.Test;

/**
 * A game-state.json and a debug scenario written for the build before the rules became
 * map driven (schema 1: no map id, the traders as a "knights" count) still load into the
 * same state. They are written back as schema 2; see GameStateStoreTest for that format.
 */
public class SaveCompatTest {
   /** A save as the previous build wrote it: turn 8, second roll, three traders, slot 2 spent. */
   static final String SAVE = "{\"schemaVersion\":1,\"game\":{\"rolls\":2,\"turnsTaken\":7,"
         + "\"builtResourceThisTurn\":false,\"scored\":false,\"knightResources\":[\"Ore\",\"Wool\"],"
         + "\"dice\":[{\"value\":\"Wool\",\"held\":true,\"used\":false},{\"value\":\"Gold\",\"held\":false,\"used\":false},"
         + "{\"value\":\"Brick\",\"held\":false,\"used\":true},{\"value\":\"Ore\",\"held\":true,\"used\":false},"
         + "{\"value\":\"Lumber\",\"held\":false,\"used\":false},{\"value\":\"Grain\",\"held\":false,\"used\":false}],"
         + "\"playsheet\":{\"knights\":3,\"resourcesAvail\":[false,true,false,true,true,true,true],"
         + "\"roads\":[true,true,true,true,true,false,true,false,false,false,false,false,false,false,false,false,false],"
         + "\"villages\":[true,true,false,false,false,false],\"cities\":[true,false,false,false],"
         + "\"turnsNothingBuilt\":1,\"turnScore\":[0,1,4,5,9,16,17,21,0,0,0,0,0,0,0,0]}}}";

   static Game load(String json) throws Exception {
      Method fromJson = GameStateStore.class.getDeclaredMethod("fromJson", String.class);
      fromJson.setAccessible(true);
      try {
         return (Game) fromJson.invoke(null, json);
      }
      catch(InvocationTargetException e) {
         throw (Exception) e.getCause();
      }
   }

   static String write(Game game) throws Exception {
      Method buildJson = GameStateStore.class.getDeclaredMethod("buildJson", Game.class);
      buildJson.setAccessible(true);
      return (String) buildJson.invoke(null, game);
   }

   /** A JSON value as text with its keys in order, so two documents compare by content. */
   static String canonical(Object value) throws JSONException {
      StringBuilder sb = new StringBuilder();
      if(value instanceof JSONObject) {
         JSONObject object = (JSONObject) value;
         ArrayList<String> keys = new ArrayList<String>();
         for(Iterator<String> it = object.keys(); it.hasNext();) {
            keys.add(it.next());
         }
         Collections.sort(keys);
         sb.append('{');
         for(String key : keys) {
            sb.append(key).append(':').append(canonical(object.get(key))).append(',');
         }
         sb.append('}');
      }
      else if(value instanceof JSONArray) {
         JSONArray array = (JSONArray) value;
         sb.append('[');
         for(int i = 0; i < array.length(); i++) {
            sb.append(canonical(array.get(i))).append(',');
         }
         sb.append(']');
      }
      else {
         sb.append(value.getClass().getSimpleName()).append(' ').append(value);
      }
      return sb.toString();
   }

   /** The store is only tested if the real org.json is what runs, not the android stub. */
   @Test
   public void theRealJsonLibraryIsOnTheTestClasspath() throws Exception {
      assertEquals(1, new JSONObject("{\"a\":1}").getInt("a"));
   }

   static boolean[] bits(String s) {
      boolean[] b = new boolean[s.length()];
      for(int i = 0; i < b.length; i++) {
         b[i] = s.charAt(i) == '1';
      }
      return b;
   }

   @Test
   public void anOldSaveLoadsIntoTheSameState() throws Exception {
      Game game = load(SAVE);
      assertEquals(MapDef.TARNHOLM, game.map);
      assertEquals(2, game.rolls);
      assertEquals(7, game.turnsTaken);
      assertFalse(game.builtResourceThisTurn);
      assertFalse(game.scored);
      assertEquals(Arrays.asList(Dice.Value.Ore, Dice.Value.Wool), game.knightResources);
      assertEquals(Dice.Value.Wool, game.dice[0].value);
      assertTrue(game.dice[0].held);
      assertEquals(Dice.Value.Brick, game.dice[2].value);
      assertTrue(game.dice[2].used);
      assertEquals(Dice.Value.Grain, game.dice[5].value);

      Playsheet sheet = game.playsheet;
      assertArrayEquals(bits("0111000"), sheet.traders);
      assertEquals(3, sheet.traderCount());
      assertArrayEquals(bits("0101111"), sheet.resourcesAvail);
      assertArrayEquals(bits("11111010000000000"), sheet.roads);
      assertArrayEquals(bits("110000"), sheet.villages);
      assertArrayEquals(bits("1000"), sheet.cities);
      assertEquals(1, sheet.turnsNothingBuilt);
      assertArrayEquals(new int[] { 0, 1, 4, 5, 9, 16, 17, 21, 0, 0, 0, 0, 0, 0, 0, 0 }, sheet.turnScore);

      // What the old sheet made of the same numbers
      LegacyPlaysheet legacy = new LegacyPlaysheet();
      legacy.knights = 3;
      legacy.resourcesAvail = bits("0101111");
      legacy.roads = bits("11111010000000000");
      legacy.villages = bits("110000");
      legacy.cities = bits("1000");
      legacy.turnsNothingBuilt = 1;
      assertEquals(legacy.getScore(), sheet.getScore());
      assertEquals(5 + 6 + 7 + 7 - 2, sheet.getScore());
      for(int slot = 0; slot <= 7; slot++) {
         assertEquals(legacy.canUseKnightResource(slot), sheet.canUseKnightResource(slot));
         assertEquals(legacy.isKnightResourceUsed(slot), sheet.isKnightResourceUsed(slot));
      }
      assertTrue(sheet.isKnightResourceUsed(2));
      assertTrue(sheet.canBuildKnight(4));
      assertFalse(sheet.canBuildKnight(5));
   }

   @Test
   public void anOldSaveIsRewrittenAsSchemaTwoWithTheSameState() throws Exception {
      JSONObject written = new JSONObject(write(load(SAVE)));
      assertEquals(2, written.getInt("schemaVersion"));
      assertEquals("tarnholm", written.getJSONObject("game").getString("mapId"));
      JSONObject playsheet = written.getJSONObject("game").getJSONObject("playsheet");
      assertFalse(playsheet.has("knights"));
      assertEquals("[false, true, true, true, false, false, false]", playsheet.getJSONArray("traders").toString().replace(",", ", "));

      // The rewrite is the old document with only those changes
      JSONObject expected = new JSONObject(SAVE);
      expected.put("schemaVersion", 2);
      expected.getJSONObject("game").put("mapId", "tarnholm");
      JSONObject expectedSheet = expected.getJSONObject("game").getJSONObject("playsheet");
      expectedSheet.remove("knights");
      expectedSheet.put("traders", playsheet.getJSONArray("traders"));
      // And every die says it is not locked, which the old build had no way to be
      org.json.JSONArray dice = expected.getJSONObject("game").getJSONArray("dice");
      for(int i = 0; i < dice.length(); i++) {
         dice.getJSONObject(i).put("locked", false);
      }
      assertEquals(canonical(expected), canonical(written));
   }

   @Test
   public void aFreshGameRoundTrips() throws Exception {
      Game game = new Game(MapDef.TARNHOLM);
      String once = write(game);
      String twice = write(load(once));
      assertEquals(canonical(new JSONObject(once)), canonical(new JSONObject(twice)));
      assertEquals(0, load(once).playsheet.traderCount());
   }

   @Test
   public void aTraderCountTheMapDoesNotHaveIsRefused() throws Exception {
      for(String bad : new String[] { "\"knights\":7", "\"knights\":-1" }) {
         try {
            load(SAVE.replace("\"knights\":3", bad));
            fail(bad + " should not load");
         }
         catch(JSONException expected) {
            assertTrue(expected.getMessage().contains("knights"));
         }
      }
   }

   @Test
   public void aDebugScenarioStillSetsTradersByCount() throws Exception {
      DebugScenario scenario = DebugScenario.fromJson("{\"dice\":[\"Ore\",\"Ore\",\"Ore\",\"Grain\",\"Grain\",\"Wool\"],"
            + "\"turnsTaken\":3,\"rolls\":2,\"playsheet\":{"
            + "\"roads\":[true,true,true,false,false,false,false,false,false,false,false,false,false,false,false,false,false],"
            + "\"knights\":4,\"knightResourcesSpent\":[1,4]}}");
      Game game = new Game(MapDef.TARNHOLM);
      game.playsheet.buildKnight(1);
      scenario.applyTo(game);
      assertArrayEquals(bits("0111100"), game.playsheet.traders);
      assertArrayEquals(bits("0011011"), game.playsheet.resourcesAvail);
      assertArrayEquals(bits("11100000000000000"), game.playsheet.roads);
      assertEquals(3, game.turnsTaken);
      assertEquals(2, game.rolls);
      assertTrue(game.playsheet.isKnightResourceUsed(4));
      assertTrue(game.playsheet.canBuildKnight(5));
      assertTrue(game.canBuildCity(0));
   }
}
