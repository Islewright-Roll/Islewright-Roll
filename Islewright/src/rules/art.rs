// SPDX-License-Identifier: GPL-3.0-or-later
//! The numbers that go with the piece and icon art (tools/pieces.txt is where they come
//! from): sprite sizes, the pixel of each sprite that sits on its board anchor, and the
//! reach of a tap. The layouts in this module tree are computed from them; the image files
//! themselves are the UI's.

// Sprite sizes of the HUD's voyage strip: the water tiles at this width
pub const TRACK_H: i32 = 13;
pub const TRACK_WATER_W: i32 = 14;
pub const TRACK_BUOY_W: i32 = 5;
pub const TRACK_BUOY_H: i32 = 7;
pub const TRACK_FLAG_W: i32 = 13;
pub const TRACK_SHIP_W: i32 = 14;
pub const TRACK_PIER_W: i32 = 10;
// The clear field of a flag, in the flag's own pixels: 11 x 7, for a 5 px tall number
pub const TRACK_FLAG_FIELD_X: i32 = 1;
pub const TRACK_FLAG_FIELD_W: i32 = 11;
pub const TRACK_FLAG_FIELD_Y: i32 = 1;
pub const TRACK_FLAG_FIELD_H: i32 = 7;
// How far the moored ship in the strip lies over the pier's left edge
pub const TRACK_PIER_OVERLAP: i32 = 2;

// The pixel of the ship that goes on the dock's mooring point (voyage::DOCK_MOORING): the
// tip of its bowsprit
pub const SHIP_CONTACT_X: i32 = 58;
pub const SHIP_CONTACT_Y: i32 = 57;
// The ship's sprite, and the box in it that the ship itself fills (alpha of 128 or more);
// the rest of the sprite is its faint wake
pub const SHIP_W: i32 = 60;
pub const SHIP_H: i32 = 72;
pub const SHIP_BOX_X: i32 = 0;
pub const SHIP_BOX_Y: i32 = 0;
pub const SHIP_BOX_W: i32 = 59;
pub const SHIP_BOX_H: i32 = 67;

// The pixel of each piece sprite that sits on its board anchor: the vertex for a village or
// castle, the hex centre for a trader. A building does not stand with its bottom edge on the
// vertex (the foot of tools/pieces.txt) but 2 px lower, so its base sits across the vertex
// and covers the round blob where the roads meet there.
pub const VILLAGE_AX: i32 = 9;
pub const VILLAGE_AY: i32 = 13;
pub const CASTLE_AX: i32 = 12;
pub const CASTLE_AY: i32 = 23;
// A trader is placed by what is measured of its sprite, not by its foot. Measured from
// piece_trader.png with alpha above 128 (the ground shadow does not count): the opaque box
// is 28 wide and 18 tall, so its middle is column 14, and the box is centred on the hex
// centre's x by putting column 14 on it.
pub const TRADER_BOX_W: i32 = 28;
pub const TRADER_BOX_H: i32 = 18;
pub const TRADER_AX: i32 = TRADER_BOX_W / 2;
// An empty spot is a group: the trader's footprint, a gap, and a row for the tier and the
// 16 px icon of what it banks. A built trader has no such row, but its cargo rides 3 px above
// its sprite. Centred on the hex centre, the group wants the sprite's top 18 px above it, and
// the built trader with its cargo wants it 8 px above: 10 px apart, which would make a trader
// jump when built. So both states use the middle, 13 px; the empty group then sits 5 px low
// and the built trader 5 px high, and the trader stays where it is.
pub const TRADER_LABEL_GAP: i32 = 2;
pub const TRADER_LABEL_ROW: i32 = 16;
pub const TRADER_AY_GROUP: i32 = TRADER_BOX_H;
pub const TRADER_AY_BUILT: i32 = 8;
pub const TRADER_AY: i32 = (TRADER_AY_GROUP + TRADER_AY_BUILT) / 2;
// Where the 12 px icon of a trader's banked resource rides: the top left of its cell, in the
// sprite's own pixels. The cell starts 3 px above the sprite's top row, and its bottom row is
// the front row of the cart's bed.
pub const TRADER_CARGO_X: i32 = 11;
pub const TRADER_CARGO_Y: i32 = -3;

// The one resource two traders share rests between their carts: a 12 px cell, its top this
// far below the middle of the two sprites' tops. A sprite's opaque box is 18 rows, so its
// middle is 9.5 rows down, and the cell's middle is 6 below its top: 3.5, up to 4.
pub const SHARED_CELL: i32 = 12;
pub const TRADER_SHARED_DROP: i32 = 4;

// How far below its anchor a trader's feet are; it sorts front to back by them
pub const TRADER_FOOT_Y: i32 = 16;
pub const TRADER_FEET: i32 = TRADER_FOOT_Y - TRADER_AY;
// How far above its anchor the middle of each sprite is, and how far the sprite reaches
// around that middle: a tap inside that circle is a tap on the piece
pub const VILLAGE_RISE: i32 = 4;
pub const VILLAGE_RADIUS: i32 = 10;
pub const CASTLE_RISE: i32 = 9;
pub const CASTLE_RADIUS: i32 = 13;
pub const TRADER_RISE: i32 = TRADER_AY - TRADER_BOX_H / 2;
pub const TRADER_RADIUS: i32 = 14;
// Where a spot's number goes: its ink is centred this far above a village or castle vertex,
// and a trader's tier and banked resource start this far below the hex centre
pub const VILLAGE_LABEL_RISE: i32 = 3;
pub const CASTLE_LABEL_RISE: i32 = 5;
pub const TRADER_LABEL_DROP: i32 = TRADER_BOX_H + TRADER_LABEL_GAP - TRADER_AY;

/// The art canvas is this wide (and the HUD with it); the screen is scaled to fit it.
pub const DESIGN_W: i32 = 216;
