// SPDX-License-Identifier: GPL-3.0-or-later
//! The art framebuffer, a port of the Android app's ArtCanvas: the whole game screen is drawn
//! here at one art pixel per buffer pixel, then shown at an integer scale with
//! nearest-neighbor sampling, so every art pixel lands as an exact block of screen pixels.
//!
//! Day draws bitmaps with the platform's own filtering, which blurs pixel art whenever an
//! image is scaled. So the frame is scaled up here, in software, to the canvas's exact size
//! in device pixels, and handed over as an image the canvas draws at one image pixel per
//! device pixel, where filtering has nothing to blend.

/// Narrowest and shortest art space the layout is designed for (ArtCanvas.DESIGN_W, MIN_H).
pub const DESIGN_W: u32 = 216;
pub const MIN_H: u32 = 352;

/// An RGBA8 image, rows top to bottom, non-premultiplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Decode a PNG, as the art scripts write them (8-bit RGBA or RGB, possibly indexed).
    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size().ok_or("png too large")?];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width, info.height);
        let px = (w * h) as usize;
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..px * 4].to_vec(),
            png::ColorType::Rgb => buf[..px * 3]
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|c| [c[0], c[1], c[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buf[..px * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|c| [c[0], c[0], c[0], c[1]])
                .collect(),
            png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => return Err("indexed png was not expanded".into()),
        };
        Ok(Self {
            width: w,
            height: h,
            rgba,
        })
    }

    /// Fill a rectangle, clipped to the image.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = (
            (x + w).min(self.width as i32),
            (y + h).min(self.height as i32),
        );
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.blend(xx as u32, yy as u32, rgba, 255);
            }
        }
    }

    /// Draw `sprite` with its top left at (x, y), clipped, at `alpha` (0 to 255).
    pub fn sprite(&mut self, sprite: &Image, x: i32, y: i32, alpha: u8) {
        for sy in 0..sprite.height as i32 {
            let dy = y + sy;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..sprite.width as i32 {
                let dx = x + sx;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                let i = ((sy as u32 * sprite.width + sx as u32) * 4) as usize;
                let s = [
                    sprite.rgba[i],
                    sprite.rgba[i + 1],
                    sprite.rgba[i + 2],
                    sprite.rgba[i + 3],
                ];
                self.blend(dx as u32, dy as u32, s, alpha);
            }
        }
    }

    /// Source-over one pixel. Over opaque art the result stays opaque. The interface layer
    /// starts out clear (see `compose_png`), so over a clear or partly clear pixel the colour
    /// is weighted by how much of each there is, which keeps a soft glow its own colour
    /// instead of darkening it toward the clear pixel's black.
    fn blend(&mut self, x: u32, y: u32, s: [u8; 4], alpha: u8) {
        let a = s[3] as u32 * alpha as u32 / 255;
        if a == 0 {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        let d = &mut self.rgba[i..i + 4];
        if a == 255 {
            d.copy_from_slice(&[s[0], s[1], s[2], 255]);
            return;
        }
        let da = d[3] as u32;
        if da == 255 {
            for c in 0..3 {
                d[c] = ((s[c] as u32 * a + d[c] as u32 * (255 - a) + 127) / 255) as u8;
            }
            return;
        }
        // What shows of the pixel underneath, in 255ths, and the two weights' total
        let under = da * (255 - a) / 255;
        let out = a + under;
        for c in 0..3 {
            d[c] = ((s[c] as u32 * a + d[c] as u32 * under + out / 2) / out) as u8;
        }
        d[3] = out as u8;
    }

    /// Scale up by an integer factor with nearest-neighbor sampling.
    pub fn upscale(&self, k: u32) -> Image {
        let (w, h) = (self.width * k, self.height * k);
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..self.height {
            let row =
                &self.rgba[(y * self.width * 4) as usize..((y + 1) * self.width * 4) as usize];
            let mut wide = Vec::with_capacity((w * 4) as usize);
            for px in row.as_chunks::<4>().0.iter() {
                for _ in 0..k {
                    wide.extend_from_slice(px);
                }
            }
            for _ in 0..k {
                out.extend_from_slice(&wide);
            }
        }
        Image {
            width: w,
            height: h,
            rgba: out,
        }
    }

    /// Encode as a PNG without compression: a frame is shown once and thrown away, so the time a
    /// deflate pass would take is worth more than the bytes it would save.
    pub fn to_png_stored(&self) -> Vec<u8> {
        self.to_png_scaled(1)
    }

    /// [`Image::to_png_stored`] of this image scaled up `k` times with nearest-neighbor
    /// sampling, without building the scaled image: each widened row is made once and written
    /// `k` times.
    pub fn to_png_scaled(&self, k: u32) -> Vec<u8> {
        let k = k.max(1);
        let (width, height) = (self.width * k, self.height * k);
        let mut png = PngWriter::new(width, height);
        let src_row = (self.width * 4) as usize;
        let mut wide = Vec::with_capacity((width * 4) as usize);
        for y in 0..self.height as usize {
            let row = &self.rgba[y * src_row..(y + 1) * src_row];
            let line: &[u8] = if k == 1 {
                row
            } else {
                wide.clear();
                for px in row.as_chunks::<4>().0.iter() {
                    for _ in 0..k {
                        wide.extend_from_slice(px);
                    }
                }
                &wide
            };
            for _ in 0..k {
                png.row(line);
            }
        }
        png.finish()
    }
}

/// An uncompressed PNG written a row at a time: a zlib stream of stored deflate blocks of at
/// most 65535 bytes, each row behind its filter byte (0, none). The frame is shown once and
/// thrown away, so a deflate pass would cost more time than the bytes it saves are worth.
struct PngWriter {
    out: Vec<u8>,
    idat_start: usize,
    adler: simd_adler32::Adler32,
    /// Raw bytes still to come, and still to come in the open block.
    left: usize,
    block_left: usize,
}

impl PngWriter {
    fn new(width: u32, height: u32) -> PngWriter {
        let row_bytes = (width * 4) as usize;
        let raw_len = (row_bytes + 1) * height as usize;
        let blocks = raw_len.div_ceil(65535).max(1);
        let idat_len = 2 + raw_len + blocks * 5 + 4;
        let mut out = Vec::with_capacity(idat_len + 64);
        out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, deflate, no filter, no interlace
        chunk(&mut out, b"IHDR", &ihdr);
        out.extend_from_slice(&(idat_len as u32).to_be_bytes());
        let idat_start = out.len();
        out.extend_from_slice(b"IDAT");
        out.extend_from_slice(&[0x78, 0x01]);
        PngWriter {
            out,
            idat_start,
            adler: simd_adler32::Adler32::new(),
            left: raw_len,
            block_left: 0,
        }
    }

    fn push(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            if self.block_left == 0 {
                let n = self.left.min(65535);
                self.out.push(if n == self.left { 1 } else { 0 });
                self.out.extend_from_slice(&(n as u16).to_le_bytes());
                self.out.extend_from_slice(&(!(n as u16)).to_le_bytes());
                self.block_left = n;
            }
            let take = bytes.len().min(self.block_left);
            self.out.extend_from_slice(&bytes[..take]);
            self.adler.write(&bytes[..take]);
            bytes = &bytes[take..];
            self.block_left -= take;
            self.left -= take;
        }
    }

    /// One row of pixels, RGBA, as wide as the image.
    fn row(&mut self, line: &[u8]) {
        self.push(&[0]);
        self.push(line);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.out.len() == self.idat_start + 6 {
            // An image with no rows still needs one (empty, final) block
            self.out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
        }
        let adler = self.adler.finish();
        self.out.extend_from_slice(&adler.to_be_bytes());
        let crc = crc32fast::hash(&self.out[self.idat_start..]);
        self.out.extend_from_slice(&crc.to_be_bytes());
        chunk(&mut self.out, b"IEND", &[]);
        self.out
    }
}

// ---------------------------------------------------------------- composing the screen

/// One image on the screen, each of its pixels a block of `scale` x `scale` screen pixels,
/// its top left at the screen's. The screen is drawn from a few of these, each part of it at
/// its own whole-number scale, so a pixel of any of them is still an exact square block.
#[derive(Clone, Copy, Debug)]
pub struct Layer<'a> {
    pub image: &'a Image,
    pub scale: u32,
}

/// A layer's row as last widened to screen pixels, and where in it the pixels are solid,
/// partly clear or clear, so laying it over the row under it copies whole runs.
struct Widened {
    src_row: Option<u32>,
    pixels: Vec<[u8; 4]>,
    /// Runs of screen columns: (start, end, alpha kind), the kinds 0 clear, 1 partly clear,
    /// 2 solid.
    runs: Vec<(usize, usize, u8)>,
}

/// Composes the layers a screen row at a time, remaking a row only where some layer shows
/// another row of its own image than on the row before.
struct Composer<'a> {
    layers: &'a [Layer<'a>],
    width: usize,
    widened: Vec<Widened>,
    row: Vec<[u8; 4]>,
    made: bool,
}

impl<'a> Composer<'a> {
    fn new(width: u32, layers: &'a [Layer<'a>]) -> Composer<'a> {
        let width = width as usize;
        Composer {
            layers,
            width,
            widened: layers
                .iter()
                .map(|_| Widened {
                    src_row: None,
                    pixels: vec![[0; 4]; width],
                    runs: Vec::new(),
                })
                .collect(),
            row: vec![[0; 4]; width],
            made: false,
        }
    }

    /// Widens one layer's row `v` to screen pixels, and finds its runs where they are wanted
    /// (the bottom layer's never are: nothing is under it).
    fn widen(layer: &Layer, v: u32, width: usize, w: &mut Widened, runs: bool) {
        let (img, k) = (layer.image, layer.scale.max(1) as usize);
        w.src_row = Some(v);
        w.runs.clear();
        if v >= img.height {
            w.pixels.fill([0; 4]);
            w.runs.push((0, width, 0));
            return;
        }
        let src = &img.rgba[(v * img.width * 4) as usize..((v + 1) * img.width * 4) as usize];
        let mut x = 0;
        for px in src.as_chunks::<4>().0 {
            if x >= width {
                break;
            }
            let x0 = x;
            x = (x + k).min(width);
            w.pixels[x0..x].fill(*px);
            if !runs {
                continue;
            }
            let kind = match px[3] {
                0 => 0,
                255 => 2,
                _ => 1,
            };
            match w.runs.last_mut() {
                Some(r) if r.2 == kind => r.1 = x,
                _ => w.runs.push((x0, x, kind)),
            }
        }
        if x < width {
            w.pixels[x..].fill([0; 4]);
            if runs {
                w.runs.push((x, width, 0));
            }
        }
    }

    /// Screen row y: the first layer as it is, each later one over it with its clear
    /// pixels left out and its partly clear ones blended.
    fn row(&mut self, y: u32) -> &[u8] {
        let mut changed = !self.made;
        for (n, (layer, w)) in self.layers.iter().zip(self.widened.iter_mut()).enumerate() {
            let v = y / layer.scale.max(1);
            if w.src_row != Some(v) {
                Self::widen(layer, v, self.width, w, n > 0);
                changed = true;
            }
        }
        if !changed {
            return self.row.as_flattened();
        }
        self.made = true;
        let row = &mut self.row;
        let Some(top) = self.widened.last() else {
            row.fill([0; 4]);
            return self.row.as_flattened();
        };
        for (n, w) in self.widened.iter().enumerate() {
            if n == 0 {
                if self.widened.len() == 1 {
                    row.copy_from_slice(&w.pixels);
                    continue;
                }
                // Only where the top layer is not solid: it covers the rest whatever is under
                for &(x0, x1, kind) in &top.runs {
                    if kind != 2 {
                        row[x0..x1].copy_from_slice(&w.pixels[x0..x1]);
                    }
                }
                continue;
            }
            for &(x0, x1, kind) in &w.runs {
                match kind {
                    2 => row[x0..x1].copy_from_slice(&w.pixels[x0..x1]),
                    1 => {
                        for (d, px) in row[x0..x1].iter_mut().zip(&w.pixels[x0..x1]) {
                            let a = px[3] as u32;
                            for c in 0..3 {
                                d[c] = ((px[c] as u32 * a + d[c] as u32 * (255 - a) + 127) / 255)
                                    as u8;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.row.as_flattened()
    }
}

/// The layers composed into a screen of `width` x `height` pixels, as an uncompressed PNG
/// (see [`Image::to_png_stored`]), without building the composed image: each layer's row is
/// widened once, and each distinct screen row is put together once and written as many
/// times as it repeats. Two screen rows are the same whenever every layer shows the same row
/// of its own image on both, which for layers scaled 4 and 5 is all but about one row in two.
pub fn compose_png(width: u32, height: u32, layers: &[Layer]) -> Vec<u8> {
    let mut png = PngWriter::new(width, height);
    let mut composer = Composer::new(width, layers);
    for y in 0..height {
        png.row(composer.row(y));
    }
    png.finish()
}

/// [`compose_png`]'s picture as an image, for tests and snapshots.
pub fn compose(width: u32, height: u32, layers: &[Layer]) -> Image {
    let mut out = Image::new(width, height);
    let row_bytes = (width * 4) as usize;
    let mut composer = Composer::new(width, layers);
    for y in 0..height {
        let at = y as usize * row_bytes;
        out.rgba[at..at + row_bytes].copy_from_slice(composer.row(y));
    }
    out
}

/// An ARGB colour (as the Android app wrote them, alpha in the top byte) as RGBA bytes.
pub fn rgba(argb: u32) -> [u8; 4] {
    [
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        (argb >> 24) as u8,
    ]
}

/// The ArtCanvas drawing operations, ported. Colours here are ARGB, as in the Java, so the
/// palette constants carry over unchanged.
impl Image {
    /// The pixel at (x, y) as ARGB, or 0 outside the image.
    pub fn argb_at(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return 0;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let p = &self.rgba[i..i + 4];
        (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
    }

    /// Every pixel as ARGB, rows top to bottom, for the rules' pixel functions (team_color).
    pub fn to_argb(&self) -> Vec<u32> {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32)
            .collect()
    }

    /// An image from ARGB pixels, rows top to bottom.
    pub fn from_argb(width: u32, height: u32, pixels: &[u32]) -> Image {
        Image {
            width,
            height,
            rgba: pixels.iter().flat_map(|&p| rgba(p)).collect(),
        }
    }

    /// [`Image::fill`] with an ARGB colour; its alpha blends over what is there.
    pub fn fill_argb(&mut self, x: i32, y: i32, w: i32, h: i32, argb: u32) {
        self.fill(x, y, w, h, rgba(argb));
    }

    /// A filled box with its four corner pixels left off.
    pub fn plate(&mut self, x: i32, y: i32, w: i32, h: i32, argb: u32) {
        self.fill_argb(x + 1, y, w - 2, h, argb);
        self.fill_argb(x, y + 1, 1, h - 2, argb);
        self.fill_argb(x + w - 1, y + 1, 1, h - 2, argb);
    }

    /// A one pixel outline round a box.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, argb: u32) {
        self.fill_argb(x, y, w, 1, argb);
        self.fill_argb(x, y + h - 1, w, 1, argb);
        self.fill_argb(x, y + 1, 1, h - 2, argb);
        self.fill_argb(x + w - 1, y + 1, 1, h - 2, argb);
    }

    /// Part of a sprite, (sx, sy, w, h) of it, drawn with its top left at (x, y).
    #[allow(clippy::too_many_arguments)]
    pub fn sprite_part(
        &mut self,
        sprite: &Image,
        sx: i32,
        sy: i32,
        w: i32,
        h: i32,
        x: i32,
        y: i32,
    ) {
        self.stretch(sprite, (sx, sy, w, h), (x, y, w, h));
    }

    /// A source rectangle of a sprite drawn into a destination rectangle with nearest
    /// neighbour sampling. The nine-slice sources are flat along the way they stretch, so
    /// this repeats pixels and never resamples anything.
    pub fn stretch(
        &mut self,
        sprite: &Image,
        src: (i32, i32, i32, i32),
        dst: (i32, i32, i32, i32),
    ) {
        let (sx, sy, sw, sh) = src;
        let (dx, dy, dw, dh) = dst;
        if sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0 {
            return;
        }
        for row in 0..dh {
            let y = dy + row;
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            let src_y = sy + row * sh / dh;
            for col in 0..dw {
                let x = dx + col;
                if x < 0 || x >= self.width as i32 {
                    continue;
                }
                let src_x = sx + col * sw / dw;
                if src_x < 0
                    || src_y < 0
                    || src_x >= sprite.width as i32
                    || src_y >= sprite.height as i32
                {
                    continue;
                }
                let i = ((src_y as u32 * sprite.width + src_x as u32) * 4) as usize;
                let s = [
                    sprite.rgba[i],
                    sprite.rgba[i + 1],
                    sprite.rgba[i + 2],
                    sprite.rgba[i + 3],
                ];
                self.blend(x as u32, y as u32, s, 255);
            }
        }
    }

    /// A sprite repeated left to right across w pixels from x, the last copy cut short. For a
    /// sprite that is seamless along its width.
    pub fn tile(&mut self, sprite: &Image, x: i32, y: i32, w: i32) {
        let tile_w = sprite.width as i32;
        let mut done = 0;
        while done < w {
            self.sprite_part(
                sprite,
                0,
                0,
                tile_w.min(w - done),
                sprite.height as i32,
                x + done,
                y,
            );
            done += tile_w;
        }
    }

    /// A square nine-slice drawn at any size: the corners copied, the edges and the middle
    /// stretched.
    pub fn nine_slice(&mut self, source: &Image, x: i32, y: i32, w: i32, h: i32, corner: i32) {
        let size = source.width as i32;
        let sx = [0, corner, size - corner, size];
        let dx = [x, x + corner, x + w - corner, x + w];
        let dy = [y, y + corner, y + h - corner, y + h];
        for row in 0..3 {
            for col in 0..3 {
                self.stretch(
                    source,
                    (
                        sx[col],
                        sx[row],
                        sx[col + 1] - sx[col],
                        sx[row + 1] - sx[row],
                    ),
                    (
                        dx[col],
                        dy[row],
                        dx[col + 1] - dx[col],
                        dy[row + 1] - dy[row],
                    ),
                );
            }
        }
    }

    /// A mask's shape filled with one colour, the colour's own alpha kept.
    pub fn silhouette(&mut self, mask: &Mask, x: i32, y: i32, argb: u32) {
        let color = rgba(argb);
        for my in 0..mask.height {
            let dy = y + my;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for mx in 0..mask.width {
                let dx = x + mx;
                if dx < 0 || dx >= self.width as i32 || !mask.bits[(my * mask.width + mx) as usize]
                {
                    continue;
                }
                self.blend(dx as u32, dy as u32, color, 255);
            }
        }
    }

    /// Each pixel's colour through `f` (ARGB in, ARGB out).
    pub fn map_argb(&self, f: impl Fn(u32) -> u32) -> Image {
        Image::from_argb(
            self.width,
            self.height,
            &self.to_argb().into_iter().map(f).collect::<Vec<_>>(),
        )
    }

    /// Turned a quarter turn clockwise, pixel for pixel: (x, y) goes to (h - 1 - y, x).
    pub fn rotate_cw(&self) -> Image {
        let (w, h) = (self.width, self.height);
        let mut out = Image::new(h, w);
        for y in 0..h {
            for x in 0..w {
                let s = ((y * w + x) * 4) as usize;
                let d = ((x * h + (h - 1 - y)) * 4) as usize;
                out.rgba[d..d + 4].copy_from_slice(&self.rgba[s..s + 4]);
            }
        }
        out
    }

    /// Turned a quarter turn counterclockwise, pixel for pixel: (x, y) goes to (y, w - 1 - x).
    pub fn rotate_ccw(&self) -> Image {
        let (w, h) = (self.width, self.height);
        let mut out = Image::new(h, w);
        for y in 0..h {
            for x in 0..w {
                let s = ((y * w + x) * 4) as usize;
                let d = (((w - 1 - x) * h + y) * 4) as usize;
                out.rgba[d..d + 4].copy_from_slice(&self.rgba[s..s + 4]);
            }
        }
        out
    }

    /// How many rows at the bottom hold nothing at least half solid.
    pub fn clear_rows_below(&self) -> i32 {
        for y in (0..self.height as i32).rev() {
            if (0..self.width as i32).any(|x| (self.argb_at(x, y) >> 24) >= 128) {
                return self.height as i32 - 1 - y;
            }
        }
        self.height as i32
    }
}

/// A one bit layer: where a sprite is at least half solid, or a flattened set of them. The
/// footprints, offers and glows are built from these (ArtCanvas.mask and rim).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub width: i32,
    pub height: i32,
    pub bits: Vec<bool>,
}

impl Mask {
    pub fn new(width: i32, height: i32) -> Mask {
        Mask {
            width,
            height,
            bits: vec![false; (width * height).max(0) as usize],
        }
    }

    /// Set wherever the sprite is at least half solid, so soft shadows drop out.
    pub fn of(sprite: &Image) -> Mask {
        Mask {
            width: sprite.width as i32,
            height: sprite.height as i32,
            bits: sprite
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[3] >= 128)
                .collect(),
        }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && x < self.width
            && y < self.height
            && self.bits[(y * self.width + x) as usize]
    }

    /// The inner rim: the set pixels that touch a clear one or the edge.
    pub fn rim(&self) -> Mask {
        let mut out = Mask::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y)
                    && !(self.get(x - 1, y)
                        && self.get(x + 1, y)
                        && self.get(x, y - 1)
                        && self.get(x, y + 1))
                {
                    out.bits[(y * self.width + x) as usize] = true;
                }
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        !self.bits.iter().any(|&b| b)
    }
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32fast::hash(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// How the art space fits a canvas of `w` x `h` device pixels: the largest integer scale at
/// which the design width and minimum height both fit, the art size that scale gives, and the
/// leftover device pixels split either side (ArtCanvas.layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub scale: u32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

pub fn fit(w: u32, h: u32) -> Fit {
    let scale = (w / DESIGN_W).min(h / MIN_H).max(1);
    let (width, height) = (w / scale, h / scale);
    Fit {
        scale,
        width,
        height,
        offset_x: (w - width * scale) / 2,
        offset_y: (h - height * scale) / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_png_round_trips() {
        let mut img = Image::new(70, 3);
        img.fill(0, 0, 70, 3, [10, 20, 30, 255]);
        img.fill(5, 1, 3, 1, [200, 100, 50, 255]);
        let big = img.upscale(3);
        assert_eq!(Image::from_png(&big.to_png_stored()).unwrap(), big);
        let back = Image::from_png(&img.to_png_scaled(3)).unwrap();
        assert_eq!(back, big);
        assert_eq!(back.width, 210);
    }

    #[test]
    fn layers_compose_as_whole_blocks_of_their_own_scale() {
        // An island image at 3 and an interface at 2 over it, on a 12 x 7 screen
        let mut under = Image::new(4, 3);
        under.fill_argb(0, 0, 4, 3, 0xFF102030);
        under.fill_argb(1, 1, 1, 1, 0xFFFF0000);
        let mut over = Image::new(6, 4);
        over.fill_argb(0, 0, 1, 4, 0xFF00FF00);
        over.fill_argb(5, 3, 1, 1, 0x80FFFFFF);
        let layers = [
            Layer {
                image: &under,
                scale: 3,
            },
            Layer {
                image: &over,
                scale: 2,
            },
        ];
        let out = compose(12, 7, &layers);
        // The island's red pixel is the 3 x 3 block from (3, 3)
        assert_eq!(out.argb_at(3, 3), 0xFFFF0000);
        assert_eq!(out.argb_at(5, 5), 0xFFFF0000);
        assert_eq!(out.argb_at(6, 5), 0xFF102030);
        assert_eq!(out.argb_at(5, 6), 0xFF102030);
        // The interface's opaque column covers 2 screen columns, its clear pixels nothing
        assert_eq!(out.argb_at(1, 6), 0xFF00FF00);
        assert_eq!(out.argb_at(2, 0), 0xFF102030);
        // A half clear pixel blends over the island under it
        let half = out.argb_at(10, 6);
        assert!(half >> 24 == 0xFF && half != 0xFF102030 && half != 0xFFFFFFFF);
        assert_eq!(out.argb_at(11, 6), half);
        // The PNG holds the same picture
        assert_eq!(Image::from_png(&compose_png(12, 7, &layers)).unwrap(), out);
        // One layer at one scale is just the scaled image
        let one = [Layer {
            image: &under,
            scale: 3,
        }];
        assert_eq!(compose(12, 9, &one), under.upscale(3));
        assert_eq!(compose_png(12, 9, &one), under.to_png_scaled(3));
    }

    #[test]
    fn blending_over_a_clear_pixel_keeps_the_colour() {
        let mut img = Image::new(2, 1);
        img.fill_argb(0, 0, 1, 1, 0x80FF8040);
        assert_eq!(img.argb_at(0, 0), 0x80FF8040);
        // Twice over itself: more solid, the same colour
        img.fill_argb(0, 0, 1, 1, 0x80FF8040);
        let p = img.argb_at(0, 0);
        assert!(p >> 24 > 0x80 && p & 0xFFFFFF == 0xFF8040, "{p:08x}");
        // Over opaque art, as it always was
        img.fill_argb(1, 0, 1, 1, 0xFF000000);
        img.fill_argb(1, 0, 1, 1, 0x80FFFFFF);
        assert_eq!(img.argb_at(1, 0), 0xFF808080);
    }

    #[test]
    fn rotations_are_exact_and_inverse() {
        let mut img = Image::new(3, 2);
        img.fill_argb(0, 0, 1, 1, 0xFF112233);
        img.fill_argb(2, 1, 1, 1, 0xFF445566);
        let cw = img.rotate_cw();
        assert_eq!((cw.width, cw.height), (2, 3));
        // (0, 0) goes to (h - 1, 0) = (1, 0) turning clockwise; (2, 1) to (0, 2)
        assert_eq!(cw.argb_at(1, 0), 0xFF112233);
        assert_eq!(cw.argb_at(0, 2), 0xFF445566);
        let ccw = img.rotate_ccw();
        // (0, 0) goes to (0, w - 1) = (0, 2) turning counterclockwise; (2, 1) to (1, 0)
        assert_eq!(ccw.argb_at(0, 2), 0xFF112233);
        assert_eq!(ccw.argb_at(1, 0), 0xFF445566);
        assert_eq!(cw.rotate_ccw(), img);
        assert_eq!(ccw.rotate_cw(), img);
        assert_eq!(img.rotate_cw().rotate_cw(), img.rotate_ccw().rotate_ccw());
    }

    #[test]
    fn plate_cuts_corners_and_rim_hugs_the_edge() {
        let mut img = Image::new(4, 4);
        img.plate(0, 0, 4, 4, 0xFF000000 | 0xFFFFFF);
        assert_eq!(img.argb_at(0, 0), 0);
        assert_eq!(img.argb_at(1, 0), 0xFFFFFFFF);
        let mask = Mask::of(&img);
        let rim = mask.rim();
        assert!(rim.get(1, 0) && !rim.get(1, 1) && !rim.get(0, 0));
    }

    #[test]
    fn nine_slice_keeps_corners() {
        let mut src = Image::new(24, 24);
        src.fill_argb(0, 0, 24, 24, 0xFF0000FF);
        src.fill_argb(0, 0, 6, 6, 0xFFFF0000);
        let mut dst = Image::new(50, 20);
        dst.nine_slice(&src, 0, 0, 50, 20, 6);
        assert_eq!(dst.argb_at(5, 5), 0xFFFF0000);
        assert_eq!(dst.argb_at(6, 6), 0xFF0000FF);
        assert_eq!(dst.argb_at(49, 19), 0xFF0000FF);
    }

    #[test]
    fn fit_uses_the_largest_integer_scale() {
        // A 1080 x 2400 phone: 1080 / 216 = 5, 2400 / 352 = 6, so 5, and 216 x 480 art pixels.
        assert_eq!(
            fit(1080, 2400),
            Fit {
                scale: 5,
                width: 216,
                height: 480,
                offset_x: 0,
                offset_y: 0
            }
        );
        assert_eq!(fit(1082, 2342).scale, 5);
        assert_eq!(fit(1082, 2342).offset_x, 1);
    }
}

#[cfg(test)]
mod bench {
    /// Composing and encoding a 2560 x 1600 tablet screen from an island at 5 and an
    /// interface at 4: `cargo test --release -- --ignored frame_cost --nocapture`. The whole
    /// frame, drawing included, is measured in the view's `frame_cost_by_size`.
    #[test]
    #[ignore]
    fn frame_cost() {
        let board = super::Image::from_png(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resource/assets/art/board_tarnholm.png"
        )))
        .unwrap();
        let n = 20;
        let (mut t1, mut t2) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
        for _ in 0..n {
            let t = std::time::Instant::now();
            let mut island = super::Image::new(512, 320);
            island.sprite(&board, 0, -100, 255);
            let mut ui = super::Image::new(640, 400);
            ui.fill_argb(0, 0, 147, 400, 0xFF33261A);
            ui.fill_argb(640 - 112, 0, 112, 400, 0xFF33261A);
            t1 += t.elapsed();
            let t = std::time::Instant::now();
            let layers = [
                super::Layer {
                    image: &island,
                    scale: 5,
                },
                super::Layer {
                    image: &ui,
                    scale: 4,
                },
            ];
            let png = super::compose_png(2560, 1600, &layers);
            t2 += t.elapsed();
            assert!(png.len() > 1000);
        }
        eprintln!("draw {:?} compose and encode {:?}", t1 / n, t2 / n);
    }
}
