// Constants
const PI: f32 = 3.1415926535897932384626433832795;
const TAU: f32 = 2.0 * PI;
const FRAC_1_PI: f32 = 1.0 / PI;
const FRAC_1_2PI: f32 = 1.0 / (2.0 * PI);

fn isNan(val: vec3<f32>) -> vec3<bool> {
    // a NaN value will return false for all of these, as it has no ordering or eq
    return !(val < vec3<f32>(0.0) || vec3<f32>(0.0) < val || val == vec3<f32>(0.0));
}

fn isInf(val: vec3<f32>) -> vec3<bool> {
    // inf * 2 === inf
    // the only other number to share this property is zero
    return (val != vec3<f32>(0.0) && val * 2.0 == val);
}