#![doc = include_str!("../README.md")]

pub mod font;
pub mod render;

use std::collections::HashMap;

pub use font::FontAtlas;
pub use render::{Filter, UiRenderer};

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Clone for Rect {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Rect {}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn inset(&self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2.0 * d).max(0.0), (self.h - 2.0 * d).max(0.0))
    }

    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }
}

pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16 | (a as u32) << 24
}

pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    fn factors(&self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::Top => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::Left => (0.0, 0.5),
            Anchor::Center => (0.5, 0.5),
            Anchor::Right => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::Bottom => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

const MODE_TEXT: f32 = 0.0;
const MODE_FILL: f32 = 1.0;
const MODE_RING: f32 = 2.0;
const MODE_IMAGE: f32 = 3.0;
pub const VERTEX_WORDS: usize = 10;

pub struct Image {
    pub id: u32,
    pub width: f32,
    pub height: f32,
}

impl Clone for Image {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Image {}

pub struct DrawCmd {
    pub clip: Rect,
    pub image: u32,
    pub start: u32,
    pub count: u32,
}

pub struct Canvas {
    pub verts: Vec<u32>,
    pub indices: Vec<u32>,
    pub cmds: Vec<DrawCmd>,
    clip: Rect,
}

impl Canvas {
    fn new() -> Self {
        Self { verts: Vec::new(), indices: Vec::new(), cmds: Vec::new(), clip: Rect::new(0.0, 0.0, 1e9, 1e9) }
    }

    fn clear(&mut self, screen: Rect) {
        self.verts.clear();
        self.indices.clear();
        self.cmds.clear();
        self.clip = screen;
    }

    fn set_clip(&mut self, clip: Rect) {
        self.clip = clip;
    }

    fn quad(&mut self, r: Rect, local: [f32; 4], half: [f32; 2], radius: f32, mode: f32, sw: f32, color: u32) {
        self.quad_with(0, r, local, half, radius, mode, sw, color);
    }

    fn quad_with(&mut self, image: u32, r: Rect, local: [f32; 4], half: [f32; 2], radius: f32, mode: f32, sw: f32, color: u32) {
        let base = (self.verts.len() / VERTEX_WORDS) as u32;
        let corners = [(r.x, r.y, local[0], local[1]), (r.x + r.w, r.y, local[2], local[1]), (r.x + r.w, r.y + r.h, local[2], local[3]), (r.x, r.y + r.h, local[0], local[3])];
        for (x, y, u, v) in corners {
            self.verts.extend_from_slice(&[
                x.to_bits(),
                y.to_bits(),
                u.to_bits(),
                v.to_bits(),
                half[0].to_bits(),
                half[1].to_bits(),
                radius.to_bits(),
                mode.to_bits(),
                sw.to_bits(),
                color,
            ]);
        }
        let need_new = match self.cmds.last() {
            Some(c) => {
                c.image != image || c.clip.x != self.clip.x || c.clip.y != self.clip.y || c.clip.w != self.clip.w || c.clip.h != self.clip.h
            }
            None => true,
        };
        if need_new {
            self.cmds.push(DrawCmd { clip: self.clip, image, start: self.indices.len() as u32, count: 0 });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.cmds.last_mut().unwrap().count += 6;
    }

    pub fn rect(&mut self, r: Rect, color: u32, radius: f32) {
        let hw = r.w * 0.5;
        let hh = r.h * 0.5;
        let q = Rect::new(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0);
        self.quad(q, [-hw - 1.0, -hh - 1.0, hw + 1.0, hh + 1.0], [hw, hh], radius.min(hw).min(hh), MODE_FILL, 0.0, color);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, color: u32) {
        self.rect(Rect::new(cx - r, cy - r, r * 2.0, r * 2.0), color, r);
    }

    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, width: f32, color: u32) {
        let q = Rect::new(cx - r - 1.0, cy - r - 1.0, r * 2.0 + 2.0, r * 2.0 + 2.0);
        self.quad(q, [-r - 1.0, -r - 1.0, r + 1.0, r + 1.0], [r, r], r, MODE_RING, width, color);
    }

    fn glyph(&mut self, r: Rect, uv: [f32; 4], color: u32) {
        self.quad(r, uv, [0.0, 0.0], 0.0, MODE_TEXT, 0.0, color);
    }

    pub fn image(&mut self, r: Rect, img: Image, uv: [f32; 4], tint: u32) {
        if r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        self.quad_with(img.id, r, uv, [0.0, 0.0], 0.0, MODE_IMAGE, 0.0, tint);
    }
}

pub struct Pointer {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    px: f32,
    py: f32,
    pub down: bool,
    pressed: bool,
    released: bool,
    owner: u64,
}

pub struct Ui {
    pub canvas: Canvas,
    pub font: FontAtlas,
    pub screen: Rect,
    pub ppp: f32,
    pub safe: f32,
    pointers: Vec<Pointer>,
    regions: Vec<Rect>,
    prev_regions: Vec<Rect>,
    scroll: HashMap<u64, f32>,
    wheel: Option<(f32, f32, f32)>,
}

pub const TEXT: u32 = rgba(232, 236, 242, 255);
pub const PANEL: u32 = rgba(14, 17, 22, 190);
pub const BUTTON: u32 = rgba(46, 54, 66, 230);
pub const BUTTON_ON: u32 = rgba(31, 111, 235, 240);
pub const BUTTON_DOWN: u32 = rgba(80, 92, 110, 240);

impl Ui {
    pub fn new(font_data: &'static [u8]) -> Self {
        Self::with_atlas(font_data, 1024)
    }

    pub fn with_atlas(font_data: &'static [u8], atlas_side: u32) -> Self {
        Self {
            canvas: Canvas::new(),
            font: FontAtlas::new(font_data, atlas_side),
            screen: Rect::new(0.0, 0.0, 1.0, 1.0),
            ppp: 1.0,
            safe: 8.0,
            pointers: Vec::new(),
            regions: Vec::new(),
            prev_regions: Vec::new(),
            scroll: HashMap::new(),
            wheel: None,
        }
    }

    pub fn pointer_down(&mut self, id: u64, x: f32, y: f32) {
        self.pointers.retain(|p| p.id != id);
        self.pointers.push(Pointer { id, x, y, px: x, py: y, down: true, pressed: true, released: false, owner: 0 });
    }

    pub fn pointer_move(&mut self, id: u64, x: f32, y: f32) {
        if let Some(p) = self.pointers.iter_mut().find(|p| p.id == id) {
            p.x = x;
            p.y = y;
        }
    }

    pub fn pointer_up(&mut self, id: u64) {
        if let Some(p) = self.pointers.iter_mut().find(|p| p.id == id) {
            p.down = false;
            p.released = true;
        }
    }

    pub fn wheel(&mut self, x: f32, y: f32, dy: f32) {
        self.wheel = Some((x, y, dy));
    }

    pub fn hit(&self, x: f32, y: f32) -> bool {
        self.prev_regions.iter().any(|r| r.contains(x, y))
    }

    pub fn begin(&mut self, w: f32, h: f32, ppp: f32) {
        self.screen = Rect::new(0.0, 0.0, w, h);
        self.ppp = ppp;
        self.canvas.clear(self.screen);
        self.regions.clear();
    }

    pub fn end(&mut self) {
        std::mem::swap(&mut self.regions, &mut self.prev_regions);
        self.pointers.retain(|p| !p.released);
        for p in &mut self.pointers {
            p.pressed = false;
            p.px = p.x;
            p.py = p.y;
        }
        self.wheel = None;
    }

    pub fn anchor(&self, a: Anchor, w: f32, h: f32, dx: f32, dy: f32) -> Rect {
        let (fx, fy) = a.factors();
        let s = self.safe;
        let x = s + (self.screen.w - 2.0 * s - w) * fx + dx;
        let y = s + (self.screen.h - 2.0 * s - h) * fy + dy;
        Rect::new(x, y, w, h)
    }

    pub fn region(&mut self, r: Rect) {
        self.regions.push(r);
    }

    pub fn panel(&mut self, r: Rect, color: u32, radius: f32) {
        self.canvas.rect(r, color, radius);
    }

    pub fn text_width(&self, size: f32, s: &str) -> f32 {
        let px = size * self.ppp;
        let mut w = 0.0;
        let mut prev = None;
        for c in s.chars() {
            let id = self.font.id(c);
            if let Some(p) = prev {
                w += self.font.kern(px, p, id);
            }
            w += self.font.advance(px, id);
            prev = Some(id);
        }
        w / self.ppp
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.font.line_height(size * self.ppp) / self.ppp
    }

    pub fn text(&mut self, x: f32, y: f32, size: f32, color: u32, s: &str) -> f32 {
        let ppp = self.ppp;
        let px = size * ppp;
        let baseline = (y * ppp + self.font.ascent(px)).round();
        let mut pen = (x * ppp).round();
        let mut prev = None;
        for c in s.chars() {
            let id = self.font.id(c);
            if let Some(p) = prev {
                pen += self.font.kern(px, p, id);
            }
            let adv = self.font.advance(px, id);
            if let Some(g) = self.font.glyph(id, px) {
                let gx = (pen + g.offset[0]).round();
                let gy = (baseline + g.offset[1]).round();
                let r = Rect::new(gx / ppp, gy / ppp, g.size[0] / ppp, g.size[1] / ppp);
                let uv = g.uv;
                self.canvas.glyph(r, uv, color);
            }
            pen += adv;
            prev = Some(id);
        }
        pen / ppp - x
    }

    pub fn text_centered(&mut self, r: Rect, size: f32, color: u32, s: &str) {
        let lines: Vec<&str> = s.split('\n').collect();
        let lh = self.line_height(size);
        let total = lh * lines.len() as f32;
        let mut y = r.y + (r.h - total) * 0.5;
        for l in lines {
            let w = self.text_width(size, l);
            self.text(r.x + (r.w - w) * 0.5, y, size, color, l);
            y += lh;
        }
    }

    fn claim_press(&mut self, id: u64, r: Rect) -> bool {
        for p in &mut self.pointers {
            if p.pressed && p.owner == 0 && r.contains(p.x, p.y) {
                p.owner = id;
                return true;
            }
        }
        false
    }

    fn held_by(&self, id: u64) -> Option<&Pointer> {
        self.pointers.iter().find(|p| p.owner == id && p.down)
    }

    pub fn button(&mut self, id: u64, r: Rect, label: &str, size: f32, selected: bool) -> bool {
        self.region(r);
        self.claim_press(id, r);
        let clicked = self.pointers.iter().any(|p| p.owner == id && p.released && r.contains(p.x, p.y));
        let held = self.held_by(id).is_some();
        let color = if held { BUTTON_DOWN } else if selected { BUTTON_ON } else { BUTTON };
        self.canvas.rect(r, color, 6.0);
        self.text_centered(r, size, TEXT, label);
        clicked
    }

    pub fn hold(&mut self, id: u64, r: Rect) -> Option<(f32, f32)> {
        self.region(r);
        self.claim_press(id, r);
        self.held_by(id).map(|p| (p.x, p.y))
    }

    pub fn progress(&mut self, r: Rect, frac: f32) {
        self.canvas.rect(r, rgba(255, 255, 255, 40), r.h * 0.5);
        let w = (r.w * frac.clamp(0.0, 1.0)).max(r.h);
        self.canvas.rect(Rect::new(r.x, r.y, w, r.h), rgba(56, 139, 253, 230), r.h * 0.5);
    }

    pub fn scroll_list(&mut self, id: u64, r: Rect, row_h: f32, rows: usize, mut row: impl FnMut(&mut Ui, usize, Rect)) {
        self.region(r);
        let max = (rows as f32 * row_h - r.h).max(0.0);
        let mut off = *self.scroll.get(&id).unwrap_or(&0.0);
        if let Some(p) = self.held_by(id) {
            off -= p.y - p.py;
        }
        if let Some((x, y, dy)) = self.wheel {
            if r.contains(x, y) {
                off -= dy;
            }
        }
        off = off.clamp(0.0, max);
        self.scroll.insert(id, off);
        let outer = self.canvas.clip;
        self.canvas.set_clip(r.intersect(&outer));
        let first = (off / row_h).floor() as usize;
        let count = (r.h / row_h).ceil() as usize + 1;
        for i in first..(first + count).min(rows) {
            let y = r.y + i as f32 * row_h - off;
            row(self, i, Rect::new(r.x, y, r.w, row_h));
        }
        self.canvas.set_clip(outer);
        self.claim_press(id, r);
    }

    pub fn ring(&mut self, cx: f32, cy: f32, radius: f32, width: f32, color: u32) {
        self.canvas.ring(cx, cy, radius, width, color);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        self.canvas.circle(cx, cy, radius, color);
    }

    fn snap(&self, v: f32) -> f32 {
        (v * self.ppp).round() / self.ppp
    }

    fn snap_rect(&self, r: Rect) -> Rect {
        let x0 = self.snap(r.x);
        let y0 = self.snap(r.y);
        Rect::new(x0, y0, self.snap(r.x + r.w) - x0, self.snap(r.y + r.h) - y0)
    }

    pub fn image(&mut self, r: Rect, img: Image, tint: u32) {
        let r = self.snap_rect(r);
        self.canvas.image(r, img, [0.0, 0.0, 1.0, 1.0], tint);
    }

    pub fn image_uv(&mut self, r: Rect, img: Image, src: Rect, tint: u32) {
        let r = self.snap_rect(r);
        let (w, h) = (img.width, img.height);
        self.canvas.image(r, img, [src.x / w, src.y / h, (src.x + src.w) / w, (src.y + src.h) / h], tint);
    }

    pub fn nine_patch(&mut self, r: Rect, img: Image, border: [f32; 4], scale: f32, tint: u32) {
        let [l, t, rr, b] = border;
        let (w, h) = (img.width, img.height);
        let k = (r.w / ((l + rr) * scale)).min(r.h / ((t + b) * scale)).min(1.0) * scale;
        let xs = [r.x, r.x + l * k, r.x + r.w - rr * k, r.x + r.w].map(|v| self.snap(v));
        let ys = [r.y, r.y + t * k, r.y + r.h - b * k, r.y + r.h].map(|v| self.snap(v));
        let us = [0.0, l / w, 1.0 - rr / w, 1.0];
        let vs = [0.0, t / h, 1.0 - b / h, 1.0];
        for j in 0..3 {
            for i in 0..3 {
                let cell = Rect::new(xs[i], ys[j], xs[i + 1] - xs[i], ys[j + 1] - ys[j]);
                self.canvas.image(cell, img, [us[i], vs[j], us[i + 1], vs[j + 1]], tint);
            }
        }
    }
}
