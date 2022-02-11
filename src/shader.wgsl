// Vertex shader

// output of vertex shader
struct VertexOutput {
    // WGPU detects this as vertex clip coordinates
    [[builtin(position)]] clip_position: vec4<f32>;
};

// WGPU detects this as a valid entry point for a vertex shader
[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutput {
    // `var` variables need a type annotation and are mutable
    var out: VertexOutput;
    // `let` variables are immutable and infer their type
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader

[[stage(fragment)]]
// WGPU detects that it should store the vec4 return value in the colour target at index 0
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // set colour to brown
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
