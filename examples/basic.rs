use std::sync::Arc;

use selo::{rgba, Anchor, Rect, Ui, UiRenderer, PANEL, TEXT};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

static FONT: &[u8] = include_bytes!("assets/DejaVuSans-subset.ttf");
const MOUSE: u64 = u64::MAX;

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: UiRenderer,
}

struct App {
    gpu: Option<Gpu>,
    ui: Ui,
    mouse: (f32, f32),
    clicks: u32,
    stick: (f32, f32),
}

impl App {
    fn scale(&self) -> f32 {
        self.gpu.as_ref().map(|g| g.window.scale_factor() as f32).unwrap_or(1.0)
    }

    fn build(&mut self, w: f32, h: f32, ppp: f32) {
        let ui = &mut self.ui;
        ui.begin(w, h, ppp);

        let lines = [
            "selo-ui basic example".to_string(),
            format!("clicks: {}", self.clicks),
            format!("stick: {:+.2} {:+.2}", self.stick.0, self.stick.1),
        ];
        let lh = ui.line_height(15.0);
        let hud = ui.anchor(Anchor::TopLeft, 230.0, lh * 3.0 + 16.0, 0.0, 0.0);
        ui.panel(hud, PANEL, 8.0);
        for (i, l) in lines.iter().enumerate() {
            ui.text(hud.x + 10.0, hud.y + 8.0 + lh * i as f32, 15.0, TEXT, l);
        }

        let button = ui.anchor(Anchor::TopRight, 140.0, 44.0, 0.0, 0.0);
        if ui.button(1, button, "Click me", 16.0, self.clicks % 2 == 1) {
            self.clicks += 1;
        }

        let list = ui.anchor(Anchor::Right, 260.0, 240.0, 0.0, 0.0);
        ui.panel(list, PANEL, 8.0);
        ui.scroll_list(2, list.inset(4.0), 28.0, 100, |ui, i, row| {
            ui.text(row.x + 8.0, row.y + 6.0, 14.0, TEXT, &format!("Row {}", i + 1));
            ui.progress(Rect::new(row.x + row.w - 108.0, row.y + 10.0, 100.0, 8.0), (i % 10) as f32 / 9.0);
        });

        let zone = ui.anchor(Anchor::BottomLeft, 180.0, 180.0, 0.0, 0.0);
        let (cx, cy) = zone.center();
        let r = zone.w * 0.5;
        self.stick = match ui.hold(3, zone) {
            Some((x, y)) => {
                let (dx, dy) = ((x - cx) / r, (y - cy) / r);
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                (dx / len, dy / len)
            }
            None => (0.0, 0.0),
        };
        ui.circle(cx, cy, r, rgba(0, 0, 0, 70));
        ui.ring(cx, cy, r, 2.0, rgba(255, 255, 255, 140));
        ui.circle(cx + self.stick.0 * r * 0.6, cy + self.stick.1 * r * 0.6, r * 0.35, rgba(255, 255, 255, 170));

        let hint = ui.anchor(Anchor::Bottom, 300.0, 24.0, 0.0, 0.0);
        ui.text_centered(hint, 13.0, rgba(255, 255, 255, 160), "drag the circle, scroll the list");

        ui.end();
    }

    fn redraw(&mut self) {
        let Some(g) = self.gpu.as_ref() else { return };
        let ppp = g.window.scale_factor() as f32;
        let (w, h) = (g.config.width, g.config.height);
        self.build(w as f32 / ppp, h as f32 / ppp, ppp);
        let g = self.gpu.as_mut().unwrap();
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                g.surface.configure(&g.device, &g.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = g.device.create_command_encoder(&Default::default());
        {
            enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.18, g: 0.32, b: 0.45, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        g.renderer.paint(&g.device, &g.queue, &mut enc, &view, &mut self.ui, (w, h));
        g.queue.submit([enc.finish()]);
        g.queue.present(frame);
        g.window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let window = Arc::new(el.create_window(Window::default_attributes().with_title("selo-ui basic")).unwrap());
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let size = window.inner_size();
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).unwrap();
        surface.configure(&device, &config);
        let renderer = UiRenderer::new(&device, config.format, self.ui.font.side);
        window.request_redraw();
        self.gpu = Some(Gpu { window, surface, device, queue, config, renderer });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let scale = self.scale();
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = self.gpu.as_mut() {
                    g.config.width = s.width.max(1);
                    g.config.height = s.height.max(1);
                    g.surface.configure(&g.device, &g.config);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x as f32 / scale, position.y as f32 / scale);
                self.ui.pointer_move(MOUSE, self.mouse.0, self.mouse.1);
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => self.ui.pointer_down(MOUSE, self.mouse.0, self.mouse.1),
                ElementState::Released => self.ui.pointer_up(MOUSE),
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / scale,
                };
                self.ui.wheel(self.mouse.0, self.mouse.1, dy);
            }
            WindowEvent::Touch(t) => {
                let (x, y) = (t.location.x as f32 / scale, t.location.y as f32 / scale);
                match t.phase {
                    TouchPhase::Started => self.ui.pointer_down(t.id, x, y),
                    TouchPhase::Moved => self.ui.pointer_move(t.id, x, y),
                    TouchPhase::Ended | TouchPhase::Cancelled => self.ui.pointer_up(t.id),
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { gpu: None, ui: Ui::new(FONT), mouse: (0.0, 0.0), clicks: 0, stick: (0.0, 0.0) };
    event_loop.run_app(&mut app).unwrap();
}
