//! Turns game state into GPU-ready data: placeholder meshes, camera, light + post uniforms.
//! Pure CPU code (no wgpu), so it's unit-testable.
//!
//! All art here is primitive-built placeholder. When Blender models arrive (glTF, milestone 2),
//! each `draw_*` function is replaced by "append this loaded mesh at this transform".

use crate::data::*;
use crate::game::*;
use crate::math::*;
use crate::mesh::*;
use crate::world::*;
use std::f32::consts::{PI, TAU};

/// 16 (view_proj) + 6*4 (cam, player, sun_dir, sun_col, ambient, fog) + 16*4 (lights).
pub const GLOBALS_FLOATS: usize = 104;
pub const POST_FLOATS: usize = 12;

const CAM_DIST: f32 = 37.0;
const CAM_PITCH_DEG: f32 = 66.0; // 90 = straight down
const FOV_DEG: f32 = 30.0;

const V0: Vec3 = Vec3::ZERO;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub fn camera_eye(game: &Game) -> (Vec3, Vec3) {
    let target = game.cam;
    let pitch = CAM_PITCH_DEG.to_radians();
    (target + v(0.0, CAM_DIST * pitch.sin(), CAM_DIST * pitch.cos()), target)
}

pub fn build_globals(game: &Game, aspect: f32) -> Vec<f32> {
    let (eye, target) = camera_eye(game);
    let proj = Mat4::perspective_rh(FOV_DEG.to_radians(), aspect, 1.0, 140.0);
    let view = Mat4::look_at_rh(eye, target, v(0.0, 1.0, 0.0));
    let vp = proj.mul(&view);
    let l = game.env.lighting();
    let p = game.player.pos;

    let mut out = Vec::with_capacity(GLOBALS_FLOATS);
    out.extend_from_slice(&vp.to_array());
    out.extend_from_slice(&[eye.x, eye.y, eye.z, game.time]);
    out.extend_from_slice(&[p.x, p.y, p.z, 0.0]);
    out.extend_from_slice(&[l.sun_dir.x, l.sun_dir.y, l.sun_dir.z, l.sun_i]);
    out.extend_from_slice(&[l.sun_col[0], l.sun_col[1], l.sun_col[2], 0.0]);
    out.extend_from_slice(&[l.ambient[0], l.ambient[1], l.ambient[2], 0.0]);
    out.extend_from_slice(&[l.fog_col[0], l.fog_col[1], l.fog_col[2], l.fog_density]);
    for light in game.lights() {
        out.extend_from_slice(&light);
    }
    debug_assert_eq!(out.len(), GLOBALS_FLOATS);
    out
}

pub fn build_post(game: &Game, low_w: u32, low_h: u32, scale: u32) -> [f32; POST_FLOATS] {
    let l = game.env.lighting();
    [
        low_w as f32,
        low_h as f32,
        scale as f32,
        game.time,
        l.rain,
        l.fog_amt,
        l.night,
        0.42,
        l.fog_col[0],
        l.fog_col[1],
        l.fog_col[2],
        0.0,
    ]
}

/// Background color the scene pass clears to (matches the fog so the horizon blends away).
pub fn clear_color(game: &Game) -> [f32; 3] {
    game.env.lighting().fog_col
}

// ------------------------------------------------------------------ static world

pub fn build_static(world: &World) -> Mesh {
    let mut m = Mesh::new();

    m.ground(140.0, 0.0, [0.24, 0.40, 0.27]);

    // Cottage yard and the stone circle in the lantern glade.
    m.cylinder(v(world.home.x, 0.0, world.home.z), 7.0, 0.05, 22, [0.36, 0.32, 0.29], 0.0);
    m.cylinder(v(world.glade.x, 0.0, world.glade.z), 8.5, 0.05, 24, [0.34, 0.34, 0.42], 0.0);

    for t in &world.trees {
        let base = v(t.x, 0.0, t.z);
        let trunk = [0.27, 0.19, 0.16];
        if t.kind == 0 {
            m.cylinder(base, 0.3, t.h * 0.35, 6, trunk, 0.0);
            let g = [0.12 + 0.07 * t.tint, 0.28 + 0.10 * t.tint, 0.26 + 0.05 * t.tint];
            for k in 0..3 {
                let kf = k as f32;
                let c = [g[0] * (0.85 + 0.12 * kf), g[1] * (0.85 + 0.12 * kf), g[2] * (0.85 + 0.12 * kf)];
                let r = (1.25 - kf * 0.32) * (t.h / 4.0);
                m.cone(v(t.x, t.h * (0.22 + 0.24 * kf), t.z), r, t.h * 0.36, 7, c, 0.0);
            }
        } else {
            m.cylinder(base, 0.32, t.h * 0.55, 6, trunk, 0.0);
            let crown = if t.tint < 0.5 { [0.85, 0.45, 0.15] } else { [0.45, 0.28, 0.55] };
            m.ellipsoid(v(t.x, t.h * 0.68, t.z), v(1.35, 1.0, 1.35), 4, 7, crown, 0.0);
            let top = [crown[0] * 1.15, crown[1] * 1.15, crown[2] * 1.15];
            m.ellipsoid(v(t.x + 0.3, t.h * 0.68 + 0.8, t.z - 0.15), v(0.85, 0.65, 0.85), 3, 6, top, 0.0);
        }
    }

    for r in &world.rocks {
        m.set_xf(v(r.x, 0.0, r.z), r.yaw, Vec3::ONE);
        m.ellipsoid(v(0.0, r.sy * 0.6, 0.0), v(r.sx, r.sy, r.sz), 3, 6, [0.38, 0.37, 0.45], 0.0);
    }
    m.reset_xf();

    let caps = [[0.40, 0.90, 0.80], [0.90, 0.50, 0.90], [1.00, 0.60, 0.30]];
    for s in &world.shrooms {
        m.set_xf_u(v(s.x, 0.0, s.z), 0.0, s.s);
        m.cylinder(V0, 0.05, 0.25, 5, [0.9, 0.88, 0.8], 0.0);
        m.ellipsoid(v(0.0, 0.27, 0.0), v(0.2, 0.1, 0.2), 2, 6, caps[s.hue as usize % 3], 0.65);
    }
    m.reset_xf();

    build_cottage(&mut m, world.home);

    // Lantern posts in the glade (the lamp itself is dynamic: it lights up).
    for l in &world.lanterns {
        m.cylinder(v(l.x, 0.0, l.z), 0.32, 0.25, 8, [0.45, 0.45, 0.52], 0.0);
        m.cylinder(v(l.x, 0.25, l.z), 0.08, 1.95, 6, [0.20, 0.17, 0.22], 0.0);
    }
    m
}

fn build_cottage(m: &mut Mesh, h: Vec3) {
    m.set_xf(h, 0.0, Vec3::ONE);
    m.box_(v(0.0, 1.3, 0.0), v(2.6, 1.3, 2.3), [0.58, 0.42, 0.36], 0.0);
    // Pyramid roof, turned 45 degrees to line up with the walls.
    m.set_xf(h, PI / 4.0, Vec3::ONE);
    m.cone(v(0.0, 2.55, 0.0), 4.5, 2.5, 4, [0.38, 0.22, 0.46], 0.0);
    m.set_xf(h, 0.0, Vec3::ONE);
    m.box_(v(1.7, 3.5, -0.9), v(0.36, 0.9, 0.36), [0.42, 0.40, 0.46], 0.0);
    m.box_(v(0.0, 0.9, 2.31), v(0.5, 0.9, 0.06), [0.30, 0.17, 0.12], 0.0);
    m.box_(v(0.0, 0.1, 2.9), v(0.9, 0.1, 0.5), [0.45, 0.44, 0.5], 0.0);
    for sx in [-1.6f32, 1.6] {
        m.box_(v(sx, 1.5, 2.31), v(0.42, 0.42, 0.05), [1.0, 0.80, 0.40], 1.0);
        m.box_(v(sx, 1.5, 2.28), v(0.5, 0.5, 0.04), [0.25, 0.15, 0.12], 0.0);
    }
    // Pumpkins by the door.
    for (i, (px, pz, s)) in [(-2.2f32, 3.2f32, 1.0f32), (-1.5, 3.6, 0.7), (2.3, 3.3, 0.9), (1.5, 3.8, 0.6), (3.1, 2.4, 0.7)]
        .iter()
        .enumerate()
    {
        let _ = i;
        m.ellipsoid(v(*px, 0.38 * s, *pz), v(0.5 * s, 0.38 * s, 0.5 * s), 3, 7, [0.95, 0.45, 0.10], 0.12);
        m.cylinder(v(*px, 0.72 * s, *pz), 0.07 * s, 0.2 * s, 5, [0.30, 0.45, 0.20], 0.0);
    }
    m.reset_xf();

    // A ring of little stones marking the garden where friends can be handed over.
    for i in 0..26 {
        let a = TAU * i as f32 / 26.0;
        if a > 1.2 && a < 1.95 {
            continue; // leave a gap at the front path
        }
        m.ellipsoid(v(h.x + a.cos() * 5.8, 0.15, h.z + a.sin() * 5.8), v(0.35, 0.2, 0.3), 2, 5, [0.5, 0.48, 0.56], 0.0);
    }
}

// ------------------------------------------------------------------ dynamic scene

fn shadow(m: &mut Mesh, pos: Vec3, r: f32) {
    m.ellipsoid(v(pos.x, 0.07, pos.z), v(r, 0.02, r), 2, 8, [0.09, 0.14, 0.11], 0.0);
}

pub fn build_dynamic(game: &Game, m: &mut Mesh) {
    let t = game.time;
    let night = game.env.night_factor();

    // Treat plants.
    for s in &game.sources {
        let tc = TREATS[s.treat].color;
        if s.ready {
            m.ellipsoid(v(s.pos.x, 0.42, s.pos.z), v(0.85, 0.55, 0.85), 3, 7, [0.16, 0.34, 0.21], 0.0);
            for k in 0..4 {
                let a = TAU * k as f32 / 4.0 + 0.4;
                let bob = (t * 2.0 + k as f32).sin() * 0.04;
                m.sphere(v(s.pos.x + a.cos() * 0.5, 0.78 + bob, s.pos.z + a.sin() * 0.5), 0.13, tc, 0.85);
            }
        } else {
            m.ellipsoid(v(s.pos.x, 0.38, s.pos.z), v(0.8, 0.5, 0.8), 3, 7, [0.12, 0.2, 0.17], 0.0);
        }
    }

    // Lantern lamps.
    for i in 0..3 {
        let l = game.world.lanterns[i];
        let lit = game.lantern_lit[i];
        let (c, e) = if lit { ([0.88, 0.68, 1.0], 1.0) } else { ([0.22, 0.22, 0.30], 0.0) };
        m.box_(v(l.x, 2.25, l.z), v(0.22, 0.28, 0.22), c, e);
        m.cone(v(l.x, 2.53, l.z), 0.32, 0.26, 6, [0.18, 0.15, 0.2], 0.0);
    }

    // Treats on the ground, each with a faint magic circle.
    for d in &game.drops {
        let tc = TREATS[d.kind].color;
        let bob = (t * 3.0 + d.id as f32).sin() * 0.05;
        m.ellipsoid(v(d.pos.x, 0.07, d.pos.z), v(0.6, 0.02, 0.6), 2, 10, tc, 0.55);
        m.ellipsoid(v(d.pos.x, 0.28 + bob, d.pos.z), v(0.22, 0.15, 0.22), 3, 6, tc, 0.8);
    }

    // Spirits living in the cottage garden.
    for sp in 0..N_SPIRITS {
        for k in 0..game.save.residents[sp].min(3) as usize {
            let a = t * 0.25 * (1.0 + 0.3 * k as f32) + sp as f32 * 1.7 + k as f32 * 2.1;
            let r = 4.1 + sp as f32 * 0.45 + k as f32 * 0.5;
            let pos = v(game.world.home.x + a.cos() * r, 0.0, game.world.home.z + a.sin() * r);
            shadow(m, pos, 0.4);
            draw_spirit(m, sp, pos, (-a.sin()).atan2(a.cos()), 0.8, t * 2.4 + a, 0.0);
        }
    }

    // Wild + following spirits.
    for s in &game.spirits {
        let mood = match s.state {
            SpState::Eat(..) => 1.0,
            _ => 0.0,
        };
        shadow(m, s.pos, 0.45 * SPIRITS[s.def].size / 0.55 * s.fade);
        draw_spirit(m, s.def, s.pos, s.yaw, s.fade, s.phase, mood);
        if let SpState::Eat(..) = s.state {
            for k in 0..3 {
                let a = t * 2.5 + k as f32 * TAU / 3.0;
                m.sphere(v(s.pos.x + a.cos() * 0.6, 1.9 + (a * 2.0).sin() * 0.1, s.pos.z + a.sin() * 0.6), 0.1, [1.0, 0.5, 0.65], 1.0);
            }
        }
    }

    draw_player(m, game);

    // Befriending sparkles.
    for b in &game.bursts {
        let life = (1.0 - b.age / 1.4).max(0.0);
        for k in 0..8 {
            let a = TAU * k as f32 / 8.0 + b.age;
            let rad = 0.4 + b.age * 1.6;
            m.sphere(v(b.pos.x + a.cos() * rad, 0.7 + b.age * 1.4, b.pos.z + a.sin() * rad), 0.11 * life, b.color, 1.0);
        }
    }

    // Fireflies drifting around you at night.
    if night > 0.15 {
        let c = game.cam;
        for i in 0..40 {
            let f = i as f32;
            let x = c.x + 20.0 * (f * 7.31 + t * 0.07 * (1.0 + (i % 3) as f32 * 0.4)).sin();
            let z = c.z + 12.0 * (f * 3.77 + t * 0.05).cos();
            let y = 0.6 + 1.5 * (0.5 + 0.5 * (f * 2.1 + t * 0.6).sin());
            let blink = ((t * 1.3 + f).sin() * 0.5 + 0.5).powf(0.5);
            m.box_(v(x, y, z), v(0.05, 0.05, 0.05) * (0.4 + blink), [0.85, 1.0, 0.5], 1.0);
        }
    }
}

fn draw_player(m: &mut Mesh, game: &Game) {
    let p = &game.player;
    let bob = if p.moving { p.walk_phase.sin().abs() * 0.07 } else { (game.time * 2.0).sin() * 0.012 };
    shadow(m, p.pos, 0.55);
    m.set_xf_u(p.pos + v(0.0, bob, 0.0), p.facing, 1.2);
    let hood = [0.24, 0.12, 0.40];
    m.cone(V0, 0.58, 1.15, 8, [0.36, 0.20, 0.54], 0.0); // cloak
    m.ellipsoid(v(0.0, 1.28, 0.05), v(0.27, 0.27, 0.27), 3, 6, [0.86, 0.72, 0.62], 0.0); // face
    m.ellipsoid(v(0.0, 1.38, -0.08), v(0.37, 0.33, 0.34), 3, 7, hood, 0.0); // hood
    m.cone(v(0.0, 1.55, -0.12), 0.26, 0.5, 6, hood, 0.0); // pointy tip
    m.sphere(v(-0.1, 1.3, 0.28), 0.05, [0.1, 0.06, 0.12], 0.0);
    m.sphere(v(0.1, 1.3, 0.28), 0.05, [0.1, 0.06, 0.12], 0.0);
    // The lantern swinging at their side.
    m.box_(v(0.46, 0.55, 0.3), v(0.11, 0.15, 0.11), [1.0, 0.78, 0.40], 1.0);
    m.box_(v(0.46, 0.73, 0.3), v(0.14, 0.03, 0.14), [0.15, 0.12, 0.15], 0.0);
    m.reset_xf();
}

/// One spirit. `pos` is its spot on the ground; it hovers above it. `appear` is 0..1 scale.
fn draw_spirit(m: &mut Mesh, def: usize, pos: Vec3, yaw: f32, appear: f32, phase: f32, _mood: f32) {
    let d = &SPIRITS[def];
    let s = d.size * 1.25 * appear.max(0.001);
    let hover = 0.85 * d.size / 0.55 + 0.12 * phase.sin();
    m.set_xf_u(pos + v(0.0, hover * appear, 0.0), yaw, s);

    let c = d.color;
    m.ellipsoid(v(0.0, 0.4, 0.0), v(0.9, 1.0, 0.9), 5, 8, c, 0.55);
    m.frustum(v(0.0, -0.95, 0.0), 0.05, 0.82, 1.35, 8, c, 0.55);
    // Cute face.
    let dark = [0.08, 0.05, 0.12];
    m.sphere(v(-0.3, 0.55, 0.78), 0.16, dark, 0.0);
    m.sphere(v(0.3, 0.55, 0.78), 0.16, dark, 0.0);
    m.sphere(v(-0.52, 0.3, 0.68), 0.1, [1.0, 0.5, 0.6], 0.4);
    m.sphere(v(0.52, 0.3, 0.68), 0.1, [1.0, 0.5, 0.6], 0.4);

    match d.id {
        "pip" => {
            m.cylinder(v(0.0, 1.35, 0.0), 0.11, 0.38, 5, [0.30, 0.60, 0.25], 0.0);
            m.box_(v(0.22, 1.6, 0.0), v(0.2, 0.04, 0.1), [0.30, 0.65, 0.28], 0.1);
        }
        "mizzle" => {
            m.cone(v(0.0, 1.0, 0.0), 0.55, 0.95, 7, c, 0.55);
        }
        "hush" => {
            let flap = (phase * 3.0).sin() * 0.18;
            let wing = [0.92, 0.86, 1.0];
            m.ellipsoid(v(-1.25, 0.65 + flap, -0.1), v(0.85, 0.12, 0.55), 2, 7, wing, 0.6);
            m.ellipsoid(v(1.25, 0.65 + flap, -0.1), v(0.85, 0.12, 0.55), 2, 7, wing, 0.6);
        }
        "luna" => {
            for k in 0..5 {
                let a = TAU * k as f32 / 5.0;
                m.cone(v(a.cos() * 0.55, 1.28, a.sin() * 0.55), 0.14, 0.5, 4, [1.0, 0.88, 0.45], 1.0);
            }
        }
        _ => {}
    }
    m.reset_xf();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Save;

    #[test]
    fn globals_and_post_have_expected_size() {
        let g = Game::new(Save::default(), 1);
        assert_eq!(build_globals(&g, 16.0 / 9.0).len(), GLOBALS_FLOATS);
        assert_eq!(build_post(&g, 640, 360, 3).len(), POST_FLOATS);
    }

    #[test]
    fn camera_looks_down_at_the_player() {
        let g = Game::new(Save::default(), 1);
        let (eye, target) = camera_eye(&g);
        assert!(eye.y > 15.0 && eye.z > target.z);
        let vp = Mat4::perspective_rh(FOV_DEG.to_radians(), 16.0 / 9.0, 1.0, 140.0)
            .mul(&Mat4::look_at_rh(eye, target, v(0.0, 1.0, 0.0)));
        let c = vp.transform_point(target);
        assert!((c[0] / c[3]).abs() < 1e-3);
    }

    /// Not a real test: exports frames so the shaders + art can be checked in a real WebGPU browser
    /// without needing the wasm toolchain. Run: DUMP_DIR=/some/dir cargo test dump_frames -- --ignored
    #[test]
    #[ignore]
    fn dump_frames_for_gpu_harness() {
        use crate::env::Weather;
        let dir = std::env::var("DUMP_DIR").expect("set DUMP_DIR");
        let bytes = |v: &[f32]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        let (lw, lh, scale) = (640u32, 360u32, 3u32);

        let scenarios: Vec<(&str, f32, Weather, Box<dyn Fn(&mut Game)>)> = vec![
            ("dusk_home", 18.8, Weather::Clear, Box::new(|g: &mut Game| {
                g.player.pos = v(2.0, 0.0, 8.0);
                g.save.residents = [1, 1, 0, 0];
                g.spawn_spirit(0, v(6.0, 0.0, 10.0));
                g.drops.push(TreatDrop { id: 900, kind: 0, pos: v(4.5, 0.0, 9.0), ttl: 50.0 });
            })),
            ("day_forest", 12.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.player.pos = v(-14.0, 0.0, 12.0);
            })),
            ("night_fog", 23.0, Weather::Fog, Box::new(|g: &mut Game| {
                g.player.pos = v(-15.0, 0.0, 10.0);
                g.spawn_spirit(2, v(-9.0, 0.0, 8.0));
            })),
            ("night_rain", 1.0, Weather::Rain, Box::new(|g: &mut Game| {
                g.player.pos = v(10.0, 0.0, 15.0);
                g.spawn_spirit(1, v(14.0, 0.0, 13.0));
                g.spawn_spirit(1, v(7.0, 0.0, 12.0));
            })),
            ("glade_luna", 23.5, Weather::Clear, Box::new(|g: &mut Game| {
                g.lantern_lit = [true; 3];
                g.puzzle_solved = true;
                g.player.pos = g.world.glade + v(0.0, 0.0, 6.5);
                g.spawn_spirit(3, g.world.glade + v(1.0, 0.0, 0.0));
            })),
        ];

        for (name, hour, weather, setup) in scenarios {
            let mut g = Game::new(Save::default(), 42);
            g.spirits.clear();
            g.env.hour = hour;
            g.env.set_weather(weather);
            setup(&mut g);
            g.cam = g.player.pos;
            g.cam = g.player.pos;
            // Let weather crossfade and spirits fade in (without anything wandering far).
            for _ in 0..(10.0 / STEP) as usize {
                g.env.hour = hour; // freeze the clock for a stable look
                g.update(STEP, &Controls::default());
            }
            let st = build_static(&g.world);
            let mut dy = Mesh::new();
            build_dynamic(&g, &mut dy);
            let globals = build_globals(&g, lw as f32 / lh as f32);
            let post = build_post(&g, lw, lh, scale);
            let c = clear_color(&g);
            std::fs::write(format!("{dir}/{name}.static.bin"), bytes(&st.data)).unwrap();
            std::fs::write(format!("{dir}/{name}.dyn.bin"), bytes(&dy.data)).unwrap();
            std::fs::write(format!("{dir}/{name}.globals.bin"), bytes(&globals)).unwrap();
            std::fs::write(format!("{dir}/{name}.post.bin"), bytes(&post)).unwrap();
            std::fs::write(
                format!("{dir}/{name}.json"),
                format!("{{\"lw\":{lw},\"lh\":{lh},\"scale\":{scale},\"clear\":[{},{},{}]}}", c[0], c[1], c[2]),
            )
            .unwrap();
        }
    }

    #[test]
    fn static_and_dynamic_meshes_are_well_formed() {
        let mut g = Game::new(Save::default(), 2);
        g.env.hour = 22.0;
        g.spawn_spirit(0, v(3.0, 0.0, 3.0));
        g.spawn_spirit(1, v(-3.0, 0.0, 3.0));
        g.spawn_spirit(2, v(0.0, 0.0, -3.0));
        g.spawn_spirit(3, v(6.0, 0.0, 3.0));
        g.save.residents = [1, 2, 3, 4];
        for _ in 0..120 {
            g.update(STEP, &Controls::default());
        }
        let st = build_static(&g.world);
        let mut dy = Mesh::new();
        build_dynamic(&g, &mut dy);
        for mesh in [&st, &dy] {
            assert!(mesh.vertex_count() > 100 && mesh.vertex_count() % 3 == 0);
            assert!(mesh.data.iter().all(|x| x.is_finite()), "NaN/inf in mesh");
        }
        // Budgets used by the renderer (see gfx.rs): keep the per-frame upload small.
        assert!(dy.vertex_count() < 100_000, "dynamic verts: {}", dy.vertex_count());
        assert!(st.vertex_count() < 400_000, "static verts: {}", st.vertex_count());
    }
}
