struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
};

@vertex
fn vert(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1) & 2), f32(index & 2));

    var out: VertexOutput;
    out.uv = uv;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);

    return out;
}

@group(0) @binding(0)
var color_texture: texture_2d<f32>;
@group(0) @binding(1)
var color_sampler: sampler;

@fragment
fn frag(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(color_texture, color_sampler, in.uv).rgb;
    return vec4<f32>(color, 1.0);
}

