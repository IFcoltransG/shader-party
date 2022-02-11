// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>;
};

[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

// input from vertex buffer
struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

// output of vertex shader
struct VertexOutput {
    // WGPU detects this as vertex clip coordinates
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

// WGPU detects this as a valid entry point for a vertex shader
[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    // `var` variables need a type annotation and are mutable
    // (`let` variables are immutable and infer their type)
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
// WGPU detects that it should store the vec4 return value in the colour target at index 0
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // set colour to brown
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
