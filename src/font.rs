use std::collections::HashMap;

use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};

pub struct GlyphEntry {
    pub uv: [f32; 4],
    pub offset: [f32; 2],
    pub size: [f32; 2],
}

pub struct Upload {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

pub struct FontAtlas {
    font: FontRef<'static>,
    pub side: u32,
    cache: HashMap<(u16, u32), Option<GlyphEntry>>,
    cursor: (u32, u32),
    row_h: u32,
    pub pending: Vec<Upload>,
    pub full: bool,
}

impl FontAtlas {
    pub fn new(data: &'static [u8], side: u32) -> Self {
        let font = FontRef::try_from_slice(data).expect("font");
        Self { font, side, cache: HashMap::new(), cursor: (1, 1), row_h: 0, pending: Vec::new(), full: false }
    }

    pub fn ascent(&self, px: f32) -> f32 {
        self.font.as_scaled(PxScale::from(px)).ascent()
    }

    pub fn line_height(&self, px: f32) -> f32 {
        let s = self.font.as_scaled(PxScale::from(px));
        s.ascent() - s.descent() + s.line_gap()
    }

    pub fn id(&self, c: char) -> GlyphId {
        self.font.glyph_id(c)
    }

    pub fn advance(&self, px: f32, id: GlyphId) -> f32 {
        self.font.as_scaled(PxScale::from(px)).h_advance(id)
    }

    pub fn kern(&self, px: f32, a: GlyphId, b: GlyphId) -> f32 {
        self.font.as_scaled(PxScale::from(px)).kern(a, b)
    }

    pub fn glyph(&mut self, id: GlyphId, px: f32) -> Option<&GlyphEntry> {
        let key = (id.0, (px * 4.0).round() as u32);
        if !self.cache.contains_key(&key) {
            let e = self.rasterize(id, px);
            self.cache.insert(key, e);
        }
        self.cache.get(&key).and_then(|e| e.as_ref())
    }

    fn rasterize(&mut self, id: GlyphId, px: f32) -> Option<GlyphEntry> {
        let g = id.with_scale(PxScale::from(px));
        let og = self.font.outline_glyph(g)?;
        let b = og.px_bounds();
        let w = b.width().ceil() as u32;
        let h = b.height().ceil() as u32;
        if w == 0 || h == 0 {
            return None;
        }
        if self.cursor.0 + w + 1 >= self.side {
            self.cursor = (1, self.cursor.1 + self.row_h + 1);
            self.row_h = 0;
        }
        if self.cursor.1 + h + 1 >= self.side {
            self.full = true;
            return None;
        }
        let (x, y) = self.cursor;
        self.cursor.0 += w + 1;
        self.row_h = self.row_h.max(h);
        let mut data = vec![0u8; (w * h) as usize];
        og.draw(|gx, gy, c| {
            if gx < w && gy < h {
                data[(gy * w + gx) as usize] = (c.clamp(0.0, 1.0) * 255.0) as u8;
            }
        });
        self.pending.push(Upload { x, y, w, h, data });
        let s = self.side as f32;
        Some(GlyphEntry {
            uv: [x as f32 / s, y as f32 / s, (x + w) as f32 / s, (y + h) as f32 / s],
            offset: [b.min.x, b.min.y],
            size: [w as f32, h as f32],
        })
    }
}
