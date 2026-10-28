use selo::{rgba, Filter, Rect, Ui, UiRenderer, TEXT};

static FONT: &[u8] = include_bytes!("assets/DejaVuSans-subset.ttf");
static BOLD: &[u8] = include_bytes!("assets/DejaVuSans-Bold-subset.ttf");
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ACCENT: u32 = rgba(56, 139, 253, 255);
const WHITE: u32 = rgba(255, 255, 255, 255);
const ICON: usize = 12;

const ICONS: [[&str; 12]; 8] = [
    [
        "............",
        ".....kkk....",
        "....kMMMk...",
        "...kMmmmk...",
        "...kmmkwk...",
        "....kk.kwk..",
        ".......kwk..",
        "........kwk.",
        "........kwk.",
        ".........kwk",
        ".........kk.",
        "............",
    ],
    [
        "............",
        "..kkkkkkk...",
        ".kMmmmmmMk..",
        "kmk..kw.kmk.",
        "kk...kw..kk.",
        ".....kw.....",
        ".....kw.....",
        ".....kw.....",
        ".....kw.....",
        ".....kw.....",
        ".....kk.....",
        "............",
    ],
    [
        "............",
        "...kkkkkk...",
        "..kWWWWWWk..",
        ".kWwwwwwwWk.",
        ".kWwWWWWwWk.",
        ".kWwWwwWwWk.",
        ".kWwWwwWwWk.",
        ".kWwWWWWwWk.",
        ".kWwwwwwwWk.",
        "..kWWWWWWk..",
        "...kkkkkk...",
        "............",
    ],
    [
        "............",
        "....kkkk....",
        "..kkSSSskk..",
        ".kSSSsssssk.",
        ".kSsssSsssk.",
        "kSssssssSssk",
        "kssSsssssssk",
        "ksssssSssssk",
        ".kssssssssk.",
        "..kkkkkkkk..",
        "............",
        "............",
    ],
    [
        ".....yy.....",
        "....yffy....",
        "....fffff...",
        "....kfffk...",
        ".....kwk....",
        ".....kwk....",
        ".....kwk....",
        ".....kwk....",
        ".....kwk....",
        ".....kwk....",
        ".....kkk....",
        "............",
    ],
    [
        "............",
        "...GG..GG...",
        "..GggkkggG..",
        "...Ggkkgg...",
        ".....kk.....",
        ".....gk.....",
        ".....gk.....",
        "...kkkkkk...",
        "..kwwwwwwk..",
        "..kwWWWWwk..",
        "...kkkkkk...",
        "............",
    ],
    [
        "............",
        "............",
        "...kkkkkk...",
        "..kBBBBBBk..",
        ".kBbBbBbBbk.",
        ".kbbbbbbbbk.",
        ".kbbbbbbbbk.",
        ".kbbbbbbbbk.",
        "..kkkkkkkk..",
        "............",
        "............",
        "............",
    ],
    [
        "............",
        ".kkkkkkkkkk.",
        ".kppppppppk.",
        ".kprrpppppk.",
        ".kppRrppgpk.",
        ".kpppprpGpk.",
        ".kpgpppRppk.",
        ".kpGgpppprk.",
        ".kppppppppk.",
        ".kkkkkkkkkk.",
        "............",
        "............",
    ],
];

const HEART: [&str; 8] = [".kk...kk.", "kRRk.kRrk", "kRrrkrrrk", "krrrrrrrk", ".krrrrrk.", "..krrrk..", "...krk...", "....k...."];

fn palette(c: char) -> [u8; 4] {
    match c {
        'k' => [28, 22, 18, 255],
        'w' => [150, 100, 50, 255],
        'W' => [196, 146, 84, 255],
        's' => [130, 132, 142, 255],
        'S' => [188, 190, 200, 255],
        'm' => [160, 170, 182, 255],
        'M' => [222, 228, 238, 255],
        'f' => [250, 150, 40, 255],
        'y' => [255, 224, 90, 255],
        'g' => [76, 160, 56, 255],
        'G' => [134, 214, 92, 255],
        'b' => [200, 140, 70, 255],
        'B' => [236, 190, 118, 255],
        'p' => [232, 218, 176, 255],
        'r' => [214, 52, 52, 255],
        'R' => [244, 120, 108, 255],
        _ => [0, 0, 0, 0],
    }
}

fn icon_atlas() -> (u32, u32, Vec<u8>) {
    let w = ICON * ICONS.len() + 9;
    let mut px = vec![0u8; w * ICON * 4];
    let mut put = |x: usize, y: usize, c: char| {
        let i = (y * w + x) * 4;
        px[i..i + 4].copy_from_slice(&palette(c));
    };
    for (n, icon) in ICONS.iter().enumerate() {
        for (y, row) in icon.iter().enumerate() {
            for (x, c) in row.chars().take(ICON).enumerate() {
                put(n * ICON + x, y, c);
            }
        }
    }
    for (y, row) in HEART.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            put(ICON * ICONS.len() + x, y + 2, c);
        }
    }
    (w as u32, ICON as u32, px)
}

fn frame(size: u32, radius: f32, outline: [u8; 3], light: [u8; 3], dark: [u8; 3], fill: [u8; 4], bevel: f32) -> Vec<u8> {
    let mut px = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (x as f32 + 0.5 - s * 0.5, y as f32 + 0.5 - s * 0.5);
            let (qx, qy) = (fx.abs() - s * 0.5 + radius, fy.abs() - s * 0.5 + radius);
            let d = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius;
            let c: [u8; 4] = if d > 0.0 {
                [0, 0, 0, 0]
            } else if d > -1.2 {
                [outline[0], outline[1], outline[2], 255]
            } else if d > -1.2 - bevel {
                let c = if fx + fy < 0.0 { light } else { dark };
                [c[0], c[1], c[2], 255]
            } else {
                fill
            };
            let i = ((y * size + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    }
    px
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Gpu {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).expect("device");
    Gpu { device, queue }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

fn gradient(ui: &mut Ui, top: [u8; 3], bottom: [u8; 3], from: f32, to: f32) {
    let bands = 48;
    let h = (to - from) / bands as f32;
    let w = ui.screen.w;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let c = rgba(lerp(top[0], bottom[0], t), lerp(top[1], bottom[1], t), lerp(top[2], bottom[2], t), 255);
        ui.panel(Rect::new(-2.0, from + i as f32 * h - 0.5, w + 4.0, h + 1.0), c, 0.0);
    }
}

fn render(g: &Gpu, size: (u32, u32), layers: &mut [(&mut Ui, &mut UiRenderer)], path: &str) {
    let tex = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let mut enc = g.device.create_command_encoder(&Default::default());
    {
        enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    for (ui, r) in layers.iter_mut() {
        r.paint(&g.device, &g.queue, &mut enc, &view, ui, size);
    }
    let row = (size.0 * 4).div_ceil(256) * 256;
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (row * size.1) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(size.1) } },
        wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
    );
    g.queue.submit([enc.finish()]);
    buf.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    g.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let data = buf.slice(..).get_mapped_range().expect("mapped range");
    let mut pixels = Vec::with_capacity((size.0 * size.1 * 4) as usize);
    for y in 0..size.1 {
        let start = (y * row) as usize;
        pixels.extend_from_slice(&data[start..start + (size.0 * 4) as usize]);
    }
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().expect("png header").write_image_data(&pixels).expect("png data");
    println!("wrote {path} ({}x{})", size.0, size.1);
}

fn banner(g: &Gpu) {
    let (w, h, ppp) = (1280.0, 360.0, 2.0);
    let size = ((w * ppp) as u32, (h * ppp) as u32);
    let mut ui = Ui::new(FONT);
    let mut bold = Ui::new(BOLD);
    let mut r1 = UiRenderer::new(&g.device, FORMAT, ui.font.side);
    let mut r2 = UiRenderer::new(&g.device, FORMAT, bold.font.side);

    ui.begin(w, h, ppp);
    gradient(&mut ui, [16, 20, 34], [28, 24, 50], 0.0, h);
    let sub = "tiny game UI for wgpu";
    let sw = ui.text_width(34.0, sub);
    ui.text((w - sw) * 0.5, 236.0, 34.0, rgba(214, 222, 236, 255), sub);
    ui.end();

    bold.begin(w, h, ppp);
    let a = bold.text_width(124.0, "Selo");
    let b = bold.text_width(124.0, "UI");
    let x = (w - a - b) * 0.5;
    bold.text(x, 56.0, 124.0, WHITE, "Selo");
    bold.text(x + a, 56.0, 124.0, ACCENT, "UI");
    bold.end();

    render(g, size, &mut [(&mut ui, &mut r1), (&mut bold, &mut r2)], "images/banner.png");
}

fn stick(ui: &mut Ui, id: u64, zone: Rect, finger: (f32, f32)) {
    let (cx, cy) = zone.center();
    let r = zone.w * 0.5;
    ui.pointer_down(id, finger.0, finger.1);
    let knob = match ui.hold(id, zone) {
        Some((x, y)) => {
            let (dx, dy) = (x - cx, y - cy);
            let len = (dx * dx + dy * dy).sqrt();
            let k = if len > r * 0.5 { r * 0.5 / len } else { 1.0 };
            (cx + dx * k, cy + dy * k)
        }
        None => (cx, cy),
    };
    ui.circle(cx, cy, r, rgba(0, 0, 0, 70));
    ui.ring(cx, cy, r, 2.0, rgba(255, 255, 255, 150));
    ui.circle(knob.0, knob.1, r * 0.42, rgba(255, 255, 255, 190));
}

fn round_button(ui: &mut Ui, id: u64, cx: f32, cy: f32, r: f32, label: &str, pressed: bool) {
    let zone = Rect::new(cx - r, cy - r, r * 2.0, r * 2.0);
    if pressed {
        ui.pointer_down(id, cx, cy);
    }
    let held = ui.hold(id, zone).is_some();
    ui.circle(cx, cy, r, if held { rgba(255, 255, 255, 120) } else { rgba(0, 0, 0, 80) });
    ui.ring(cx, cy, r, 2.0, rgba(255, 255, 255, 140));
    ui.text_centered(zone, r * 0.7, rgba(255, 255, 255, 230), label);
}

fn showcase(g: &Gpu) {
    let (w, h, ppp) = (960.0, 540.0, 2.0);
    let size = ((w * ppp) as u32, (h * ppp) as u32);
    let mut ui = Ui::new(FONT);
    let mut r = UiRenderer::new(&g.device, FORMAT, ui.font.side);

    let (aw, ah, atlas) = icon_atlas();
    let icons = r.add_rgba(&g.device, &g.queue, aw, ah, &atlas, Filter::Nearest);
    let panel_px = frame(32, 7.0, [24, 18, 14], [214, 172, 112], [118, 84, 50], [34, 28, 24, 246], 2.5);
    let panel = r.add_rgba(&g.device, &g.queue, 32, 32, &panel_px, Filter::Nearest);
    let slot_px = frame(24, 5.0, [20, 16, 14], [70, 60, 52], [150, 128, 104], [52, 44, 38, 235], 2.0);
    let slot = r.add_rgba(&g.device, &g.queue, 24, 24, &slot_px, Filter::Nearest);
    let icon = |i: usize| Rect::new((i * ICON) as f32, 0.0, ICON as f32, ICON as f32);
    let heart = Rect::new((ICON * ICONS.len()) as f32, 2.0, 9.0, 8.0);

    ui.begin(w, h, ppp);
    gradient(&mut ui, [96, 150, 214], [170, 205, 235], 0.0, h * 0.58);
    gradient(&mut ui, [92, 150, 70], [58, 104, 44], h * 0.58, h);
    for (x, y, s) in [(120.0, 300.0, 70.0), (300.0, 286.0, 90.0), (560.0, 296.0, 60.0)] {
        ui.panel(Rect::new(x, y, s, h * 0.58 - y + 2.0), rgba(70, 110, 60, 255), 0.0);
        ui.panel(Rect::new(x - 16.0, y - s * 0.9, s + 32.0, s * 0.95), rgba(54, 120, 52, 255), s * 0.45);
    }

    let hud = ui.anchor(selo::Anchor::TopLeft, 226.0, 84.0, 0.0, 0.0);
    ui.nine_patch(hud, panel, [10.0; 4], 1.5, WHITE);
    for i in 0..5 {
        let tint = if i < 4 { WHITE } else { rgba(90, 90, 90, 200) };
        ui.image_uv(Rect::new(hud.x + 16.0 + i as f32 * 30.0, hud.y + 16.0, 27.0, 24.0), icons, heart, tint);
    }
    ui.text(hud.x + 16.0, hud.y + 50.0, 15.0, TEXT, "Coins 1 250");

    let menu = ui.anchor(selo::Anchor::TopRight, 100.0, 42.0, 0.0, 0.0);
    ui.nine_patch(menu, panel, [10.0; 4], 1.0, WHITE);
    ui.text_centered(menu, 15.0, TEXT, "Menu");
    let pause = ui.anchor(selo::Anchor::TopRight, 42.0, 42.0, -108.0, 0.0);
    ui.button(2, pause, "II", 15.0, false);

    let bar = ui.anchor(selo::Anchor::Bottom, 8.0 * 54.0 + 12.0, 66.0, 0.0, 0.0);
    ui.nine_patch(bar, panel, [10.0; 4], 1.0, WHITE);
    for i in 0..8 {
        let cell = Rect::new(bar.x + 8.0 + i as f32 * 54.0, bar.y + 8.0, 50.0, 50.0);
        let tint = if i == 2 { rgba(120, 180, 255, 255) } else { WHITE };
        ui.nine_patch(cell, slot, [7.0; 4], 1.0, tint);
        ui.image_uv(cell.inset(10.0), icons, icon(i), WHITE);
        ui.text(cell.x + 5.0, cell.y + 2.0, 11.0, rgba(255, 255, 255, 210), &format!("{}", i + 1));
    }

    let list = ui.anchor(selo::Anchor::Right, 256.0, 196.0, 0.0, -20.0);
    ui.nine_patch(list, panel, [10.0; 4], 1.5, WHITE);
    ui.text(list.x + 16.0, list.y + 12.0, 15.0, TEXT, "Quests");
    let quests = ["Chop wood", "Mine stone", "Bake bread", "Plant seeds", "Light torches", "Draw a map"];
    let quest_icons = [2, 3, 6, 5, 4, 7];
    ui.scroll_list(30, Rect::new(list.x + 10.0, list.y + 38.0, list.w - 20.0, list.h - 48.0), 28.0, 30, |ui, i, row| {
        ui.image_uv(Rect::new(row.x + 4.0, row.y + 5.0, 18.0, 18.0), icons, icon(quest_icons[i % 6]), WHITE);
        ui.text(row.x + 30.0, row.y + 6.0, 13.0, TEXT, quests[i % 6]);
        ui.progress(Rect::new(row.x + row.w - 66.0, row.y + 10.0, 60.0, 7.0), ((i * 37) % 100) as f32 / 100.0);
    });

    let zone = ui.anchor(selo::Anchor::BottomLeft, 150.0, 150.0, 8.0, -8.0);
    let (cx, cy) = zone.center();
    stick(&mut ui, 40, zone, (cx + 38.0, cy - 30.0));
    round_button(&mut ui, 41, w - 70.0, h - 120.0, 32.0, "↑", true);
    round_button(&mut ui, 42, w - 70.0, h - 48.0, 32.0, "↓", false);

    let note = |ui: &mut Ui, x: f32, y: f32, s: &str| {
        let tw = ui.text_width(12.0, s);
        ui.panel(Rect::new(x - 6.0, y - 3.0, tw + 12.0, 20.0), rgba(31, 111, 235, 225), 10.0);
        ui.text(x, y, 12.0, TEXT, s);
    };
    note(&mut ui, hud.x + 6.0, hud.y + hud.h + 8.0, "nine_patch() + pixel-art hearts");
    note(&mut ui, list.x + 6.0, list.y + list.h + 8.0, "scroll list, clipped");
    note(&mut ui, bar.x + 6.0, bar.y - 26.0, "image_uv() from one atlas, Filter::Nearest");
    note(&mut ui, zone.x + 6.0, zone.y - 26.0, "finger 1: hold area");
    let tw = ui.text_width(12.0, "finger 2: pressed");
    note(&mut ui, w - 70.0 - 32.0 - tw - 20.0, h - 128.0, "finger 2: pressed");
    ui.end();

    render(g, size, &mut [(&mut ui, &mut r)], "images/showcase.png");
}

fn main() {
    std::fs::create_dir_all("images").expect("images dir");
    let g = gpu();
    banner(&g);
    showcase(&g);
}
