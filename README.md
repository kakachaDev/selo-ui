![SeloUI](https://raw.githubusercontent.com/kakachaDev/selo-ui/main/images/banner.png)

# selo-ui

*selo* — toki pona for "skin, outer layer". A thin layer of UI on top of your game.

`selo-ui` is a tiny immediate-mode UI for games rendered with [wgpu](https://wgpu.rs). It is built for
mobile and web first: every finger is its own pointer, so a virtual joystick, a camera drag and a button
press can all happen at the same time. The whole UI is drawn in one render pass with a single pipeline.

![A mobile game screen built with selo-ui: nine-patch panels, pixel-art icons from one atlas, a clipped quest list, a joystick and a button held by two fingers at once](https://raw.githubusercontent.com/kakachaDev/selo-ui/main/images/showcase.png)

```toml
[dependencies]
selo-ui = "0.1"
```

```rust,ignore
use selo::{Anchor, Ui, UiRenderer};
```

The package is called `selo-ui`, the library is imported as `selo`.

## Why

General-purpose GUI crates are great for tools, but a mobile game needs something else:

- **Multitouch.** Desktop-style GUIs track a single pointer. Games need a joystick under one thumb and
  a look/aim drag under the other.
- **Size.** On the web every kilobyte of wasm is load time. `selo-ui` depends only on `ab_glyph`,
  `bytemuck` and `wgpu`. In the engine it was written for, replacing a full GUI crate with it cut the
  compressed wasm from 1.45 MB to 0.87 MB (brotli), of which 54 KB is the font.
- **Game layout.** HUDs are pinned to screen corners and edges, not flowed like documents. Nine anchors
  plus a safe-area margin cover most of it.

## Features

- Nine-point anchors (`TopLeft` … `BottomRight`) with offsets and a safe-area margin.
- Multitouch pointers with ownership: the widget that receives a press keeps that finger until release.
- Widgets: panel, text (single and multi-line, centered), button with selected and pressed states,
  hold area (returns the finger position while held, a base for joysticks and fire buttons),
  progress bar, scroll list with drag and mouse wheel that only builds visible rows.
- Shapes: rounded rectangles, filled circles and rings, all anti-aliased with signed distance fields.
- Images: any RGBA texture or your own `wgpu::TextureView`, whole or a sub-rectangle of an atlas, with a
  tint color and a choice of smooth or pixel-art (`Filter::Nearest`) sampling.
- Nine-patch: frames and panels with fixed corners that stretch to any size.
- Text: glyphs are rasterized on demand with `ab_glyph` into an `R8` atlas, positioned on the physical
  pixel grid, with kerning.
- Clipping with scissor rectangles, so scroll lists cut their content cleanly.
- Hit testing for input routing: `ui.hit(x, y)` tells you whether a touch should go to the UI or to
  your game.
- Works with the native backends of wgpu (Vulkan, DX12, Metal, GL) and in the browser on WebGL2 and
  WebGPU. The shader keeps texture sampling in uniform control flow, which the browser WebGPU compiler
  requires.

## Frame loop

All coordinates are logical points. `ppp` is the number of physical pixels per point, usually the window
scale factor.

```rust,ignore
let mut ui = selo::Ui::new(include_bytes!("assets/MyFont.ttf"));
let mut renderer = selo::UiRenderer::new(&device, surface_format, ui.font.side);

ui.begin(width_px as f32 / ppp, height_px as f32 / ppp, ppp);

let hud = ui.anchor(selo::Anchor::TopLeft, 220.0, 64.0, 0.0, 0.0);
ui.panel(hud, selo::PANEL, 8.0);
ui.text(hud.x + 10.0, hud.y + 8.0, 16.0, selo::TEXT, "Score: 42");

let pause = ui.anchor(selo::Anchor::TopRight, 48.0, 48.0, 0.0, 0.0);
if ui.button(1, pause, "II", 20.0, false) {
    paused = !paused;
}

let stick_zone = ui.anchor(selo::Anchor::BottomLeft, 200.0, 200.0, 0.0, 0.0);
if let Some((x, y)) = ui.hold(2, stick_zone) {
    let (cx, cy) = stick_zone.center();
    move_dir = ((x - cx) / 100.0, (y - cy) / 100.0);
}

ui.end();

renderer.paint(&device, &queue, &mut encoder, &frame_view, &mut ui, (width_px, height_px));
```

`begin` clears the draw list, widgets record shapes and text, `end` closes the input frame. `paint`
uploads newly rasterized glyphs, writes the vertex and index buffers and records one render pass that
loads the existing color target, so call it after your scene pass.

Widget ids are plain `u64` values chosen by you. They only have to be unique within a frame.

## Images and nine-patch

Register a texture once, then draw it as often as you like. Each image keeps its own size in source
pixels, so atlas regions and nine-patch borders are given in the pixels of the original picture.

```rust,ignore
use selo::{Filter, Rect};

let icons = renderer.add_rgba(&device, &queue, 108, 12, &icon_pixels, Filter::Nearest);
let frame = renderer.add_rgba(&device, &queue, 32, 32, &frame_pixels, Filter::Linear);

ui.nine_patch(panel_rect, frame, [10.0, 10.0, 10.0, 10.0], 1.5, selo::rgba(255, 255, 255, 255));
ui.image_uv(slot_rect, icons, Rect::new(24.0, 0.0, 12.0, 12.0), selo::rgba(255, 255, 255, 255));
ui.image(avatar_rect, portrait, selo::rgba(255, 255, 255, 255));
```

`nine_patch` takes the border widths (left, top, right, bottom) in source pixels and a scale for how big
the corners appear on screen. The corners are never stretched; when the target is smaller than the
corners, they shrink together so the frame stays intact.

For pixel art, use `Filter::Nearest` and draw at whole multiples of the source size in physical pixels
(a 12 px icon at 30 points on a 2x screen is exactly 5x). Images and nine-patch edges are snapped to the
physical pixel grid, so integer scales stay crisp.

Textures are created in the format that matches the render target (`Rgba8UnormSrgb` for sRGB surfaces),
so colors look the same as in your image editor. If you already have a texture, pass its view to
`add_view` instead. Draw calls are only split when the texture or the clip rectangle changes, so keeping
icons in one atlas keeps the UI at one or two draw calls.

## Input

Feed raw events in logical points. Each finger needs a stable id; a mouse is just one more pointer.

```rust,ignore
const MOUSE: u64 = u64::MAX;

match event {
    WindowEvent::CursorMoved { position, .. } => {
        let p = position.to_logical::<f32>(scale);
        ui.pointer_move(MOUSE, p.x, p.y);
    }
    WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
        ElementState::Pressed => ui.pointer_down(MOUSE, mouse.x, mouse.y),
        ElementState::Released => ui.pointer_up(MOUSE),
    },
    WindowEvent::Touch(t) => {
        let p = t.location.to_logical::<f32>(scale);
        match t.phase {
            TouchPhase::Started => ui.pointer_down(t.id, p.x, p.y),
            TouchPhase::Moved => ui.pointer_move(t.id, p.x, p.y),
            TouchPhase::Ended | TouchPhase::Cancelled => ui.pointer_up(t.id),
        }
    }
    _ => {}
}
```

To split touches between the UI and the game, check `ui.hit(x, y)` when a finger goes down. It tests
the interactive rectangles of the previous frame: buttons, hold areas, scroll lists and anything you
mark with `ui.region(rect)`. Plain panels and text are not interactive, so a touch on a HUD label still
reaches your camera.

## Fonts

No font is bundled. Pass any TrueType or OpenType font as `&'static [u8]`. For the web, subset the font
to the scripts you need; for example, DejaVu Sans with Latin and Cyrillic goes from 760 KB to 54 KB:

```sh
pyftsubset DejaVuSans.ttf \
  --unicodes="U+0020-007E,U+00A0-00FF,U+0400-045F,U+2010-2027,U+2190-2193" \
  --output-file=DejaVuSans-subset.ttf
```

The example uses such a subset. Its license is in `examples/assets/DejaVuSans-LICENSE.txt`.

## In a real game

![selo-ui in a voxel engine: HUD, radius stepper, on-screen joystick and up/down buttons](https://raw.githubusercontent.com/kakachaDev/selo-ui/main/images/in-game.png)

The HUD, the radius stepper and the touch controls of a voxel engine that runs natively and in the
browser (WebGL2 and WebGPU). The screenshot comes from a software renderer, so the frame time on it
says nothing about real hardware.

## Examples

```sh
cargo run --example basic
```

A window with an anchored HUD, a counter button, a scroll list and a hold area that works as a joystick
with mouse or touch.

```sh
cargo run --example images
```

Renders `images/banner.png` and `images/showcase.png` offscreen, without a window. Both pictures in
this README are drawn by `selo-ui` itself, so they can be regenerated after any visual change. The
pixel-art icons and nine-patch frames in the showcase are generated in code, so the example has no
image assets.

## Limitations

- No text input or text selection.
- No complex text shaping: no ligatures and no right-to-left scripts. Latin, Cyrillic, Greek and
  similar scripts render correctly with kerning.
- The glyph atlas does not evict. When it is full, new glyphs are skipped and `ui.font.full` becomes
  `true`; create the UI with a larger atlas via `Ui::with_atlas`.
- No layout engine beyond anchors and rectangles. Rows, grids and lists are a few lines of arithmetic
  on `Rect`.

## License

MIT, see [LICENSE](LICENSE).
