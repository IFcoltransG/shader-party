use bytemuck::Pod;
use wgpu::{util::DeviceExt, *};

use super::{MouseUniform, TimeUniform};

#[derive(Debug)]
pub(in crate::shader) struct UniformBinding<T> {
    uniform: T,
    buffer: Buffer,
    bind_group: BindGroup,
}

impl<T> UniformBinding<T> {
    pub(in crate::shader) fn uniform(&self) -> &T {
        &self.uniform
    }

    pub(in crate::shader) fn uniform_mut(&mut self) -> &mut T {
        &mut self.uniform
    }

    pub(in crate::shader) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(in crate::shader) fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}

pub(in crate::shader) trait Uniform {
    const BUFFER_LABEL: &'static str;
    const BIND_GROUP_LABEL: &'static str;

    fn make_binding(
        self,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
    ) -> UniformBinding<Self>
    where
        Self: Sized + Pod,
    {
        let buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[self]),
            usage: BufferUsages::all(),
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Time Bind Group"),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        UniformBinding::<Self> {
            uniform: self,
            buffer,
            bind_group,
        }
    }
}

impl Uniform for TimeUniform {
    const BIND_GROUP_LABEL: &'static str = "Time Bind Group";
    const BUFFER_LABEL: &'static str = "Time Buffer";
}

impl Uniform for MouseUniform {
    const BIND_GROUP_LABEL: &'static str = "Mouse Bind Group";
    const BUFFER_LABEL: &'static str = "Mouse Buffer";
}
