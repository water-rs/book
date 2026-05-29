fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.45, 0.50, 0.55);
    let b = vec3<f32>(0.42, 0.36, 0.30);
    let c = vec3<f32>(1.00, 0.72, 0.48);
    let d = vec3<f32>(0.02, 0.18, 0.36);
    return a + b * cos(6.28318 * (c * t + d));
}

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let aspect = uniforms.resolution.x / max(uniforms.resolution.y, 1.0);
    let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
    let ripple = sin(length(p) * 18.0);
    let bands = sin((p.x + p.y) * 9.0);
    let glow = smoothstep(0.72, 0.05, length(p));
    let color = palette(0.42 + ripple * 0.08 + bands * 0.06) * glow;
    return vec4<f32>(color, 1.0);
}
