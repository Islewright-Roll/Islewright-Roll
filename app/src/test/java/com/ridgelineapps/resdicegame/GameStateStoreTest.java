package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import org.json.JSONException;
import org.json.JSONObject;
import org.junit.Test;

/**
 * The in-progress save, schema 2: it names its map and writes the traders as the array
 * itself. A schema 1 save (see SaveCompatTest for a full one) is the default map's.
 */
public class GameStateStoreTest {
   private static final MapDef[] MAPS = { MapDef.TARNHOLM, RaceFixture.MAP };

   private static boolean[] bits(String s) {
      return SaveCompatTest.bits(s);
   }

   /** A race game part way through, with traders built in no tier order at all. */
   private static Game raceGame() {
      Game game = new Game(RaceFixture.MAP);
      game.rolls = 1;
      game.turnsTaken = 4;
      game.playsheet.traders = bits("0010100100");
      game.playsheet.roads[0] = true;
      game.playsheet.roads[5] = true;
      game.playsheet.villages[3] = true;
      game.playsheet.villages[0] = true;
      game.playsheet.cities[2] = true;
      game.playsheet.turnScore = new int[] { 0, 0, 7 };
      return game;
   }

   @Test
   public void theSchemaIsTwoAndNamesTheMap() throws Exception {
      JSONObject root = new JSONObject(GameStateStore.buildJson(new Game(MapDef.TARNHOLM)));
      assertEquals(2, root.getInt("schemaVersion"));
      assertEquals("tarnholm", root.getJSONObject("game").getString("mapId"));
      assertFalse(root.getJSONObject("game").getJSONObject("playsheet").has("knights"));
   }

   @Test
   public void aSchemaOneSaveLoadsAsTheDefaultMapWithTradersFromTheCount() throws Exception {
      // The file shape the previous build wrote: no mapId anywhere, "knights" as a count
      assertFalse(SaveCompatTest.SAVE.contains("mapId"));
      assertTrue(SaveCompatTest.SAVE.contains("\"knights\":3"));
      Game game = GameStateStore.fromJson(SaveCompatTest.SAVE, MAPS);
      assertSame(MapDef.DEFAULT, game.map);
      assertArrayEquals(bits("0111000"), game.playsheet.traders);
      assertEquals(7, game.turnsTaken);
      assertEquals(2, game.rolls);
   }

   @Test
   public void aSchemaTwoSaveRoundTripsOnAMapWithUnorderedTraders() throws Exception {
      Game game = raceGame();
      String json = GameStateStore.buildJson(game);
      assertEquals("race-fixture", new JSONObject(json).getJSONObject("game").getString("mapId"));

      Game back = GameStateStore.fromJson(json, MAPS);
      assertSame(RaceFixture.MAP, back.map);
      // Tiers 3, 5 and 8 and nothing between them: not a count, an array
      assertArrayEquals(bits("0010100100"), back.playsheet.traders);
      assertEquals(3, back.playsheet.traderCount());
      assertArrayEquals(game.playsheet.roads, back.playsheet.roads);
      assertArrayEquals(game.playsheet.villages, back.playsheet.villages);
      assertArrayEquals(game.playsheet.cities, back.playsheet.cities);
      assertArrayEquals(game.playsheet.turnScore, back.playsheet.turnScore);
      assertEquals(4, back.turnsTaken);
      assertEquals(game.playsheet.getScore(), back.playsheet.getScore());
      assertEquals(SaveCompatTest.canonical(new JSONObject(json)), SaveCompatTest.canonical(new JSONObject(GameStateStore.buildJson(back))));
   }

   @Test
   public void aSchemaTwoSaveRoundTripsOnTheDefaultMap() throws Exception {
      Game game = new Game(MapDef.TARNHOLM);
      game.playsheet.traders = bits("0110000");
      Game back = GameStateStore.fromJson(GameStateStore.buildJson(game), MAPS);
      assertSame(MapDef.TARNHOLM, back.map);
      assertArrayEquals(bits("0110000"), back.playsheet.traders);
   }

   @Test
   public void anUnknownMapIdIsRefusedSoTheFileIsQuarantined() throws Exception {
      String json = GameStateStore.buildJson(raceGame());
      try {
         // The default list does not hold the fixture, so to this build it is a stranger
         GameStateStore.fromJson(json);
         fail("an unknown map id must not load");
      }
      catch(JSONException expected) {
         assertTrue(expected.getMessage().contains("race-fixture"));
      }

      try {
         GameStateStore.fromJson(json.replace("race-fixture", "no-such-map"), MAPS);
         fail("an unknown map id must not fall back to the default map");
      }
      catch(JSONException expected) {
         assertTrue(expected.getMessage().contains("no-such-map"));
      }
   }

   @Test
   public void aSchemaTwoSaveNeedsItsMapIdAndTradersOfTheRightLength() throws Exception {
      String json = GameStateStore.buildJson(new Game(MapDef.TARNHOLM));
      try {
         GameStateStore.fromJson(json.replace("\"mapId\":\"tarnholm\",", ""), MAPS);
         fail("a schema 2 save without a mapId must not load");
      }
      catch(JSONException expected) {
      }

      // Seven entries are right for Tarnholm; the race map's ten are not
      String raceTraders = new JSONObject(GameStateStore.buildJson(raceGame())).getJSONObject("game").getJSONObject("playsheet").getJSONArray("traders").toString();
      JSONObject root = new JSONObject(json);
      root.getJSONObject("game").getJSONObject("playsheet").put("traders", new org.json.JSONArray(raceTraders));
      try {
         GameStateStore.fromJson(root.toString(), MAPS);
         fail("traders of the wrong length must not load");
      }
      catch(JSONException expected) {
         assertTrue(expected.getMessage().contains("traders"));
      }
   }

   /** Dunholm once had a cap of twenty turns, so its saves held 21 turn scores. They still load, and play on past twenty. */
   @Test
   public void aDunholmSaveFromWhenItHadACapStillLoads() throws Exception {
      Game game = raceGame();
      game.playsheet.turnScore = new int[21];
      game.playsheet.turnScore[1] = 2;
      game.playsheet.turnScore[2] = 2;
      game.playsheet.turnScore[3] = 2;
      game.playsheet.turnScore[4] = 7;
      String json = GameStateStore.buildJson(game);
      assertTrue(json.contains("\"turnScore\":[0,2,2,2,7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]"));

      Game back = GameStateStore.fromJson(json, MAPS);
      assertEquals(21, back.playsheet.turnScore.length);
      assertEquals(5, back.playsheet.getTurnScore(4));
      back.turnsTaken = 24;
      back.newTurn(false);
      assertEquals(25, back.turnsTaken);
      assertEquals(26, back.playsheet.turnScore.length);
      assertEquals(0, back.playsheet.getTurnScore(30));
   }

   @Test
   public void aSaveWithTheWrongNumberOfTurnScoresIsStillRefusedOnACappedMap() throws Exception {
      String json = GameStateStore.buildJson(new Game(MapDef.TARNHOLM)).replace("\"turnScore\":[0,", "\"turnScore\":[0,0,");
      try {
         GameStateStore.fromJson(json, MAPS);
         fail("Tarnholm has fifteen turns, so sixteen entries exactly");
      }
      catch(JSONException expected) {
         assertTrue(expected.getMessage().contains("turnScore"));
      }
   }

   /** A race saves the target it is played to, which the setting may have moved. */
   @Test
   public void aRaceSavesItsOwnTargetAndAnOlderSaveRacesToTheIslandsOwn() throws Exception {
      Game game = raceGame();
      game.targetScore = 15;
      String json = GameStateStore.buildJson(game);
      assertEquals(15, new JSONObject(json).getJSONObject("game").getInt("targetScore"));
      assertEquals(15, GameStateStore.fromJson(json, MAPS).targetScore);

      // Before the setting there was no field, and every race was to the island's 10
      JSONObject old = new JSONObject(json);
      old.getJSONObject("game").remove("targetScore");
      assertEquals(10, GameStateStore.fromJson(old.toString(), MAPS).targetScore);

      // A hand edit past what the island can give is pulled back into reach
      JSONObject stray = new JSONObject(json);
      stray.getJSONObject("game").put("targetScore", 50);
      assertEquals(19, GameStateStore.fromJson(stray.toString(), MAPS).targetScore);
   }

   @Test
   public void aPointsMapWritesNoTarget() throws Exception {
      JSONObject root = new JSONObject(GameStateStore.buildJson(new Game(MapDef.TARNHOLM)));
      assertFalse(root.getJSONObject("game").has("targetScore"));
      assertEquals(0, GameStateStore.fromJson(root.toString(), MAPS).targetScore);
   }

   @Test
   public void aFutureSchemaIsStillRefused() throws Exception {
      String json = GameStateStore.buildJson(new Game(MapDef.TARNHOLM)).replace("\"schemaVersion\":2", "\"schemaVersion\":3");
      try {
         GameStateStore.fromJson(json, MAPS);
         fail("schema 3 is not understood");
      }
      catch(JSONException expected) {
         assertTrue(expected.getMessage().contains("schemaVersion"));
      }
   }
}
