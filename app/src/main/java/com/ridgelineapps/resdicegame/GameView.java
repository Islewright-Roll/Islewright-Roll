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
import java.util.Collections;
import java.util.Comparator;

import android.content.Context;
import android.content.res.Resources;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.os.SystemClock;
import android.util.Log;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.WindowInsets;

/**
 * The game screen. Everything is drawn at one art pixel per pixel into ArtCanvas and
 * blitted once at an integer scale, so the layout below is all in art pixels. From the
 * top: a header (score, on a race map its two bonus badges, menu button), the turn track
 * (a strip of sea with the King's ship, and a flag per finished turn on the island that
 * scores every turn), the cost legend, the island filling
 * whatever height is left, centred in that space (see Voyage.boardOrigin), the six dice, and
 * the roll button. Which island, and which of the two tracks, comes from game.map.
 */
public class GameView extends View {
   // Layout, in art pixels. The top bar is two rows. Row 1 is three boxes side by side, BOX_GAP
   // apart and HUD_INSET in from each side of the HUD: the score tray, the costs tray (two rows
   // of recipe plates), and the menu button, which is the whole third box. Row 2 is the voyage
   // strip, full width, in a frame whose top edge is STRIP_Y. Only the score has a word on it:
   // the pieces on the cost plates and the ship on the strip say what those two are.
   static final int TOP_H = 62;
   static final int BOTTOM_H = 66;
   static final int DICE_GAP = 6;
   // How far in from each side of the HUD the boxes and the strip stand
   static final int HUD_INSET = 6;
   static final int BOX_GAP = 3;
   // The three boxes: their top edge below the bar's top, their height, and their widths. The
   // costs tray is as wide as its two rows of plates need; the score tray is as wide as its
   // widest label ("Punkte", "Puntos") needs; the menu button has what is left.
   static final int BOX_Y = 6;
   static final int BOX_H = 34;
   static final int SCORE_W = 40;
   static final int COSTS_W = 135;
   static final int MENU_W = ArtCanvas.DESIGN_W - 2 * HUD_INSET - 2 * BOX_GAP - SCORE_W - COSTS_W;
   // The voyage strip's frame: its top edge, and the top row of the strip of sea inside it
   static final int STRIP_Y = 43;
   // A race map's two bonuses are on a wooden sign hung from the bar's bottom edge at the
   // right, over the top of the sea: SIGN_DROP rows of rope, then the sign, SIGN_H rows tall,
   // its label SIGN_PAD in from its left and the badges in a strip sunk into it
   static final int SIGN_DROP = 3;
   static final int SIGN_H = 13;
   static final int SIGN_PAD = 3;
   // The cost recipes: two rows of plates, each RECIPE_H tall with a clear pixel between them
   // and a gap between a row's two plates
   static final int RECIPE_H = 14;
   static final int RECIPE_ROW = RECIPE_H + 1;
   static final int RECIPE_GAP = 4;
   // The plates' left and top from the costs tray's own left and top edge
   static final int RECIPE_INSET_X = 3;
   static final int RECIPE_INSET_Y = 3;
   // A buoy's rows below the top of the strip, so it floats with water over and under it
   static final int BUOY_DROP = 4;
   // The undo button is as big as the menu button's tray was; it stands at the HUD's right
   static final int UNDO_SIZE = 16;
   // A bonus badge on the sign: a 7 px glyph, a gap and what the bonus is worth, and the
   // space between the two badges
   static final int BADGE_GLYPH = 7;
   static final int BADGE_GAP_IN = 2;
   static final int BADGE_GAP = 5;
   // How solid a bonus badge's glyph is before the bonus is earned
   static final int BADGE_DIM_ALPHA = 140;

   // How far from its anchor a tap still reaches a piece, in art pixels
   static final int REACH_EDGE = 14;
   // A road is hit anywhere along its length, this close to the line
   static final int REACH_ROAD = 7;
   static final int REACH_TRADER = 20;

   // The UI kit's palette (tools/make_ui.py)
   static final int INK = 0xFF1E140A;
   static final int WOOD_DARK = 0xFF33261A;
   static final int WOOD = 0xFF5A3C14;
   static final int WOOD_LIGHT = 0xFF86622E;
   static final int IVORY = 0xFFFFF4D6;
   static final int PAPER = 0xFFF2E8CC;
   // The tile under each legend glyph: the dark kit's accent (tools/pieces.txt,
   // Palette.DARK.mark), the same colour as the dice faces. The HUD is always the dark wood
   static final int LEGEND_TILE = 0xFFE0B870;
   static final int AMBER = 0xFFE6A817;
   // Numbers on unbuilt spots
   static final int QUIET = 0xFFD6C7A0;
   // The top bar's trays: a face a little darker than the bar's wood; the label above the
   // score and the gold of the frame labels, the gain and the earned bonuses
   static final int TRAY_FACE = 0xFF261B12;
   static final int MUTED = 0xFFD2BE94;
   static final int GOLD = LEGEND_TILE;
   // The menu button: a brown close to the trays', with a lighter top and left edge and a
   // darker bottom and right edge; pressed, the face darkens and the bars drop a pixel
   static final int MENU_FACE = 0xFF4A331C;
   static final int MENU_FACE_DOWN = 0xFF3A2916;
   static final int MENU_LIGHT = 0xFF6E4E2A;
   static final int MENU_DARK = 0xFF2E2012;
   static final int MENU_BAR_W = 11;
   static final int MENU_BAR_H = 2;
   static final int MENU_BAR_PITCH = 4;
   // The shadow of everything not built yet: black at 40%
   static final int FOOTPRINT = Color.argb(102, 0, 0, 0);
   // The glow round a spot that could be built next (whatever the dice say), in the player's
   // colour: a one pixel outline, and outside it an optional fainter ring. Opacity of each,
   // 0 to 255. GLOW_DARK puts a one pixel dark line outside the rings, for a player colour
   // that is lost in the terrain.
   static final int GLOW_ALPHA = 166;
   static final boolean GLOW_RING = true;
   static final int GLOW_RING_ALPHA = 64;
   // The outside ring can instead be a halo that keeps the line readable on any terrain:
   // black round a player colour that is light enough to stand out of the shadow, white
   // round one that is not. Which, by the colour's brightness (0 to 255).
   static final boolean GLOW_HALO = true;
   static final int GLOW_HALO_ALPHA = 64;
   static final int GLOW_HALO_LIGHT_ABOVE = 110;
   // The waiting tier, for a spot whose road is built but whose turn in the order has not
   // come: true draws the outline dashed at full strength (lit for two pixels, unlit for
   // two, along it), false draws it solid at WAITING_FADE of the full glow's opacity
   static final boolean WAITING_DASHED = true;
   static final float WAITING_FADE = 0.5f;
   // The outline round a buildable road is the player's colour with its saturation scaled by
   // this (1 is the full colour, 0 is grey), so the roads' glow is quieter than a building's.
   // Only an outline pixel that grew from roads alone is greyed; one that touches a building
   // keeps the full colour.
   static final float GLOW_ROAD_SATURATION = 0.55f;


   // How solid the piece itself is drawn under an offer's colour
   static final int OFFER_SPRITE_ALPHA = 150;
   // The tutorial's callouts. The panel is the dark UI panel, whose frame is 6 px of corner;
   // its pointer is drawn in code in the panel's own colours (sampled from the art).
   static final int CALLOUT_CORNER = 6;
   // The Got it button: a plate this tall inside its hit area, and the Skip button's underline
   static final int CALLOUT_PLATE_H = 11;
   static final int CALLOUT_PLATE_TOP = 1;
   // Ark Pixel's line box is 14 rows: its capitals and digits are rows 4 to 10 (7 tall), an
   // accented capital starts at row 1, and a descender reaches row 12
   static final int FONT_CAP_ROW = 4;
   static final int FONT_CAPS = 7;
   static final int FONT_TOP_ROW = 1;
   // The numerals font (Micro 5) is for the turn strip's flags only: its digits are rows 3 to 7
   // of its line box, 5 tall
   static final int NUMERAL_ROW = 3;
   static final int NUMERAL_H = 5;
   // A banked resource waiting to be used: a "mini die", a plate this big (a pixel of ink,
   // then a pixel of face, round the 12 px icon, with the corners cut). They stand in a row above the dice at the left, on a tab of the
   // bottom bar's wood that rises over the sea, in bank slot order, closing up as slots are
   // spent. MINI_PER_ROW to a row; a rare seventh starts a second row above.
   static final int MINI_DIE = 16;
   static final int MINI_GAP = 3;
   static final int MINI_PER_ROW = 6;
   // The tab's wood round the dice, and the grid's distance from the screen's left edge
   static final int MINI_PAD = 3;
   static final int MINI_MARGIN = 2;
   // The widest the roll button draws: "Play again" in the display font and its corners
   // (UIDiceRoll widens the button to fit a longer label)
   static final int ROLL_WIDEST = 96;
   // Where an unhired trader's number goes: its line box's top, this far below its anchor.
   // The cart's footprint ends 4 rows under the anchor, and the number's ink starts 3 rows
   // into the line box, so the ink begins 2 rows under the footprint, and the pixel of
   // outline round it 1 row under (any lower and Tarnholm's first two numbers touch the
   // castle box beside them). Ark Pixel's digits start at row 4 of the line box, not 3.
   static final int TRADER_NUMBER_DROP = 3;
   // Offer colours from the bottom layer up, and from the weakest claim on a shared pixel to
   // the strongest: everything that needs another build first, then everything buildable now
   static final int[] OFFER_COLORS = {
      UIEntity.HIGHLIGHT_CHAIN, UIEntity.HIGHLIGHT_KNIGHT_CHAIN, UIEntity.HIGHLIGHT_WILDCARD_CHAIN,
      UIEntity.HIGHLIGHT, UIEntity.HIGHLIGHT_KNIGHT, UIEntity.HIGHLIGHT_WILDCARD,
   };
   // Which family each offer colour belongs to: 0 the greens, 1 the blues
   static final int[] OFFER_FAMILY = { 0, 1, 1, 0, 1, 1 };
   // The part of the board image the footprint and offer layers cover: the island and every
   // sprite that can stand on it
   static final int LAYER_X = 96, LAYER_Y = 96, LAYER_W = 312, LAYER_H = 312;

   Game game;
   MainActivity activity;
   UIDiceRoll uiDiceRoll;

   /** Rectangle entities: the dice and the roll button. */
   ArrayList<UIEntity> entities = new ArrayList<UIEntity>();
   /** Board pieces, hit by nearest anchor. */
   ArrayList<UIEntity> pieces = new ArrayList<UIEntity>();
   ArrayList<UIEntity> roads = new ArrayList<UIEntity>();
   /** Villages, castles and traders from the back of the island to the front. */
   ArrayList<UIEntity> standing = new ArrayList<UIEntity>();
   UIDice[] uiDice;

   final ArtCanvas art = new ArtCanvas();
   BitmapFont fontDisplay;
   BitmapFont fontBody;
   // 0 to 9 and "+" in Micro 5, 5 px tall: the numbers on the turn strip's flags
   BitmapFont fontNumerals;

   Bitmap board;
   Bitmap[] road = new Bitmap[3];
   Bitmap[] roadMask = new Bitmap[3];
   /** Each road sprite's cobbles without its kerb. See drawIsland(). */
   Bitmap[] roadSurface = new Bitmap[3];
   Bitmap village, villageMask;
   Bitmap castle, castleMask;
   Bitmap trader, traderMask;
   Bitmap kingShip;
   /** The dock's above water art (the board has it too), drawn over the roads. See drawIsland(). */
   Bitmap dock;
   Bitmap trackWater, trackBuoy, trackFlag, trackFlagEmpty, trackShip, trackPier;
   Bitmap undoGlyph;
   Bitmap panelDark;
   // The dark panel's own colours, for the callout's pointer: its outermost line, its gold
   // line, and what fills it
   int panelInk, panelGold, panelFill;
   Bitmap glyphRoad, glyphTrader, glyphVillage, glyphCastle;
   // The bonus badges' 7 px glyphs
   Bitmap badgeRoad, badgeTrader;
   Bitmap iconWool, iconGrain, iconBrick, iconOre, iconLumber, iconGold, iconAny;
   // Where each trader stands (TraderLayout), in the map's trader order
   Pt[] traderAt;
   // Per bank slot, where the resource its several traders share rests (TraderLayout.sharedAt),
   // or null for a slot with one trader
   Pt[] sharedAt;

   // Set by layout(): the art position of the board image's top left, and of the 216 wide HUD
   int boardX;
   int boardY;
   int hudX;
   int menuX;
   // The undo button, in the bottom bar level with the roll button, at the menu button's edge
   int undoX;
   int undoY;
   // The top left of the first mini die; the rest follow to its right (see drawMiniDice)
   int miniX;
   int miniY;
   // How many columns of mini dice fit beside the roll button on this canvas
   // The HUD strip's layout for this map
   Voyage.Strip strip;
   // The ship's sail (see drawIsland): the stage drawn last frame (-1 before the first), and
   // the sail now under way, from shipFrom since shipSailStart (uptime ms) for shipSailMs
   int shipStageShown = -1;
   Pt shipFrom;
   long shipSailStart;
   int shipSailMs;
   // Traders hired a moment ago whose animation is still playing (see traderHired)
   private final ArrayList<Hire> hires = new ArrayList<Hire>();
   // The glow round a resource being banked, drawn into this each frame it is up: room for the
   // largest thing it hugs (the mini die's plate) and the two pixels of outline and ring round it
   private static final int HIRE_GLOW_REACH = 2;
   private final Bitmap hireGlow = Bitmap.createBitmap(MINI_DIE + 2 * HIRE_GLOW_REACH, MINI_DIE + 2 * HIRE_GLOW_REACH, Bitmap.Config.ARGB_8888);
   boolean menuDown = false;
   boolean undoDown = false;
   // The tutorial callout as last drawn (null when none is up), and which of its two buttons
   // a touch in progress began on: 0 neither, 1 Got it, 2 Skip tutorial
   private TutorialLayout.Placement callout;
   private int calloutDown;
   // Where the touch in progress began, in screen pixels, and whether it has since strayed
   // further than a tap may (see TapSlop). A touch that did never builds anything.
   /** The view on screen, for the debug build's touch injector (DebugTouchReceiver). */
   static java.lang.ref.WeakReference<GameView> current = new java.lang.ref.WeakReference<GameView>(null);

   // Touches, one per finger, by pointer id: whether it is down, where it went down (screen
   // px), and whether it has strayed too far from there to be a tap
   private static final int MAX_POINTERS = 10;
   private final boolean[] pointerDown = new boolean[MAX_POINTERS];
   private final float[] pointerDownX = new float[MAX_POINTERS];
   private final float[] pointerDownY = new float[MAX_POINTERS];
   private final boolean[] pointerStrayed = new boolean[MAX_POINTERS];
   // The finger that pressed the roll button, so only its sliding off releases the button
   private int rollPointer = -1;
   // Trader resource classic mode: the one finger dragging a mini die (-1 when none), the bank
   // slot it picked up, and where the finger is now, in art px (see touchDrag and drawDrag)
   private int dragPointer = -1;
   private int dragSlot;
   private int dragX;
   private int dragY;
   // The dragged die is drawn this far above the finger, so the fingertip does not hide it
   private static final int DRAG_RAISE = 12;
   // Whether the touch being handled strayed (read by the buttons and the callout)
   private boolean touchStrayed;
   private final float tapSlopPx;
   // Screen pixels of the view that system bars (or an action bar) sit over, and the
   // art pixels the two bars grow by to keep their contents clear of them
   int insetTop;
   int insetBottom;
   int padTop;
   int padBottom;

   // See refreshFootprints() and refreshOffers(): the unbuilt roads, and the unbuilt villages,
   // castles and traders, as two layers that share no pixel
   Bitmap roadFootprints;
   Bitmap standFootprints;
   long footprintKey;
   // The glow layer (see refreshGlow()) and what it was built from
   Bitmap glow;
   long glowBuilt;
   MapDef glowMap;
   int glowColor;
   int[] glowOffers;
   /** Offers of villages, castles and traders, one fill and one rim per colour. */
   Bitmap[] standFill = new Bitmap[OFFER_COLORS.length];
   Bitmap[] standRim = new Bitmap[OFFER_COLORS.length];
   /** Offers of roads, clipped to the ground they would add. */
   Bitmap[] roadFill = new Bitmap[OFFER_COLORS.length];
   Bitmap[] roadRim = new Bitmap[OFFER_COLORS.length];
   int[] offers;
   // What the offer layers were built from besides the offers themselves: the built
   // pieces (roads are clipped by them) and the map
   long offerBuilt;
   MapDef offerMap;
   final java.util.IdentityHashMap<Bitmap, int[]> maskPixels = new java.util.IdentityHashMap<Bitmap, int[]>();
   final Paint stampPaint = ArtCanvas.nearestPaint();
   /** Per resource icon, the empty rows under its art. See cargoY(). */
   final java.util.IdentityHashMap<Bitmap, Integer> cargoDrop = new java.util.IdentityHashMap<Bitmap, Integer>();

   public GameView(Context context, Game game) {
      super(context);
      current = new java.lang.ref.WeakReference<GameView>(this);
      activity = (MainActivity) context;
      this.game = game;

      tapSlopPx = ViewConfiguration.get(context).getScaledTouchSlop() * TapSlop.SLOP_FACTOR;
      Log.d("GameView", "tap slop " + tapSlopPx + " px");

      Resources res = context.getResources();
      fontDisplay = BitmapFont.load(res, R.drawable.font_display, R.raw.font_display);
      fontBody = BitmapFont.load(res, R.drawable.font_body, R.raw.font_body);
      fontNumerals = BitmapFont.load(res, R.drawable.font_numerals, R.raw.font_numerals);

      board = ArtCanvas.load(res, game.map.boardRes);
      road[Board.ROAD_H] = ArtCanvas.load(res, R.drawable.road_h);
      road[Board.ROAD_SLASH] = ArtCanvas.load(res, R.drawable.road_slash);
      road[Board.ROAD_BACKSLASH] = ArtCanvas.load(res, R.drawable.road_backslash);
      for(int i = 0; i < road.length; i++) {
         roadMask[i] = ArtCanvas.mask(road[i]);
         roadSurface[i] = TeamColor.unpainted(road[i], TeamColor.MAGENTA);
      }
      village = ArtCanvas.load(res, Art.PIECE_VILLAGE);
      villageMask = ArtCanvas.mask(village);
      castle = ArtCanvas.load(res, Art.PIECE_CASTLE);
      castleMask = ArtCanvas.mask(castle);
      trader = ArtCanvas.load(res, Art.PIECE_TRADER);
      traderMask = ArtCanvas.mask(trader);
      kingShip = ArtCanvas.load(res, Art.PIECE_KINGSHIP);
      dock = ArtCanvas.load(res, R.drawable.dock);
      trackWater = ArtCanvas.load(res, Art.TRACK_WATER);
      trackBuoy = ArtCanvas.load(res, Art.TRACK_BUOY);
      trackFlag = ArtCanvas.load(res, Art.TRACK_FLAG);
      trackFlagEmpty = ArtCanvas.load(res, Art.TRACK_FLAG_EMPTY);
      trackShip = ArtCanvas.load(res, Art.TRACK_SHIP);
      trackPier = ArtCanvas.load(res, Art.TRACK_PIER);
      // The board has its dock where the start road meets the sea, on every map
      if(!Voyage.coastalEnd(game.map.hex, game.board.roadEnds[0]).equals(Voyage.DOCK_FOOT)) {
         throw new IllegalStateException("The start road does not end at the dock");
      }
      strip = Voyage.Strip.forMap(game.map, ArtCanvas.DESIGN_W);
      undoGlyph = ArtCanvas.load(res, R.drawable.undo_glyph);
      panelDark = ArtCanvas.load(res, R.drawable.ui_panel_dark);
      panelInk = panelDark.getPixel(0, panelDark.getHeight() / 2);
      panelGold = panelDark.getPixel(2, panelDark.getHeight() / 2);
      panelFill = panelDark.getPixel(panelDark.getWidth() / 2, panelDark.getHeight() / 2);
      glyphRoad = ArtCanvas.load(res, R.drawable.glyph_road_12);
      glyphTrader = ArtCanvas.load(res, Art.GLYPH_TRADER);
      glyphVillage = ArtCanvas.load(res, R.drawable.glyph_village_12);
      glyphCastle = ArtCanvas.load(res, Art.GLYPH_CASTLE);
      badgeRoad = ArtCanvas.load(res, R.drawable.badge_road_7);
      badgeTrader = ArtCanvas.load(res, R.drawable.badge_trader_7);
      iconWool = ArtCanvas.load(res, R.drawable.icon_wool_12);
      iconGrain = ArtCanvas.load(res, R.drawable.icon_grain_12);
      iconBrick = ArtCanvas.load(res, R.drawable.icon_brick_12);
      iconOre = ArtCanvas.load(res, R.drawable.icon_ore_12);
      iconLumber = ArtCanvas.load(res, R.drawable.icon_lumber_12);
      iconGold = ArtCanvas.load(res, R.drawable.icon_gold_12);
      iconAny = Bitmap.createBitmap(ArtCanvas.load(res, Art.ICON_ANY_12), Art.ICON_ANY_12_INSET, Art.ICON_ANY_12_INSET, 12, 12);

      traderAt = TraderLayout.anchors(game.map);
      sharedAt = new Pt[game.map.bankResource.length];
      for(int slot = 1; slot < sharedAt.length; slot++) {
         sharedAt[slot] = TraderLayout.sharedAt(game.map, slot);
      }

      UIDice.loadBitmaps(res);
      uiDice = new UIDice[game.dice.length];
      for(int i = 0; i < game.dice.length; i++) {
         uiDice[i] = new UIDice(game, i);
         entities.add(uiDice[i]);
      }

      uiDiceRoll = new UIDiceRoll(game,
            ArtCanvas.load(res, R.drawable.ui_button_normal),
            ArtCanvas.load(res, R.drawable.ui_button_pressed),
            ArtCanvas.load(res, R.drawable.ui_button_disabled),
            fontDisplay);
      entities.add(uiDiceRoll);

      for(int i = 0; i < game.board.roadAnchor.length; i++) {
         Pt p = game.board.roadAnchor[i];
         roads.add(new UIEntity(game, UIEntity.Type.road, i, p.x, p.y, REACH_ROAD));
      }
      for(int i = 0; i < game.board.village.length; i++) {
         Pt p = game.board.village[i];
         standing.add(new UIEntity(game, UIEntity.Type.village, i, p.x, p.y, REACH_EDGE));
      }
      for(int i = 0; i < game.board.city.length; i++) {
         Pt p = game.board.city[i];
         standing.add(new UIEntity(game, UIEntity.Type.city, i, p.x, p.y, REACH_EDGE));
      }
      for(int i = 0; i < traderAt.length; i++) {
         // Trader tiers and their bank slots are 1 based, as in Playsheet
         Pt p = traderAt[i];
         standing.add(new UIEntity(game, UIEntity.Type.knight, i + 1, p.x, p.y, REACH_TRADER));
      }
      pieces.addAll(roads);
      pieces.addAll(standing);
      offers = new int[pieces.size()];
      // No real colour, so the first frame builds the offer layers
      java.util.Arrays.fill(offers, -1);
      // Painter's algorithm: whatever stands further down the island is drawn later
      Collections.sort(standing, new Comparator<UIEntity>() {
         @Override
         public int compare(UIEntity a, UIEntity b) {
            return footY(a) - footY(b);
         }
      });
   }

   private static int footY(UIEntity e) {
      return e.type == UIEntity.Type.knight ? e.anchorY + Art.TRADER_FEET : e.anchorY;
   }

   @Override
   protected void onSizeChanged(int w, int h, int oldw, int oldh) {
      super.onSizeChanged(w, h, oldw, oldh);
      layout();
   }

   /**
    * The window is edge to edge on current Android, so this view runs underneath the
    * system bars. The art still fills all of it; only the layout keeps clear.
    */
   @Override
   public WindowInsets onApplyWindowInsets(WindowInsets insets) {
      insetTop = insets.getSystemWindowInsetTop();
      insetBottom = insets.getSystemWindowInsetBottom();
      layout();
      postInvalidate();
      return insets;
   }

   /** Places everything for the current art size. */
   private void layout() {
      if(getWidth() <= 0 || getHeight() <= 0) {
         return;
      }
      art.resize(getWidth(), getHeight(), insetTop + insetBottom);
      padTop = (insetTop + art.scale - 1) / art.scale;
      padBottom = (insetBottom + art.scale - 1) / art.scale;

      hudX = (art.width - ArtCanvas.DESIGN_W) / 2;
      menuX = hudX + ArtCanvas.DESIGN_W - HUD_INSET - MENU_W;

      // The map, moored ship included, goes in the middle of the space between the two bars
      Pt boardAt = Voyage.boardOrigin(art.width, padTop + TOP_H, art.height - BOTTOM_H - padBottom);
      boardX = boardAt.x;
      boardY = boardAt.y;

      int diceRow = uiDice.length * UIDice.size + (uiDice.length - 1) * DICE_GAP;
      int diceX = (art.width - diceRow) / 2;
      int diceY = art.height - padBottom - BOTTOM_H + 8;
      for(int i = 0; i < uiDice.length; i++) {
         uiDice[i].setPosition(diceX + i * (UIDice.size + DICE_GAP), diceY);
      }
      int rollY = diceY + UIDice.size + 6;
      miniX = hudX + MINI_MARGIN + MINI_PAD;
      // The bottom row of mini dice: just above the bar, on its tab
      miniY = art.height - padBottom - BOTTOM_H - MINI_PAD - MINI_DIE;
      uiDiceRoll.setBounds((art.width - UIDiceRoll.WIDTH) / 2, rollY,
            UIDiceRoll.WIDTH, UIDiceRoll.HEIGHT);
      undoX = undoX(hudX);
      undoY = rollY + (UIDiceRoll.HEIGHT - UNDO_SIZE) / 2;
   }

   @Override
   protected void onDraw(Canvas canvas) {
      if(art.bitmap == null) {
         return;
      }

      // The sea goes on forever: past the board inside the art, and into the few
      // screen pixels the integer scale leaves over outside it.
      canvas.drawColor(game.map.seaFill);
      art.fill(0, 0, art.width, art.height, game.map.seaFill);
      updateHires();
      art.sprite(board, boardX, boardY);

      drawIsland();
      drawTopBar();
      drawBottomBar();
      drawHires();
      drawTutorial();
      drawDrag();

      art.blit(canvas);
      if(!hires.isEmpty()) {
         postInvalidateOnAnimation();
      }
   }

   // ---------------------------------------------------------------- island

   private void drawIsland() {
      Playsheet sheet = game.playsheet;
      int color = game.playerColor;

      // Every road not built yet, as one flat shadow of where it would go. The villages,
      // castles and traders not built yet have their own shadow, drawn later over the roads
      // (see below), and are cut out of this one
      refreshFootprints(sheet);
      art.silhouette(roadFootprints, boardX + LAYER_X, boardY + LAYER_Y, FOOTPRINT);
      // A glow round what could be built next, on the ground with the footprints and under
      // everything built
      refreshGlow(sheet, builtKey(sheet), color);
      art.sprite(glow, boardX + LAYER_X, boardY + LAYER_Y);

      // Roads lie flat, so they all go down before anything stands on them. Every road
      // sprite has a kerb in the player's colour all round it, and where two roads meet
      // one's kerb would lie across the other's cobbles like a wall across the way. So
      // the cobbles of every built road go down a second time, over all the kerbs: a kerb
      // then survives only where no road runs, which is the outside of the network and
      // the round end of a road that leads nowhere yet.
      for(UIEntity e : roads) {
         if(sheet.roads[e.index]) {
            Pt o = game.board.roadOrigin[e.index];
            art.sprite(TeamColor.recolor(road[game.board.roadShape[e.index]], TeamColor.MAGENTA, color), boardX + o.x, boardY + o.y);
         }
      }
      for(UIEntity e : roads) {
         if(sheet.roads[e.index]) {
            Pt o = game.board.roadOrigin[e.index];
            art.sprite(roadSurface[game.board.roadShape[e.index]], boardX + o.x, boardY + o.y);
         }
      }
      // The dock is baked into the board, and its above water art is drawn again here, over
      // both road passes, so the deck lies over the start road's round end and not under it
      art.sprite(dock, boardX + Voyage.DOCK_SPRITE.x, boardY + Voyage.DOCK_SPRITE.y);
      // The start road is the one nobody builds; the board has a dock where it meets the
      // sea, and the King's ship comes to it once it is in sight. The ship lies over the sea
      // and under every offer and building. Its stops on the way in depend on how much sea
      // this screen shows (see Voyage.shipOrigin). Going on one stop (a turn ended), it
      // sails there on the art grid: in from off the screen to the far stop, then on. Any
      // other change (the first frame, an undo, a new game, the flag going up) just puts it there.
      int stage = Voyage.stage(game.map, game.turnsTaken, game.isGameDone(), sheet.getScore());
      Pt at = Voyage.shipOrigin(stage, Voyage.DOCK_MOORING, -boardX);
      long now = SystemClock.uptimeMillis();
      if(stage != shipStageShown) {
         shipSailMs = Voyage.sailMs(shipStageShown, stage);
         shipFrom = Voyage.sailStart(shipStageShown, Voyage.DOCK_MOORING, -boardX);
         shipSailStart = now;
         shipStageShown = stage;
      }
      if(shipSailMs > 0) {
         int elapsed = (int)(now - shipSailStart);
         if(elapsed < shipSailMs) {
            at = Voyage.sailAt(shipFrom, at, elapsed, shipSailMs);
            postInvalidateOnAnimation();
         }
         else {
            shipSailMs = 0;
         }
      }
      if(stage != Voyage.STAGE_NONE) {
         // Its flag is team magenta, flying the player's colour as the strip's ship does
         art.sprite(TeamColor.recolor(kingShip, TeamColor.MAGENTA, game.playerColor), boardX + at.x, boardY + at.y);
      }

      // Offers of roads lie on the ground, so they go over the roads and under every
      // building: a road offer is cut away wherever a built road, a building or the offer
      // of a building already holds the ground, and shows only the new ground
      refreshOffers(sheet, builtKey(sheet));
      for(int i = 0; i < OFFER_COLORS.length; i++) {
         drawOffer(roadFill[i], roadRim[i], OFFER_COLORS[i]);
      }

      // The ground of a village, castle or trader not built yet lies above every road that
      // leads to it, built or not: its shadow goes over the roads and their offers, so a
      // road does not seem to run across the spot. The road offers were cut at its edge
      // and the road footprints cut out of it, so nothing here darkens a pixel twice.
      art.silhouette(standFootprints, boardX + LAYER_X, boardY + LAYER_Y, FOOTPRINT);

      for(UIEntity e : standing) {
         if(e.alreadyBuilt()) {
            art.sprite(sprite(e, color), boardX + originX(e), boardY + originY(e), cartAlpha(e));
         }
      }

      // Then the offers of buildings over those: a ghost of each piece on offer, and one
      // flat wash per offer colour
      for(UIEntity e : standing) {
         if(e.highlightColor() != 0) {
            art.sprite(sprite(e, color), boardX + originX(e), boardY + originY(e), OFFER_SPRITE_ALPHA);
         }
      }
      for(int i = 0; i < OFFER_COLORS.length; i++) {
         drawOffer(standFill[i], standRim[i], OFFER_COLORS[i]);
      }

      // Labels last, above every offer
      for(UIEntity e : standing) {
         drawLabel(e, sheet);
      }
      drawLongRoadMarker(sheet);
      drawSharedResources(sheet);
   }

   /** One offer colour's wash, then its rim at twice the opacity. */
   private void drawOffer(Bitmap fill, Bitmap rim, int argb) {
      if(fill == null) {
         return;
      }
      int rimAlpha = Math.min(255, Color.alpha(argb) * 2);
      art.silhouette(fill, boardX + LAYER_X, boardY + LAYER_Y, argb);
      if(rim != null) {
         art.silhouette(rim, boardX + LAYER_X, boardY + LAYER_Y, (argb & 0xFFFFFF) | (rimAlpha << 24));
      }
   }

   private Bitmap sprite(UIEntity e, int color) {
      Bitmap plain = e.type == UIEntity.Type.village ? village : e.type == UIEntity.Type.city ? castle : trader;
      return TeamColor.recolor(plain, TeamColor.MAGENTA, color);
   }

   private Bitmap mask(UIEntity e) {
      switch(e.type) {
         case road:
            return roadMask[game.board.roadShape[e.index]];
         case village:
            return villageMask;
         case city:
            return castleMask;
         default:
            return traderMask;
      }
   }

   /** Board position of the top left of a piece's sprite. */
   private int originX(UIEntity e) {
      switch(e.type) {
         case road:
            return game.board.roadOrigin[e.index].x;
         case village:
            return e.anchorX - Art.VILLAGE_AX;
         case city:
            return e.anchorX - Art.CASTLE_AX;
         default:
            return e.anchorX - Art.TRADER_AX;
      }
   }

   private int originY(UIEntity e) {
      switch(e.type) {
         case road:
            return game.board.roadOrigin[e.index].y;
         case village:
            return e.anchorY - Art.VILLAGE_AY;
         case city:
            return e.anchorY - Art.CASTLE_AY;
         default:
            return e.anchorY - Art.TRADER_AY;
      }
   }

   private void stamp(Canvas layer, UIEntity e) {
      layer.drawBitmap(mask(e), originX(e) - LAYER_X, originY(e) - LAYER_Y, stampPaint);
   }

   /**
    * Which positions are built, one bit each. The traders go in tier by tier: a count
    * would not tell two sheets apart on a map whose traders can be built in any order.
    * Both the footprint layer and the offer layers depend on it.
    */
   private static long builtKey(Playsheet sheet) {
      long key = 0;
      for(int tier = 1; tier < sheet.traders.length; tier++) {
         key = key * 2 + (sheet.traders[tier] ? 1 : 0);
      }
      for(boolean b : sheet.roads) {
         key = key * 2 + (b ? 1 : 0);
      }
      for(boolean b : sheet.villages) {
         key = key * 2 + (b ? 1 : 0);
      }
      for(boolean b : sheet.cities) {
         key = key * 2 + (b ? 1 : 0);
      }
      return key;
   }

   /**
    * The footprint layers: the solid silhouette of everything not built yet, in two layers.
    * The road layer holds every unbuilt road, minus any pixel an unbuilt village, castle or
    * trader covers; the standing layer holds those buildings' own silhouettes and is drawn
    * after the roads, so a building's ground sits above the roads that lead to it. A pixel
    * is in at most one layer (BoardLayers.cutOut), and each layer is stamped into one mask
    * and drawn once: drawing pieces one by one at 40% would darken twice wherever two
    * overlap, which is every vertex where roads meet. Rebuilt only when something is built.
    */
   private void refreshFootprints(Playsheet sheet) {
      long key = builtKey(sheet);
      if(roadFootprints != null && key == footprintKey) {
         return;
      }
      footprintKey = key;
      boolean[] roads = new boolean[LAYER_W * LAYER_H];
      boolean[] stands = new boolean[roads.length];
      for(UIEntity e : pieces) {
         if(!e.alreadyBuilt()) {
            stampHeld(e.type == UIEntity.Type.road ? roads : stands, e);
         }
      }
      roadFootprints = maskBitmap(BoardLayers.cutOut(roads, stands), roadFootprints);
      standFootprints = maskBitmap(stands, standFootprints);
   }

   /** Opaque white where the mask is set, into the given bitmap if there is one. */
   private static Bitmap maskBitmap(boolean[] mask, Bitmap into) {
      int[] pixels = new int[mask.length];
      for(int p = 0; p < pixels.length; p++) {
         pixels[p] = mask[p] ? 0xFFFFFFFF : 0;
      }
      if(into == null) {
         into = Bitmap.createBitmap(LAYER_W, LAYER_H, Bitmap.Config.ARGB_8888);
      }
      into.setPixels(pixels, 0, LAYER_W, 0, 0, LAYER_W, LAYER_H);
      return into;
   }

   // The glow tiers, weakest to strongest: where two meet on a pixel the stronger owns it.
   // FAINT and WEAK are the traders still to hire on a map that hires them in order, two and
   // one steps behind the next (see GlowFade), solid at a fraction of the full glow.
   static final int GLOW_NONE = 0, GLOW_FAINT = 1, GLOW_WEAK = 2, GLOW_WAITING = 3, GLOW_FULL = 4;

   /**
    * Which glow a spot earns, from the map's own rules with the dice left out. Full: it
    * could be built next if it could be paid for (Playsheet's position and order checks).
    * Waiting: where the map orders its villages or castles, a site a built road has
    * reached that is not next in its order. A site no road has reached, a road that touches
    * nothing built, and anything built get none. Traders need no road: where the map orders
    * them, the ones after the next fade (GlowFade): half the glow, then a fifth, then none.
    */
   private int glowTier(UIEntity e) {
      Playsheet sheet = game.playsheet;
      MapDef map = game.map;
      switch(e.type) {
         case road:
            return sheet.canBuildRoad(e.index) ? GLOW_FULL : GLOW_NONE;
         case village:
            if(sheet.canBuildVillage(e.index)) {
               return GLOW_FULL;
            }
            return !sheet.villages[e.index] && map.orderedVillages && reached(map.villageRoads[e.index]) ? GLOW_WAITING : GLOW_NONE;
         case city:
            if(sheet.canBuildCity(e.index)) {
               return GLOW_FULL;
            }
            return !sheet.cities[e.index] && map.orderedCities && reached(map.cityRoads[e.index]) ? GLOW_WAITING : GLOW_NONE;
         case knight:
            if(sheet.canBuildKnight(e.index)) {
               return GLOW_FULL;
            }
            if(sheet.isTraderBuilt(e.index) || !map.orderedTraders) {
               return GLOW_NONE;
            }
            switch(GlowFade.step(sheet.traders, e.index)) {
               case 0:
                  return GLOW_FULL;
               case 1:
                  return GLOW_WEAK;
               case 2:
                  return GLOW_FAINT;
               default:
                  return GLOW_NONE;
            }
         default:
            return GLOW_NONE;
      }
   }

   /** Whether any one of these roads is built: the map's "any one of" prerequisite. */
   private boolean reached(int[] roadNeeds) {
      for(int need : roadNeeds) {
         if(game.playsheet.roads[need]) {
            return true;
         }
      }
      return false;
   }

   /**
    * The colour of the ring outside a glow's outline: the glow's own colour, or with
    * GLOW_HALO black round a colour light enough to stand out of the shadow and white round
    * one that is not. Shared by the buildable spots' glow and the glow of a banking resource.
    */
   private static int glowRingRgb(int rgb) {
      return GLOW_HALO ? BoardLayers.haloRgb(rgb, GLOW_HALO_LIGHT_ABOVE) : rgb;
   }

   /**
    * The glow: every spot that could be built next and is not on offer right now (an offer
    * has its own rim), flattened into one mask, then a one pixel outline just outside the
    * mask in the player's colour and optionally a fainter ring outside that. One mask means
    * one outline round the union, where two glowing spots meet there is no line between
    * them, and a pixel is never drawn twice. Rebuilt when what is built, the map, the
    * player's colour or the offers change.
    */
   private void refreshGlow(Playsheet sheet, long built, int playerColor) {
      playerColor |= 0xFF000000;
      boolean same = glow != null && glowMap == game.map && glowBuilt == built && glowColor == playerColor;
      int[] now = new int[pieces.size()];
      for(int i = 0; i < now.length; i++) {
         now[i] = pieces.get(i).highlightColor();
         if(glowOffers == null || glowOffers[i] != now[i]) {
            same = false;
         }
      }
      if(same) {
         return;
      }
      glowMap = game.map;
      glowBuilt = built;
      glowColor = playerColor;
      glowOffers = now;
      if(glow == null) {
         glow = Bitmap.createBitmap(LAYER_W, LAYER_H, Bitmap.Config.ARGB_8888);
      }

      int[] dist = new int[LAYER_W * LAYER_H];
      int[] tier = new int[dist.length];
      // Whether every seed a pixel grew from is a road: those outline pixels are greyed
      boolean[] roadOnly = new boolean[dist.length];
      java.util.Arrays.fill(dist, -1);
      for(int i = 0; i < now.length; i++) {
         UIEntity e = pieces.get(i);
         int t = now[i] == 0 && !e.alreadyBuilt() ? glowTier(e) : GLOW_NONE;
         if(t != GLOW_NONE) {
            stampTier(dist, tier, roadOnly, e, t);
         }
      }
      int rgb = playerColor & 0xFFFFFF;
      int roadRgb = BoardLayers.desaturate(rgb, GLOW_ROAD_SATURATION) & 0xFFFFFF;
      int ringRgb = glowRingRgb(rgb);
      int ringAlpha = GLOW_HALO ? GLOW_HALO_ALPHA : GLOW_RING_ALPHA;
      // Distance in steps from the flattened mask: 1 is the outline, 2 the ring or halo. A
      // pixel takes the stronger tier of the pixels it grows from, so where a waiting spot
      // meets a full one the full glow owns the shared outline.
      BoardLayers.grow(dist, tier, roadOnly, LAYER_W, LAYER_H, GLOW_RING || GLOW_HALO ? 2 : 1);
      int[] out = new int[dist.length];
      for(int p = 0; p < dist.length; p++) {
         int d = dist[p];
         if(d != 1 && d != 2) {
            continue;
         }
         float strength = 1f;
         if(tier[p] == GLOW_WEAK) {
            strength = GlowFade.strength(1);
         }
         else if(tier[p] == GLOW_FAINT) {
            strength = GlowFade.strength(2);
         }
         else if(tier[p] == GLOW_WAITING) {
            if(WAITING_DASHED) {
               if((((p % LAYER_W) + (p / LAYER_W)) & 3) >= 2) {
                  continue;
               }
            }
            else {
               strength = WAITING_FADE;
            }
         }
         // The outline of a road is the player's colour drained a little; a building's, and
         // any pixel a building's outline shares with a road's, keeps all of it
         out[p] = BoardLayers.glowArgb(d, roadOnly[p] ? roadRgb : rgb, ringRgb, GLOW_ALPHA, ringAlpha, strength);
      }
      glow.setPixels(out, 0, LAYER_W, 0, 0, LAYER_W, LAYER_H);
   }

   /**
    * The offer layers, a fill and a rim for each colour, in two sets: one for roads, one
    * for villages, castles and traders. The fill is the build plan's colour as is; the rim
    * is the same colour at twice the opacity, so the shape still reads over busy terrain.
    * Everything is flattened first, so two offers never double up where they meet.
    *
    * Buildings: every position offered in a colour is stamped into one mask, and the
    * rim goes round that mask.
    *
    * Roads show only the new ground. Each pixel is owned by the strongest colour that
    * offers it (so where a buildable road meets one that needs a chain, the buildable
    * one wins the shared pixels), and the rim goes round the whole of a family of
    * colours (the greens, the blues), found before anything is cut away. Then every
    * pixel held by a built road (its round end included) or by any building, built or
    * offered or not yet built, is cut from both fill and rim (an unbuilt building's
    * footprint is drawn over the roads, so the offer stops at its edge). What is left starts at the end of
    * what is built and runs outward, with a rim only on its outside.
    *
    * Rebuilt only when the offers change or something is built: the cut depends on both.
    */
   private void refreshOffers(Playsheet sheet, long built) {
      boolean same = offerMap == game.map && offerBuilt == built;
      for(int i = 0; i < pieces.size(); i++) {
         int offer = pieces.get(i).highlightColor();
         if(offer != offers[i]) {
            offers[i] = offer;
            same = false;
         }
      }
      if(same) {
         return;
      }
      offerMap = game.map;
      offerBuilt = built;

      for(int c = 0; c < OFFER_COLORS.length; c++) {
         standFill[c] = null;
         standRim[c] = null;
         roadFill[c] = null;
         roadRim[c] = null;
         Canvas layer = null;
         for(int i = 0; i < pieces.size(); i++) {
            UIEntity e = pieces.get(i);
            if(e.type == UIEntity.Type.road || offers[i] != OFFER_COLORS[c]) {
               continue;
            }
            if(layer == null) {
               standFill[c] = Bitmap.createBitmap(LAYER_W, LAYER_H, Bitmap.Config.ARGB_8888);
               layer = new Canvas(standFill[c]);
            }
            stamp(layer, e);
         }
         if(standFill[c] != null) {
            standRim[c] = ArtCanvas.rim(standFill[c]);
         }
      }

      int[] owner = new int[LAYER_W * LAYER_H];
      java.util.Arrays.fill(owner, -1);
      boolean anyRoad = false;
      for(int i = 0; i < pieces.size(); i++) {
         UIEntity e = pieces.get(i);
         int c = colorIndex(offers[i]);
         if(e.type == UIEntity.Type.road && c >= 0) {
            stampOwner(owner, e, c);
            anyRoad = true;
         }
      }
      if(!anyRoad) {
         return;
      }
      boolean[] held = new boolean[owner.length];
      for(int i = 0; i < pieces.size(); i++) {
         UIEntity e = pieces.get(i);
         if(e.alreadyBuilt() || e.type != UIEntity.Type.road) {
            stampHeld(held, e);
         }
      }

      int[][] fill = new int[OFFER_COLORS.length][];
      int[][] rim = new int[OFFER_COLORS.length][];
      for(int y = 0; y < LAYER_H; y++) {
         for(int x = 0; x < LAYER_W; x++) {
            int p = y * LAYER_W + x;
            int c = owner[p];
            if(c < 0 || held[p]) {
               continue;
            }
            if(fill[c] == null) {
               fill[c] = new int[owner.length];
               rim[c] = new int[owner.length];
            }
            fill[c][p] = 0xFFFFFFFF;
            int family = OFFER_FAMILY[c];
            if(x == 0 || y == 0 || x == LAYER_W - 1 || y == LAYER_H - 1
                  || !inFamily(owner[p - 1], family) || !inFamily(owner[p + 1], family)
                  || !inFamily(owner[p - LAYER_W], family) || !inFamily(owner[p + LAYER_W], family)) {
               rim[c][p] = 0xFFFFFFFF;
            }
         }
      }
      for(int c = 0; c < OFFER_COLORS.length; c++) {
         if(fill[c] != null) {
            roadFill[c] = Bitmap.createBitmap(fill[c], LAYER_W, LAYER_H, Bitmap.Config.ARGB_8888);
            roadRim[c] = Bitmap.createBitmap(rim[c], LAYER_W, LAYER_H, Bitmap.Config.ARGB_8888);
         }
      }
   }

   /** Whether a pixel's owner is one of a family of colours: the greens, or the blues. */
   private static boolean inFamily(int colorIndex, int family) {
      return colorIndex >= 0 && OFFER_FAMILY[colorIndex] == family;
   }

   private static int colorIndex(int argb) {
      for(int c = 0; c < OFFER_COLORS.length; c++) {
         if(OFFER_COLORS[c] == argb) {
            return c;
         }
      }
      return -1;
   }

   private int[] pixelsOf(Bitmap mask) {
      int[] pixels = maskPixels.get(mask);
      if(pixels == null) {
         pixels = new int[mask.getWidth() * mask.getHeight()];
         mask.getPixels(pixels, 0, mask.getWidth(), 0, 0, mask.getWidth(), mask.getHeight());
         maskPixels.put(mask, pixels);
      }
      return pixels;
   }

   /** Marks every pixel of a piece's silhouette as offered in this colour, if no stronger one has it. */
   private void stampOwner(int[] owner, UIEntity e, int c) {
      Bitmap mask = mask(e);
      int[] pixels = pixelsOf(mask);
      int w = mask.getWidth();
      int ox = originX(e) - LAYER_X;
      int oy = originY(e) - LAYER_Y;
      for(int i = 0; i < pixels.length; i++) {
         int x = ox + i % w;
         int y = oy + i / w;
         if(pixels[i] != 0 && x >= 0 && y >= 0 && x < LAYER_W && y < LAYER_H) {
            owner[y * LAYER_W + x] = Math.max(owner[y * LAYER_W + x], c);
         }
      }
   }

   /**
    * Marks every pixel of a piece's silhouette as inside the glow mask, at its tier, and
    * whether it holds roads alone: a pixel any building stamps is not road only, in
    * whatever order they are stamped.
    */
   private void stampTier(int[] dist, int[] tier, boolean[] roadOnly, UIEntity e, int t) {
      boolean isRoad = e.type == UIEntity.Type.road;
      Bitmap mask = mask(e);
      int[] pixels = pixelsOf(mask);
      int w = mask.getWidth();
      int ox = originX(e) - LAYER_X;
      int oy = originY(e) - LAYER_Y;
      for(int i = 0; i < pixels.length; i++) {
         int x = ox + i % w;
         int y = oy + i / w;
         if(pixels[i] != 0 && x >= 0 && y >= 0 && x < LAYER_W && y < LAYER_H) {
            int p = y * LAYER_W + x;
            roadOnly[p] = isRoad && (dist[p] != 0 || roadOnly[p]);
            dist[p] = 0;
            tier[p] = Math.max(tier[p], t);
         }
      }
   }

   /** Marks every pixel of a piece's silhouette as held. */
   private void stampHeld(boolean[] held, UIEntity e) {
      Bitmap mask = mask(e);
      int[] pixels = pixelsOf(mask);
      int w = mask.getWidth();
      int ox = originX(e) - LAYER_X;
      int oy = originY(e) - LAYER_Y;
      for(int i = 0; i < pixels.length; i++) {
         int x = ox + i % w;
         int y = oy + i / w;
         if(pixels[i] != 0 && x >= 0 && y >= 0 && x < LAYER_W && y < LAYER_H) {
            held[y * LAYER_W + x] = true;
         }
      }
   }

   /**
    * What the printed sheet used to say about each spot, without a plate, and for the
    * empty spot only: once something is built there the piece speaks for itself and the
    * label goes, whatever the piece is. An empty village or castle spot carries its
    * points inside its footprint. An empty trader spot carries its tier (where the map
    * builds traders in order, which is also its points) and the resource it will bank.
    * Quiet, so the terrain stays in charge.
    */
   private void drawLabel(UIEntity e, Playsheet sheet) {
      if(e.alreadyBuilt()) {
         return;
      }
      if(e.type == UIEntity.Type.knight) {
         drawTraderLabel(e, sheet);
         return;
      }
      int x = boardX + e.anchorX;
      int y = boardY + e.anchorY;
      boolean isVillage = e.type == UIEntity.Type.village;
      String points = "" + (isVillage ? game.map.villagePoints[e.index] : game.map.cityPoints[e.index]);
      int rise = isVillage ? Art.VILLAGE_LABEL_RISE : Art.CASTLE_LABEL_RISE;
      // The font's ink is rows 4 to 10 of its line box, so its middle row is 7
      int left = x - fontBody.inkWidth(points) / 2;
      int top = y - rise - 7;
      fontBody.drawOutlined(art.canvas, points, left, top, QUIET, INK);
   }

   /**
    * An empty trader spot: the resource it will bank rides on the bed of its cart, as it
    * does on no hired cart (a hired trader's resource goes to the bottom bar, see
    * drawMiniDice), and below the cart its number where the map hires traders in order.
    * A slot that several traders stand behind carries no resource on any cart: it rests
    * between them (drawSharedResources).
    */
   private void drawTraderLabel(UIEntity e, Playsheet sheet) {
      MapDef map = game.map;
      int slot = map.traderBank[e.index - 1];
      if(slot > 0 && sharedAt[slot] == null) {
         Pt anchor = traderAt[e.index - 1];
         Bitmap resource = icon(map.bankResource[slot]);
         art.sprite(resource, cargoX(anchor), cargoY(anchor, resource), 255);
      }
      if(map.orderedTraders) {
         String tier = "" + e.index;
         int left = boardX + e.anchorX - fontBody.inkWidth(tier) / 2;
         // The font's ink is rows 4 to 10 of its line box
         fontBody.drawOutlined(art.canvas, tier, left, boardY + e.anchorY + TRADER_NUMBER_DROP, QUIET, INK);
      }
   }

   /**
    * The resource of each bank slot with several traders (Dunholm's desert pairs), once, at
    * full strength, resting between their carts from the start until the last of them is
    * hired. After that it is in the tray or spent, and the board has none.
    */
   private void drawSharedResources(Playsheet sheet) {
      for(int slot = 1; slot < sharedAt.length; slot++) {
         if(sharedAt[slot] != null && !sharedComplete(slot)) {
            art.sprite(icon(game.map.bankResource[slot]), boardX + sharedAt[slot].x, boardY + sharedAt[slot].y);
         }
      }
   }

   /** Whether every trader behind a bank slot stands. */
   private boolean sharedComplete(int slot) {
      for(int tier : game.map.bankTraders[slot]) {
         if(!game.playsheet.isTraderBuilt(tier)) {
            return false;
         }
      }
      return true;
   }

   /** Whether a point of the board lies on a resting shared resource (a tap there builds nothing). */
   private boolean onSharedResource(int boardPx, int boardPy) {
      for(int slot = 1; slot < sharedAt.length; slot++) {
         Pt at = sharedAt[slot];
         if(at != null && !sharedComplete(slot)
               && boardPx >= at.x - 1 && boardPx < at.x + Art.SHARED_CELL + 1
               && boardPy >= at.y - 1 && boardPy < at.y + Art.SHARED_CELL + 1) {
            return true;
         }
      }
      return false;
   }

   /** Art position of the cargo cell of a trader anchored at this board point. */
   private int cargoX(Pt anchor) {
      return boardX + anchor.x - Art.TRADER_AX + Art.TRADER_CARGO_X;
   }

   /**
    * As cargoX, for an icon: lowered by however many empty rows its art leaves at the
    * bottom of its cell, so that every cargo stands on the bed and none floats above it.
    */
   private int cargoY(Pt anchor, Bitmap resource) {
      Integer drop = cargoDrop.get(resource);
      if(drop == null) {
         drop = ArtCanvas.clearRowsBelow(resource);
         cargoDrop.put(resource, drop);
      }
      return boardY + anchor.y - Art.TRADER_AY + Art.TRADER_CARGO_Y + drop;
   }

   /**
    * The resources a hired trader has banked and nobody has used yet, each a mini die: a
    * small die face in the legend tile's light tone, a pixel of ink round it, and the 12 px
    * icon on it. In bank slot order along the row, so a die never moves but to close up
    * when one before it is spent. Of two traders that share a slot, the die appears when
    * both stand. They sit on a tab of the bar's wood, drawn only when there is one to show.
    */
   private void drawMiniDice(Playsheet sheet) {
      int count = 0;
      for(int slot = 1; slot <= sheet.bankSize(); slot++) {
         if(sheet.canUseKnightResource(slot)) {
            count++;
         }
      }
      if(count == 0) {
         return;
      }
      int pitch = MINI_DIE + MINI_GAP;
      int rows = (count + MINI_PER_ROW - 1) / MINI_PER_ROW;
      int barTop = art.height - padBottom - BOTTOM_H;
      for(int row = 0; row < rows; row++) {
         int inRow = Math.min(MINI_PER_ROW, count - row * MINI_PER_ROW);
         int w = inRow * pitch - MINI_GAP + 2 * MINI_PAD;
         int top = miniY - row * pitch - MINI_PAD;
         // The tab: an ink outline and the bar's wood, with its two top corners cut
         int left = miniX - MINI_PAD - 1;
         int h = barTop - top + 2;
         art.fill(left + 1, top - 1, w, 1, INK);
         art.fill(left, top, w + 2, h - 1, INK);
         art.fill(left + 2, top, w - 2, 1, WOOD_DARK);
         art.fill(left + 1, top + 1, w, h - 2, WOOD_DARK);
      }
      int shown = 0;
      for(int slot = 1; slot <= sheet.bankSize(); slot++) {
         if(!sheet.canUseKnightResource(slot)) {
            continue;
         }
         // A die still on its way in has its place on the tab but is not in it yet, and so
         // does one a finger is dragging
         if(!flying(slot) && !(dragPointer >= 0 && slot == dragSlot)) {
            drawMiniDie(slotX(shown), slotY(shown), game.map.bankResource[slot]);
         }
         shown++;
      }
   }

   /** The top left of the mini die in the nth place (0 based) of the tab. */
   private int slotX(int n) {
      return miniX + (n % MINI_PER_ROW) * (MINI_DIE + MINI_GAP);
   }

   private int slotY(int n) {
      return miniY - (n / MINI_PER_ROW) * (MINI_DIE + MINI_GAP);
   }

   /**
    * One mini die with its top left at (x, y) wherever that is, on the tab or on its way
    * there: the plate, then the 12 px icon on it.
    */
   private void drawMiniDie(int x, int y, Dice.Value resource) {
      drawMiniPlate(x, y);
      art.sprite(icon(resource), x + 2, y + 2 + miniIconDrop(resource));
   }

   /** An ink outline and a face, each with its corner pixels cut to soften the die. */
   private void drawMiniPlate(int x, int y) {
      art.plate(x, y, MINI_DIE, MINI_DIE, INK);
      art.plate(x + 1, y + 1, MINI_DIE - 2, MINI_DIE - 2, LEGEND_TILE);
   }

   /**
    * How far down an icon sits on a mini die so its ink is centred on the face: the 12 px
    * icons are drawn to stand on a cart bed, so not all are centred in their own cell.
    */
   private static int miniIconDrop(Dice.Value resource) {
      if(resource == Dice.Value.Brick) {
         return -2;
      }
      if(resource == Dice.Value.Any) {
         return 1;
      }
      return 0;
   }

   /**
    * The one road that earns the map's long road bonus carries the bonus beside it, in the
    * same quiet ink as the points on an empty village spot, and in amber once it is built
    * and the bonus is won. The amber matches the road badge in the top bar.
    */
   private void drawLongRoadMarker(Playsheet sheet) {
      int road = game.map.longRoad;
      if(road < 0) {
         return;
      }
      String bonus = "+" + game.map.longRoadBonus;
      Pt middle = game.board.roadAnchor[road];
      // Beside the road, not on it: a road is 8 px thick and the marker is wider than that, so
      // centred on it the marker would sit across the road's rim, and across an offer's too.
      // To the left of a slanting road, above a level one.
      int width = fontBody.inkWidth(bonus);
      boolean level = game.board.roadShape[road] == Board.ROAD_H;
      int x = level ? middle.x - width / 2 : middle.x - 2 * Board.ROAD_CAP - 1 - width;
      // The ink is rows 4 to 10 of the line box: centred on the road's middle beside a slanting
      // road, and with its bottom row where the old 5 px digits' was above a level one
      int y = level ? middle.y - 8 - 2 * Board.ROAD_CAP : middle.y - 7;
      fontBody.drawOutlined(art.canvas, bonus, boardX + x, boardY + y, sheet.roads[road] ? AMBER : QUIET, INK);
   }

   Bitmap icon(Dice.Value value) {
      switch(value) {
         case Wool:
            return iconWool;
         case Grain:
            return iconGrain;
         case Brick:
            return iconBrick;
         case Ore:
            return iconOre;
         case Lumber:
            return iconLumber;
         case Gold:
            return iconGold;
         default:
            return iconAny;
      }
   }

   // ---------------------------------------------------------------- bars

   /** Where the undo button's left edge goes: the HUD's right side, 2 px in. */
   static int undoX(int hudX) {
      return hudX + ArtCanvas.DESIGN_W - UNDO_SIZE - 2;
   }

   /** The costs tray (the tutorial points at it), for a HUD at hudX under padTop rows of system bar. */
   static TutorialLayout.Rect costsRect(int hudX, int padTop) {
      return new TutorialLayout.Rect(hudX + HUD_INSET + SCORE_W + BOX_GAP, padTop + BOX_Y, COSTS_W, BOX_H);
   }

   /** The voyage strip's frame (the tutorial points at it). */
   static TutorialLayout.Rect stripRect(int hudX, int padTop) {
      return new TutorialLayout.Rect(hudX + HUD_INSET, padTop + STRIP_Y, ArtCanvas.DESIGN_W - 2 * HUD_INSET, Art.TRACK_H + 2);
   }

   private void drawTopBar() {
      int bottom = padTop + TOP_H;
      art.fill(0, 0, art.width, bottom - 2, WOOD_DARK);
      art.fill(0, bottom - 2, art.width, 1, WOOD);
      art.fill(0, bottom - 1, art.width, 1, INK);

      int top = padTop + BOX_Y;
      drawScore(hudX + HUD_INSET, top);
      drawCosts(costsRect(hudX, padTop).x, top);
      drawMenu(menuX, top);
      drawVoyage(padTop + STRIP_Y + 1);
      if(game.isRace()) {
         drawBonuses();
      }
   }

   /** An inset tray: an ink outline with its four corner pixels cut, and a face darker than the bar. */
   private void tray(int x, int y, int w, int h) {
      art.plate(x, y, w, h, INK);
      art.plate(x + 1, y + 1, w - 2, h - 2, TRAY_FACE);
   }

   /** Text centred across a box of width w starting at x, its ink (not its trailing space) centred. */
   private static void drawCentred(BitmapFont font, Canvas canvas, String text, int x, int w, int lineTop, int argb) {
      font.draw(canvas, text, x + (w - font.inkWidth(text) + 1) / 2, lineTop, argb);
   }

   /**
    * The score tray: its label, the total in the display font, and under it what the turn in
    * progress would score if it ended now (nothing when that is nothing). Each stays where it
    * is whether or not the others show.
    */
   private void drawScore(int x, int y) {
      tray(x, y, SCORE_W, BOX_H);
      drawCentred(fontBody, art.canvas, game.getString(R.string.score), x, SCORE_W, y + 3 - FONT_CAP_ROW, MUTED);
      // The number alone, centred in the tray below its label: a turn's gain is on the turn strip
      int below = 3 + 7 + 1;
      drawCentred(fontDisplay, art.canvas, "" + game.playsheet.getScore(), x, SCORE_W, y + below + (BOX_H - 1 - below - 8) / 2, IVORY);
   }

   /**
    * What each piece costs (Game's own cost tables): four recipes on plates of their own in
    * one block of two rows of the same width, the road and the castle over the village and the
    * trader. On a plate the piece you get
    * stands on a light tile at the left, and what it costs lies in a dark recessed strip.
    */
   private void drawCosts(int x, int y) {
      tray(x, y, COSTS_W, BOX_H);
      int left = x + RECIPE_INSET_X;
      int top = y + RECIPE_INSET_Y;
      drawRecipe(left, top, roadGlyph(), Game.RESOURCES_ROAD);
      // The village's roof and the castle's roofs and banner are team magenta, in the player's colour
      drawRecipe(left + recipeWidth(Game.RESOURCES_ROAD) + RECIPE_GAP, top, TeamColor.recolor(glyphCastle, TeamColor.MAGENTA, game.playerColor), Game.RESOURCES_CITY);
      drawRecipe(left, top + RECIPE_ROW, TeamColor.recolor(glyphVillage, TeamColor.MAGENTA, game.playerColor), LEGEND_VILLAGE);
      drawRecipe(left + recipeWidth(LEGEND_VILLAGE) + RECIPE_GAP, top + RECIPE_ROW, glyphTrader, Game.RESOURCES_KNIGHT);
   }

   /**
    * The village's cost as the key lists it: brick first. The rule's own table (Game) is
    * lumber first and stays as it is; a cost is a set, so only the picture differs.
    */
   static final Dice.Value[] LEGEND_VILLAGE = new Dice.Value[]{ Dice.Value.Brick, Dice.Value.Lumber, Dice.Value.Wool, Dice.Value.Grain };

   /**
    * The menu button, which is the whole third box: an ink outline with its corners cut, a
    * face, a lighter top and left edge and a darker bottom and right edge, and three cream
    * bars. Pressed, the face is darker and the bars are a pixel lower.
    */
   private void drawMenu(int x, int y) {
      int w = MENU_W;
      int h = BOX_H;
      art.plate(x, y, w, h, INK);
      art.plate(x + 1, y + 1, w - 2, h - 2, menuDown ? MENU_FACE_DOWN : MENU_FACE);
      art.fill(x + 2, y + 1, w - 4, 1, MENU_LIGHT);
      art.fill(x + 1, y + 2, 1, h - 5, MENU_LIGHT);
      art.fill(x + 1, y + h - 3, w - 2, 2, MENU_DARK);
      art.fill(x + w - 2, y + 2, 1, h - 5, MENU_DARK);
      int bars = 2 * MENU_BAR_PITCH + MENU_BAR_H;
      int barY = y + (h - bars) / 2 + (menuDown ? 1 : 0);
      for(int i = 0; i < 3; i++) {
         art.fill(x + (w - MENU_BAR_W) / 2, barY + i * MENU_BAR_PITCH, MENU_BAR_W, MENU_BAR_H, IVORY);
      }
   }

   /**
    * A race map's two bonuses, on a wooden sign hung by two ropes from the bar's bottom edge
    * at the right: their label on the wood, and the badges in a dark strip sunk into it, each
    * dim until earned and amber after.
    */
   private void drawBonuses() {
      MapDef map = game.map;
      Playsheet sheet = game.playsheet;
      boolean trader = map.traderBonusAt > 0;
      boolean road = map.longRoad >= 0;
      String label = game.getString(R.string.strip_bonuses);
      int labelW = fontBody.inkWidth(label);
      int stripW = (trader ? badgeWidth(map.traderBonus) : 0) + (road ? badgeWidth(map.longRoadBonus) : 0)
            + (trader && road ? BADGE_GAP : 0) + 4;
      int w = 1 + SIGN_PAD + labelW + SIGN_PAD + stripW + 2 + 1;
      int x = hudX + ArtCanvas.DESIGN_W - HUD_INSET - w;
      int y = padTop + TOP_H + SIGN_DROP;
      art.fill(x + 4, y - SIGN_DROP, 1, SIGN_DROP, MUTED);
      art.fill(x + w - 5, y - SIGN_DROP, 1, SIGN_DROP, MUTED);
      art.plate(x, y, w, SIGN_H, INK);
      art.plate(x + 1, y + 1, w - 2, SIGN_H - 2, WOOD);
      art.fill(x + 2, y + 1, w - 4, 1, WOOD_LIGHT);
      fontBody.draw(art.canvas, label, x + 1 + SIGN_PAD, y + 3 - FONT_CAP_ROW, MUTED);
      int stripX = x + w - 3 - stripW;
      art.fill(stripX, y + 2, stripW, SIGN_H - 4, INK);
      int at = stripX + 2;
      if(road) {
         drawBadge(at, y + 3, badgeRoadGlyph(), map.longRoadBonus, sheet.roads[map.longRoad]);
         at += badgeWidth(map.longRoadBonus) + BADGE_GAP;
      }
      if(trader) {
         drawBadge(at, y + 3, badgeTraderGlyph(), map.traderBonus, sheet.traderCount() >= map.traderBonusAt);
      }
   }

   private int badgeWidth(int bonus) {
      return BADGE_GLYPH + BADGE_GAP_IN + fontBody.inkWidth("+" + bonus);
   }

   /** One bonus badge, its glyph's and its caps' top row at top: the 7 px glyph, and what the bonus is worth. */
   private void drawBadge(int x, int top, Bitmap glyph, int bonus, boolean earned) {
      art.sprite(glyph, x, top, earned ? 255 : BADGE_DIM_ALPHA);
      fontBody.draw(art.canvas, "+" + bonus, x + BADGE_GLYPH + BADGE_GAP_IN, top - FONT_CAP_ROW, earned ? AMBER : WOOD_LIGHT);
   }

   private Bitmap badgeRoadGlyph() {
      return TeamColor.recolor(badgeRoad, TeamColor.MAGENTA, game.playerColor);
   }

   private Bitmap badgeTraderGlyph() {
      return badgeTrader;
   }

   /**
    * The road's glyph in the player's colour, its kerb stones
    * recoloured as the board's roads are. TeamColor keeps it until the colour changes, so a
    * new colour from the Player Colour dialog shows on the next frame.
    */
   private Bitmap roadGlyph() {
      return TeamColor.recolor(glyphRoad, TeamColor.MAGENTA, game.playerColor);
   }


   /**
    * The King's voyage, a strip of sea with a cell per turn and the pier at the end. The
    * ship is in the cell of the turn being played (see Voyage.cell) and lies at the pier from
    * the last turn on, as it does on the board. Turns still to come are buoys in the player's colour, one a
    * turn where the strip has flags and one every second turn where it has none (see
    * Voyage.Strip.marked); a turn that has passed has no buoy. Where the strip has flags (the island that scores
    * every turn) each finished turn is a flag carrying what it scored, or a torn flag when
    * nothing was built; where it has none, the cells behind the ship are bare water. All
    * of it lies in a band of water with a 1 px outline (HUD_INSET in from each side).
    *
    * A map with no turn cap has a window strip (see Voyage.FLAG_WINDOW): the flags of the
    * latest turns, the oldest falling off the left, and the ship in harbour once the target
    * is reached.
    */
   private void drawVoyage(int y) {
      MapDef map = game.map;
      Playsheet sheet = game.playsheet;
      boolean done = game.isGameDone();
      int first;
      int flags;
      int cell;
      if(map.turnCap == 0) {
         first = Voyage.windowStart(game.turnsTaken, done);
         flags = Voyage.windowFlags(game.turnsTaken, done);
         cell = Voyage.windowShipCell(game.turnsTaken, done);
      }
      else {
         boolean reached = game.targetScore > 0 && sheet.getScore() >= game.targetScore;
         first = 0;
         flags = strip.flags ? game.turnsTaken : 0;
         cell = Voyage.cell(map.turnCap, game.turnsTaken, done, reached);
      }
      // The water runs the whole band, inside a dark frame, from the legend's left inset to
      // its right one; the cells, ship and pier stay where Voyage.Strip centres them
      int bandX = hudX + HUD_INSET;
      int bandW = ArtCanvas.DESIGN_W - 2 * HUD_INSET;
      art.frame(bandX, y - 1, bandW, Art.TRACK_H + 2, INK);
      art.tile(trackWater, bandX + 1, y, bandW - 2);
      // The frame's four corner pixels are cut, as the trays' are, and the water's corner with them
      int right = bandX + bandW - 1;
      int bottomRow = y + Art.TRACK_H;
      art.fill(bandX, y - 1, 1, 1, WOOD_DARK);
      art.fill(right, y - 1, 1, 1, WOOD_DARK);
      art.fill(bandX, bottomRow, 1, 1, WOOD_DARK);
      art.fill(right, bottomRow, 1, 1, WOOD_DARK);
      art.fill(bandX + 1, y, 1, 1, INK);
      art.fill(right - 1, y, 1, 1, INK);
      art.fill(bandX + 1, bottomRow - 1, 1, 1, INK);
      art.fill(right - 1, bottomRow - 1, 1, 1, INK);
      int left = hudX + strip.left;
      // Recoloured as the road glyph is; TeamColor keeps it until the colour changes
      Bitmap buoy = TeamColor.recolor(trackBuoy, TeamColor.MAGENTA, game.playerColor);
      for(int i = 0; i < strip.turns; i++) {
         int x = left + strip.cellX(i);
         if(i < flags) {
            drawFlag(x, y, sheet.getTurnScore(first + i + 1));
         }
         else if(i > cell && strip.marked(i)) {
            art.sprite(buoy, left + strip.buoyX(i), y + BUOY_DROP);
         }
      }
      // The ship's and the island's flags are team magenta, in the player's colour
      art.sprite(TeamColor.recolor(trackShip, TeamColor.MAGENTA, game.playerColor), left + strip.shipX(cell), y);
      // The pier goes over the ship lying at it, so its tower is whole
      art.sprite(TeamColor.recolor(trackPier, TeamColor.MAGENTA, game.playerColor), left + strip.pierX(), y);
   }

   /** A finished turn's flag: its score in the field, or a plus where the score is too wide for it. */
   private void drawFlag(int x, int y, int turnScore) {
      if(turnScore <= 0) {
         art.sprite(trackFlagEmpty, x, y);
         return;
      }
      art.sprite(trackFlag, x, y);
      String text = fitted("" + turnScore, "+");
      int ink = fontNumerals.inkWidth(text);
      // The numerals' ink is rows 3 to 7 of their line box: put those in the middle of the field
      int top = y + Art.TRACK_FLAG_FIELD_Y + (Art.TRACK_FLAG_FIELD_H - NUMERAL_H) / 2 - NUMERAL_ROW;
      fontNumerals.draw(art.canvas, text, x + Art.TRACK_FLAG_FIELD_X + (Art.TRACK_FLAG_FIELD_W - ink + 1) / 2, top, INK);
   }

   /** The text if it leaves a clear pixel inside each side of a flag's field, else the fallback. */
   private String fitted(String text, String fallback) {
      return fontNumerals.inkWidth(text) <= Art.TRACK_FLAG_FIELD_W - 2 ? text : fallback;
   }

   // A recipe plate, left to right: the 14 px tile, a pixel of plate, the strip (a pixel
   // of rim, then the 12 px icons a pixel apart, a pixel of rim)
   private static int legendStripWidth(Dice.Value[] cost) {
      return cost.length * 13 + 1;
   }

   private static int recipeWidth(Dice.Value[] cost) {
      return RECIPE_H + 1 + legendStripWidth(cost) + 1;
   }

   private void drawRecipe(int x, int y, Bitmap glyph, Dice.Value[] cost) {
      art.plate(x, y, recipeWidth(cost), RECIPE_H, WOOD);
      // The piece, on a tile as light as the track's scored boxes
      art.plate(x, y, RECIPE_H, RECIPE_H, LEGEND_TILE);
      art.sprite(glyph, x + 1, y + 1);
      // Its cost, sunk into the plate
      int stripX = x + RECIPE_H + 1;
      art.fill(stripX, y + 1, legendStripWidth(cost), RECIPE_H - 2, INK);
      int iconX = stripX + 1;
      for(Dice.Value value : cost) {
         // The brick's ink sits low in its cell (it is drawn to stand on a cart bed): lift it a pixel
         art.sprite(icon(value), iconX, y + 1 + (value == Dice.Value.Brick ? -1 : 0));
         iconX += 13;
      }
   }

   private void drawBottomBar() {
      int top = art.height - padBottom - BOTTOM_H;
      art.fill(0, top, art.width, 1, INK);
      art.fill(0, top + 1, art.width, 1, WOOD);
      art.fill(0, top + 2, art.width, art.height - top - 2, WOOD_DARK);
      for(UIEntity e : entities) {
         e.draw(art);
      }
      drawMiniDice(game.playsheet);
      if(game.canUndo()) {
         if(undoDown) {
            art.plate(undoX, undoY, UNDO_SIZE, UNDO_SIZE, WOOD);
         }
         art.sprite(undoGlyph, undoX, undoY);
      }
   }


   // ---------------------------------------------------------------- hiring

   /** One hire's animation (see HireAnim): the trader, when it began, and what it turned out to be. */
   private static final class Hire {
      final int tier;
      final int slot;
      final long startMs;
      final int epoch;
      /** Whether this hire completes its bank slot, so there is a resource to bank. */
      final boolean completes;
      /** 0 not yet decided, FLIES, or FADES (the cart fades in and nothing else). */
      int mode;
      /** The place on the tab the resource lands in, once known; the animation ends if it moves. */
      int place = -1;

      Hire(int tier, int slot, long startMs, int epoch, boolean completes) {
         this.tier = tier;
         this.slot = slot;
         this.startMs = startMs;
         this.epoch = epoch;
         this.completes = completes;
      }
   }

   private static final int FLIES = 1;
   private static final int FADES = 2;

   /**
    * Game says a trader of this tier has just been hired, delayMs from now (a chain of hires
    * staggers them). Nothing else starts an animation, so a load, a scenario and an undo never
    * play one. Whether a resource flies is decided at the next frame, when the tap has finished
    * (a chain can spend the resource it just banked).
    */
   void traderHired(int tier, int delayMs) {
      MapDef map = game.map;
      int slot = map.traderBank[tier - 1];
      if(slot <= 0) {
         return;
      }
      boolean completes = true;
      for(int t : map.bankTraders[slot]) {
         completes &= game.playsheet.isTraderBuilt(t);
      }
      hires.add(new Hire(tier, slot, SystemClock.uptimeMillis() + delayMs, game.stateEpoch, completes));
      postInvalidateOnAnimation();
   }

   /** Ends every hire animation at once, with the normal static drawing: the app is leaving the screen. */
   void endAnimations() {
      hires.clear();
   }

   @Override
   protected void onDetachedFromWindow() {
      hires.clear();
      super.onDetachedFromWindow();
   }

   @Override
   protected void onWindowVisibilityChanged(int visibility) {
      if(visibility != VISIBLE) {
         hires.clear();
      }
      super.onWindowVisibilityChanged(visibility);
   }

   /**
    * Settles each hire before a frame: decides whether its resource flies, and drops those
    * that are over or stale. Stale is anything that makes the picture wrong: the game replaced
    * (a restart, a new island, a scenario, an undo: Game.stateEpoch), the trader no longer
    * standing, a flying resource spent, or its place on the tab having moved.
    */
   private void updateHires() {
      if(hires.isEmpty()) {
         return;
      }
      long now = SystemClock.uptimeMillis();
      Playsheet sheet = game.playsheet;
      for(int i = hires.size() - 1; i >= 0; i--) {
         Hire h = hires.get(i);
         boolean keep = h.epoch == game.stateEpoch && sheet.isTraderBuilt(h.tier);
         boolean available = sheet.canUseKnightResource(h.slot);
         if(keep && h.mode == 0) {
            h.mode = h.completes && available ? FLIES : FADES;
         }
         if(keep && h.mode == FLIES) {
            int place = placeOf(h.slot);
            keep = available && (h.place < 0 || h.place == place);
            h.place = place;
         }
         if(keep && now - h.startMs >= HireAnim.total(h.mode == FLIES)) {
            keep = false;
         }
         if(!keep) {
            hires.remove(i);
         }
      }
   }

   /** The place (0 based) a bank slot's die takes on the tab: how many usable slots come before it. */
   private int placeOf(int slot) {
      int place = 0;
      for(int s = 1; s < slot; s++) {
         if(game.playsheet.canUseKnightResource(s)) {
            place++;
         }
      }
      return place;
   }

   /** Whether a resource of this bank slot is on its way to the tab. */
   private boolean flying(int slot) {
      for(Hire h : hires) {
         if(h.slot == slot && h.mode == FLIES) {
            return true;
         }
      }
      return false;
   }

   private int flyingCount() {
      int count = 0;
      for(Hire h : hires) {
         if(h.mode == FLIES) {
            count++;
         }
      }
      return count;
   }

   /** The opacity a standing cart is drawn at: full, or whatever its hire's fade has reached. */
   private int cartAlpha(UIEntity e) {
      if(e.type == UIEntity.Type.knight) {
         for(Hire h : hires) {
            if(h.tier == e.index) {
               return HireAnim.cartAlpha((int)(SystemClock.uptimeMillis() - h.startMs));
            }
         }
      }
      return 255;
   }

   /**
    * The resource of each hire in progress, over everything: on the bed while the cart fades
    * in, then (when it flies) lifted with its glow, the plate behind it, on its way across,
    * set down in its place, and the glow fading. The last frame is the static drawing's.
    */
   private void drawHires() {
      long now = SystemClock.uptimeMillis();
      for(Hire h : hires) {
         int elapsed = (int)(now - h.startMs);
         Dice.Value resource = game.map.bankResource[h.slot];
         Bitmap icon = icon(resource);
         Pt anchor = traderAt[h.tier - 1];
         Pt from = new Pt(cargoX(anchor), cargoY(anchor, icon));
         boolean shared = sharedAt[h.slot] != null;
         if(shared) {
            // It rests between the carts, not on this one's bed
            from = new Pt(boardX + sharedAt[h.slot].x, boardY + sharedAt[h.slot].y);
         }
         if(h.mode != FLIES) {
            // The first trader of a pair banks nothing, and the shared resource stays where it is
            int alpha = shared && !h.completes ? 0 : HireAnim.iconAlpha(elapsed, false);
            if(alpha > 0) {
               art.sprite(icon, from.x, from.y, alpha);
            }
            continue;
         }
         int drop = miniIconDrop(resource);
         Pt to = new Pt(slotX(h.place) + 2, slotY(h.place) + 2 + drop);
         Pt at = HireAnim.iconAt(from, to, elapsed);
         float strength = HireAnim.glow(elapsed);
         // The tint: the player's colour taken toward white, outline and ring as the buildable spots' glow
         int tint = BoardLayers.lighten(game.playerColor | 0xFF000000, HireAnim.GLOW_LIGHTEN) & 0xFFFFFF;
         if(HireAnim.plateShown(elapsed)) {
            if(strength > 0) {
               drawHireGlow(null, at.x - 2, at.y - 2 - drop, tint, strength);
            }
            drawMiniDie(at.x - 2, at.y - 2 - drop, resource);
         }
         else {
            if(strength > 0) {
               drawHireGlow(icon, at.x, at.y, tint, strength);
            }
            art.sprite(icon, at.x, at.y);
         }
      }
   }

   /**
    * The glow of a resource being banked, at the top left of what it hugs: the icon (solid
    * pixels of the sprite) or, when the sprite is null, the mini die's plate. The same
    * outline-and-ring maker as the glow round buildable spots (BoardLayers.grow), in a lighter colour.
    */
   private void drawHireGlow(Bitmap icon, int x, int y, int rgb, float strength) {
      int side = MINI_DIE + 2 * HIRE_GLOW_REACH;
      boolean[] mask = new boolean[side * side];
      int[] pixels = icon == null ? null : pixelsOf(icon);
      int size = icon == null ? MINI_DIE : icon.getWidth();
      for(int py = 0; py < size; py++) {
         for(int px = 0; px < size; px++) {
            boolean solid = icon == null ? HireAnim.plateHas(px, py, MINI_DIE) : (pixels[py * size + px] >>> 24) >= 128;
            mask[(py + HIRE_GLOW_REACH) * side + px + HIRE_GLOW_REACH] = solid;
         }
      }
      int ringAlpha = GLOW_HALO ? GLOW_HALO_ALPHA : GLOW_RING_ALPHA;
      int[] out = BoardLayers.outline(mask, side, side, rgb, glowRingRgb(rgb), GLOW_ALPHA, ringAlpha, strength);
      hireGlow.setPixels(out, 0, side, 0, 0, side, side);
      art.sprite(hireGlow, x - HIRE_GLOW_REACH, y - HIRE_GLOW_REACH);
   }

   // ---------------------------------------------------------------- tutorial

   /**
    * The tutorial callout, over everything. It looks at the game, asks the Tutorial which
    * step is due, and draws that step's panel beside the thing it describes (see
    * TutorialLayout). Nothing is drawn while a dialog is open, or while a debug scenario
    * holds the game. Taps never stop at the panel except on its two buttons (touchCallout).
    */
   private void drawTutorial() {
      callout = null;
      Tutorial tutorial = activity.tutorial;
      if(tutorial == null) {
         return;
      }
      if(game.tutorialPaused) {
         tutorial.forget();
         return;
      }
      Tutorial.Step step = tutorial.update(tutorialSnapshot());
      if(tutorial.takeDirty()) {
         activity.saveTutorial();
      }
      if(step == null) {
         return;
      }
      if(PixelDialog.isOpen() || MainActivity.welcomePending) {
         // Back when the dialog is gone: this view is not told that, so look again soon
         postInvalidateDelayed(250);
         return;
      }
      TutorialLayout.Rect target = tutorialTarget(step);
      if(target == null) {
         return;
      }
      Resources res = getResources();
      String gotIt = res.getString(R.string.tutorial_got_it);
      String skip = res.getString(R.string.tutorial_skip);
      TutorialLayout.Placement p = TutorialLayout.place(target, tutorialText(step), gotIt, skip,
            art.width, art.height, padTop, art.height - padBottom, new TutorialLayout.Measure() {
               @Override
               public int width(String text) {
                  return fontBody.inkWidth(text);
               }
            });
      if(p == null) {
         return;
      }
      callout = p;

      // The eye finds the target by a one pixel outline in the player's colour
      art.frame(target.x - TutorialLayout.OUTLINE, target.y - TutorialLayout.OUTLINE,
            target.w + 2 * TutorialLayout.OUTLINE, target.h + 2 * TutorialLayout.OUTLINE, game.playerColor | 0xFF000000);

      TutorialLayout.Rect panel = p.panel;
      art.nineSlice(panelDark, panel.x, panel.y, panel.w, panel.h, CALLOUT_CORNER);
      drawPointer(p);
      int lineTop = p.textY - FONT_TOP_ROW;
      for(String line : p.lines) {
         fontBody.draw(art.canvas, line, p.textX, lineTop, IVORY);
         lineTop += TutorialLayout.LINE_PITCH;
      }

      // Got it, on a plate; Skip tutorial, underlined and quieter. The hit areas are the
      // whole 14 px rows, a little wider than the labels.
      TutorialLayout.Rect g = p.gotIt;
      boolean gotDown = calloutDown == 1;
      art.plate(g.x, g.y + CALLOUT_PLATE_TOP, g.w, CALLOUT_PLATE_H, INK);
      art.plate(g.x + 1, g.y + CALLOUT_PLATE_TOP + 1, g.w - 2, CALLOUT_PLATE_H - 2, gotDown ? WOOD_DARK : WOOD);
      fontBody.draw(art.canvas, gotIt, p.gotItLabelX, g.y + CALLOUT_PLATE_TOP + (CALLOUT_PLATE_H - FONT_CAPS) / 2 - FONT_CAP_ROW + (gotDown ? 1 : 0), IVORY);
      TutorialLayout.Rect k = p.skip;
      int skipInk = fontBody.inkWidth(skip);
      int skipColor = calloutDown == 2 ? IVORY : QUIET;
      int skipTop = k.y + CALLOUT_PLATE_TOP + (CALLOUT_PLATE_H - FONT_CAPS) / 2 - FONT_CAP_ROW;
      fontBody.draw(art.canvas, skip, p.skipLabelX, skipTop, skipColor);
      art.fill(p.skipLabelX, skipTop + FONT_CAP_ROW + FONT_CAPS + 2, skipInk, 1, skipColor);
   }

   /**
    * The callout's pointer: a triangle five pixels deep on the panel's edge that faces the
    * target, in the panel's own outline, gold line and fill. Its base opens the panel's frame
    * so the fill runs into it and the gold line turns the corner. u counts outward from the
    * panel's edge (negative inside it) and v across.
    */
   private void drawPointer(TutorialLayout.Placement p) {
      int half = TutorialLayout.HALF_BASE;
      for(int u = -4; u < TutorialLayout.POINTER; u++) {
         int hw = u < 0 ? half - 1 : half - u;
         for(int v = -hw; v <= hw; v++) {
            int color;
            if(u < 0) {
               // Inside the frame: fill, with a gold wall down each side until the frame's own gold line
               color = (Math.abs(v) == hw && u >= -3) ? panelGold : panelFill;
            }
            else if(Math.abs(v) == hw) {
               color = panelInk;
            }
            else if(Math.abs(v) == hw - 1) {
               color = panelGold;
            }
            else {
               color = panelFill;
            }
            int x;
            int y;
            switch(p.side) {
               case ABOVE:
                  x = p.baseX + v;
                  y = p.baseY + u;
                  break;
               case BELOW:
                  x = p.baseX + v;
                  y = p.baseY - u;
                  break;
               case LEFT:
                  x = p.baseX + u;
                  y = p.baseY + v;
                  break;
               default:
                  x = p.baseX - u;
                  y = p.baseY + v;
                  break;
            }
            art.fill(x, y, 1, 1, color);
         }
      }
   }

   /** What the tutorial needs to know about the game right now. */
   private Tutorial.Snapshot tutorialSnapshot() {
      Tutorial.Snapshot s = new Tutorial.Snapshot();
      s.rolls = game.rolls;
      s.turnsTaken = game.turnsTaken;
      for(Dice die : game.dice) {
         s.anyHeld |= die.isHeld();
         if(die.isUsable() && die.getValue() == Dice.Value.Gold) {
            s.goldDice++;
         }
      }
      for(UIEntity piece : pieces) {
         if(piece.highlightColor() != 0) {
            s.offered = true;
            break;
         }
      }
      s.builtThisTurn = game.builtResourceThisTurn;
      s.canUndo = game.canUndo();
      s.gameOver = game.isGameDone();
      s.rollEndsTurn = !game.canRoll() && !s.gameOver;
      // A die still on its way to the tab is not banked yet
      s.miniDice = miniDiceCount(game.playsheet) - flyingCount();
      s.classicBank = game.traderClassic;
      return s;
   }

   private static int miniDiceCount(Playsheet sheet) {
      int count = 0;
      for(int slot = 1; slot <= sheet.bankSize(); slot++) {
         if(sheet.canUseKnightResource(slot)) {
            count++;
         }
      }
      return count;
   }

   private String tutorialText(Tutorial.Step step) {
      Resources res = getResources();
      switch(step) {
         case ROLL:
            return res.getString(R.string.tutorial_roll);
         case KEEP:
            return res.getString(R.string.tutorial_keep);
         case COSTS:
            return res.getString(R.string.tutorial_costs);
         case BUILD:
            return res.getString(R.string.tutorial_build);
         case UNDO:
            return res.getString(R.string.tutorial_undo);
         case END:
            return res.getString(R.string.tutorial_end);
         case SHIP:
            return game.isRace()
                  ? res.getString(R.string.tutorial_ship_race, game.targetScore)
                  : res.getString(R.string.tutorial_ship_score);
         case GOLD:
            return res.getString(R.string.tutorial_gold);
         case BANK_CLASSIC:
            return res.getString(R.string.tutorial_bank_classic);
         default:
            return res.getString(R.string.tutorial_bank);
      }
   }

   /** The rectangle a step's callout describes, in art pixels, or null when it is not on screen. */
   private TutorialLayout.Rect tutorialTarget(Tutorial.Step step) {
      switch(step) {
         case ROLL:
         case END: {
            int[] b = uiDiceRoll.drawnBounds();
            return new TutorialLayout.Rect(b[0], b[1], b[2], b[3]);
         }
         case KEEP: {
            UIDice first = uiDice[0];
            UIDice last = uiDice[uiDice.length - 1];
            return new TutorialLayout.Rect(first.x1, first.y1, last.x2 - first.x1, first.y2 - first.y1);
         }
         case COSTS:
            return costsRect(hudX, padTop);
         case BUILD:
            return offeredSpot();
         case UNDO:
            return new TutorialLayout.Rect(undoX, undoY, UNDO_SIZE, UNDO_SIZE);
         case SHIP:
            return stripRect(hudX, padTop);
         case GOLD:
            for(UIDice die : uiDice) {
               if(die.dice.isUsable() && die.dice.getValue() == Dice.Value.Gold) {
                  return new TutorialLayout.Rect(die.x1, die.y1, die.x2 - die.x1, die.y2 - die.y1);
               }
            }
            return null;
         default: {
            int count = miniDiceCount(game.playsheet) - flyingCount();
            if(count == 0) {
               return null;
            }
            int pitch = MINI_DIE + MINI_GAP;
            int rows = (count + MINI_PER_ROW - 1) / MINI_PER_ROW;
            int inRow = Math.min(MINI_PER_ROW, count);
            return new TutorialLayout.Rect(miniX, miniY - (rows - 1) * pitch, inRow * pitch - MINI_GAP, rows * pitch - MINI_GAP);
         }
      }
   }

   /**
    * One spot to point at: a building that could be built right now, else a road that could,
    * else anything on offer (it needs another build first). The same spot every frame for the
    * same game, so the callout does not hop.
    */
   private TutorialLayout.Rect offeredSpot() {
      UIEntity best = null;
      int bestRank = 0;
      for(UIEntity e : standing) {
         int rank = offerRank(e, 3);
         if(rank > bestRank) {
            best = e;
            bestRank = rank;
         }
      }
      for(UIEntity e : roads) {
         int rank = offerRank(e, 2);
         if(rank > bestRank) {
            best = e;
            bestRank = rank;
         }
      }
      if(best == null) {
         return null;
      }
      Bitmap sprite = best.type == UIEntity.Type.road ? road[game.board.roadShape[best.index]] : sprite(best, game.playerColor);
      return new TutorialLayout.Rect(boardX + originX(best), boardY + originY(best), sprite.getWidth(), sprite.getHeight());
   }

   /** 0 when nothing is on offer here; otherwise buildable now beats chained, and the kind breaks ties. */
   private static int offerRank(UIEntity e, int kind) {
      int color = e.highlightColor();
      if(color == 0) {
         return 0;
      }
      boolean now = color == UIEntity.HIGHLIGHT || color == UIEntity.HIGHLIGHT_KNIGHT || color == UIEntity.HIGHLIGHT_WILDCARD;
      return (now ? 10 : 0) + kind;
   }

   /** The two buttons of the callout: a press on one is kept and acts on the lift. */
   private boolean touchCallout(int x, int y, int action) {
      TutorialLayout.Placement p = callout;
      if(action == MotionEvent.ACTION_DOWN) {
         int hit = p == null ? 0 : p.gotIt.contains(x, y) ? 1 : p.skip.contains(x, y) ? 2 : 0;
         if(hit == 0) {
            return false;
         }
         calloutDown = hit;
         Haptics.tap(this);
         Sounds.play(Sfx.CLICK);
         postInvalidate();
         return true;
      }
      if(calloutDown != 0 && (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL)) {
         int which = calloutDown;
         calloutDown = 0;
         TutorialLayout.Rect button = p == null ? null : which == 1 ? p.gotIt : p.skip;
         if(action == MotionEvent.ACTION_UP && button != null && button.contains(x, y) && !touchStrayed) {
            Tutorial tutorial = activity.tutorial;
            if(which == 1) {
               tutorial.gotIt();
            }
            else {
               tutorial.skip();
            }
            if(tutorial.takeDirty()) {
               activity.saveTutorial();
            }
         }
         postInvalidate();
         return true;
      }
      return false;
   }

   // ---------------------------------------------------------------- touch

   @Override
   public boolean onTouchEvent(MotionEvent event) {
      // Every finger is a touch of its own: two dice can be held at once, or a die held
      // while another finger builds. Each is followed by its pointer id from its own down
      // to its own lift, with its own "strayed too far to be a tap" flag.
      int masked = event.getActionMasked();
      if(masked == MotionEvent.ACTION_MOVE) {
         for(int i = 0; i < event.getPointerCount(); i++) {
            int id = event.getPointerId(i);
            if(id < MAX_POINTERS && pointerDown[id] && !pointerStrayed[id]
                  && !TapSlop.isTap(pointerDownX[id], pointerDownY[id], event.getX(i), event.getY(i), tapSlopPx)) {
               pointerStrayed[id] = true;
            }
            if(id == rollPointer && uiDiceRoll.down && !uiDiceRoll.isWithin(art.toArtX(event.getX(i)), art.toArtY(event.getY(i)))) {
               uiDiceRoll.down = false;
               postInvalidate();
            }
            if(id == dragPointer) {
               dragX = art.toArtX(event.getX(i));
               dragY = art.toArtY(event.getY(i));
               postInvalidate();
            }
         }
         return true;
      }
      if(masked == MotionEvent.ACTION_CANCEL) {
         // The gesture was taken away: every finger still down lifts as a lost tap
         for(int i = 0; i < event.getPointerCount(); i++) {
            int id = event.getPointerId(i);
            if(id < MAX_POINTERS && pointerDown[id]) {
               pointerDown[id] = false;
               touchStrayed = true;
               touchAt(art.toArtX(event.getX(i)), art.toArtY(event.getY(i)), MotionEvent.ACTION_CANCEL, id);
            }
         }
         return true;
      }
      boolean down = masked == MotionEvent.ACTION_DOWN || masked == MotionEvent.ACTION_POINTER_DOWN;
      boolean up = masked == MotionEvent.ACTION_UP || masked == MotionEvent.ACTION_POINTER_UP;
      if(!down && !up) {
         return true;
      }
      int index = event.getActionIndex();
      int id = event.getPointerId(index);
      if(id >= MAX_POINTERS) {
         return true;
      }
      if(down) {
         pointerDown[id] = true;
         pointerDownX[id] = event.getX(index);
         pointerDownY[id] = event.getY(index);
         pointerStrayed[id] = false;
      }
      else {
         if(!pointerDown[id]) {
            return true;
         }
         pointerDown[id] = false;
         if(!pointerStrayed[id] && !TapSlop.isTap(pointerDownX[id], pointerDownY[id], event.getX(index), event.getY(index), tapSlopPx)) {
            pointerStrayed[id] = true;
         }
      }
      // The buttons and pieces read this for the touch being handled
      touchStrayed = pointerStrayed[id];
      touchAt(art.toArtX(event.getX(index)), art.toArtY(event.getY(index)), down ? MotionEvent.ACTION_DOWN : MotionEvent.ACTION_UP, id);
      return true;
   }

   /** One finger going down, lifting, or being taken away, at an art pixel. */
   private void touchAt(int x, int y, int action, int pointerId) {
      // The dragging finger's lift goes to the drag alone. It never pressed a callout button
      // or the menu, so none of them is waiting for it, and it must not end another finger's
      // press on one.
      if(pointerId == dragPointer) {
         endDrag(x, y, action);
         return;
      }
      if(touchCallout(x, y, action) || touchMenu(x, y, action) || touchUndo(x, y, action)) {
         return;
      }
      // A finger down on a banked mini die in classic mode picks it up, and is nothing else:
      // not a tap on a die or the roll button beside the tab
      if(action == MotionEvent.ACTION_DOWN && startDrag(x, y, pointerId)) {
         return;
      }
      for(UIEntity e : entities) {
         if(e.isWithin(x, y)) {
            if(e == uiDiceRoll && action == MotionEvent.ACTION_DOWN) {
               rollPointer = pointerId;
            }
            e.touch(action);
            return;
         }
      }
      // A piece acts on the lift. A touch that travelled too far, or was taken away,
      // was not a tap on this spot and builds nothing
      boolean lostTap = action == MotionEvent.ACTION_CANCEL || (action == MotionEvent.ACTION_UP && touchStrayed);
      if(!lostTap && y >= padTop + TOP_H && y < art.height - padBottom - BOTTOM_H) {
         UIEntity piece = pieceAt(x - boardX, y - boardY);
         if(piece != null) {
            piece.touch(action);
         }
      }
   }

   // ---------------------------------------------------------------- trader resource classic mode

   /**
    * Picks up the mini die under (x, y) if this finger may drag it: classic mode, after a
    * roll, the game not over, no other drag going, and a die that is on the tab rather than
    * still on its way there. Returns whether it did.
    */
   private boolean startDrag(int x, int y, int pointerId) {
      if(!game.traderClassic || game.rolls < 1 || game.isGameDone() || dragPointer >= 0) {
         return false;
      }
      int slot = miniSlotAt(x, y);
      if(slot == 0 || flying(slot)) {
         return false;
      }
      dragPointer = pointerId;
      dragSlot = slot;
      dragX = x;
      dragY = y;
      Haptics.tap(this);
      postInvalidate();
      return true;
   }

   /** The bank slot of the mini die drawn at (x, y), or 0. As drawMiniDice lays them out. */
   private int miniSlotAt(int x, int y) {
      int shown = 0;
      for(int slot = 1; slot <= game.playsheet.bankSize(); slot++) {
         if(!game.playsheet.canUseKnightResource(slot)) {
            continue;
         }
         int left = slotX(shown);
         int top = slotY(shown);
         // A pixel of slack round each die, half the gap to the next
         if(x >= left - 1 && x < left + MINI_DIE + 1 && y >= top - 1 && y < top + MINI_DIE + 1) {
            return slot;
         }
         shown++;
      }
      return 0;
   }

   /** The die a drag lifted at (x, y) would land on: one showing, not spent and not locked. */
   private int dropDieAt(int x, int y, int slot) {
      for(UIDice die : uiDice) {
         if(die.isWithin(x, y) && game.canOverwriteDie(die.index, slot)) {
            return die.index;
         }
      }
      return -1;
   }

   /**
    * The dragging finger lifted or was taken away. Lifted over a die it may overwrite, the
    * die turns to the resource (a "?" first asks which one); anywhere else, or taken away,
    * the mini die simply goes back to its place on the tab.
    */
   private void endDrag(int x, int y, int action) {
      final int slot = dragSlot;
      dragPointer = -1;
      dragSlot = 0;
      postInvalidate();
      if(action != MotionEvent.ACTION_UP) {
         return;
      }
      final int die = dropDieAt(x, y, slot);
      if(die < 0) {
         return;
      }
      Dice.Value banked = game.playsheet.getKnightResource(slot);
      if(banked != Dice.Value.Any) {
         game.overwriteDie(die, slot, banked);
         Haptics.confirm(this);
         return;
      }
      final Dice.Value[] choices = { Dice.Value.Wool, Dice.Value.Grain, Dice.Value.Brick, Dice.Value.Ore, Dice.Value.Lumber };
      int[] labels = { R.string.resource_wool, R.string.resource_grain, R.string.resource_brick, R.string.resource_ore, R.string.resource_lumber };
      // Cancelling the question (back, or a tap outside) leaves the die and the bank as they were
      PixelDialog.choose(activity, R.string.trader_choose_title, labels, new PixelDialog.OnChoice() {
         @Override
         public void onChoice(int which) {
            if(game.canOverwriteDie(die, slot)) {
               game.overwriteDie(die, slot, choices[which]);
               Haptics.confirm(GameView.this);
            }
         }
      });
   }

   /**
    * The mini die a finger is dragging, drawn over everything at the finger and a little
    * above it, and a ring in the player's colour round the die it would land on.
    */
   private void drawDrag() {
      if(dragPointer < 0) {
         return;
      }
      if(!game.playsheet.canUseKnightResource(dragSlot)) {
         // The game moved under the drag (an undo, a restart): there is nothing to carry
         dragPointer = -1;
         dragSlot = 0;
         return;
      }
      int target = dropDieAt(dragX, dragY, dragSlot);
      if(target >= 0) {
         UIDice die = uiDice[target];
         int ring = 0xFF000000 | game.playerColor;
         int w = die.x2 - die.x1;
         int h = die.y2 - die.y1;
         art.fill(die.x1 - 1, die.y1 - 1, w + 2, 1, ring);
         art.fill(die.x1 - 1, die.y2, w + 2, 1, ring);
         art.fill(die.x1 - 1, die.y1, 1, h, ring);
         art.fill(die.x2, die.y1, 1, h, ring);
      }
      drawMiniDie(dragX - MINI_DIE / 2, dragY - MINI_DIE / 2 - DRAG_RAISE, game.map.bankResource[dragSlot]);
   }

   private boolean touchMenu(int x, int y, int action) {
      // The whole third box is the button, and a couple of pixels round it
      boolean inside = x >= menuX - 2 && x < menuX + MENU_W + 2 && y >= padTop && y < padTop + BOX_Y + BOX_H + 2;
      if(action == MotionEvent.ACTION_DOWN && inside) {
         menuDown = true;
         Haptics.tap(this);
         Sounds.play(Sfx.CLICK);
         postInvalidate();
         return true;
      }
      if(menuDown && (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL)) {
         menuDown = false;
         postInvalidate();
         if(action == MotionEvent.ACTION_UP && inside) {
            activity.showMenu();
         }
         return true;
      }
      return false;
   }

   private boolean touchUndo(int x, int y, int action) {
      boolean inside = x >= undoX - 2 && x < undoX + UNDO_SIZE + 2 && y >= undoY - 2 && y < undoY + UNDO_SIZE + 2;
      if(action == MotionEvent.ACTION_DOWN && inside && game.canUndo()) {
         undoDown = true;
         postInvalidate();
         return true;
      }
      if(undoDown && (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL)) {
         undoDown = false;
         postInvalidate();
         if(action == MotionEvent.ACTION_UP && inside && !touchStrayed) {
            game.undo();
         }
         return true;
      }
      return false;
   }

   /**
    * The piece a tap on the island means, among the positions on offer only: a position
    * that cannot be built has nothing to do with a tap, so it never takes one away from
    * a buildable neighbour. A tap on a building or a horseman itself wins first, since
    * a house stands over the ends of its roads. Then the nearest road, measured to the
    * line between its two vertices so a tap near either end still counts. Then the
    * nearest spot within the wider reach of its anchor.
    */
   private UIEntity pieceAt(int boardPx, int boardPy) {
      // The resource two traders share lies between their carts, close to both: a tap on
      // it is on neither, and does not hire whichever happens to be nearer
      if(onSharedResource(boardPx, boardPy)) {
         return null;
      }
      UIEntity best = null;
      int bestDistance = Integer.MAX_VALUE;
      for(UIEntity e : standing) {
         int dx = boardPx - e.anchorX;
         int dy = boardPy - (e.anchorY - bodyRise(e));
         int d = dx * dx + dy * dy;
         int body = bodyRadius(e);
         if(d <= body * body && d < bestDistance && e.highlightColor() != 0) {
            best = e;
            bestDistance = d;
         }
      }
      if(best != null) {
         return best;
      }
      float bestRoad = REACH_ROAD * REACH_ROAD;
      for(UIEntity e : roads) {
         float d = game.board.roadDistanceSquared(e.index, boardPx, boardPy);
         if(d <= bestRoad && e.highlightColor() != 0) {
            best = e;
            bestRoad = d;
         }
      }
      if(best != null) {
         return best;
      }
      for(UIEntity e : standing) {
         int d = e.distanceSquared(boardPx, boardPy);
         if(d >= 0 && d < bestDistance && e.highlightColor() != 0) {
            best = e;
            bestDistance = d;
         }
      }
      return best;
   }

   // The middle of a piece's sprite sits this far above its anchor, and the sprite
   // reaches about this far around that middle
   private static int bodyRise(UIEntity e) {
      return e.type == UIEntity.Type.village ? Art.VILLAGE_RISE : e.type == UIEntity.Type.city ? Art.CASTLE_RISE : Art.TRADER_RISE;
   }

   private static int bodyRadius(UIEntity e) {
      return e.type == UIEntity.Type.village ? Art.VILLAGE_RADIUS : e.type == UIEntity.Type.city ? Art.CASTLE_RADIUS : Art.TRADER_RADIUS;
   }
}
