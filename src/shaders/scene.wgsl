// Scene pass: renders the world into a LOW-RES offscreen target (the chunky pixel look).
// Toon-banded lighting, point lights for lanterns/spirits, distance fog.

struct Globals {
    view_proj: mat4x4<f32>,
    cam: vec4<f32>,        // xyz = camera position, w = time (s)
    player: vec4<f32>,     // xyz = player position
    sun_dir: vec4<f32>,    // xyz = direction TOWARD the light, w = intensity
    sun_col: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,        // rgb = fog color, w = density
    lights: array<vec4<f32>, 16>, // 8 lights x (pos.xyz + radius, color.rgb + intensity)
};

@group(0) @binding(0) var<uniform> g: Globals;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,   // 0 lit .. 1 self-lit, -1 = procedural grass
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
};

@vertex
fn vs(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    o.world = v.pos;
    o.normal = v.normal;
    o.color = v.color;
    o.emissive = v.emissive;
    return o;
}

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

@fragment
fn fs(i: VsOut) -> @location(0) vec4<f32> {
    // See-through dither for tall things (tree canopies, roofs) between the camera and the player.
    let to_player = i.world.xz - g.player.xz;
    if (i.world.y > 2.3 && i.world.z > g.player.z - 0.4 && length(to_player) < 3.2 && length(to_player) > 0.8) {
        let px = vec2<u32>(floor(i.clip.xy));
        if (((px.x + px.y) & 1u) == 0u) {
            discard;
        }
    }

    var base = i.color;
    var emi = i.emissive;
    if (emi < 0.0) {
        // Procedural grass: blocky tufts + big soft patches of lighter/darker green.
        let tuft = hash2(floor(i.world.xz * 1.3));
        let blot = hash2(floor(i.world.xz * 0.12));
        base = base * (0.90 + 0.12 * tuft + 0.14 * blot);
        emi = 0.0;
    }

    let n = normalize(i.normal);
    let ndl = max(dot(n, normalize(g.sun_dir.xyz)), 0.0);
    var band = 0.35;
    if (ndl > 0.25) { band = 0.68; }
    if (ndl > 0.60) { band = 1.0; }
    var light = g.ambient.rgb + g.sun_col.rgb * (g.sun_dir.w * band);

    for (var k = 0u; k < 8u; k = k + 1u) {
        let lp = g.lights[k * 2u];
        let lc = g.lights[k * 2u + 1u];
        let to_l = lp.xyz - i.world;
        let d = length(to_l);
        var att = clamp(1.0 - d / max(lp.w, 0.001), 0.0, 1.0);
        att = att * att;
        att = floor(att * 5.0 + 0.5) / 5.0;   // toon-quantized falloff
        let facing = clamp(dot(n, normalize(to_l)) * 0.5 + 0.6, 0.0, 1.0);
        light = light + lc.rgb * (lc.w * att * facing);
    }

    var col = base * light;
    col = mix(col, base * 1.15, clamp(emi, 0.0, 1.0));

    let dist = length(i.world - g.cam.xyz);
    let f = 1.0 - exp(-g.fog.w * dist);
    col = mix(col, g.fog.rgb, clamp(f, 0.0, 1.0));
    return vec4<f32>(col, 1.0);
}
