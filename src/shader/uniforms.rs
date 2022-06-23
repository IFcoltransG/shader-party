use bytemuck::{Pod, Zeroable};
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct TimeUniform {
    time: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct MouseUniform {
    cursor_pos: [f32; 2],
    // click_time: [u32; 3],
    // clicking: [u8; 3],
    // cursor_over_window: u8,
}

impl TimeUniform {
    pub(super) fn new() -> Self {
        Self { time: 0 }
    }

    pub(super) fn update_time(&mut self, start_time: Instant) {
        // update time to number of milliseconds since program start
        self.time = start_time.elapsed().as_millis() as u32
    }
}

impl MouseUniform {
    pub(super) fn new() -> Self {
        Self {
            cursor_pos: [0.0, 0.0],
        }
    }

    pub(super) fn update_position(&mut self, x: f32, y: f32) {
        // update cursor position
        // y axis is reversed from GPU coords
        self.cursor_pos = [x, 1.0 - y];
    }

    // fn update_hovering(&mut self, hovering_over_window: bool) {
    //    todo!()
    //}
}
