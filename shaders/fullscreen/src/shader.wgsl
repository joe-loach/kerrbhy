struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
};

@vertex
fn vert(@builtin(vertex_index) index: u32) -> VertexOutput {
    // creates a full screen triangle using clever bit manipulation
    // the gpu then clips this automatically
    // https://wallisc.github.io/rendering/2021/04/18/Fullscreen-Pass.html
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
    // for the fragment shader:
    // sample the input texture at the uv coordinate
    // output the color with full alpha
    var uv = vec2<f32>(
        in.uv.x,
        1.0 - in.uv.y
    );
    let color = textureSample(color_texture, color_sampler, uv).rgb;
    return vec4<f32>(color, 1.0);
}

