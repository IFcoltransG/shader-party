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

// signed distance field for square centred around 0,0
fn cube(coords: vec2<f32>) -> f32 {
    return max(abs(coords.x), abs(coords.y));
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

// Vertex shader runs for each vertex.
// Between vertex shader and fragment shader, GPU will interpolate between vertex outputs
// to get many fragment inputs.
// Then fragment shader runs for each fragment (which are like pixels).

// Fragment shader

[[stage(fragment)]]
// WGPU detects that it should store the vec4 return value in the colour target at index 0
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let time_part = ((f32(time.time) / 6000.0));

    // let dist = to_signed_coords(in.tex_coords) - to_signed_coords(mouse.position);
    // let square_dist = cube(dist);
    // let square = vec3<f32>(fract(square_dist * 3.0 + time_part));
    // let colour = abs(dist.yxy) + in.tex_coords.xxy;
    //let hashed = vec3<u32>(pcg32_hash(seed), pcg32_hash(seed + 1u), pcg32_hash(seed + 2u));
    // return vec4<f32>(vec4<u32>(hashed.xyz, 1u)) / f32(u32(-1)); // vec4<f32>(vec4<u32>(seed)) / f32(u32(-1))
    //let seed = seed(grid.xxyy);
    //let centred = to_signed_coords(in.tex_coords);

    let coords = in.tex_coords * (25.0 + time_part * 0.0);
    let grid = vec2<u32>(floor(coords));
    let random_grid = rand(grid.xyxy, 0u);
    let boolean = vec4<f32>(step(random_grid.x, 0.5));
    let tile_uv = fract(coords);

    let out = vec4<f32>(boolean.xyz * tile_uv.xxx, 1.0);
    return out;
}


