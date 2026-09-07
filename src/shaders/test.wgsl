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
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let x = (model.position.x / globals.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (model.position.y / globals.screen_size.y) * 2.0;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = model.color;
    out.uv = model.uv;
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1313);
    p3 += dot(p3, p3.yzx + 3.333);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;

    let cos_rot = 0.87758;
    let sin_rot = 0.47942;
    let rot = mat2x2<f32>(cos_rot, sin_rot, -sin_rot, cos_rot);

    for (var i = 0; i < 5; i++) {
        v += a * value_noise(pos);
        pos = rot * pos * 2.0 + vec2<f32>(100.0, 100.0);
        a *= 0.5;
    }
    return v;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv.xy * 2.0 - 1.0;

    uv.y += sin(globals.time * 2);
    let sky_color = vec3<f32>(0.2, 0.45, 0.70);
    if (uv.y <= 0.001) {
        return vec4<f32>(sky_color, 1.0);
    }

    let depth = abs(sin(globals.time)*2.0) / uv.y;
    var surface_uv = vec2<f32>(uv.x * depth, depth );

    surface_uv.y += globals.time * 2.0;
    surface_uv *= 3.0;

    let height = fbm(surface_uv);

    let green_color = vec3<f32>(0.16, 0.45, 0.03);
    let green2_color = vec3<f32>(0.17, 0.5, 0.);
    var ground_color = mix(green_color, green2_color, smoothstep(0.2, 0.8, height));

    let fog = exp(-depth * 0.15);
    let final_color = mix(sky_color, ground_color, smoothstep(0.0, abs(sin(globals.time)*.5)+0.2, fog));

    return vec4<f32>(final_color, 1.0);
}