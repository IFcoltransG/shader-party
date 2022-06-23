// Vertex shader

struct Time {
    time: u32;
};

[[group(0), binding(0)]]
// time in milliseconds
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


// hash function for u32s
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
    let hashes = vec4<u32>(
        pcg32_hash(pcg32_hash(input.x) + 0u),
        pcg32_hash(pcg32_hash(input.y) + 1u),
        pcg32_hash(pcg32_hash(input.z) + 2u),
        pcg32_hash(pcg32_hash(input.w) + 3u)
    );
    return pcg32_hash(hashes.x ^ hashes.y ^ hashes.z ^ hashes.w);
}

// turn vec4 of coordinates into vec4 of random outputs in 0..1
fn rand(point: vec4<u32>, seed_value: u32) -> vec4<f32> {
    let input = seed(point) ^ seed_value;
    let max_u32 = u32(-1);
    return vec4<f32>(vec4<u32>(
        pcg32_hash(input + 0u),
        pcg32_hash(input + 1u),
        pcg32_hash(input + 2u),
        pcg32_hash(input + 3u)
    )) / f32(max_u32);
}


// signed distance field for square centred around 0,0
fn cube(coords: vec2<f32>) -> f32 {
    return max(abs(coords.x), abs(coords.y));
}

// signed distance of a circle about 0,0
fn circ(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(length(uv));
}

// diagonal line
fn line(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(abs(dot(uv, vec2<f32>(1.0, -1.0))));
}

// flips line backwards if the flip boolean is set
fn line_or_flip(flip: f32, uv: vec2<f32>) -> vec4<f32> {
    return mix(
        line(uv),
        line(vec2<f32>(uv.y, 1.0 - uv.x)),
        flip
    );
}

// fadeout function
fn fade(t: f32) -> f32 {
    return ((6.0 * t - 15.0) * t + 10.0) * t * t * t;
}

fn cube_to_one(t: f32) -> f32 {
    let v = 1.0 - t;
    return 1.0 - v * v * v;
}

// useful for perlin noise because:
// f(0) = 1
// if |x| >= 1, f(x) = 0
fn falloff(t: f32) -> f32 {
    return 1.0 - (3.0 - 2.0 * abs(t)) * t * t;
}

// opposite of mix, converts from a..b to 0..1
fn from_range(a: f32, b: f32, s: f32) -> f32 {
    return (s - a) / (b - a);
}

// presents a scalar -1..1 as a colour blue..yellow
// for debugging
fn show_scalar(s: f32) -> vec4<f32> {
    return mix(
        vec4<f32>(0.0, 0.0, 1.0, 1.0),
        vec4<f32>(1.0, 1.0, 0.0, 1.0),
        from_range(-1.0, 1.0, s)
    );
}

// draws a 2d surflet about origin, facing in direction of vector
// surflet is Ken Perlin's word for a bump next to the origin, with a negative copy mirrored about the origin
fn surflet_2d(point: vec2<f32>, vector: vec2<f32>) -> f32 {
    let falloff_value = falloff(point.x) * falloff(point.y);
    // let falloff_value = max(0.0, 1.0 - length(point));
    let gradient = dot(normalize(vector), point);
    return gradient * falloff_value;
}

// draws a 3d surflet about origin, facing in direction of vector
// surflet is Ken Perlin's word for a bump next to the origin, with a negative copy mirrored about the origin
fn surflet_3d(point: vec3<f32>, vector: vec3<f32>) -> f32 {
    let falloff_value = falloff(point.x) * falloff(point.y) * falloff(point.z);
    let gradient = dot(normalize(vector), point);
    return gradient * falloff_value;
}

// point is absolute coordinates for where to draw
// corner_direction 0..1 is vector that goes from floor(point) to the corner of the square in question
fn random_surflet_2d(point: vec2<f32>, corner_direction: vec2<f32>, seed: u32) -> f32 {
    let square_origin = floor(point);
    let corner = square_origin + corner_direction;
    // convert to i32 first to avoid saturating negative floats to 0u
    let corner_unsigned = vec2<u32>(vec2<i32>(corner));
    // random vector based on the absolute corner
    let random_vector = rand(corner_unsigned.xyxy, seed) - 0.5;
    let from_corner = point - corner;
    return surflet_2d(from_corner, random_vector.xy);
}

// point is absolute coordinates for where to draw
// corner_direction 0..1 is vector that goes from floor(point) to the corner of the square in question
fn random_surflet_3d(point: vec3<f32>, corner_direction: vec3<f32>, seed: u32) -> f32 {
    let square_origin = floor(point);
    let corner = square_origin + corner_direction;
    // convert to i32 first to avoid saturating negative floats to 0u
    let corner_unsigned = vec3<u32>(vec2<i32>(corner));
    // random vector based on the absolute corner
    let random_vector = rand(corner_unsigned.xyzx, seed) - 0.5;
    let from_corner = point - corner;
    return surflet_3d(from_corner, random_vector.xyz);
}

// perlin noise in 2d
// output in -1..1
fn perlin_2d(point: vec2<f32>, seed: u32) -> f32 {
    var total: f32 = 0.0;

    // iterate through i = 0 and i = 1
    for (var i: i32 = 0; i < 2; i = i + 1) {
        // iterate through j = 0 and j = 1
        for (var j: i32 = 0; j < 2; j = j + 1) {
            // for each vector with components in {0, 1}, which form the corners of a square 0,0 to 1,1
            total = total + random_surflet_2d(point, vec2<f32>(vec2<i32>(i, j)), seed);
        }
    }
    return total;
}

// perlin noise in 3d
// output in -1..1
fn perlin_3d(point: vec3<f32>, seed: u32) -> f32 {
    var total: f32 = 0.0;

    // for loops iterate i, j and k through 0 and 1
    // each (i, j, k) representing the corner of a cube
    for (var i: i32 = 0; i < 2; i = i + 1) {
        for (var j: i32 = 0; j < 2; j = j + 1) {
            for (var k: i32 = 0; k < 2; k = k + 1) {
                // add together the surflets for each cube corner
                total = total + random_surflet_3d(point, vec3<f32>(vec3<i32>(i, j, k)), seed);
            }
        }
    }
    return total;
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

    let zoom = length(mouse.position - 0.5) * 0.0 + 20.0;
    let zoom_centre = vec2<f32>(0.5);
    let coords = (in.tex_coords - zoom_centre) * zoom;

    let noise = vec3<f32>(
        cube_to_one(perlin_3d(vec3<f32>(coords, time_part), 0u)),
        cube_to_one(perlin_3d(vec3<f32>(coords, time_part), 1u)),
        cube_to_one(perlin_3d(vec3<f32>(coords, time_part), 2u))
    );

    return vec4<f32>(0.5 - noise * 0.5, 1.0);
}


