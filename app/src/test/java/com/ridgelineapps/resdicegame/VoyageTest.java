package com.ridgelineapps.resdicegame;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.util.zip.Inflater;

import org.junit.Test;

/**
 * The King's voyage: which stage the ship is at, where the dock goes, how the HUD strip
 * fits 216 px, how the traders are centred, and where the ship lies: moored in open water
 * beside the dock and wholly on screen, and on its way in at three stops well apart, over
 * the sea, that the screen shows.
 */
public class VoyageTest {
   /**
    * The two screens the ship has to work on, in art pixels: width, height, top inset and
    * bottom inset. A 1080 x 2400 phone at scale 5 and a 720 x 1280 one at scale 3, each
    * with no status bar and its navigation bar (63 and 48 px) rounded up to art pixels.
    */
   static final int[][] SCREENS = { { 216, 480, 0, 13 }, { 240, 426, 0, 16 } };

   // ---------------------------------------------------------------- stages

   @Test
   public void tarnholmShipAppearsThreeTurnsFromTheEndAndMoorsForTheLastTurn() {
      MapDef map = MapDef.TARNHOLM;
      for(int taken = 0; taken <= 11; taken++) {
         assertEquals("turn " + (taken + 1), Voyage.STAGE_NONE, Voyage.stage(map, taken, false, 0));
      }
      assertEquals(Voyage.STAGE_FAR, Voyage.stage(map, 12, false, 0));
      assertEquals(Voyage.STAGE_MID, Voyage.stage(map, 13, false, 0));
      assertEquals("the last turn is played with the ship at the dock", Voyage.STAGE_MOORED, Voyage.stage(map, 14, false, 0));
      assertEquals("the 15th turn taken is over: the flag is up", Voyage.STAGE_DONE, Voyage.stage(map, 15, true, 40));
   }

   @Test
   public void dunholmHasNoCapSoItsShipNeverComesOntoTheBoard() {
      MapDef map = MapDef.DUNHOLM;
      for(int taken = 0; taken <= 60; taken++) {
         assertEquals("turn " + (taken + 1), Voyage.STAGE_NONE, Voyage.stage(map, taken, false, 4));
         assertEquals("won on turn " + taken, Voyage.STAGE_NONE, Voyage.stage(map, taken, true, 10));
      }
   }

   @Test
   public void aCappedRaceShipFollowsTheTurnCap() {
      MapDef map = RaceFixture.CAPPED;
      assertEquals(Voyage.STAGE_NONE, Voyage.stage(map, 16, false, 4));
      assertEquals(Voyage.STAGE_FAR, Voyage.stage(map, 17, false, 4));
      assertEquals(Voyage.STAGE_MID, Voyage.stage(map, 18, false, 4));
      assertEquals(Voyage.STAGE_MOORED, Voyage.stage(map, 19, false, 4));
      assertEquals("out of turns", Voyage.STAGE_DONE, Voyage.stage(map, 20, true, 9));
   }

   @Test
   public void aRaceWonEarlyLeavesTheShipWhereItWas() {
      MapDef map = RaceFixture.CAPPED;
      // Reaching the target on turn 1 or turn 10: no ship yet
      assertEquals(Voyage.STAGE_NONE, Voyage.stage(map, 1, true, 10));
      assertEquals(Voyage.STAGE_NONE, Voyage.stage(map, 10, true, 11));
      // On turn 18 (turnsTaken counts it): the far stop was 17 taken, 18th turn is R = 3
      assertEquals(Voyage.STAGE_FAR, Voyage.stage(map, 18, true, 10));
      // On the last turn: moored as it is for the whole last turn, but the King has not
      // arrived, so there is no flag
      assertEquals(Voyage.STAGE_MOORED, Voyage.stage(map, 20, true, 10));
   }

   @Test
   public void cellsRunFromTheFirstTurnToThePier() {
      assertEquals(0, Voyage.cell(15, 0, false, false));
      assertEquals(14, Voyage.cell(15, 14, false, false));
      assertEquals(15, Voyage.cell(15, 15, true, false));
      assertEquals(19, Voyage.cell(20, 20, true, true));
      assertEquals(0, Voyage.cell(20, 1, true, true));
      assertEquals(20, Voyage.cell(20, 20, true, false));
      // No cap: the turn being played, or the one the target was reached in
      assertEquals(0, Voyage.cell(0, 0, false, false));
      assertEquals(40, Voyage.cell(0, 40, false, false));
      assertEquals(39, Voyage.cell(0, 40, true, true));
   }

   // ---------------------------------------------------------------- the dock

   @Test
   public void theDockIsWhereTheStartRoadMeetsTheSeaOnBothMapsAsTheBakeWroteIt() throws IOException {
      for(MapDef map : MapDef.ALL) {
         Board board = Board.forMap(map);
         Pt vertex = Voyage.coastalEnd(map.hex, board.roadEnds[0]);
         // The start road is the NE edge of the NW hex: its upper left end is the sea's
         assertEquals(map.id, Voyage.DOCK_FOOT, vertex);
         assertTrue(board.roadTouches(0, vertex));
         // The numbers in Voyage are the ones the bake wrote beside the board it drew the dock on
         int[] foot = geometry(map, "dock_foot");
         int[] mooring = geometry(map, "dock_mooring");
         int[] sprite = geometry(map, "dock_sprite");
         assertEquals(map.id, Voyage.DOCK_FOOT, new Pt(foot[0], foot[1]));
         assertEquals(map.id, Voyage.DOCK_MOORING, new Pt(mooring[0], mooring[1]));
         assertEquals(map.id, Voyage.DOCK_SPRITE, new Pt(sprite[0], sprite[1]));
      }
   }

   /**
    * The dock is part of the terrain, and stands on sand and water: no pixel of it is in
    * the mountain's hex or any other hex but the desert it leaves from (within 10 px of
    * the road's end across), and it covers no village, castle or trader spot and no
    * road but the end of the start road it continues. The bake itself refuses to draw it
    * over a pixel of the mountain's sprite, which rises above its hex.
    */
   @Test
   public void theDockStandsOnSandAndWaterAndCoversNoPiece() throws IOException {
      boolean[][] village = mask(read("piece_village"));
      boolean[][] castle = mask(read("piece_castle"));
      boolean[][] trader = mask(read("piece_trader"));
      boolean[][][] roads = { mask(read("road_h")), mask(read("road_slash")), mask(read("road_backslash")) };
      for(MapDef map : MapDef.ALL) {
         boolean[][] dock = dock(map);
         Pt at = new Pt(0, 0);
         Board board = Board.forMap(map);
         assertTrue(map.id, count(dock) > 300);
         for(int y = 0; y < dock.length; y++) {
            for(int x = 0; x < dock[y].length; x++) {
               if(!dock[y][x]) {
                  continue;
               }
               assertTrue(map.id + " dock at " + x + ", " + y + " runs up from the vertex",
                     y < Voyage.DOCK_FOOT.y && Math.abs(x - Voyage.DOCK_FOOT.x) <= 10);
               for(int h = 0; h < map.hex.length; h++) {
                  int dx = Math.abs(x - map.hex[h][0]), dy = Math.abs(y - map.hex[h][1]);
                  boolean inside = dy < 36 && 2 * dx + dy < 72;
                  // The north hex is desert on both maps: its corner by the road's end is the
                  // sand the dock's foot stands on. Every other hex is out of bounds.
                  if(h != Board.N) {
                     assertFalse(map.id + " dock at " + x + ", " + y + " in hex " + h, inside);
                  }
               }
            }
         }
         for(int i = 1; i < board.roadOrigin.length; i++) {
            assertEquals(map.id + " road " + i, 0, overlap(dock, at, roads[board.roadShape[i]], board.roadOrigin[i]));
         }
         // The dock's foot lies over the start road's round end (the game draws dock.png over
         // the roads), and no more of the road than that
         int onStart = overlap(dock, at, roads[board.roadShape[0]], board.roadOrigin[0]);
         assertTrue(map.id + " start road under " + onStart, onStart >= 16 && onStart <= 40);
         for(int i = 0; i < board.village.length; i++) {
            Pt p = board.village[i];
            assertEquals(map.id + " village " + i, 0, overlap(dock, at, village, new Pt(p.x - Art.VILLAGE_AX, p.y - Art.VILLAGE_AY)));
         }
         for(int i = 0; i < board.city.length; i++) {
            Pt p = board.city[i];
            assertEquals(map.id + " castle " + i, 0, overlap(dock, at, castle, new Pt(p.x - Art.CASTLE_AX, p.y - Art.CASTLE_AY)));
         }
         Pt[] traders = TraderLayout.anchors(map);
         for(int i = 0; i < traders.length; i++) {
            Pt p = traders[i];
            assertEquals(map.id + " trader " + (i + 1), 0, overlap(dock, at, trader, new Pt(p.x - Art.TRADER_AX, p.y - Art.TRADER_AY)));
         }
      }
   }

   /**
    * dock.png is the dock's above water art alone, drawn over the roads at dock_sprite: its
    * opaque pixels are exactly the bake's dock_spans, none is half transparent, and each is
    * the colour the baked board has at the same place, so drawing it changes no pixel of
    * the board but the ones a road lies over.
    */
   @Test
   public void theDockSpriteIsTheBakedDocksAboveWaterArtOnBothMaps() throws IOException {
      int[][] sprite = read("dock");
      for(MapDef map : MapDef.ALL) {
         boolean[][] dock = dock(map);
         int[][] board = read(map == MapDef.TARNHOLM ? "board_tarnholm" : "board_dunholm");
         int opaque = 0;
         for(int y = 0; y < sprite.length; y++) {
            for(int x = 0; x < sprite[y].length; x++) {
               int alpha = sprite[y][x] >>> 24;
               assertTrue(map.id + " dock.png alpha " + alpha, alpha == 0 || alpha == 255);
               int bx = Voyage.DOCK_SPRITE.x + x, by = Voyage.DOCK_SPRITE.y + y;
               assertEquals(map.id + " dock.png at " + bx + ", " + by, dock[by][bx], alpha == 255);
               if(alpha == 255) {
                  opaque++;
                  assertEquals(map.id + " board under dock.png at " + bx + ", " + by, sprite[y][x] & 0xFFFFFF, board[by][bx] & 0xFFFFFF);
               }
            }
         }
         assertEquals(map.id, count(dock), opaque);
      }
   }

   @Test
   public void aStartRoadWithNoCoastalEndIsAnError() {
      // Two hexes side by side, and a road along an outer edge of one: both ends touch it alone
      int[][] hex = { { 100, 100 }, { 154, 136 } };
      Pt[] ends = { new Pt(82, 64), new Pt(118, 64) };
      try {
         Voyage.coastalEnd(hex, ends);
         fail("expected an error");
      }
      catch(IllegalStateException expected) {
         // fine
      }
   }

   // ---------------------------------------------------------------- the HUD strip

   @Test
   public void fifteenTurnsFitWithFlagsInTheHud() {
      Voyage.Strip strip = new Voyage.Strip(15, true, ArtCanvas.DESIGN_W);
      assertEquals(12, strip.pitch);
      checkFits(strip);
      // A flag is 13 wide, so at 12 the next one stands on its right edge
      assertEquals(strip.cellX(0) + 12, strip.cellX(1));
      // The pier directly follows the last flag; the last turn's ship lies alongside it, and
      // the finished ship is in harbour behind it
      assertEquals(strip.cellX(14) + Art.TRACK_FLAG_W + 1, strip.pierX());
      assertEquals(strip.pierX() - Art.TRACK_SHIP_W + 1, strip.shipX(14));
      assertTrue(strip.shipX(14) >= strip.cellX(14));
      assertEquals(strip.pierX() + Voyage.PIER_SHIP_IN, strip.shipX(15));
      // and the ship before it is in the cell of its turn, left of the last flag
      assertTrue(strip.shipX(13) + Art.TRACK_SHIP_W <= strip.cellX(14) + 2);
      // and in a cell, the ship stands where its flag will
      assertEquals(strip.cellX(7), strip.shipX(7));
      // Every turn to come is marked here: each becomes a flag
      for(int c = 0; c < 15; c++) {
         assertTrue(strip.marked(c));
      }
   }

   // ---------------------------------------------------------------- the window strip

   @Test
   public void dunholmsStripIsAWindowOfTarnholmsSize() {
      Voyage.Strip dunholm = Voyage.Strip.forMap(MapDef.DUNHOLM, ArtCanvas.DESIGN_W);
      Voyage.Strip tarnholm = Voyage.Strip.forMap(MapDef.TARNHOLM, ArtCanvas.DESIGN_W);
      assertEquals(Voyage.FLAG_WINDOW, dunholm.turns);
      assertTrue(dunholm.flags);
      assertEquals(tarnholm.turns, dunholm.turns);
      assertEquals(tarnholm.pitch, dunholm.pitch);
      assertEquals(tarnholm.width, dunholm.width);
      checkFits(dunholm);
      // A race with a cap keeps the narrow strip without flags
      assertFalse(Voyage.Strip.forMap(RaceFixture.CAPPED, ArtCanvas.DESIGN_W).flags);
   }

   @Test
   public void untilTheWindowFillsEveryTurnHasItsFlagAndTheShipFollows() {
      for(int taken = 0; taken <= Voyage.FLAG_WINDOW - 2; taken++) {
         assertEquals(0, Voyage.windowStart(taken, false));
         assertEquals(taken, Voyage.windowFlags(taken, false));
         assertEquals(taken, Voyage.windowShipCell(taken, false));
      }
   }

   @Test
   public void onceTheWindowIsFullTheOldestFlagFallsOffTheLeft() {
      int full = Voyage.FLAG_WINDOW - 2;
      for(int taken = full + 1; taken <= 60; taken++) {
         int start = Voyage.windowStart(taken, false);
         assertEquals(taken - full, start);
         assertEquals(full, Voyage.windowFlags(taken, false));
         // The newest flag is the last turn taken
         assertEquals(taken, start + Voyage.windowFlags(taken, false));
         // The ship never reaches the cell beside the pier while the race is on
         assertEquals(Voyage.FLAG_WINDOW - 2, Voyage.windowShipCell(taken, false));
      }
      // After 15 turns: turns 3 to 15 fly flags, the 16th is being played
      assertEquals(2, Voyage.windowStart(15, false));
   }

   @Test
   public void aWonRaceFlagsTheShipsCellAndPutsTheShipInHarbour() {
      Voyage.Strip strip = Voyage.Strip.forMap(MapDef.DUNHOLM, ArtCanvas.DESIGN_W);
      // Won on turn 7: seven flags from the first turn, the ship in harbour
      assertEquals(0, Voyage.windowStart(7, true));
      assertEquals(7, Voyage.windowFlags(7, true));
      assertEquals(Voyage.FLAG_WINDOW, Voyage.windowShipCell(7, true));
      assertEquals(strip.pierX() + Voyage.PIER_SHIP_IN, strip.shipX(Voyage.windowShipCell(7, true)));
      // Won in the 31st turn: the flags already up stay where they were, and the winning
      // turn's flag goes up in the cell the ship was sailing in
      assertEquals(Voyage.windowStart(30, false), Voyage.windowStart(31, true));
      assertEquals(Voyage.windowShipCell(30, false) + 1, Voyage.windowFlags(31, true));
      for(int taken = 1; taken <= 60; taken++) {
         assertEquals("won in turn " + taken, Voyage.windowStart(taken - 1, false), Voyage.windowStart(taken, true));
         // The cell beside the pier never has a flag
         assertTrue(Voyage.windowFlags(taken, true) <= Voyage.FLAG_WINDOW - 1);
      }
   }

   /** The strip's cells, ship and pier lie inside the framed band of water drawn from the legend's inset to its inset. */
   @Test
   public void bothStripsLieInsideTheFramedWaterBand() {
      for(MapDef map : MapDef.ALL) {
         Voyage.Strip strip = Voyage.Strip.forMap(map, ArtCanvas.DESIGN_W);
         assertTrue(map.id, strip.left >= GameView.HUD_INSET + 1);
         assertTrue(map.id, strip.left + strip.width <= ArtCanvas.DESIGN_W - GameView.HUD_INSET - 1);
      }
   }

   @Test
   public void twentyTurnsFitWithoutFlagsInTheHud() {
      Voyage.Strip strip = new Voyage.Strip(20, false, ArtCanvas.DESIGN_W);
      assertEquals(9, strip.pitch);
      checkFits(strip);
      // The pier hugs the last buoy: its room ends at the pier, give or take the buoy's own margin
      assertTrue(strip.pierX() >= strip.buoyX(19) + Art.TRACK_BUOY_W + 1);
      assertTrue(strip.pierX() <= strip.buoyX(19) + Art.TRACK_BUOY_W + 1 + (strip.pitch - Art.TRACK_BUOY_W) / 2);
      assertEquals(strip.pierX() - Art.TRACK_SHIP_W + 1, strip.shipX(19));
      assertEquals(strip.pierX() + Voyage.PIER_SHIP_IN, strip.shipX(20));
      // A buoy is centred in its cell
      assertEquals(strip.cellX(3) + 2, strip.buoyX(3));
      // A ship is centred on its cell, and at most its last column touches the next cell's buoy
      for(int c = 0; c < 19; c++) {
         assertTrue(strip.shipX(c) + Art.TRACK_SHIP_W <= strip.buoyX(c + 1) + 1);
      }
      // Half the turns are marked: ten buoys, on the even turns, evenly spaced with clear
      // water between them, and the last one is the last turn before the pier
      int marked = 0, last = -1;
      for(int c = 0; c < 20; c++) {
         if(strip.marked(c)) {
            marked++;
            if(last >= 0) {
               assertEquals(2 * strip.pitch, strip.buoyX(c) - strip.buoyX(last));
               assertTrue(2 * strip.pitch >= 2 * Art.TRACK_BUOY_W);
            }
            last = c;
         }
      }
      assertEquals(10, marked);
      assertEquals(19, last);
      assertTrue(strip.marked(1) && !strip.marked(0));
   }

   private void checkFits(Voyage.Strip strip) {
      assertTrue("margin left", strip.left >= Voyage.STRIP_MARGIN);
      assertTrue("margin right", strip.left + strip.width <= ArtCanvas.DESIGN_W - Voyage.STRIP_MARGIN);
      // The water ends at the later of the pier and the ship that lies at it
      assertEquals(strip.width, Math.max(strip.pierX() + Art.TRACK_PIER_W, strip.shipX(strip.turns) + Art.TRACK_SHIP_W));
      // Everything inside the water, from the first sprite to the pier, left to right until
      // the ship is at the pier, where the last turn's ship and the finished one are one
      assertTrue(strip.shipX(0) >= Voyage.STRIP_LEAD);
      for(int c = 1; c < strip.turns; c++) {
         assertTrue("ship cells run left to right", strip.shipX(c) > strip.shipX(c - 1));
      }
      assertEquals("the last turn's ship lies alongside the pier", strip.pierX() - Art.TRACK_SHIP_W + 1, strip.shipX(strip.turns - 1));
      assertEquals("the finished ship is in harbour", strip.pierX() + Voyage.PIER_SHIP_IN, strip.shipX(strip.turns));
      // No empty cell between the last cell and the pier
      assertTrue(strip.pierX() - strip.cellX(strip.turns - 1) <= strip.pitch + Art.TRACK_FLAG_W);
   }

   // ---------------------------------------------------------------- the traders

   @Test
   public void aSingleTraderIsAlreadyCentredAndAPairIsMovedToCentreItsBox() {
      for(MapDef map : MapDef.ALL) {
         Pt[] at = TraderLayout.anchors(map);
         for(int i = 0; i < at.length; i++) {
            int hex = map.traders[i][0];
            int count = 0;
            for(int[] t : map.traders) {
               count += t[0] == hex ? 1 : 0;
            }
            int[] shift = TraderLayout.pairShift(map, hex);
            if(count == 1) {
               assertEquals(0, shift[0]);
               assertEquals(0, shift[1]);
            }
         }
      }
      // Dunholm's desert pair: one cart above the other, boxes at (0,-15) and (0,15) from the anchor
      int[] shift = TraderLayout.pairShift(MapDef.DUNHOLM, Board.N);
      assertEquals(0, shift[0]);
      assertEquals(4, shift[1]);
      Pt[] at = TraderLayout.anchors(MapDef.DUNHOLM);
      int[] hex = MapDef.DUNHOLM.hex[Board.N];
      int left = Math.min(at[5].x, at[6].x) - Art.TRADER_AX;
      int right = Math.max(at[5].x, at[6].x) - Art.TRADER_AX + Art.TRADER_BOX_W;
      int top = Math.min(at[5].y, at[6].y) - Art.TRADER_AY;
      int bottom = Math.max(at[5].y, at[6].y) - Art.TRADER_AY + Art.TRADER_BOX_H;
      assertEquals(hex[0], (left + right) / 2);
      assertEquals(hex[1], (top + bottom) / 2);
   }

   @Test
   public void aSingleTradersOpaqueBoxIsCentredOnTheHexX() throws IOException {
      int[][] trader = read("piece_trader");
      int min = Integer.MAX_VALUE, max = -1, top = Integer.MAX_VALUE, bottom = -1;
      for(int y = 0; y < trader.length; y++) {
         for(int x = 0; x < trader[y].length; x++) {
            if((trader[y][x] >>> 24) > 128) {
               min = Math.min(min, x);
               max = Math.max(max, x);
               top = Math.min(top, y);
               bottom = Math.max(bottom, y);
            }
         }
      }
      // The numbers in Art come from this measurement
      assertEquals(Art.TRADER_BOX_W, max - min + 1);
      assertEquals(Art.TRADER_BOX_H, bottom - top + 1);
      assertEquals(0, min);
      assertEquals(0, top);
      assertEquals(Art.TRADER_AX, (min + max + 1) / 2);
   }

   // ---------------------------------------------------------------- the ship and the board

   /** What the screen shows of the board between its bars: left, top, right, bottom, in board pixels. */
   private static int[] shown(MapDef map, int[] screen) {
      // The top bar of an island that is not a race (a race adds its bonus line)
      int barBottom = screen[2] + GameView.TOP_H;
      int trayTop = screen[1] - GameView.BOTTOM_H - screen[3];
      Pt origin = Voyage.boardOrigin(screen[0], barBottom, trayTop);
      return new int[] { -origin.x, barBottom - origin.y, screen[0] - origin.x, trayTop - origin.y };
   }

   private static Pt stop(MapDef map, int stage, int[] shown) {
      return Voyage.shipOrigin(stage, Voyage.DOCK_MOORING, shown[0]);
   }

   @Test
   public void theShipsNumbersAreMeasuredFromItsSprite() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      assertEquals(Art.SHIP_H, ship.length);
      assertEquals(Art.SHIP_W, ship[0].length);
      int[] box = box(ship);
      assertEquals(Art.SHIP_BOX_X, box[0]);
      assertEquals(Art.SHIP_BOX_Y, box[1]);
      assertEquals(Art.SHIP_BOX_W, box[2] - box[0] + 1);
      assertEquals(Art.SHIP_BOX_H, box[3] - box[1] + 1);
      // Moored, its top row is no higher than the first row the layout keeps in sight
      Pt moored = Voyage.shipOrigin(Voyage.STAGE_MOORED, Voyage.DOCK_MOORING, 132);
      assertTrue(moored.y + Art.SHIP_BOX_Y >= Voyage.HARBOUR_TOP);
      // The pixel that goes on the mooring point is the ship's own: the tip of its bowsprit
      assertTrue(ship[Art.SHIP_CONTACT_Y][Art.SHIP_CONTACT_X]);
      assertEquals(Art.SHIP_CONTACT_X, box[2]);
   }

   /**
    * Moored, the ship is in open water: on no hex, on no dry land of the baked board, and
    * over no road, village site, castle site or trader spot, built or not, on either island.
    */
   @Test
   public void theMooredShipLiesInOpenWaterClearOfEverything() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      for(MapDef map : MapDef.ALL) {
         Pt at = stop(map, Voyage.STAGE_MOORED, shown(map, SCREENS[0]));
         assertClearOfTheIsland(map.id + " moored", map, ship, at);
      }
   }

   /** Moored, the whole ship shows between the bars, on both screens and both islands. */
   @Test
   public void theMooredShipIsWhollyOnScreen() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      for(MapDef map : MapDef.ALL) {
         for(int[] screen : SCREENS) {
            int[] shown = shown(map, screen);
            Pt at = stop(map, Voyage.STAGE_MOORED, shown);
            assertEquals(map.id + " on " + screen[0] + " x " + screen[1], count(ship), showing(ship, at, shown));
         }
      }
   }

   /**
    * Moored, the ship lies beside the jetty with its bow at it: the tip of its bowsprit is two
    * columns onto the deck's west edge, some way back from the jetty's seaward end, and the
    * only pixels of the ship over the dock are the bowsprit's, none of them past the mooring
    * point's column (so the ship never crosses the deck).
    */
   @Test
   public void theMooredShipsBowIsAtTheJettyAndOnlyItsBowspritIsOverTheDock() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      for(MapDef map : MapDef.ALL) {
         boolean[][] dock = dock(map);
         Pt at = stop(map, Voyage.STAGE_MOORED, shown(map, SCREENS[0]));
         Pt moor = Voyage.DOCK_MOORING;
         assertTrue(dock[moor.y][moor.x]);
         assertTrue(dock[moor.y][moor.x - 1]);
         assertFalse(dock[moor.y][moor.x - 4]);
         int over = 0;
         for(int y = 0; y < ship.length; y++) {
            for(int x = 0; x < ship[y].length; x++) {
               if(ship[y][x] && dock[at.y + y][at.x + x]) {
                  over++;
                  assertTrue(map.id + " ship over the dock at " + (at.x + x) + ", " + (at.y + y), at.x + x <= moor.x);
               }
            }
         }
         assertTrue(map.id + " ship over the dock: " + over, over > 0 && over <= 40);
         int top = 0;
         while(count(dock[top]) == 0) {
            top++;
         }
         assertTrue("the bow is " + (moor.y - top) + " rows down the jetty", moor.y - top >= 8 && moor.y - top <= 30);
      }
   }

   /**
    * On its way in the ship makes two stops before the dock, each at least 20 px from the next,
    * each further right and no higher than the last, over open sea and clear of the dock and
    * of every piece. The middle stop shows nearly all of the ship (nine tenths or more) and
    * the far one at least a quarter: its bow, coming in at the screen's left edge.
    */
   @Test
   public void theShipComesInByTwoStopsWellApartOverOpenSeaThatTheScreenShows() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      int all = count(ship);
      for(MapDef map : MapDef.ALL) {
         boolean[][] dock = dock(map);
         Pt dockAt = new Pt(0, 0);
         for(int[] screen : SCREENS) {
            int[] shown = shown(map, screen);
            String where = map.id + " on " + screen[0] + " x " + screen[1];
            for(int stage = Voyage.STAGE_FAR; stage <= Voyage.STAGE_MID; stage++) {
               String name = where + " stage " + stage;
               Pt at = stop(map, stage, shown);
               Pt next = stop(map, stage + 1, shown);
               int dx = next.x - at.x, dy = next.y - at.y;
               assertTrue(name + " is " + dx + ", " + dy + " from the next", dx * dx + dy * dy >= Voyage.MIN_STEP * Voyage.MIN_STEP);
               assertTrue(name + " heads right and not up", dx > 0 && dy >= 0 && dx <= Voyage.MAX_STEP && dy <= Voyage.MAX_STEP);
               assertEquals(name + " dock", 0, overlap(ship, at, dock, dockAt));
               assertClearOfTheIsland(name, map, ship, at);
               int showing = showing(ship, at, shown);
               int least = stage == Voyage.STAGE_FAR ? all / 4 : all * 9 / 10;
               assertTrue(name + " shows " + showing + " of " + all, showing >= least);
            }
         }
      }
   }

   @Test
   public void theStopsFollowWhatTheScreenShowsButNeverStrayFar() {
      Pt moor = Voyage.DOCK_MOORING;
      Pt moored = Voyage.shipOrigin(Voyage.STAGE_MOORED, moor, 144);
      assertEquals(new Pt(moor.x - Art.SHIP_CONTACT_X, moor.y - Art.SHIP_CONTACT_Y), moored);
      // Room to the left: the far stop hangs half off the screen's edge, level with the mooring
      int roomy = moored.x - 2 * Voyage.MIN_STEP - 4 + Voyage.FAR_OUT_X;
      assertEquals(new Pt(roomy - Voyage.FAR_OUT_X, moored.y), Voyage.shipOrigin(Voyage.STAGE_FAR, moor, roomy));
      // Less room: it starts higher instead, and still sails two full steps
      Pt far = Voyage.shipOrigin(Voyage.STAGE_FAR, moor, 150);
      assertEquals(150 - Voyage.FAR_OUT_X, far.x);
      assertTrue(far.y < moored.y);
      int dx = moored.x - far.x, dy = moored.y - far.y;
      assertTrue(dx * dx + dy * dy >= 4 * Voyage.MIN_STEP * Voyage.MIN_STEP);
      // A screen that shows the whole board: the far stop is two full steps out, no more
      assertEquals(new Pt(moored.x - 2 * Voyage.MAX_STEP, moored.y), Voyage.shipOrigin(Voyage.STAGE_FAR, moor, -200));
      // A screen that shows no sea to the left at all: straight down from above
      assertEquals(moored.x, Voyage.shipOrigin(Voyage.STAGE_FAR, moor, 400).x);
      // Done is moored: the flag is the dock's, the ship does not move
      assertEquals(moored, Voyage.shipOrigin(Voyage.STAGE_DONE, moor, 144));
   }

   /** On both screens the ship sails in level and the far stop shows its bow, a quarter or more of it. */
   @Test
   public void theFarStopOnEachScreenIsLevelWithTheDockAndShowsItsBow() throws IOException {
      boolean[][] ship = mask(read("piece_kingship"));
      Pt[] boards = { new Pt(-144, -14), new Pt(-132, -32) };
      for(int i = 0; i < SCREENS.length; i++) {
         int[] shown = shown(MapDef.TARNHOLM, SCREENS[i]);
         Pt origin = Voyage.boardOrigin(SCREENS[i][0], GameView.TOP_H, SCREENS[i][1] - GameView.BOTTOM_H - SCREENS[i][3]);
         assertEquals(boards[i], origin);
         Pt far = stop(MapDef.TARNHOLM, Voyage.STAGE_FAR, shown);
         assertEquals(stop(MapDef.TARNHOLM, Voyage.STAGE_MOORED, shown).y, far.y);
         assertTrue("shows " + showing(ship, far, shown) + " of " + count(ship), showing(ship, far, shown) >= count(ship) / 4);
      }
   }

   // ---------------------------------------------------------------- sailing

   @Test
   public void onlyTheNextStageIsASailAndNothingElseIsAnythingButAJump() {
      assertEquals(Voyage.SAIL_IN_MS, Voyage.sailMs(Voyage.STAGE_NONE, Voyage.STAGE_FAR));
      assertEquals(Voyage.SAIL_STEP_MS, Voyage.sailMs(Voyage.STAGE_FAR, Voyage.STAGE_MID));
      assertEquals(Voyage.SAIL_STEP_MS, Voyage.sailMs(Voyage.STAGE_MID, Voyage.STAGE_MOORED));
      // The first frame (-1), no change, the flag going up, undoing, and skipping a stage all snap
      assertEquals(0, Voyage.sailMs(-1, Voyage.STAGE_FAR));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_MID, Voyage.STAGE_MID));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_MOORED, Voyage.STAGE_DONE));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_MID, Voyage.STAGE_FAR));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_FAR, Voyage.STAGE_MOORED));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_NONE, Voyage.STAGE_MID));
      assertEquals(0, Voyage.sailMs(Voyage.STAGE_MOORED, Voyage.STAGE_NONE));
   }

   /** The ship sails in from wholly off the left edge of the screen, level with the far stop, on both screens. */
   @Test
   public void aShipSailingInStartsWhollyOffTheScreensLeftEdgeLevelWithTheFarStop() {
      for(MapDef map : MapDef.ALL) {
         for(int[] screen : SCREENS) {
            int[] shown = shown(map, screen);
            Pt start = Voyage.sailStart(Voyage.STAGE_NONE, Voyage.DOCK_MOORING, shown[0]);
            Pt far = stop(map, Voyage.STAGE_FAR, shown);
            assertEquals(far.y, start.y);
            assertEquals(shown[0], start.x + Art.SHIP_W);
            assertTrue(start.x < far.x);
         }
      }
      // Between stops it starts from the stop it was at
      Pt mid = Voyage.shipOrigin(Voyage.STAGE_MID, Voyage.DOCK_MOORING, 144);
      assertEquals(mid, Voyage.sailStart(Voyage.STAGE_MID, Voyage.DOCK_MOORING, 144));
   }

   @Test
   public void theSailRunsFromStartToEndEasedAndOnWholePixels() {
      Pt from = new Pt(100, 50), to = new Pt(140, 63);
      assertEquals(from, Voyage.sailAt(from, to, 0, 650));
      assertEquals(from, Voyage.sailAt(from, to, -5, 650));
      assertEquals(to, Voyage.sailAt(from, to, 650, 650));
      assertEquals(to, Voyage.sailAt(from, to, 9999, 650));
      assertEquals("no duration is no sail", to, Voyage.sailAt(from, to, 0, 0));
      // Halfway is the halfway point exactly (smoothstep(0.5) is 0.5)
      assertEquals(new Pt(120, 57), Voyage.sailAt(from, to, 325, 650));
      // Slow at the start and at the end, quick in the middle
      Pt early = Voyage.sailAt(from, to, 65, 650);
      Pt late = Voyage.sailAt(from, to, 585, 650);
      assertTrue("early " + early, early.x - from.x < 4 * 40 / 10 / 2);
      assertTrue("late " + late, to.x - late.x < 4 * 40 / 10 / 2);
      // Never goes backwards, never overshoots, and each step is a whole pixel the way it goes
      int lastX = from.x, lastY = from.y;
      for(int ms = 0; ms <= 650; ms += 10) {
         Pt at = Voyage.sailAt(from, to, ms, 650);
         assertTrue(at.x >= lastX && at.x <= to.x && at.y >= lastY && at.y <= to.y);
         lastX = at.x;
         lastY = at.y;
      }
      // Sailing left (a screen whose far stop is right of its start) works the same way
      assertEquals(new Pt(120, 57), Voyage.sailAt(to, from, 325, 650));
   }

   /**
    * The map sits MAP_RAISE above the middle of the water between the bars, but never with the
    * masthead under the top bar, and never with the island's foot under the bottom bar.
    */
   @Test
   public void theMapWithTheShipInHarbourIsCentredBetweenTheBars() {
      int[][] bars = { { 62, 401 }, { 62, 500 }, { 80, 640 } };
      for(int[] bar : bars) {
         Pt at = Voyage.boardOrigin(216, bar[0], bar[1]);
         int above = at.y + Voyage.HARBOUR_TOP - bar[0];
         int below = bar[1] - (at.y + Voyage.ISLAND_FOOT);
         assertTrue(bar[0] + ": " + above + " above, " + below + " below", Math.abs(below - above - 2 * Voyage.MAP_RAISE) <= 1);
         assertTrue(above >= 0);
         assertTrue(below >= 0);
      }
      // Too little water to raise it all the way: the masthead stops at the top bar
      assertEquals(76 - Voyage.HARBOUR_TOP, Voyage.boardOrigin(216, 76, 375).y);
      // A screen too short to show it all: the island's foot wins, and stays above the bottom bar
      assertEquals(300 - Voyage.ISLAND_FOOT, Voyage.boardOrigin(216, 62, 300).y);
      assertEquals(300 - Voyage.ISLAND_FOOT, Voyage.boardOrigin(216, 62, 300).y);
   }

   /** No pixel of the ship on a hex, on the board's dry land, or on a road or piece of the map (the dock's own pixels aside). */
   private void assertClearOfTheIsland(String name, MapDef map, boolean[][] ship, Pt at) throws IOException {
      boolean[][] village = mask(read("piece_village"));
      boolean[][] castle = mask(read("piece_castle"));
      boolean[][] trader = mask(read("piece_trader"));
      boolean[][][] roads = { mask(read("road_h")), mask(read("road_slash")), mask(read("road_backslash")) };
      int[][] sea = read(map == MapDef.TARNHOLM ? "board_tarnholm" : "board_dunholm");
      Board board = Board.forMap(map);
      Pt[] traders = TraderLayout.anchors(map);
      assertTrue(name + " on the board", at.x >= 0 && at.y >= 0 && at.x + Art.SHIP_W <= Board.WIDTH && at.y + Art.SHIP_H <= Board.HEIGHT);
      boolean[][] dock = dock(map);
      int onHex = 0, onLand = 0;
      for(int y = 0; y < ship.length; y++) {
         for(int x = 0; x < ship[y].length; x++) {
            if(!ship[y][x]) {
               continue;
            }
            int bx = at.x + x, by = at.y + y;
            if(dock[by][bx]) {
               // The moored bowsprit lies over the deck's edge (and the pile there): the dock's own
               // brown is not dry land, and theMooredShipsBow... holds how far over it may lie.
               continue;
            }
            for(int[] centre : map.hex) {
               int dx = Math.abs(bx - centre[0]), dy = Math.abs(by - centre[1]);
               if(dy < 36 && 2 * dx + dy < 72) {
                  onHex++;
               }
            }
            // Dry land in the bake is sand, rock and grass: red well above blue. The sea and
            // its murky shallows are blue or level.
            int rgb = sea[by][bx];
            if(((rgb >> 16) & 255) > (rgb & 255) + 32) {
               onLand++;
            }
         }
      }
      assertEquals(name + " hexes", 0, onHex);
      assertEquals(name + " dry land", 0, onLand);
      for(int i = 0; i < board.roadOrigin.length; i++) {
         assertEquals(name + " road " + i, 0, overlap(ship, at, roads[board.roadShape[i]], board.roadOrigin[i]));
      }
      for(int i = 0; i < board.village.length; i++) {
         Pt p = board.village[i];
         assertEquals(name + " village " + i, 0, overlap(ship, at, village, new Pt(p.x - Art.VILLAGE_AX, p.y - Art.VILLAGE_AY)));
      }
      for(int i = 0; i < board.city.length; i++) {
         Pt p = board.city[i];
         assertEquals(name + " castle " + i, 0, overlap(ship, at, castle, new Pt(p.x - Art.CASTLE_AX, p.y - Art.CASTLE_AY)));
      }
      for(int i = 0; i < traders.length; i++) {
         Pt p = traders[i];
         assertEquals(name + " trader " + (i + 1), 0, overlap(ship, at, trader, new Pt(p.x - Art.TRADER_AX, p.y - Art.TRADER_AY)));
      }
   }

   /** A line of the bake's geometry file for a map, as its numbers. */
   private static int[] geometry(MapDef map, String key) throws IOException {
      String[] words = geometryLine(map, key);
      int[] out = new int[words.length - 1];
      for(int i = 0; i < out.length; i++) {
         out[i] = Integer.parseInt(words[i + 1]);
      }
      return out;
   }

   private static String[] geometryLine(MapDef map, String key) throws IOException {
      File file = new File("../tools/board_geometry_" + map.id + ".txt");
      java.io.BufferedReader in = new java.io.BufferedReader(new java.io.InputStreamReader(new FileInputStream(file), "UTF-8"));
      try {
         for(String line = in.readLine(); line != null; line = in.readLine()) {
            if(line.startsWith(key + " ")) {
               return line.trim().split(" ");
            }
         }
      }
      finally {
         in.close();
      }
      throw new IOException(key + " is not in " + file);
   }

   /** The dock's pixels on a map's board, from the row spans the bake wrote: indexed [y][x] over the whole board. */
   private static boolean[][] dock(MapDef map) throws IOException {
      boolean[][] out = new boolean[Board.HEIGHT][Board.WIDTH];
      String[] words = geometryLine(map, "dock_spans");
      for(int i = 1; i < words.length; i++) {
         String[] rowAndSpan = words[i].split(":");
         String[] span = rowAndSpan[1].split("-");
         int y = Integer.parseInt(rowAndSpan[0]);
         for(int x = Integer.parseInt(span[0]); x <= Integer.parseInt(span[1]); x++) {
            out[y][x] = true;
         }
      }
      return out;
   }

   private static int count(boolean[] row) {
      int n = 0;
      for(boolean b : row) {
         n += b ? 1 : 0;
      }
      return n;
   }

   private static int count(boolean[][] a) {
      int n = 0;
      for(boolean[] row : a) {
         for(boolean b : row) {
            n += b ? 1 : 0;
         }
      }
      return n;
   }

   /** The smallest box around a mask: left, top, right, bottom, all inclusive. */
   private static int[] box(boolean[][] a) {
      int[] box = { Integer.MAX_VALUE, Integer.MAX_VALUE, -1, -1 };
      for(int y = 0; y < a.length; y++) {
         for(int x = 0; x < a[y].length; x++) {
            if(a[y][x]) {
               box[0] = Math.min(box[0], x);
               box[1] = Math.min(box[1], y);
               box[2] = Math.max(box[2], x);
               box[3] = Math.max(box[3], y);
            }
         }
      }
      return box;
   }

   /** How many of a sprite's pixels, placed by its top left, are inside a box (right and bottom exclusive). */
   private static int showing(boolean[][] a, Pt at, int[] box) {
      int n = 0;
      for(int y = 0; y < a.length; y++) {
         for(int x = 0; x < a[y].length; x++) {
            int bx = at.x + x, by = at.y + y;
            if(a[y][x] && bx >= box[0] && bx < box[2] && by >= box[1] && by < box[3]) {
               n++;
            }
         }
      }
      return n;
   }

   // ---------------------------------------------------------------- helpers

   /**
    * A PNG as ARGB ints, read straight from the file: the unit tests run on the JVM without
    * Android's decoder. Every sprite here is 8 bit RGBA and the baked boards are 8 bit RGB
    * (read as opaque), none interlaced.
    */
   static int[][] read(String name) throws IOException {
      File file = new File("src/main/res/drawable-nodpi/" + name + ".png");
      DataInputStream in = new DataInputStream(new FileInputStream(file));
      try {
         in.skipBytes(8);
         int width = 0, height = 0, bytes = 4;
         ByteArrayOutputStream data = new ByteArrayOutputStream();
         while(true) {
            int length = in.readInt();
            byte[] type = new byte[4];
            in.readFully(type);
            byte[] body = new byte[length];
            in.readFully(body);
            in.readInt();
            String t = new String(type, "US-ASCII");
            if(t.equals("IHDR")) {
               width = ((body[0] & 255) << 24) | ((body[1] & 255) << 16) | ((body[2] & 255) << 8) | (body[3] & 255);
               height = ((body[4] & 255) << 24) | ((body[5] & 255) << 16) | ((body[6] & 255) << 8) | (body[7] & 255);
               assertEquals(name, 8, body[8]);
               assertTrue(name, body[9] == 6 || body[9] == 2);
               bytes = body[9] == 6 ? 4 : 3;
               assertEquals(name, 0, body[12]);
            }
            else if(t.equals("IDAT")) {
               data.write(body);
            }
            else if(t.equals("IEND")) {
               break;
            }
         }
         Inflater inflater = new Inflater();
         inflater.setInput(data.toByteArray());
         int stride = width * bytes;
         byte[] raw = new byte[(stride + 1) * height];
         try {
            int got = 0;
            while(got < raw.length && !inflater.finished()) {
               got += inflater.inflate(raw, got, raw.length - got);
            }
         }
         catch(java.util.zip.DataFormatException e) {
            throw new IOException(e);
         }
         int[][] out = new int[height][width];
         byte[] prior = new byte[stride];
         byte[] row = new byte[stride];
         for(int y = 0; y < height; y++) {
            int filter = raw[y * (stride + 1)];
            for(int i = 0; i < stride; i++) {
               int x = raw[y * (stride + 1) + 1 + i] & 255;
               int a = i >= bytes ? row[i - bytes] & 255 : 0;
               int b = prior[i] & 255;
               int c = i >= bytes ? prior[i - bytes] & 255 : 0;
               int v;
               switch(filter) {
                  case 1: v = x + a; break;
                  case 2: v = x + b; break;
                  case 3: v = x + ((a + b) >> 1); break;
                  case 4: {
                     int pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
                     v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
                     break;
                  }
                  default: v = x;
               }
               row[i] = (byte) v;
            }
            for(int x = 0; x < width; x++) {
               int alpha = bytes == 4 ? row[x * 4 + 3] & 255 : 255;
               out[y][x] = (alpha << 24) | ((row[x * bytes] & 255) << 16) | ((row[x * bytes + 1] & 255) << 8) | (row[x * bytes + 2] & 255);
            }
            byte[] swap = prior;
            prior = row;
            row = swap;
         }
         return out;
      }
      finally {
         in.close();
      }
   }

   static boolean[][] mask(int[][] image) {
      boolean[][] out = new boolean[image.length][];
      for(int y = 0; y < out.length; y++) {
         out[y] = new boolean[image[y].length];
         for(int x = 0; x < out[y].length; x++) {
            out[y][x] = (image[y][x] >>> 24) >= 128;
         }
      }
      return out;
   }

   /** Pixels both sprites cover, each placed by its top left. */
   private static int overlap(boolean[][] a, Pt aAt, boolean[][] b, Pt bAt) {
      int count = 0;
      for(int y = 0; y < a.length; y++) {
         for(int x = 0; x < a[y].length; x++) {
            if(!a[y][x]) {
               continue;
            }
            int bx = aAt.x + x - bAt.x;
            int by = aAt.y + y - bAt.y;
            if(by >= 0 && by < b.length && bx >= 0 && bx < b[by].length && b[by][bx]) {
               count++;
            }
         }
      }
      return count;
   }
}
