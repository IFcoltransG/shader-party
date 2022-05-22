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

    // let dist = to_signed_coords(in.tex_coords) - to_signed_coords(mouse.position);
    // let square_dist = cube(dist);
    // let square = vec3<f32>(fract(square_dist * 3.0 + time_part));
    // let colour = abs(dist.yxy) + in.tex_coords.xxy;
    //let hashed = vec3<u32>(pcg32_hash(seed), pcg32_hash(seed + 1u), pcg32_hash(seed + 2u));
    // return vec4<f32>(vec4<u32>(hashed.xyz, 1u)) / f32(u32(-1)); // vec4<f32>(vec4<u32>(seed)) / f32(u32(-1))
    //let seed = seed(grid_pos.xxyy);
    //let centred = to_signed_coords(in.tex_coords);

    let zoom = mouse.position.x;
    let coords = in.tex_coords * (10.0 + time_part * 0.0) + mouse.position.y;
    let grid_pos = vec2<u32>(grid(coords, zoom));

    let random_grid = rand(grid_pos.xyxy, 0u);

    let top_left = vec2<f32>(0.0, 0.0);
    let top_right = vec2<f32>(0.0, 1.0);
    let bottom_left = vec2<f32>(1.0, 0.0);
    let bottom_right = vec2<f32>(1.0, 1.0);

    let top_left_vector = rand(vec4<u32>(3u), 0u);
    let top_right_vector = rand(vec4<u32>(3u), 0u);
    let bottom_left_vector = rand(vec4<u32>(3u), 0u);
    let bottom_right_vector = rand(vec4<u32>(3u), 0u);

    let top_left_direction = tile(coords, zoom) - vec2<f32>(0.0, 0.0);
    let top_right_direction = tile(coords, zoom) - vec2<f32>(0.0, 1.0);
    let bottom_left_direction = tile(coords, zoom) - vec2<f32>(1.0, 0.0);
    let bottom_right_direction = tile(coords, zoom) - vec2<f32>(1.0, 1.0);

    let top_left_scale = dot(top_left_vector.xy, top_left_direction);
    let top_right_scale = dot(top_right_vector.xy, top_right_direction);
    let bottom_left_scale = dot(bottom_left_vector.xy, bottom_left_direction);
    let bottom_right_scale = dot(bottom_right_vector.xy, bottom_right_direction);

    let top_scale = mix(top_left_scale, top_right_scale, tile(coords, zoom).x);
    let bottom_scale = mix(bottom_left_scale, bottom_right_scale, tile(coords, zoom).x);
    let scale = mix(top_scale, bottom_scale, tile(coords, zoom).y);



    let boolean = vec4<f32>(step(random_grid.x, 0.5));
    let tile_uv = tile(coords, zoom);
    let tile_offset = tile(coords * zoom + 0.5, 1.0) - 0.5;

    let line_res = vec4<f32>(line_or_flip(boolean.x, tile_uv).xyz, 1.0);
    let square_res = cube(tile_offset);

    let one_zero = vec2<f32>(1.0, 0.0);
    let red = one_zero.xyyx;
    let green = one_zero.yxyx;
    let blue = one_zero.yyxx;

    let out = min(line_res, vec4<f32>(square_res * 1.05)) * 2.0;
    // let out = line_res;
    let mixer = vec4<f32>(sqrt(out).x, out.x, out.x * out.x, 1.0);
    return mix(blue * 0.1, red + 0.5 + green * 0.05, mixer);
    // return vec4<f32>(scale);
}


