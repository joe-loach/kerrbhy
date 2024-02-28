fn main() -> anyhow::Result<()> {

    wgsl_bindgen::build_shader("src/shader.wgsl")?;

    Ok(())
}
