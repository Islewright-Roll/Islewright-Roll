package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

/**
 * The decisions behind the Map setting that need no device: which map a launch opens on
 * (the setting; the saves are per island and do not vote), and what counts as progress.
 */
public class MapSwitchTest {

   @Test
   public void theSettingDecidesTheMapAtLaunch() {
      // Each island has its own save, so a save cannot say which one opens
      assertSame(MapDef.DUNHOLM, MapDef.forLaunch(MapDef.DUNHOLM.id));
      assertSame(MapDef.TARNHOLM, MapDef.forLaunch(MapDef.TARNHOLM.id));
   }

   @Test
   public void aSettingNobodyKnowsOrNoneAtAllIsTheFirstIsland() {
      // A settings file written before there were two maps has no map in it, and neither has
      // the settings of a player who has only ever had the one save
      assertSame(MapDef.TARNHOLM, MapDef.forLaunch(null));
      assertSame(MapDef.TARNHOLM, MapDef.forLaunch("atlantis"));
      assertSame(MapDef.TARNHOLM, MapDef.forLaunch(new HighScoresAndSettings().map));
   }

   @Test
   public void anUntouchedGameHasNoProgressOnEitherMap() {
      for(MapDef map : MapDef.ALL) {
         assertFalse(map.id, new Game(map).hasProgress());
      }
   }

   @Test
   public void aRollATurnOrAnythingBuiltIsProgress() {
      for(MapDef map : MapDef.ALL) {
         Game rolled = new Game(map);
         rolled.rolls = 1;
         assertTrue(map.id + " rolled", rolled.hasProgress());

         Game later = new Game(map);
         later.turnsTaken = 1;
         assertTrue(map.id + " a turn in", later.hasProgress());

         Game road = new Game(map);
         road.playsheet.roads[1] = true;
         assertTrue(map.id + " road", road.hasProgress());

         Game village = new Game(map);
         village.playsheet.villages[0] = true;
         assertTrue(map.id + " village", village.hasProgress());

         Game castle = new Game(map);
         castle.playsheet.cities[0] = true;
         assertTrue(map.id + " castle", castle.hasProgress());

         Game trader = new Game(map);
         trader.playsheet.traders[1] = true;
         assertTrue(map.id + " trader", trader.hasProgress());
      }
   }

   @Test
   public void theStartRoadAloneIsNotProgress() {
      for(MapDef map : MapDef.ALL) {
         Game game = new Game(map);
         assertTrue(game.playsheet.roads[0]);
         assertFalse(map.id, game.hasProgress());
      }
   }

   @Test
   public void aFinishedGameHasNothingLeftToLose() {
      Game outOfTurns = new Game(MapDef.TARNHOLM);
      outOfTurns.turnsTaken = MapDef.TARNHOLM.turnCap;
      assertTrue(outOfTurns.isGameDone());
      assertFalse(outOfTurns.hasProgress());

      Game won = new Game(MapDef.DUNHOLM);
      won.rolls = 1;
      for(int i = 0; i < won.playsheet.cities.length; i++) {
         won.playsheet.cities[i] = true;
      }
      won.playsheet.villages[0] = true;
      won.playsheet.villages[1] = true;
      assertTrue(won.isGameDone());
      assertFalse(won.hasProgress());
   }

   @Test
   public void startingANewGameOnAnotherMapLeavesNoProgressBehind() {
      Game game = new Game(MapDef.TARNHOLM);
      game.rolls = 2;
      game.playsheet.roads[1] = true;
      game.startNewGame(MapDef.DUNHOLM);
      assertSame(MapDef.DUNHOLM, game.map);
      assertFalse(game.hasProgress());
   }
}
