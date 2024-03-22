const A = 0.35875;
const B = 0.48829;
const C = 0.14128;
const D = 0.01168;

fn aa_filter(coord: vec2<f32>) -> vec2<f32> {
    // https://en.wikipedia.org/wiki/Window_function#Blackman%E2%80%93Harris_window
    // Window functions:
    // "Used to smoothly bring a sampled signal down to zero at the edges of the region"
    let n = 0.5 * rand() + 0.5;
    let w = A - B * cos(2.0 * PI * n) + C * cos(4.0 * PI * n) - D * cos(6.0 * PI * n);
    // add the "smooth offset" to the coordinate
    return coord + (udir2() * 2.0 * w);
}