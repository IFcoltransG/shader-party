// Vertex shader

struct Time {
    time: u32;
};

[[group(0), binding(0)]]
var<uniform> time: Time;

struct Mouse {
    position: vec2<f32>;
};

[[group(1), binding(0)]]
var<uniform> mouse: Mouse;

// input for the vertex buffer
struct VertexInput {
    // position relative to parent
    [[location(0)]] position: vec3<f32>;
    // UV, or where on the texture this will be
    [[location(1)]] tex_coords: vec2<f32>;
};

// output of vertex shader
// creates one of these structs per vertex
// for every fragment, GPU then interpolates the struct linearly between its three vertices
struct VertexOutput {
    // WGPU detects this as vertex clip coordinates
    [[builtin(position)]] clip_position: vec4<f32>;
    // arbitrary data produced by vert shader, can be used as a position within the provided texture
    [[location(0)]] tex_coords: vec2<f32>;
};

// WGPU detects this as a valid entry point for a vertex shader
[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    // `var` variables need a type annotation and are mutable
    var out: VertexOutput;
    // `let` variables are immutable and infer their type
    // just passes the data through
    out.tex_coords = model.tex_coords;
    //out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// convert from a range 0..1 to -1..1
fn to_signed_coords(coords: vec2<f32>) -> vec2<f32> {
    return mix(vec2<f32>(-1.0), vec2<f32>(1.0), coords);
}

fn pcg32_hash(input: u32) -> u32 {
    // from https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering/
    // mixed congruential step first
    let state = input * 747796405u + 2891336453u;
    // RXS-M-XS 32bit on state
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word; // XSH 22 (xorshift)
}

// random u32 from vec of 4 u32s
fn seed(input: vec4<u32>) -> u32 {
    return pcg32_hash(
        pcg32_hash(pcg32_hash(input.x) + 0u) ^
        pcg32_hash(pcg32_hash(input.y) + 1u) ^
        pcg32_hash(pcg32_hash(input.z) + 2u) ^
        pcg32_hash(pcg32_hash(input.w) + 3u)
    );
}

// turn vec4 of coordinates
fn rand(point: vec4<u32>, seed_value: u32) -> vec4<f32> {
    let input = seed(point) ^ seed_value;
    return vec4<f32>(vec4<u32>(
        pcg32_hash(input + 0u),
        pcg32_hash(input + 1u),
        pcg32_hash(input + 2u),
        pcg32_hash(input + 3u)
    )) / f32(u32(-1));
}


// signed distance field for square centred around 0,0
fn cube(coords: vec2<f32>) -> f32 {
    return max(abs(coords.x), abs(coords.y));
}

// signed distance of a circle about 0.5, 0.5
fn circ(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(distance(uv, vec2<f32>(0.5)));
}

// diagonal line
fn line(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(abs(dot(uv, vec2<f32>(1.0, -1.0))));
}

// flips line backwards if the boolean is set
fn line_or_flip(flip: f32, uv: vec2<f32>) -> vec4<f32> {
    return mix(
        line(uv),
        line(vec2<f32>(uv.y, 1.0 - uv.x)),
        flip
    );
}

fn tile(pos: vec2<f32>, zoom: f32) -> vec2<f32> {
    return fract(pos * zoom);
}

fn grid(pos: vec2<f32>, zoom: f32) -> vec2<f32> {
    return floor(pos * zoom);
}
// Vertex shader runs for each vertex.
// Between vertex shader and fragment shader, GPU will interpolate between vertex outputs
// to get many fragment inputs.
// Then fragment shader runs for each fragment (which are like pixels).


// Fragment shader

[[stage(fragment)]]
// WGPU detects that it should store the vec4 return value in the colour target at index 0
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let time_part = ((f32(time.time) / 6000.0));

    let zoom = mouse.position.x * 2.0;
    let coords = in.tex_coords * (10.0 + time_part * 0.0) + mouse.position.y;

    let within_tile = tile(coords, zoom);
    let tile_number = grid(coords, zoom);

    // let here = vec2<f32>(0.0, 0.0);
    let up = vec2<f32>(0.0, 1.0);
    let right = vec2<f32>(1.0, 0.0);

    // corners at each compass direction
    let nw = tile_number + up;
    let ne = tile_number + up + right;
    let sw = tile_number;
    let se = tile_number + right;

    let nw_noise = rand(vec4<u32>(nw.xyxy), 0u);
    let ne_noise = rand(vec4<u32>(ne.xyxy), 0u);
    let sw_noise = rand(vec4<u32>(sw.xyxy), 0u);
    let se_noise = rand(vec4<u32>(se.xyxy), 0u);

    let n_noise = mix(nw_noise, ne_noise, within_tile.x);
    let s_noise = mix(sw_noise, se_noise, within_tile.x);
    let value_noise = mix(n_noise, s_noise, 1.0 - within_tile.y);

    return vec4<f32>(value_noise.xyz, 1.0);
}


