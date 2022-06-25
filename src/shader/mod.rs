use std::{fs, time::Instant};
use wgpu::{util::DeviceExt, *};
use winit::{dpi::PhysicalSize, event::*, window::Window};

mod geometry;
mod uniforms;

use self::{
    geometry::{Vertex, INDICES, VERTICES},
    uniforms::{
        bindings::{Uniform, UniformBinding},
        MouseUniform, TimeUniform,
    },
};
use super::config::Config;

fn new_shader(device: &Device, path: &str) -> ShaderModule {
    log::info!("Reading shader");

    // load shader from file
    // let shader_source = include_str!("shader.wgsl").into();
    let shader_source = fs::read_to_string(path)
        .expect("Failed reading shader")
        .into();
    device.create_shader_module(&ShaderModuleDescriptor {
        label: Some("Shader"),
        source: ShaderSource::Wgsl(shader_source),
    })
}

fn new_pipeline(
    device: &Device,
    surface_config: &SurfaceConfiguration,
    render_pipeline_layout: &PipelineLayout,
    shader: ShaderModule,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(render_pipeline_layout),
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
                format: surface_config.format,
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

#[derive(Debug)]
pub(super) struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    size: PhysicalSize<u32>,
    surface_config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    render_pipeline_layout: PipelineLayout,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    background_colour: Color,
    start_time: Instant,
    time: UniformBinding<TimeUniform>,
    mouse: UniformBinding<MouseUniform>,
    config: Config,
}

impl State {
    // need async for creating some wgpu types
    pub(super) async fn new(window: &Window, config: Config) -> Self {
        // make sure dimensions are nonzero (or crash)
        let size = window.inner_size();

        // GET GPU DEVICE
        log::debug!("Setting up GPU device");

        // instance is a handle to the GPU
        // Backends::all = Vulkan, Metal, DX12, Browser WebGPU
        let instance = wgpu::Instance::new(Backends::all()); // for making adapters and surfaces
                                                             // SAFETY: window has to allow creating surface and reference must remain valid
                                                             // until surface dropped
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Could not find GPU adapter");
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
            .expect("Could not acquire GPU device");
        // config for the surface
        log::debug!("Configuring surface");
        let surface_config = SurfaceConfiguration {
            // allows rendering textures to screen
            usage: TextureUsages::RENDER_ATTACHMENT,
            // choose texture format to match what the screen prefers
            format: surface
                .get_preferred_format(&adapter)
                .expect("Couldn't get adapter preferred surface format"),
            width: size.width,
            height: size.height,
            // vsync on, is the only good option on mobile devices
            present_mode: PresentMode::Fifo,
        };
        surface.configure(&device, &surface_config);

        log::debug!("Setting up uniform bindings");

        // TIME BINDING
        let start_time = Instant::now();
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
        let time = TimeUniform::new(start_time).make_binding(&device, &time_bind_group_layout);

        // MOUSE BINDING
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

        let mouse = MouseUniform::new().make_binding(&device, &mouse_bind_group_layout);

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
        let shader = new_shader(&device, &config.path);

        // COLLECT BIND GROUPS AND SHADERS INTO PIPELINE

        let render_pipeline =
            new_pipeline(&device, &surface_config, &render_pipeline_layout, shader);

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
            size,
            surface_config,
            render_pipeline,
            render_pipeline_layout,
            vertex_buffer,
            index_buffer,
            num_indices,
            background_colour,
            start_time,
            time,
            mouse,
            config,
        }
    }

    pub(super) fn refresh_shader(&mut self) {
        self.render_pipeline = new_pipeline(
            &self.device,
            &self.surface_config,
            &self.render_pipeline_layout,
            new_shader(&self.device, &self.config.path),
        )
    }

    pub(super) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub(super) fn current_size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub(super) fn input(&mut self, event: &WindowEvent) -> bool {
        // bool represents whether the event has been fully processed
        match *event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.uniform_mut().update_position(
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
        self.time.uniform_mut().update_time(self.start_time);
        self.queue.write_buffer(
            self.time.buffer(),
            0,
            bytemuck::cast_slice(&[*self.time.uniform()]),
        );
        self.queue.write_buffer(
            self.mouse.buffer(),
            0,
            bytemuck::cast_slice(&[*self.mouse.uniform()]),
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
        render_pass.set_bind_group(0, self.time.bind_group(), &[]);
        render_pass.set_bind_group(1, self.mouse.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        // draw three vertices with one instance
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1_u32);

        // drop render pass (which owns a &mut encoder) so it can be .finish()ed
        drop(render_pass);
        // submit() takes any IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
