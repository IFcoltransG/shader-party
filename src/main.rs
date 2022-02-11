// using https://sotrh.github.io/learn-wgpu/

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod controller;
mod texture;

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // can't bytemuck a cgmath matrix
    view_proj: [[f32; 4]; 4],
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    background_colour: wgpu::Color,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: controller::CameraController,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const VERTICES: &[Vertex] = &[
    // Changed
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new(); // make an event loop
    let window = WindowBuilder::new().build(&event_loop).unwrap(); // make a window from it
    let mut state = pollster::block_on(State::new(&window)); // could also use an async main with a crate

    event_loop.run(move |event, _, control_flow| match event {
        // start running
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            // prioritise surface handling event
            if !state.input(event) {
                // main should handle event
                match event {
                    // if window event for right window...
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape), // close or escape
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit, // exit
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // deref it twice because it's &&mut
                        state.resize(**new_inner_size);
                    }
                    _ => {} // do nothing
                }
            }
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // reconfig the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // quit if out of memory
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // should resolve other errors, (Outdated, Timeout), by next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // only one RedrawRequested will happen automatically
            // so request it manually
            window.request_redraw();
        }
        _ => {}
    });
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into()
    }
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

impl State {
    // need async for creating some wgpu types
    async fn new(window: &Window) -> Self {
        // make sure dimensions are nonzero (or crash)
        let size = window.inner_size();

        // instance is a handle to the GPU
        // Backends::all = Vulkan, Metal, DX12, Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all()); // for making adapters and surfaces
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        // request a device with that adapter
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(), // no features
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // trace path
            )
            .await
            .unwrap();
        // config for the surface
        let config = wgpu::SurfaceConfiguration {
            // allows rendering textures to screen
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // choose texture format to match what the screen prefers
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            // vsync on, is the only good option on mobile devices
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        // bitfield options: NONE, VERTEX, FRAGMENT, COMPUTE
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            // SamplerBindingType::Comparison for TextureSampleType::Depth
                            // ::Filtering for ::Float { filterable: true }
                            // Otherwise, error
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
        });

        let camera = Camera {
            // camera is 1.0 upwards and 2.0 backwards (z coord is normal to screen)
            eye: (0.0, 1.0, 2.0).into(),
            // at origin
            target: (0.0, 0.0, 0.0).into(),
            // up is the y axis
            up: cgmath::Vector3::unit_x(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        // buffer size will not change
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let camera_controller = controller::CameraController::new(0.2);

        // load shader from file
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    // same format as the surface for easier copying
                    format: config.format,
                    // don't care about old pixels, just replace them
                    blend: Some(wgpu::BlendState::REPLACE),
                    // write to every colour channel including alpha
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            // how to interpret vertices as triangles
            primitive: wgpu::PrimitiveState {
                // chunk vertices into triplets as triangles
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // a triangle is facing forward whenever vertices are arranged 'counter-clockwise'
                front_face: wgpu::FrontFace::Ccw,
                // cull back faces
                cull_mode: Some(wgpu::Face::Back),
                // must be Fill unless GPU supports NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // must be false unless DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // must be false unless CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            // use one buffer
            multisample: wgpu::MultisampleState {
                // only one sample
                count: 1,
                // bits set to use all samples
                mask: !0,
                // no antialiasing
                alpha_to_coverage_enabled: false,
            },
            // not using array textures
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        // a bluish colour as default
        let background_colour = wgpu::Color {
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
            vertex_buffer,
            index_buffer,
            num_indices,
            background_colour,
            diffuse_bind_group,
            diffuse_texture,
            camera,
            camera_bind_group,
            camera_uniform,
            camera_buffer,
            camera_controller,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        // bool represents whether the event has been fully processed
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.background_colour.r = position.x / self.size.width as f64;
                self.background_colour.g = position.y / self.size.height as f64;
                true
            }
            _ => self.camera_controller.process_events(event),
        }
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        // TextureView for controlling how render code interaction with the texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // encoder builds command buffer and creates commands for sending to GPU
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            // where to draw colour to
            color_attachments: &[
                // `[[location(0)]]` in the fragment shader is this attachment
                wgpu::RenderPassColorAttachment {
                    // render to the TextureView on the screen's surface
                    view: &view,
                    // defaults to &view if multisampling is off
                    resolve_target: None,
                    // what to do with colours on the screen from `view`
                    ops: wgpu::Operations {
                        // clear them (because not all screen is covered by objects)
                        load: wgpu::LoadOp::Clear(self.background_colour),
                        // yes we do want to store the result
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // draw three vertices with one instance
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        // drop render pass (which owns a &mut encoder) so it can be .finish()ed
        drop(render_pass);
        // submit() takes any IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
