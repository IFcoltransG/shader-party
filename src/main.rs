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
        // set to bluish
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
                // set colour based on distance of mouse from top left
                self.colour.r = position.x / self.size.width as f64;
                self.colour.g = position.y / self.size.height as f64;
                // main should not process further
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
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            // where to draw colour to
            color_attachments: &[wgpu::RenderPassColorAttachment {
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
            }],
            depth_stencil_attachment: None,
        });
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
