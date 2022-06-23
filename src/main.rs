// using https://sotrh.github.io/learn-wgpu/

use shader::State;
use wgpu::SurfaceError;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    log::info!("Creating event loop");
    let event_loop = EventLoop::new(); // make an event loop
    log::info!("Creating window");
    let window = WindowBuilder::new().build(&event_loop).unwrap(); // make a window from it
    log::info!("Initialising State");
    let mut state = pollster::block_on(State::new(&window)); // could also use an async main with a crate

    log::info!("Starting event loop");
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
                    } => {
                        log::info!("Exiting");
                        *control_flow = ControlFlow::Exit
                    } // exit
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Return),
                                ..
                            },
                        ..
                    } => {
                        log::info!("Reloading shader");
                        state.refresh_shader()
                    }
                    WindowEvent::Resized(physical_size) => {
                        log::debug!("Resizing");
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        log::debug!("Rescaling");
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
                Err(SurfaceError::Lost) => state.resize(state.current_size()),
                // quit if out of memory
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
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

mod shader;
