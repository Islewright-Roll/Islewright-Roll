package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import org.json.JSONException;
import org.junit.Test;

/** The debug scenario parser: schema, defaults and rejections. Needs no emulator. */
public class DebugScenarioTest {

   private static void rejects(String json, String fragment) {
      try {
         DebugScenario.fromJson(json);
         fail("accepted: " + json);
      }
      catch(JSONException e) {
         assertTrue(e.getMessage(), e.getMessage().contains(fragment));
      }
   }

   @Test
   public void noMapMeansTheDefaultMap() throws Exception {
      assertEquals(MapDef.DEFAULT, DebugScenario.fromJson("{}").map);
   }

   @Test
   public void unknownMapIsRejected() {
      rejects("{\"map\":\"nowhere\"}", "unknown map");
   }

   @Test
   public void knightsCountBecomesTraderFlags() throws Exception {
      DebugScenario s = DebugScenario.fromJson("{\"playsheet\":{\"knights\":2}}");
      assertEquals(MapDef.DEFAULT.traders.length, s.traders.length);
      assertTrue(s.traders[0] && s.traders[1]);
      assertFalse(s.traders[2]);
   }

   @Test
   public void knightsAndTradersMustAgree() throws Exception {
      int n = MapDef.DEFAULT.traders.length;
      StringBuilder flags = new StringBuilder();
      for(int i = 0; i < n; i++) {
         flags.append(i < 2 ? "true" : "false").append(i < n - 1 ? "," : "");
      }
      DebugScenario.fromJson("{\"playsheet\":{\"knights\":2,\"traders\":[" + flags + "]}}");
      rejects("{\"playsheet\":{\"knights\":3,\"traders\":[" + flags + "]}}", "disagree");
   }

   @Test
   public void orderedMapRejectsGappedTraders() {
      int n = MapDef.TARNHOLM.traders.length;
      StringBuilder flags = new StringBuilder("false,true");
      for(int i = 2; i < n; i++) {
         flags.append(",false");
      }
      rejects("{\"playsheet\":{\"traders\":[" + flags + "]}}", "prefix");
   }

   @Test
   public void lengthsAndRangesComeFromTheMap() {
      MapDef m = MapDef.DEFAULT;
      rejects("{\"turnsTaken\":" + m.turnCap + "}", "turnsTaken");
      rejects("{\"playsheet\":{\"roads\":[true]}}", "playsheet.roads");
      rejects("{\"playsheet\":{\"bankSpent\":[" + m.bankResource.length + "]}}", "bankSpent");
      rejects("{\"playsheet\":{\"bankSpent\":[1],\"knightResourcesSpent\":[1]}}", "not both");
   }

   @Test
   public void appliesToAGame() throws Exception {
      Game game = new Game(MapDef.DEFAULT);
      DebugScenario.fromJson("{\"rolls\":3,\"playsheet\":{\"knights\":2,\"knightResourcesSpent\":[1]}}").applyTo(game);
      assertEquals(3, game.rolls);
      assertEquals(2, game.playsheet.traderCount());
      assertFalse(game.playsheet.resourcesAvail[1]);
   }

   @Test
   public void passedTurnsGetRunningTotalsEndingOnTheScore() throws Exception {
      String json = new String(java.nio.file.Files.readAllBytes(
            new java.io.File("../tools/scenarios/tarnholm-midgame.json").toPath()), "UTF-8");
      Game game = new Game(MapDef.TARNHOLM);
      DebugScenario.fromJson(json).applyTo(game);
      int total = game.playsheet.getScore();
      int[] totals = game.playsheet.turnScore;
      assertEquals(total, totals[6]);
      assertEquals(0, total - totals[game.turnsTaken]);
      for(int turn = 1; turn <= 6; turn++) {
         assertTrue(totals[turn] >= totals[turn - 1]);
      }
      assertEquals(0, totals[7]);
   }

   @Test
   public void turnScoresOverrideTheSpread() throws Exception {
      Game game = new Game(MapDef.TARNHOLM);
      DebugScenario.fromJson("{\"map\":\"tarnholm\",\"turnsTaken\":3,\"turnScores\":[2,0,5]}").applyTo(game);
      assertEquals(2, game.playsheet.turnScore[1]);
      assertEquals(2, game.playsheet.turnScore[2]);
      assertEquals(7, game.playsheet.turnScore[3]);
   }

   @Test
   public void turnScoresMustMatchTheCompletedTurns() {
      rejects("{\"turnsTaken\":3,\"turnScores\":[1,2]}", "turnScores must have exactly");
      rejects("{\"turnScores\":[1]}", "turnScores must have exactly");
      rejects("{\"turnsTaken\":1,\"turnScores\":[-1]}", "0 or more");
   }

   /** Every committed scenario parses against its own map; files for a map not built yet are skipped. */
   @Test
   public void committedScenarioFilesParse() throws Exception {
      java.io.File dir = new java.io.File("../tools/scenarios");
      java.io.File[] files = dir.listFiles();
      assertTrue("no scenario files at " + dir.getAbsolutePath(), files != null);
      int parsed = 0;
      for(java.io.File f : files) {
         if(!f.getName().endsWith(".json")) {
            continue;
         }
         String json = new String(java.nio.file.Files.readAllBytes(f.toPath()), "UTF-8");
         String id = new org.json.JSONObject(json).optString("map", MapDef.DEFAULT.id);
         if(DebugScenario.findMap(id) == null) {
            continue;
         }
         DebugScenario.fromJson(json);
         parsed++;
      }
      assertTrue(parsed > 0);
   }
}
