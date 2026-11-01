package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import org.json.JSONException;
import org.json.JSONObject;
import org.junit.Test;

/**
 * The scores and settings document: the first island's list stays under the key the
 * previous build reads, every other map gets its own list in the order its rules say, and
 * the document the previous build wrote loads and rewrites unchanged.
 */
public class HighScoresAndSettingsTest {
   /** A race map (fewest turns wins) next to the default island, as the store sees the world. */
   private static final MapDef[] MAPS = { MapDef.TARNHOLM, RaceFixture.MAP };
   private static final String RACE = RaceFixture.MAP.id;

   /** The file the previous build wrote: scores, colour, prompts and haptics, no map anywhere. */
   private static final String CURRENT = "{\n"
         + "  \"schemaVersion\": 1,\n"
         + "  \"highScores\": [\n"
         + "    {\"score\": 31, \"playedAt\": \"2026-03-01T10:00:00Z\"},\n"
         + "    {\"score\": 18, \"playedAt\": \"2026-02-01T10:00:00Z\"},\n"
         + "    {\"score\": 7, \"playedAt\": \"2026-01-01T10:00:00Z\"}\n"
         + "  ],\n"
         + "  \"settings\": {\"playerColor\": \"#C03030\", \"earlyBuildPrompt\": 6, \"endTurnPrompt\": false, \"hapticFeedback\": false}\n"
         + "}";

   private static HighScoresAndSettings load(String json) throws JSONException {
      return HighScoresAndSettings.fromJson(json, MAPS);
   }

   private static HighScoresAndSettings store() {
      return new HighScoresAndSettings(MAPS);
   }

   private static List<Integer> values(List<HighScores.ScoreAndDate> list) {
      ArrayList<Integer> out = new ArrayList<Integer>();
      for(HighScores.ScoreAndDate sad : list) {
         out.add(sad.score);
      }
      return out;
   }

   private static HighScores.ScoreAndDate entry(int score, long date) {
      HighScores.ScoreAndDate sad = new HighScores.ScoreAndDate();
      sad.score = score;
      sad.date = date;
      return sad;
   }

   @Test
   public void theRealJsonLibraryIsOnTheTestClasspath() throws Exception {
      assertEquals(1, new JSONObject("{\"a\":1}").getInt("a"));
   }

   @Test
   public void aCurrentFileLoadsUnchanged() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT);
      assertEquals(Arrays.asList(31, 18, 7), values(loaded.scores));
      assertEquals(Arrays.asList(31, 18, 7), values(loaded.scoresFor("tarnholm")));
      assertTrue(loaded.scoresFor(RACE).isEmpty());
      assertEquals(0xFFC03030, loaded.playerColor);
      assertTrue(loaded.playerColorWasSpecified);
      assertEquals(6, loaded.earlyBuildPrompt);
      assertFalse(loaded.endTurnPrompt);
      assertFalse(loaded.hapticFeedback);
      assertEquals("tarnholm", loaded.map);
      assertFalse(loaded.mapWasSpecified);
   }

   @Test
   public void aCurrentFileRewritesAsTheSameDocument() throws Exception {
      JSONObject before = new JSONObject(CURRENT);
      JSONObject after = new JSONObject(load(CURRENT).toJson());
      assertFalse(after.has("highScoresByMap"));
      assertFalse(after.getJSONObject("settings").has("map"));
      assertEquals(SaveCompatTest.canonical(before), SaveCompatTest.canonical(after));
   }

   @Test
   public void aNewFileKeepsTheDefaultMapsScoresUnderTheOldKey() throws Exception {
      HighScoresAndSettings store = load(CURRENT);
      store.addScore(RACE, 12);
      store.addScore("tarnholm", 25);
      store.map = RACE;

      JSONObject written = new JSONObject(store.toJson());
      assertEquals(1, written.getInt("schemaVersion"));
      // What an older build reads: the same key and shape, with the island's scores and only those
      assertEquals(4, written.getJSONArray("highScores").length());
      assertEquals(31, written.getJSONArray("highScores").getJSONObject(0).getInt("score"));
      assertEquals(25, written.getJSONArray("highScores").getJSONObject(1).getInt("score"));
      assertEquals(12, written.getJSONObject("highScoresByMap").getJSONArray(RACE).getJSONObject(0).getInt("score"));
      assertFalse(written.getJSONObject("highScoresByMap").has("tarnholm"));
      assertEquals(RACE, written.getJSONObject("settings").getString("map"));

      HighScoresAndSettings back = load(store.toJson());
      assertEquals(Arrays.asList(31, 25, 18, 7), values(back.scores));
      assertEquals(Arrays.asList(12), values(back.scoresFor(RACE)));
      assertEquals(RACE, back.map);
      assertTrue(back.mapWasSpecified);
   }

   @Test
   public void aSettingNamedTheDefaultMapIsKeptOnceTheFileHasOne() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"map\": \"tarnholm\""));
      assertTrue(loaded.mapWasSpecified);
      assertEquals("tarnholm", new JSONObject(loaded.toJson()).getJSONObject("settings").getString("map"));
   }

   @Test
   public void aMapNobodyKnowsInTheSettingsLeavesTheDefault() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"map\": \"atlantis\""));
      assertEquals("tarnholm", loaded.map);
      assertFalse(loaded.mapWasSpecified);
   }

   @Test
   public void mapsNeverShareAList() throws Exception {
      HighScoresAndSettings store = store();
      store.addScore("tarnholm", 40);
      store.addScore(RACE, 9);
      store.addScore(RACE, 14);
      store.addScore("tarnholm", 3);

      assertEquals(Arrays.asList(40, 3), values(store.scoresFor("tarnholm")));
      assertEquals(Arrays.asList(9, 14), values(store.scoresFor(RACE)));
      assertEquals(Arrays.asList(40, 3), values(store.scores));

      // And after a trip through the file
      HighScoresAndSettings back = load(store.toJson());
      assertEquals(Arrays.asList(40, 3), values(back.scoresFor("tarnholm")));
      assertEquals(Arrays.asList(9, 14), values(back.scoresFor(RACE)));
   }

   @Test
   public void theDefaultMapListsHighestFirstAndAnAddedTieEntersAbove() throws Exception {
      HighScoresAndSettings store = store();
      for(int score : new int[] { 10, 30, 20, 20 }) {
         store.addScore("tarnholm", score);
      }
      assertEquals(Arrays.asList(30, 20, 20, 10), values(store.scoresFor("tarnholm")));

      // A tie goes at or above the equal score: this one is first among the 20s
      store.scores.get(1).date = 1;
      store.scores.get(2).date = 2;
      assertEquals(2, store.addScore("tarnholm", 20));
      assertEquals(Arrays.asList(1L, 2L), Arrays.asList(store.scores.get(2).date, store.scores.get(3).date));
   }

   @Test
   public void aRaceMapListsFewestTurnsFirstAndTrimsTheWorst() throws Exception {
      HighScoresAndSettings store = store();
      assertEquals(1, store.addScore(RACE, 14));
      assertEquals(1, store.addScore(RACE, 11));
      assertEquals(3, store.addScore(RACE, 17));
      assertEquals(2, store.addScore(RACE, 12));
      assertEquals(Arrays.asList(11, 12, 14, 17), values(store.scoresFor(RACE)));
      // A tie enters at or above the equal result, in this direction too
      assertEquals(3, store.addScore(RACE, 14));
      assertEquals(Arrays.asList(11, 12, 14, 14, 17), values(store.scoresFor(RACE)));

      // Full table: more turns than the worst entry does not make it, fewer pushes the worst out
      assertEquals(0, store.addScore(RACE, 18));
      assertEquals(1, store.addScore(RACE, 8));
      assertEquals(Arrays.asList(8, 11, 12, 14, 14), values(store.scoresFor(RACE)));
      assertEquals(HighScoresAndSettings.MAX_SCORES, store.scoresFor(RACE).size());
   }

   @Test
   public void aFileIsSortedForEachMapOnLoad() throws Exception {
      String json = "{\"schemaVersion\":1,\"highScores\":[{\"score\":5,\"playedAt\":\"2026-01-01T00:00:00Z\"},{\"score\":50,\"playedAt\":\"2026-01-02T00:00:00Z\"}],"
            + "\"highScoresByMap\":{\"" + RACE + "\":["
            + "{\"score\":15,\"playedAt\":\"2026-01-01T00:00:00Z\"},{\"score\":9,\"playedAt\":\"2026-01-02T00:00:00Z\"},"
            + "{\"score\":12,\"playedAt\":\"2026-01-03T00:00:00Z\"},{\"score\":20,\"playedAt\":\"2026-01-04T00:00:00Z\"},"
            + "{\"score\":10,\"playedAt\":\"2026-01-05T00:00:00Z\"},{\"score\":11,\"playedAt\":\"2026-01-06T00:00:00Z\"}]}}";
      HighScoresAndSettings loaded = load(json);
      assertEquals(Arrays.asList(50, 5), values(loaded.scores));
      assertEquals(Arrays.asList(9, 10, 11, 12, 15), values(loaded.scoresFor(RACE)));
   }

   @Test
   public void aBadEntryOrAMapListThatIsNotAListIsSkipped() throws Exception {
      String json = "{\"schemaVersion\":1,\"highScoresByMap\":{\"" + RACE + "\":[{\"score\":\"x\"},{\"score\":13},7,{\"nope\":1}],\"tarnholm\":[{\"score\":99}],\"" + "junk\":3}}";
      HighScoresAndSettings loaded = load(json);
      assertEquals(Arrays.asList(13), values(loaded.scoresFor(RACE)));
      // The first island's list lives under "highScores" only
      assertTrue(loaded.scores.isEmpty());
   }

   @Test
   public void aMapThisBuildDoesNotKnowIsKeptAsReadAndNeverMixed() throws Exception {
      String json = "{\"schemaVersion\":1,\"highScores\":[{\"score\":5}],"
            + "\"highScoresByMap\":{\"third-isle\":[{\"score\":9,\"playedAt\":\"2026-01-01T00:00:00Z\"},{\"score\":2,\"playedAt\":\"2026-01-02T00:00:00Z\"}]}}";
      HighScoresAndSettings loaded = load(json);
      assertEquals(Arrays.asList(5), values(loaded.scores));
      assertEquals(Arrays.asList(9, 2), values(loaded.scoresFor("third-isle")));
      assertEquals(Arrays.asList(9, 2), values(new ArrayList<HighScores.ScoreAndDate>(load(loaded.toJson()).scoresFor("third-isle"))));
      try {
         loaded.addScore("third-isle", 1);
         fail("a result for a map the build does not know has no list to go in");
      }
      catch(IllegalArgumentException expected) {
      }
      assertEquals(Arrays.asList(9, 2), values(loaded.scoresFor("third-isle")));
   }

   @Test
   public void mergingIsPerMapAndDedupesOnScoreAndDate() throws Exception {
      HighScoresAndSettings store = store();
      store.mergeScores(RACE, Arrays.asList(entry(13, 100), entry(10, 200)));
      int added = store.mergeScores(RACE, Arrays.asList(entry(13, 100), entry(9, 300), entry(15, 400)));
      assertEquals(2, added);
      assertEquals(Arrays.asList(9, 10, 13, 15), values(store.scoresFor(RACE)));
      assertTrue(store.scoresFor("tarnholm").isEmpty());

      // The old single-list form is the default map
      assertEquals(1, store.mergeScores(Arrays.asList(entry(4, 1))));
      assertEquals(Arrays.asList(4), values(store.scores));
   }

   @Test
   public void exportThenImportRoundTripsEverything() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.addScore(RACE, 12);
      source.addScore(RACE, 16);
      source.map = RACE;
      source.mapWasSpecified = true;
      source.playerColor = 0xFF2040C0;
      source.hapticFeedback = true;
      String exported = source.toJson();

      HighScoresAndSettings fresh = store();
      int added = fresh.importFrom(load(exported));
      assertEquals(5, added);
      assertEquals(Arrays.asList(31, 18, 7), values(fresh.scoresFor("tarnholm")));
      assertEquals(Arrays.asList(12, 16), values(fresh.scoresFor(RACE)));
      assertEquals(RACE, fresh.map);
      assertEquals(0xFF2040C0, fresh.playerColor);
      assertTrue(fresh.hapticFeedback);
      assertEquals(SaveCompatTest.canonical(new JSONObject(exported).getJSONObject("highScoresByMap")),
            SaveCompatTest.canonical(new JSONObject(fresh.toJson()).getJSONObject("highScoresByMap")));

      // Importing it again adds nothing
      assertEquals(0, fresh.importFrom(load(exported)));
   }

   @Test
   public void importingAnOldExportStillWorksAndLeavesTheMapAlone() throws Exception {
      HighScoresAndSettings store = store();
      store.addScore(RACE, 11);
      store.map = RACE;
      store.mapWasSpecified = true;

      int added = store.importFrom(load(CURRENT));
      assertEquals(3, added);
      assertEquals(Arrays.asList(31, 18, 7), values(store.scoresFor("tarnholm")));
      assertEquals(Arrays.asList(11), values(store.scoresFor(RACE)));
      // The file named no map, so the choice on this device stands
      assertEquals(RACE, store.map);
      assertEquals(0xFFC03030, store.playerColor);
   }

   @Test
   public void aFileFromAnOlderBuildWithOnlyScoresStillLoads() throws Exception {
      HighScoresAndSettings loaded = load("{\"schemaVersion\":1,\"highScores\":[{\"score\":4,\"playedAt\":1700000000000}]}");
      assertEquals(Arrays.asList(4), values(loaded.scores));
      assertEquals(1700000000000L, loaded.scores.get(0).date);
   }

   // ---- Theme and the welcome dialog -------------------------------------------------

   @Test
   public void themeAndWelcomeDefaultToSystemAndOn() throws Exception {
      HighScoresAndSettings fresh = store();
      assertEquals(HighScoresAndSettings.THEME_SYSTEM, fresh.theme);
      assertTrue(fresh.showWelcome);

      // A file from before either setting existed: an upgrading player sees the welcome once
      HighScoresAndSettings old = load(CURRENT);
      assertEquals(HighScoresAndSettings.THEME_SYSTEM, old.theme);
      assertTrue(old.showWelcome);
      assertFalse(old.themeWasSpecified);
      assertFalse(old.showWelcomeWasSpecified);
   }

   @Test
   public void themeAndWelcomeRoundTripThroughTheDocument() throws Exception {
      for(int theme : new int[] { HighScoresAndSettings.THEME_SYSTEM, HighScoresAndSettings.THEME_LIGHT, HighScoresAndSettings.THEME_DARK }) {
         HighScoresAndSettings source = load(CURRENT);
         source.theme = theme;
         source.showWelcome = false;
         HighScoresAndSettings back = load(source.toJson());
         assertEquals(theme, back.theme);
         assertFalse(back.showWelcome);
         assertEquals(theme != HighScoresAndSettings.THEME_SYSTEM, back.themeWasSpecified);
         assertTrue(back.showWelcomeWasSpecified);
      }

      // Defaults are not written, so the previous build's file rewrites as the same document
      assertFalse(new JSONObject(store().toJson()).getJSONObject("settings").has("theme"));
      assertFalse(new JSONObject(store().toJson()).getJSONObject("settings").has("showWelcome"));

      HighScoresAndSettings dark = store();
      dark.theme = HighScoresAndSettings.THEME_DARK;
      assertEquals("dark", new JSONObject(dark.toJson()).getJSONObject("settings").getString("theme"));
   }

   @Test
   public void anUnknownThemeNameIsIgnored() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"theme\": \"sepia\""));
      assertEquals(HighScoresAndSettings.THEME_SYSTEM, loaded.theme);
      assertFalse(loaded.themeWasSpecified);
   }

   @Test
   public void exportThenImportCarriesThemeAndWelcome() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.theme = HighScoresAndSettings.THEME_DARK;
      source.showWelcome = false;

      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertEquals(HighScoresAndSettings.THEME_DARK, fresh.theme);
      assertFalse(fresh.showWelcome);
   }

   @Test
   public void importingAnOldExportLeavesThemeAndWelcomeAlone() throws Exception {
      HighScoresAndSettings store = store();
      store.theme = HighScoresAndSettings.THEME_LIGHT;
      store.showWelcome = false;

      store.importFrom(load(CURRENT));
      assertEquals(HighScoresAndSettings.THEME_LIGHT, store.theme);
      assertFalse(store.showWelcome);
   }

   // ---- The tutorial -----------------------------------------------------------------

   @Test
   public void tutorialDefaultsToOnWithNothingSeen() throws Exception {
      HighScoresAndSettings fresh = store();
      assertTrue(fresh.tutorial);
      assertEquals(0, fresh.tutorialSeen);

      // A file from before the tutorial existed: an upgrading player gets it
      HighScoresAndSettings old = load(CURRENT);
      assertTrue(old.tutorial);
      assertEquals(0, old.tutorialSeen);
      assertFalse(old.tutorialWasSpecified);
      assertFalse(old.tutorialSeenWasSpecified);
   }

   @Test
   public void tutorialRoundTripsThroughTheDocument() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.tutorial = false;
      source.tutorialSeen = 0x15;
      HighScoresAndSettings back = load(source.toJson());
      assertFalse(back.tutorial);
      assertEquals(0x15, back.tutorialSeen);
      assertTrue(back.tutorialWasSpecified);
      assertTrue(back.tutorialSeenWasSpecified);

      // Defaults are not written, so the previous build's file rewrites as the same document
      JSONObject settings = new JSONObject(store().toJson()).getJSONObject("settings");
      assertFalse(settings.has("tutorial"));
      assertFalse(settings.has("tutorialSeen"));

      // A value that left its default is written, and so is one the file already carried
      HighScoresAndSettings partway = store();
      partway.tutorialSeen = 3;
      assertEquals(3, new JSONObject(partway.toJson()).getJSONObject("settings").getInt("tutorialSeen"));
      HighScoresAndSettings reset = load(source.toJson());
      reset.tutorial = true;
      reset.tutorialSeen = 0;
      JSONObject written = new JSONObject(reset.toJson()).getJSONObject("settings");
      assertTrue(written.getBoolean("tutorial"));
      assertEquals(0, written.getInt("tutorialSeen"));
   }

   @Test
   public void aStrayTutorialSeenIsCutDownToTheStepsThatExist() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"tutorialSeen\": 65535"));
      assertEquals(Tutorial.ALL, loaded.tutorialSeen);
      assertTrue(loaded.tutorialSeenWasSpecified);
   }

   @Test
   public void exportThenImportCarriesTheTutorial() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.tutorial = false;
      source.tutorialSeen = 7;

      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertFalse(fresh.tutorial);
      assertEquals(7, fresh.tutorialSeen);
   }

   @Test
   public void importingAnOldExportLeavesTheTutorialAlone() throws Exception {
      HighScoresAndSettings store = store();
      store.tutorial = false;
      store.tutorialSeen = 5;

      store.importFrom(load(CURRENT));
      assertFalse(store.tutorial);
      assertEquals(5, store.tutorialSeen);
   }

   // ---- Music volume -----------------------------------------------------------------

   @Test
   public void musicVolumeDefaultsToSixtyAndIsNotWrittenWhileDefault() throws Exception {
      HighScoresAndSettings fresh = store();
      assertEquals(60, fresh.musicVolume);
      assertFalse(fresh.musicVolumeWasSpecified);
      assertFalse(new JSONObject(fresh.toJson()).getJSONObject("settings").has("musicVolume"));

      // A file from before the setting existed loads as the default and rewrites the same
      HighScoresAndSettings old = load(CURRENT);
      assertEquals(60, old.musicVolume);
      assertFalse(old.musicVolumeWasSpecified);
      assertFalse(new JSONObject(old.toJson()).getJSONObject("settings").has("musicVolume"));
   }

   @Test
   public void musicVolumeRoundTripsThroughTheDocument() throws Exception {
      for(int volume : new int[] { 0, 1, 59, 61, 100 }) {
         HighScoresAndSettings source = load(CURRENT);
         source.musicVolume = volume;
         assertEquals(volume, new JSONObject(source.toJson()).getJSONObject("settings").getInt("musicVolume"));
         HighScoresAndSettings back = load(source.toJson());
         assertEquals(volume, back.musicVolume);
         assertTrue(back.musicVolumeWasSpecified);
      }

      // Once the file names it, it stays named even at the default
      HighScoresAndSettings named = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"musicVolume\": 60"));
      assertTrue(named.musicVolumeWasSpecified);
      assertEquals(60, new JSONObject(named.toJson()).getJSONObject("settings").getInt("musicVolume"));
   }

   @Test
   public void aStrayMusicVolumeIsClampedOrIgnored() throws Exception {
      assertEquals(100, load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"musicVolume\": 250")).musicVolume);
      assertEquals(0, load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"musicVolume\": -5")).musicVolume);
      HighScoresAndSettings text = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"musicVolume\": \"loud\""));
      assertEquals(60, text.musicVolume);
      assertFalse(text.musicVolumeWasSpecified);
   }

   @Test
   public void exportThenImportCarriesMusicVolumeAndAnOldExportLeavesItAlone() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.musicVolume = 15;
      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertEquals(15, fresh.musicVolume);
      assertTrue(fresh.musicVolumeWasSpecified);

      HighScoresAndSettings kept = store();
      kept.musicVolume = 85;
      kept.importFrom(load(CURRENT));
      assertEquals(85, kept.musicVolume);
   }

   // ---- Dunholm target -----------------------------------------------------------------

   @Test
   public void raceTargetDefaultsToTheIslandsOwnAndIsNotWrittenWhileDefault() throws Exception {
      HighScoresAndSettings fresh = store();
      assertEquals(MapDef.DUNHOLM.targetScore, fresh.raceTarget);
      assertEquals(10, fresh.raceTarget);
      assertFalse(fresh.raceTargetWasSpecified);
      assertFalse(new JSONObject(fresh.toJson()).getJSONObject("settings").has("raceTarget"));

      // A file from before the setting existed loads as the default and rewrites the same
      HighScoresAndSettings old = load(CURRENT);
      assertEquals(10, old.raceTarget);
      assertFalse(old.raceTargetWasSpecified);
      assertFalse(new JSONObject(old.toJson()).getJSONObject("settings").has("raceTarget"));
   }

   @Test
   public void raceTargetRoundTripsThroughTheDocument() throws Exception {
      for(int target : new int[] { 11, 15, 19 }) {
         HighScoresAndSettings source = load(CURRENT);
         source.raceTarget = target;
         assertEquals(target, new JSONObject(source.toJson()).getJSONObject("settings").getInt("raceTarget"));
         HighScoresAndSettings back = load(source.toJson());
         assertEquals(target, back.raceTarget);
         assertTrue(back.raceTargetWasSpecified);
      }
   }

   @Test
   public void aStrayRaceTargetIsClampedToWhatTheIslandCanGiveOrIgnored() throws Exception {
      assertEquals(19, HighScoresAndSettings.raceTargetMax());
      assertEquals(19, load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"raceTarget\": 40")).raceTarget);
      assertEquals(10, load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"raceTarget\": 0")).raceTarget);
      HighScoresAndSettings text = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"raceTarget\": \"lots\""));
      assertEquals(10, text.raceTarget);
      assertFalse(text.raceTargetWasSpecified);
   }

   @Test
   public void exportThenImportCarriesRaceTargetAndAnOldExportLeavesItAlone() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.raceTarget = 15;
      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertEquals(15, fresh.raceTarget);
      assertTrue(fresh.raceTargetWasSpecified);

      HighScoresAndSettings kept = store();
      kept.raceTarget = 7;
      kept.importFrom(load(CURRENT));
      assertEquals(7, kept.raceTarget);
   }

   @Test
   public void paletteFollowsTheThemeAndTheOsOnlyForSystem() {
      assertFalse(Palette.isDark(HighScoresAndSettings.THEME_SYSTEM, false));
      assertTrue(Palette.isDark(HighScoresAndSettings.THEME_SYSTEM, true));
      assertFalse(Palette.isDark(HighScoresAndSettings.THEME_LIGHT, true));
      assertTrue(Palette.isDark(HighScoresAndSettings.THEME_DARK, false));
      assertTrue(Palette.forMode(HighScoresAndSettings.THEME_DARK, false) == Palette.DARK);
      assertTrue(Palette.forMode(HighScoresAndSettings.THEME_LIGHT, true) == Palette.LIGHT);
   }

   @Test
   public void soundEffectsAreOnByDefaultAndAFileWithoutThemRewritesWithout() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT);
      assertTrue(loaded.soundEffects);
      assertFalse(loaded.soundEffectsWasSpecified);
      assertFalse(new JSONObject(loaded.toJson()).getJSONObject("settings").has("soundEffects"));
      assertTrue(store().soundEffects);
   }

   @Test
   public void soundEffectsOffRoundTripsThroughTheDocument() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.soundEffects = false;
      assertFalse(new JSONObject(source.toJson()).getJSONObject("settings").getBoolean("soundEffects"));
      HighScoresAndSettings back = load(source.toJson());
      assertFalse(back.soundEffects);
      assertTrue(back.soundEffectsWasSpecified);

      // Once the file names it, it stays named even when back to the default
      HighScoresAndSettings named = load(CURRENT.replace("\"hapticFeedback\": false", "\"hapticFeedback\": false, \"soundEffects\": true"));
      assertTrue(named.soundEffects);
      assertTrue(named.soundEffectsWasSpecified);
      assertTrue(new JSONObject(named.toJson()).getJSONObject("settings").getBoolean("soundEffects"));
   }

   @Test
   public void exportThenImportCarriesSoundEffectsAndAnOldExportLeavesThemAlone() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.soundEffects = false;
      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertFalse(fresh.soundEffects);
      assertTrue(fresh.soundEffectsWasSpecified);

      HighScoresAndSettings kept = store();
      kept.soundEffects = false;
      kept.importFrom(load(CURRENT));
      assertFalse("a backup from before the setting does not turn sound back on", kept.soundEffects);

      HighScoresAndSettings on = store();
      HighScoresAndSettings wasOn = load(CURRENT);
      wasOn.soundEffects = true;
      wasOn.soundEffectsWasSpecified = true;
      on.soundEffects = false;
      on.importFrom(load(wasOn.toJson()));
      assertTrue(on.soundEffects);
   }

   @Test
   public void traderClassicIsOffByDefaultAndAFileWithoutItRewritesWithout() throws Exception {
      HighScoresAndSettings loaded = load(CURRENT);
      assertFalse(loaded.traderClassic);
      assertFalse(loaded.traderClassicWasSpecified);
      assertFalse(new JSONObject(loaded.toJson()).getJSONObject("settings").has("traderClassic"));
      assertFalse(store().traderClassic);
   }

   @Test
   public void traderClassicRoundTripsThroughTheDocumentAndAnImport() throws Exception {
      HighScoresAndSettings source = load(CURRENT);
      source.traderClassic = true;
      assertTrue(new JSONObject(source.toJson()).getJSONObject("settings").getBoolean("traderClassic"));
      HighScoresAndSettings back = load(source.toJson());
      assertTrue(back.traderClassic);
      assertTrue(back.traderClassicWasSpecified);

      // Once the file names it, it stays named when turned back off
      back.traderClassic = false;
      assertFalse(new JSONObject(back.toJson()).getJSONObject("settings").getBoolean("traderClassic"));

      HighScoresAndSettings fresh = store();
      fresh.importFrom(load(source.toJson()));
      assertTrue(fresh.traderClassic);
      HighScoresAndSettings kept = store();
      kept.traderClassic = true;
      kept.importFrom(load(CURRENT));
      assertTrue("a backup from before the setting leaves it alone", kept.traderClassic);
   }
}
