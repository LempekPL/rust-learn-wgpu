struct Globals {
    screen_size: vec2<f32>,
    time: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let x = (model.position.x / globals.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (model.position.y / globals.screen_size.y) * 2.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = model.color;
    out.uv = model.uv;
    return out;
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;

//    let centered_uv = in.uv - vec2<f32>(0.5, 0.5);
//    let dist = length(centered_uv);
//    let angle = atan2(centered_uv.y, centered_uv.x);
//    let hue = (angle + 3.14159265) / (2.0 * 3.14159265);
//    let saturation = dist * 2.0;
//    let rgb = hsv2rgb(vec3<f32>(hue, saturation, 1.0));
//    return vec4<f32>(rgb, 1.0);
}