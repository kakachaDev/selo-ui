use crate::{Image, Ui, VERTEX_WORDS};

pub enum Filter {
    Linear,
    Nearest,
}

pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::Texture,
    bind: wgpu::BindGroup,
    image_layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    images: Vec<wgpu::BindGroup>,
    uniform: wgpu::Buffer,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    vcap: u64,
    icap: u64,
    srgb: bool,
}

fn buffer(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor { label: Some("ui"), size, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
}

impl UiRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, atlas_side: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui-atlas"),
            size: wgpu::Extent3d { width: atlas_side, height: atlas_side, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = atlas.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = buffer(device, 16, wgpu::BufferUsages::UNIFORM);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let image_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout), Some(&image_layout)],
            immediate_size: 0,
        });
        let attrs = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 16, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Unorm8x4, offset: 36, shader_location: 4 },
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (VERTEX_WORDS * 4) as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attrs,
                })],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let vcap = 64 * 1024;
        let icap = 64 * 1024;
        let mut r = Self {
            pipeline,
            atlas,
            bind,
            image_layout,
            linear: sampler,
            nearest,
            images: Vec::new(),
            uniform,
            vbuf: buffer(device, vcap, wgpu::BufferUsages::VERTEX),
            ibuf: buffer(device, icap, wgpu::BufferUsages::INDEX),
            vcap,
            icap,
            srgb: format.is_srgb(),
        };
        r.images.push(r.bind_image(device, &r.blank(device), Filter::Nearest));
        r
    }

    fn blank(&self, device: &wgpu::Device) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("ui-blank"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    fn bind_image(&self, device: &wgpu::Device, view: &wgpu::TextureView, filter: Filter) -> wgpu::BindGroup {
        let sampler = match filter {
            Filter::Linear => &self.linear,
            Filter::Nearest => &self.nearest,
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui-image"),
            layout: &self.image_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        })
    }

    pub fn image_format(&self) -> wgpu::TextureFormat {
        if self.srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm }
    }

    pub fn add_view(&mut self, device: &wgpu::Device, view: &wgpu::TextureView, width: u32, height: u32, filter: Filter) -> Image {
        let bind = self.bind_image(device, view, filter);
        self.images.push(bind);
        Image { id: (self.images.len() - 1) as u32, width: width as f32, height: height as f32 }
    }

    pub fn add_rgba(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32, rgba: &[u8], filter: Filter) -> Image {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui-image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.image_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            size,
        );
        self.add_view(device, &tex.create_view(&Default::default()), width, height, filter)
    }

    pub fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        ui: &mut Ui,
        size_px: (u32, u32),
    ) {
        for u in ui.font.pending.drain(..) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &self.atlas, mip_level: 0, origin: wgpu::Origin3d { x: u.x, y: u.y, z: 0 }, aspect: wgpu::TextureAspect::All },
                &u.data,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(u.w), rows_per_image: Some(u.h) },
                wgpu::Extent3d { width: u.w, height: u.h, depth_or_array_layers: 1 },
            );
        }
        let c = &ui.canvas;
        if c.indices.is_empty() {
            return;
        }
        let ppp = ui.ppp;
        let size = [size_px.0 as f32 / ppp, size_px.1 as f32 / ppp, ppp, if self.srgb { 1.0 } else { 0.0 }];
        queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&size));
        let vbytes = (c.verts.len() * 4) as u64;
        let ibytes = (c.indices.len() * 4) as u64;
        if vbytes > self.vcap {
            self.vcap = vbytes.next_power_of_two();
            self.vbuf = buffer(device, self.vcap, wgpu::BufferUsages::VERTEX);
        }
        if ibytes > self.icap {
            self.icap = ibytes.next_power_of_two();
            self.ibuf = buffer(device, self.icap, wgpu::BufferUsages::INDEX);
        }
        queue.write_buffer(&self.vbuf, 0, bytemuck::cast_slice(&c.verts));
        queue.write_buffer(&self.ibuf, 0, bytemuck::cast_slice(&c.indices));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, self.vbuf.slice(..vbytes));
        pass.set_index_buffer(self.ibuf.slice(..ibytes), wgpu::IndexFormat::Uint32);
        let (w, h) = size_px;
        for cmd in &c.cmds {
            let x0 = (cmd.clip.x * ppp).floor().clamp(0.0, w as f32) as u32;
            let y0 = (cmd.clip.y * ppp).floor().clamp(0.0, h as f32) as u32;
            let x1 = ((cmd.clip.x + cmd.clip.w) * ppp).ceil().clamp(0.0, w as f32) as u32;
            let y1 = ((cmd.clip.y + cmd.clip.h) * ppp).ceil().clamp(0.0, h as f32) as u32;
            if x1 <= x0 || y1 <= y0 || cmd.count == 0 {
                continue;
            }
            pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
            let img = self.images.get(cmd.image as usize).unwrap_or(&self.images[0]);
            pass.set_bind_group(1, img, &[]);
            pass.draw_indexed(cmd.start..cmd.start + cmd.count, 0, 0..1);
        }
    }
}
