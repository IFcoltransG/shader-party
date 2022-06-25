use bytemuck::{Pod, Zeroable};
use wgpu::{VertexBufferLayout, BufferAddress, vertex_attr_array, VertexAttribute, VertexStepMode};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

// square, a quad's corners
pub(super) const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 0.0],
    }, // Top left
    Vertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 0.0],
    }, // Top right
    Vertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 1.0],
    }, // Bottom left
    Vertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 1.0],
    }, // Bottom right
];

// a simple quad shape
pub(super) const INDICES: &[u16] = &[2, 3, 0, 1, 2, 0];

impl Vertex {
    const ATTRIBS: [VertexAttribute; 2] = vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub(super) fn desc<'pipeline>() -> VertexBufferLayout<'pipeline> {
        use std::mem;
        VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
