@group(0) @binding(0) var out_tex: texture_storage_2d<rgba16float, write>;

fn render(id: vec2<u32>) -> vec4<f32> {
    let coord = vec2<f32>(id);
    let dims = vec2<f32>(textureDimensions(out_tex));

    var color = vec4(1.0, 1.0, 1.0, 1.0);
    return color;
}

@compute @workgroup_size(8, 8, 1)
fn comp(@builtin(global_invocation_id) id: vec3<u32>) {
    let color = render(id.xy);
    textureStore(out_tex, id.xy, color);
}
