use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use winit::window::Window;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    alternate_pipeline: wgpu::RenderPipeline,
    colour: wgpu::Color,
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
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        // load shader from file
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[], // vertex types empty because vertices are specified in the shader
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

        let alternative_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Alternative Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("alt-shader.wgsl").into()),
        });
        let alternate_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &alternative_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &alternative_shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // a bluish colour as default
        let colour = wgpu::Color {
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
            alternate_pipeline,
            colour,
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
                self.colour.r = position.x / self.size.width as f64;
                self.colour.g = position.y / self.size.height as f64;
                true
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Space),
                        ..
                    },
                ..
            } => {
                std::mem::swap(&mut self.alternate_pipeline, &mut self.render_pipeline);
                true
            }
            _ => {
                // main should process further
                false
            }
        }
    }

    fn update(&mut self) {}

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
                        load: wgpu::LoadOp::Clear(self.colour),
                        // yes we do want to store the result
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        // draw something with three vertices and one instance
        render_pass.draw(0..3, 0..1);

        // drop render pass (which owns a &mut encoder) so it can be .finish()ed
        drop(render_pass);
        // submit() takes any IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    // using https://sotrh.github.io/learn-wgpu/
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
