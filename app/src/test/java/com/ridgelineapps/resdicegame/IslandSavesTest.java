package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.file.Files;

import org.json.JSONObject;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;

/**
 * One saved game per island, in the same directory. Runs the store over a temporary
 * directory, the same code the app runs over its files dir.
 */
public class IslandSavesTest {
   private File dir;

   @Before
   public void makeDir() throws Exception {
      dir = Files.createTempDirectory("islandsaves").toFile();
   }

   @After
   public void removeDir() {
      File[] files = dir.listFiles();
      if(files != null) {
         for(File file : files) {
            file.delete();
         }
      }
      dir.delete();
   }

   private static Game tarnholmGame() {
      Game game = new Game(MapDef.TARNHOLM);
      game.turnsTaken = 3;
      game.rolls = 2;
      game.builtResourceThisTurn = true;
      game.dice[1].value = Dice.Value.Ore;
      game.dice[1].held = true;
      game.dice[4].value = Dice.Value.Wool;
      game.dice[4].used = true;
      game.playsheet.roads[1] = true;
      game.playsheet.traders[1] = true;
      game.playsheet.resourcesAvail[2] = false;
      game.playsheet.turnScore[1] = 2;
      game.playsheet.turnScore[2] = 5;
      game.playsheet.turnScore[3] = 6;
      return game;
   }

   private static Game dunholmGame() {
      Game game = new Game(MapDef.DUNHOLM);
      game.turnsTaken = 2;
      game.rolls = 1;
      game.dice[0].value = Dice.Value.Gold;
      game.dice[0].held = true;
      game.dice[5].value = Dice.Value.Brick;
      game.playsheet.roads[2] = true;
      game.playsheet.villages[0] = true;
      game.playsheet.resourcesAvail[1] = false;
      game.playsheet.turnScore = new int[] { 0, 1, 3 };
      return game;
   }

   private static String same(Game game) throws Exception {
      return SaveCompatTest.canonical(new JSONObject(GameStateStore.toJson(game)));
   }

   private void writeText(String name, String text) throws Exception {
      FileOutputStream out = new FileOutputStream(new File(dir, name));
      try {
         out.write(text.getBytes("UTF-8"));
      }
      finally {
         out.close();
      }
   }

   private int filesNamed(String infix) {
      int count = 0;
      for(String name : dir.list()) {
         if(name.contains(infix)) {
            count++;
         }
      }
      return count;
   }

   /** What MainActivity.switchMap does with the store: keep the game on screen, bring up the other. */
   private Game switchTo(Game current, MapDef target) {
      GameStateStore.persist(dir, current);
      Game saved = GameStateStore.load(dir, target);
      return saved != null ? saved : new Game(target);
   }

   @Test
   public void eachIslandHasItsOwnFileAndTheDefaultKeepsTheOldName() {
      assertEquals("game-state.json", GameStateStore.fileName(MapDef.TARNHOLM));
      assertEquals("game-state.json", GameStateStore.fileName(MapDef.DEFAULT));
      assertEquals(GameStateStore.FILE_NAME, GameStateStore.fileName(MapDef.DEFAULT));
      assertEquals("game-state-dunholm.json", GameStateStore.fileName(MapDef.DUNHOLM));
   }

   @Test
   public void savingWritesTheGamesOwnFileOnly() {
      GameStateStore.save(dir, dunholmGame());
      assertTrue(new File(dir, "game-state-dunholm.json").exists());
      assertFalse(new File(dir, "game-state.json").exists());

      GameStateStore.save(dir, tarnholmGame());
      assertTrue(new File(dir, "game-state.json").exists());
      assertEquals(2, dir.list().length);
   }

   @Test
   public void switchingAndSwitchingBackRestoresEachGameExactly() throws Exception {
      Game tarn = tarnholmGame();
      String tarnBefore = same(tarn);
      String dunBefore = same(dunholmGame());

      // Tarnholm mid game, over to Dunholm which has no save: a fresh game there
      Game onDunholm = switchTo(tarn, MapDef.DUNHOLM);
      assertSame(MapDef.DUNHOLM, onDunholm.map);
      assertFalse(onDunholm.hasProgress());

      // Two turns of play on Dunholm, replaced here by a known mid game
      Game played = dunholmGame();
      Game backOnTarnholm = switchTo(played, MapDef.TARNHOLM);
      assertSame(MapDef.TARNHOLM, backOnTarnholm.map);
      assertEquals(tarnBefore, same(backOnTarnholm));
      assertTrue(backOnTarnholm.dice[1].held);
      assertEquals(Dice.Value.Ore, backOnTarnholm.dice[1].value);
      assertEquals(2, backOnTarnholm.rolls);
      assertEquals(3, backOnTarnholm.turnsTaken);
      assertFalse(backOnTarnholm.playsheet.resourcesAvail[2]);
      assertEquals(6, backOnTarnholm.playsheet.turnScore[3]);

      Game backOnDunholm = switchTo(backOnTarnholm, MapDef.DUNHOLM);
      assertSame(MapDef.DUNHOLM, backOnDunholm.map);
      assertEquals(dunBefore, same(backOnDunholm));
      assertEquals(2, backOnDunholm.turnsTaken);
      assertTrue(backOnDunholm.dice[0].held);
      assertEquals(3, backOnDunholm.playsheet.turnScore[2]);

      // And Tarnholm is still exactly as it was left
      assertEquals(tarnBefore, same(switchTo(backOnDunholm, MapDef.TARNHOLM)));
   }

   @Test
   public void aFinishedGameClearsOnlyItsOwnIslandsSave() {
      GameStateStore.save(dir, tarnholmGame());
      GameStateStore.save(dir, dunholmGame());

      Game finished = tarnholmGame();
      finished.turnsTaken = MapDef.TARNHOLM.turnCap;
      assertTrue(finished.isGameDone());
      GameStateStore.persist(dir, finished);

      assertFalse(new File(dir, "game-state.json").exists());
      assertTrue(new File(dir, "game-state-dunholm.json").exists());
      assertNull(GameStateStore.load(dir, MapDef.TARNHOLM));
      assertNotNull(GameStateStore.load(dir, MapDef.DUNHOLM));
   }

   @Test
   public void aSchemaOneSaveStillLoadsAsTheDefaultIsland() throws Exception {
      writeText("game-state.json", SaveCompatTest.SAVE);

      assertNull(GameStateStore.load(dir, MapDef.DUNHOLM));
      assertTrue("Dunholm's load must leave the default file alone", new File(dir, "game-state.json").exists());

      Game game = GameStateStore.load(dir, MapDef.TARNHOLM);
      assertNotNull(game);
      assertSame(MapDef.TARNHOLM, game.map);
      assertEquals(7, game.turnsTaken);
      assertEquals(2, game.rolls);
   }

   @Test
   public void aDunholmGameInTheDefaultFileMovesToDunholmsOnFirstLoad() throws Exception {
      // What a build from before the split wrote when Dunholm was on screen
      String json = GameStateStore.toJson(dunholmGame());
      writeText("game-state.json", json);

      // The default island sees nothing of it, and the game is not lost
      assertNull(GameStateStore.load(dir, MapDef.TARNHOLM));
      assertFalse(new File(dir, "game-state.json").exists());
      assertTrue(new File(dir, "game-state-dunholm.json").exists());

      Game game = GameStateStore.load(dir, MapDef.DUNHOLM);
      assertNotNull(game);
      assertSame(MapDef.DUNHOLM, game.map);
      assertEquals(same(dunholmGame()), same(game));
      assertEquals(0, filesNamed(".corrupt-"));
   }

   @Test
   public void theMigrationAlsoRunsWhenTheLaunchOpensOnTheOtherIslandFirst() throws Exception {
      writeText("game-state.json", GameStateStore.toJson(dunholmGame()));

      Game game = GameStateStore.load(dir, MapDef.DUNHOLM);
      assertNotNull(game);
      assertEquals(same(dunholmGame()), same(game));
      assertFalse(new File(dir, "game-state.json").exists());
   }

   @Test
   public void aMisplacedGameNeverReplacesOrDestroysAnIslandsOwnFile() throws Exception {
      // Both exist: the island's own file is newer, so the stray one is set aside, not deleted
      writeText("game-state.json", GameStateStore.toJson(dunholmGame()));
      Game own = dunholmGame();
      own.turnsTaken = 5;
      GameStateStore.save(dir, own);
      assertTrue(new File(dir, "game-state.json").setLastModified(1000000L));

      Game game = GameStateStore.load(dir, MapDef.DUNHOLM);
      assertEquals(5, game.turnsTaken);
      assertEquals(1, filesNamed(".misplaced-"));
      assertFalse(new File(dir, "game-state.json").exists());
   }

   @Test
   public void anIslandsFileHoldingAnotherIslandsGameIsQuarantinedNotLoaded() throws Exception {
      writeText("game-state-dunholm.json", GameStateStore.toJson(tarnholmGame()));

      assertNull(GameStateStore.load(dir, MapDef.DUNHOLM));
      assertFalse(new File(dir, "game-state-dunholm.json").exists());
      assertEquals(1, filesNamed(".corrupt-"));
   }

   @Test
   public void aFinishedSaveIsNotResurrected() throws Exception {
      // Dunholm has no turn cap, so a finished game is one that reached its target
      Game finished = dunholmGame();
      for(int i = 0; i < 4; i++) {
         finished.playsheet.cities[i] = true;
      }
      finished.playsheet.villages[1] = true;
      assertTrue(finished.isGameDone());
      writeText("game-state-dunholm.json", GameStateStore.toJson(finished));

      assertNull(GameStateStore.load(dir, MapDef.DUNHOLM));
      assertFalse(new File(dir, "game-state-dunholm.json").exists());
   }
}
