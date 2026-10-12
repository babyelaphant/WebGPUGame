// Post pass: integer-scales the low-res scene to the screen with NO filtering (crisp pixels),
// then adds weather (rain streaks, drifting fog) and a vignette, all on the low-res pixel grid.

struct Post {
    a: vec4<f32>, // low_w, low_h, scale, time
    b: vec4<f32>, // rain, fog, night, vignette
    c: vec4<f32>, // fog color rgb, fade-to-black (a)
};

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var<uniform> p: Post;

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // One oversized triangle covering the screen.
    var xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(xy[vi], 0.0, 1.0);
}

fn hash1(x: f32) -> f32 {
    return fract(sin(x * 127.1) * 43758.5453);
}

fn hash2(q: vec2<f32>) -> f32 {
    let s = fract(q * vec2<f32>(123.34, 456.21));
    let r = s + dot(s, s + 45.32);
    return fract(r.x * r.y);
}

fn vnoise(q: vec2<f32>) -> f32 {
    let i = floor(q);
    let f = fract(q);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(q0: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var q = q0;
    for (var k = 0; k < 4; k = k + 1) {
        v = v + amp * vnoise(q);
        q = q * 2.03 + vec2<f32>(17.0, 9.0);
        amp = amp * 0.5;
    }
    return v;
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let scale = p.a.z;
    let dims = vec2<i32>(textureDimensions(scene_tex));
    let cell = clamp(vec2<i32>(floor(frag.xy / scale)), vec2<i32>(0, 0), dims - vec2<i32>(1, 1));
    var col = textureLoad(scene_tex, cell, 0).rgb;

    let time = p.a.w;
    let rain = p.b.x;
    let fog = p.b.y;
    let vig = p.b.w;
    let lp = vec2<f32>(cell);
    let uv = lp / p.a.xy;

    // Drifting fog banks.
    if (fog > 0.001) {
        let n = fbm(uv * vec2<f32>(p.a.x / p.a.y, 1.0) * 3.2 + vec2<f32>(time * 0.03, time * 0.012));
        col = mix(col, p.c.rgb + vec3<f32>(0.12), fog * 0.55 * smoothstep(0.20, 0.80, n));
    }

    // Rain: slanted one-pixel streaks that fall at a per-column speed.
    if (rain > 0.001) {
        let sx = floor(lp.x + lp.y * 0.30);
        let speed = 26.0 + hash1(sx) * 34.0;
        let yy = lp.y - time * speed + hash1(sx * 1.7) * 100.0;
        let seg = floor(yy / 10.0);
        let on = step(1.0 - 0.05 * rain, hash1(sx * 13.7 + seg * 7.3));
        let tail = fract(yy / 10.0);
        col = mix(col, vec3<f32>(0.78, 0.88, 1.0), on * (0.2 + 0.5 * tail) * 0.7 * rain);
        col = col * mix(vec3<f32>(1.0), vec3<f32>(0.93, 0.96, 1.03), rain);
    }

    let d = distance(uv, vec2<f32>(0.5, 0.5));
    col = col * (1.0 - vig * smoothstep(0.35, 0.95, d));
    col = col * (1.0 - p.c.w);
    return vec4<f32>(col, 1.0);
}
