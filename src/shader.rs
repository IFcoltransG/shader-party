use std::{fs, time::Instant};
use wgpu::{util::DeviceExt, *};
use winit::{dpi::PhysicalSize, event::*, window::Window};

mod geometry;
mod uniforms;

use self::{
    geometry::{Vertex, INDICES, VERTICES},
    uniforms::{MouseUniform, TimeUniform},
};

pub(super) struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    render_pipeline_layout: PipelineLayout,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    background_colour: Color,
    start_time: Instant,
    time_uniform: TimeUniform,
    time_buffer: Buffer,
    time_bind_group: BindGroup,
    mouse_uniform: MouseUniform,
    mouse_buffer: Buffer,
    mouse_bind_group: BindGroup,
}

impl State {
    // need async for creating some wgpu types
    pub(super) async fn new(window: &Window) -> Self {
        // make sure dimensions are nonzero (or crash)
        let size = window.inner_size();

        // GET GPU DEVICE
        log::debug!("Setting up GPU device");

        // instance is a handle to the GPU
        // Backends::all = Vulkan, Metal, DX12, Browser WebGPU
        let instance = wgpu::Instance::new(Backends::all()); // for making adapters and surfaces
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        // request a device with that adapter
        // devices are where the magic happens
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(), // no features
                    limits: Limits::default(),
                    label: None,
                },
                None, // trace path
            )
            .await
            .unwrap();
        // config for the surface
        log::debug!("Configuring surface");
        let config = SurfaceConfiguration {
            // allows rendering textures to screen
            usage: TextureUsages::RENDER_ATTACHMENT,
            // choose texture format to match what the screen prefers
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            // vsync on, is the only good option on mobile devices
            present_mode: PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        log::debug!("Setting up uniform bindings");
        // TIME BINDING

        let start_time = Instant::now();
        let mut time_uniform = TimeUniform::new();
        time_uniform.update_time(start_time);
        let time_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[time_uniform]),
            usage: BufferUsages::all(),
        });
        let time_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Time Buffer Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let time_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("time_bind_group"),
            layout: &time_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        // MOUSE BINDINGS
        let mouse_uniform = MouseUniform::new();
        let mouse_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Mouse Buffer"),
            contents: bytemuck::cast_slice(&[mouse_uniform]),
            usage: BufferUsages::all(),
        });
        let mouse_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Mouse Buffer Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let mouse_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("mouse_bind_group"),
            layout: &mouse_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: mouse_buffer.as_entire_binding(),
            }],
        });

        // Collect bind group layouts into one pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            // collect bind groups here
            // first elem is `[[group(0)]]` etc
            bind_group_layouts: &[&time_bind_group_layout, &mouse_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Make geometry buffers
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        // LOAD SHADER
        let shader = Self::new_shader(&device);

        // COLLECT BIND GROUPS AND SHADERS INTO PIPELINE

        let render_pipeline = Self::new_pipeline(&device, &config, &render_pipeline_layout, shader);

        // a bluish colour as default
        let background_colour = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            render_pipeline_layout,
            vertex_buffer,
            index_buffer,
            num_indices,
            background_colour,
            start_time,
            time_uniform,
            time_buffer,
            time_bind_group,
            mouse_uniform,
            mouse_buffer,
            mouse_bind_group,
        }
    }

    fn new_pipeline(
        device: &Device,
        config: &SurfaceConfiguration,
        render_pipeline_layout: &PipelineLayout,
        shader: ShaderModule,
    ) -> RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            // vertex shader and buffers
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            // fragment shader and buffers and blending modes
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState {
                    // same format as the surface for easier copying
                    format: config.format,
                    // don't care about old pixels, just replace them
                    blend: Some(BlendState::REPLACE),
                    // write to every colour channel including alpha
                    write_mask: ColorWrites::ALL,
                }],
            }),
            // how to interpret vertices as triangles
            primitive: PrimitiveState {
                // chunk vertices into triplets as triangles
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // a triangle is facing forward whenever vertices are arranged
                // 'counter-clockwise'
                front_face: FrontFace::Ccw,
                // cull back faces
                cull_mode: Some(Face::Back),
                // must be Fill unless GPU supports NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // must be false unless DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // must be false unless CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            // use one buffer
            multisample: MultisampleState {
                // only one sample
                count: 1,
                // bits set to use all samples
                mask: !0,
                // no antialiasing
                alpha_to_coverage_enabled: false,
            },
            // not using array textures
            multiview: None,
        })
    }

    fn new_shader(device: &Device) -> ShaderModule {
        log::info!("Reading shader");

        // load shader from file
        // let shader_source = include_str!("shader.wgsl").into();
        let shader_source = fs::read_to_string("./src/shader.wgsl")
            .expect("Failed reading shader")
            .into();
        device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(shader_source),
        })
    }

    pub(super) fn refresh_shader(&mut self) {
        self.render_pipeline = State::new_pipeline(
            &self.device,
            &self.config,
            &self.render_pipeline_layout,
            State::new_shader(&self.device),
        )
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub(super) fn current_size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub(super) fn input(&mut self, event: &WindowEvent) -> bool {
        // bool represents whether the event has been fully processed
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_uniform.update_position(
                    (position.x / self.size.width as f64) as f32,
                    (position.y / self.size.height as f64) as f32,
                );
                self.background_colour.r = position.x / self.size.width as f64;
                self.background_colour.g = position.y / self.size.height as f64;
                true
            }
            // WindowEvent::CursorEntered { .. } => {
            //    self.mouse_uniform.update_hovering(true);
            //    true
            //}
            // WindowEvent::CursorLeft { .. } => {
            //    self.mouse_uniform.update_hovering(false);
            //    true
            //}
            // WindowEvent::MouseInput {
            //    state: (),
            //    button: (),
            //    ..
            //} => {
            //    todo!()
            //}
            _ => false,
        }
    }

    pub(super) fn update(&mut self) {
        self.time_uniform.update_time(self.start_time);
        self.queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[self.time_uniform]),
        );
        self.queue.write_buffer(
            &self.mouse_buffer,
            0,
            bytemuck::cast_slice(&[self.mouse_uniform]),
        );
    }

    pub(super) fn render(&mut self) -> Result<(), SurfaceError> {
        // surface gives us somewhere to render to
        let output = self.surface.get_current_texture()?;
        // TextureView for controlling render code interaction with the texture
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        // encoder builds command buffer and creates commands for sending to GPU
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            // where to draw colour to
            color_attachments: &[
                // `[[location(0)]]` in the fragment shader's return val is this attachment
                RenderPassColorAttachment {
                    // render to the TextureView on the screen's surface
                    // in other words, render output will be displayed in the window when it's
                    // submitted and presented
                    view: &view,
                    // defaults to &view if multisampling is off
                    resolve_target: None,
                    // what to do with colours on the screen from `view`
                    ops: Operations {
                        // clear them (because not all screen is covered by objects)
                        load: LoadOp::Clear(self.background_colour),
                        // yes we do want to store the result
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.time_bind_group, &[]);
        render_pass.set_bind_group(1, &self.mouse_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        // draw three vertices with one instance
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1 as _);

        // drop render pass (which owns a &mut encoder) so it can be .finish()ed
        drop(render_pass);
        // submit() takes any IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
