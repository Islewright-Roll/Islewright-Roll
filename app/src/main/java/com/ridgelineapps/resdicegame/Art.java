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

/**
 * The piece and icon art, named in one place with the numbers that go with each file
 * (tools/pieces.txt is where they come from).
 */
final class Art {
   static final int PIECE_VILLAGE = R.drawable.piece_village;
   static final int PIECE_CASTLE = R.drawable.piece_castle;
   static final int PIECE_TRADER = R.drawable.piece_trader;
   static final int GLYPH_CASTLE = R.drawable.glyph_castle_12;
   static final int GLYPH_TRADER = R.drawable.glyph_trader_12;
   static final int ICON_ANY_16 = R.drawable.icon_any_16;
   static final int ICON_ANY_12 = R.drawable.icon_any_12;
   // The 12 px icon fills its file, so nothing is trimmed
   static final int ICON_ANY_12_INSET = 0;

   // The King's voyage: the ship (the dock is part of the board), and the HUD strip's sprites
   static final int PIECE_KINGSHIP = R.drawable.piece_kingship;
   static final int TRACK_WATER = R.drawable.track_water;
   static final int TRACK_BUOY = R.drawable.track_buoy;
   static final int TRACK_FLAG = R.drawable.track_flag;
   static final int TRACK_FLAG_EMPTY = R.drawable.track_flag_empty;
   static final int TRACK_SHIP = R.drawable.track_ship;
   static final int TRACK_PIER = R.drawable.track_pier;
   // Sprite sizes of the strip (tools/pieces.txt): the water tiles at this width
   static final int TRACK_H = 13;
   static final int TRACK_WATER_W = 14;
   static final int TRACK_BUOY_W = 5, TRACK_BUOY_H = 7;
   static final int TRACK_FLAG_W = 13;
   static final int TRACK_SHIP_W = 14;
   static final int TRACK_PIER_W = 14;
   // The clear field of a flag, in the flag's own pixels: 11 x 7, for a 5 px tall number
   static final int TRACK_FLAG_FIELD_X = 1, TRACK_FLAG_FIELD_W = 11;
   static final int TRACK_FLAG_FIELD_Y = 1, TRACK_FLAG_FIELD_H = 7;
   // How far the moored ship in the strip lies over the pier's left edge
   static final int TRACK_PIER_OVERLAP = 2;

   // The pixel of the ship that goes on the dock's mooring point (Voyage.DOCK_MOORING):
   // the tip of its bowsprit
   static final int SHIP_CONTACT_X = 58, SHIP_CONTACT_Y = 57;
   // The ship's sprite, and the box in it that the ship itself fills (alpha of 128 or
   // more); the rest of the sprite is its faint wake
   static final int SHIP_W = 60, SHIP_H = 72;
   static final int SHIP_BOX_X = 0, SHIP_BOX_Y = 0, SHIP_BOX_W = 59, SHIP_BOX_H = 67;

   // The pixel of each piece sprite that sits on its board anchor: the vertex for a
   // village or castle, the hex centre for a trader. A building does not stand with its
   // bottom edge on the vertex (the foot of tools/pieces.txt) but 2 px lower, so its base
   // sits across the vertex and covers the round blob where the roads meet there.
   static final int VILLAGE_AX = 9, VILLAGE_AY = 13;
   static final int CASTLE_AX = 12, CASTLE_AY = 23;
   // A trader is placed by what is measured of its sprite, not by its foot. Measured from
   // piece_trader.png with alpha above 128 (the ground shadow does not count): the opaque
   // box is 28 wide (columns 0 to 27) and 18 tall (rows 0 to 17), so its middle is column
   // 14, and the box is centred on the hex centre's x by putting column 14 on it. Before,
   // column 13 sat there, which left the whole trader 1 px right of centre.
   static final int TRADER_BOX_W = 28, TRADER_BOX_H = 18;
   static final int TRADER_AX = TRADER_BOX_W / 2;
   // An empty spot is a group: the trader's footprint, a gap, and a row for the tier and
   // the 16 px icon of what it banks (16 tall; the number is centred on the icon). A built
   // trader has no such row, but its cargo rides 3 px above its sprite. Centred on the hex
   // centre, the group wants the sprite's top 18 px above it, and the built trader with its
   // cargo wants it 8 px above: 10 px apart, which would make a trader jump when built. So
   // both states use the middle, 13 px; the empty group then sits 5 px low and the built
   // trader 5 px high, and the trader stays where it is.
   static final int TRADER_LABEL_GAP = 2;
   static final int TRADER_LABEL_ROW = 16;
   static final int TRADER_AY_GROUP = TRADER_BOX_H;
   static final int TRADER_AY_BUILT = 8;
   static final int TRADER_AY = (TRADER_AY_GROUP + TRADER_AY_BUILT) / 2;
   // Where the 12 px icon of a trader's banked resource rides: the top left of its cell,
   // in the sprite's own pixels. The cell starts 3 px above the sprite's top row, and its
   // bottom row is the front row of the cart's bed.
   static final int TRADER_CARGO_X = 11, TRADER_CARGO_Y = -3;

   // The one resource two traders share rests between their carts: a 12 px cell, its top
   // this far below the middle of the two sprites' tops. A sprite's opaque box is 18 rows,
   // so its middle is 9.5 rows down, and the cell's middle is 6 below its top: 3.5, up to 4.
   static final int SHARED_CELL = 12;
   static final int TRADER_SHARED_DROP = 4;

   // How far below its anchor a trader's feet are; it sorts front to back by them
   static final int TRADER_FOOT_Y = 16;
   static final int TRADER_FEET = TRADER_FOOT_Y - TRADER_AY;
   // How far above its anchor the middle of each sprite is, and how far the sprite
   // reaches around that middle: a tap inside that circle is a tap on the piece
   static final int VILLAGE_RISE = 4, VILLAGE_RADIUS = 10;
   static final int CASTLE_RISE = 9, CASTLE_RADIUS = 13;
   static final int TRADER_RISE = TRADER_AY - TRADER_BOX_H / 2, TRADER_RADIUS = 14;
   // Where a spot's number goes: its ink is centred this far above a village or castle
   // vertex, and a trader's tier and banked resource start this far below the hex centre
   static final int VILLAGE_LABEL_RISE = 3;
   static final int CASTLE_LABEL_RISE = 5;
   static final int TRADER_LABEL_DROP = TRADER_BOX_H + TRADER_LABEL_GAP - TRADER_AY;

   private Art() {
   }
}
