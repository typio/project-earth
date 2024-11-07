@group(0) @binding(0) var pixels: texture_2d<f32>;
@group(0) @binding(1) var<uniform> screenResolution: vec2<f32>;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
	@location(0) tex_coords: vec2<f32>
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );


    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    var out: VertexOutput;
    out.pos = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.tex_coords = uv[vertex_index];
    return out;
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>, @location(0) tex_coords: vec2<f32>) -> @location(0) vec4<f32> {
    let coords = vec2<u32>(u32(tex_coords.x * screenResolution.x), u32(tex_coords.y * screenResolution.y));
    return textureLoad(pixels, coords, 0);
}
