fn main() {
    wgsl_bindgen::create_bindings_for("src/shader.wgsl").unwrap();
}
