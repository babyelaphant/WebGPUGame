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
    let l = game.lighting();
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
    let l = game.lighting();
    // The "Weather effects" and "Vignette" options only change how the screen looks, never the light.
    let wx = game.settings.weather_mul();
    [
        low_w as f32,
        low_h as f32,
        scale as f32,
        game.time,
        l.rain * wx,
        l.fog_amt * wx,
        l.night,
        game.settings.vignette_amt(),
        l.fog_col[0],
        l.fog_col[1],
        l.fog_col[2],
        game.fade(),
    ]
}

/// Background color the scene pass clears to (matches the fog so the horizon blends away).
pub fn clear_color(game: &Game) -> [f32; 3] {
    game.lighting().fog_col
}

// ------------------------------------------------------------------ static world

pub fn build_static(world: &World) -> Mesh {
    let mut m = Mesh::new();
    let hollow = world.area == Area::Hollow;
    let pond = world.area == Area::Pond;

    let ground = match world.area {
        Area::Glade => [0.24, 0.40, 0.27],
        Area::Hollow => [0.21, 0.27, 0.28],
        Area::Pond => [0.20, 0.38, 0.33],
    };
    m.ground(140.0, 0.0, ground);

    if hollow {
        build_hollow_ground(&mut m, world);
    } else if pond {
        build_pond_ground(&mut m, world);
    } else {
        // Cottage yard and the stone circle in the lantern glade.
        m.cylinder(v(world.home.x, 0.0, world.home.z), 7.0, 0.05, 22, [0.36, 0.32, 0.29], 0.0);
        m.cylinder(v(world.glade.x, 0.0, world.glade.z), 8.5, 0.05, 24, [0.34, 0.34, 0.42], 0.0);
    }

    for t in &world.trees {
        let base = v(t.x, 0.0, t.z);
        let trunk = [0.27, 0.19, 0.16];
        if t.kind == 2 {
            build_dead_tree(&mut m, t);
        } else if t.kind == 3 {
            build_willow(&mut m, t);
        } else if t.kind == 0 {
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

    let rock_col = if hollow { [0.33, 0.33, 0.41] } else { [0.38, 0.37, 0.45] };
    for r in &world.rocks {
        m.set_xf(v(r.x, 0.0, r.z), r.yaw, Vec3::ONE);
        m.ellipsoid(v(0.0, r.sy * 0.6, 0.0), v(r.sx, r.sy, r.sz), 3, 6, rock_col, 0.0);
    }
    m.reset_xf();

    let caps = if hollow {
        [[0.62, 0.78, 1.00], [0.78, 0.58, 1.00], [0.55, 1.00, 0.80]]
    } else {
        [[0.40, 0.90, 0.80], [0.90, 0.50, 0.90], [1.00, 0.60, 0.30]]
    };
    for s in &world.shrooms {
        m.set_xf_u(v(s.x, 0.0, s.z), 0.0, s.s);
        m.cylinder(V0, 0.05, 0.25, 5, [0.9, 0.88, 0.8], 0.0);
        m.ellipsoid(v(0.0, 0.27, 0.0), v(0.2, 0.1, 0.2), 2, 6, caps[s.hue as usize % 3], 0.65);
    }
    m.reset_xf();

    if hollow {
        build_crypt(&mut m, world);
        for g in &world.graves {
            build_grave(&mut m, g);
        }
        for (i, q) in world.runes.iter().enumerate() {
            // Stone column with a band in the rune's color (the gem on top is dynamic).
            m.cylinder(v(q.x, 0.0, q.z), 0.55, 1.35, 6, [0.40, 0.40, 0.48], 0.0);
            m.cylinder(v(q.x, 0.0, q.z), 0.7, 0.22, 6, [0.34, 0.34, 0.42], 0.0);
            m.cylinder(v(q.x, 0.95, q.z), 0.58, 0.1, 6, RUNE_COLORS[i], 0.35);
            m.cylinder(v(q.x, 1.35, q.z), 0.62, 0.12, 6, [0.44, 0.44, 0.52], 0.0);
        }
    } else if pond {
        build_shed(&mut m, world.shed);
        for l in &world.lamps {
            // Stone base and a green stem; the lily bloom on top is dynamic (it lights up).
            m.cylinder(v(l.x, 0.0, l.z), 0.5, 0.25, 7, [0.40, 0.42, 0.46], 0.0);
            m.cylinder(v(l.x, 0.25, l.z), 0.07, 1.25, 5, [0.22, 0.42, 0.28], 0.0);
            m.cylinder(v(l.x, 0.9, l.z), 0.28, 0.06, 7, [0.25, 0.55, 0.30], 0.0);
        }
    } else {
        build_cottage(&mut m, world.home);
        // Lantern posts in the glade (the lamp itself is dynamic: it lights up).
        for l in &world.lanterns {
            m.cylinder(v(l.x, 0.0, l.z), 0.32, 0.25, 8, [0.45, 0.45, 0.52], 0.0);
            m.cylinder(v(l.x, 0.25, l.z), 0.08, 1.95, 6, [0.20, 0.17, 0.22], 0.0);
        }
    }
    for g in &world.gates {
        build_gate(&mut m, g.pos, world.area, g.to);
    }
    m
}

/// A worn dirt path from the gate up to the crypt door, and a few patches of paler grass.
fn build_hollow_ground(m: &mut Mesh, world: &World) {
    let door = world.crypt + v(0.0, 0.0, 4.4);
    let start = world.gates[0].pos + v(0.0, 0.0, -2.0);
    let dir = door - start;
    let yaw = dir.x.atan2(dir.z);
    let len = dir.length();
    let n = (len / 2.6) as usize;
    for i in 0..n {
        let f = (i as f32 + 0.5) / n as f32;
        let p = start + dir * f;
        let wob = ((i as f32) * 1.7).sin() * 0.25;
        m.set_xf(v(p.x, 0.0, p.z), yaw + wob * 0.2, Vec3::ONE);
        m.box_(v(0.0, 0.02, 0.0), v(1.25 + wob * 0.3, 0.025, 1.7), [0.30, 0.26, 0.27], 0.0);
    }
    m.reset_xf();
    // Flat stone slabs in front of the crypt where the rune stones stand.
    m.cylinder(v(world.crypt.x, 0.0, world.crypt.z + 9.2), 11.5, 0.05, 24, [0.27, 0.27, 0.34], 0.0);
}

/// The pond itself, its muddy bank, lily pads, cattails, a little pier + rowboat and the dirt path to the shed.
fn build_pond_ground(m: &mut Mesh, world: &World) {
    // Dirt path from the gate up to the shed door.
    let start = world.gates[0].pos + v(0.0, 0.0, -2.0);
    let door = world.shed + v(0.0, 0.0, 3.8);
    let dir = door - start;
    let yaw = dir.x.atan2(dir.z);
    let n = (dir.length() / 2.6) as usize;
    for i in 0..n {
        let f = (i as f32 + 0.5) / n as f32;
        let p = start + dir * f;
        let wob = ((i as f32) * 1.9).sin() * 0.25;
        m.set_xf(v(p.x, 0.0, p.z), yaw + wob * 0.2, Vec3::ONE);
        m.box_(v(0.0, 0.02, 0.0), v(1.2 + wob * 0.3, 0.025, 1.7), [0.36, 0.30, 0.22], 0.0);
    }
    m.reset_xf();
    // Packed earth in front of the shed where the lily lamps stand.
    m.cylinder(v(world.shed.x, 0.0, world.shed.z + 10.0), 11.0, 0.05, 24, [0.28, 0.36, 0.30], 0.0);

    let w = &world.ponds[0];
    // Muddy bank, the water, and a deeper centre.
    m.ellipsoid(v(w.x, 0.02, w.z), v(w.rx + 1.2, 0.03, w.rz + 1.1), 2, 30, [0.30, 0.27, 0.22], 0.0);
    m.ellipsoid(v(w.x, 0.05, w.z), v(w.rx, 0.04, w.rz), 2, 30, [0.18, 0.42, 0.55], 0.12);
    m.ellipsoid(v(w.x, 0.07, w.z), v(w.rx * 0.62, 0.03, w.rz * 0.62), 2, 24, [0.14, 0.34, 0.48], 0.1);

    for p in &world.pads {
        m.cylinder(v(p.x, 0.09, p.z), 0.55 * p.s, 0.03, 8, [0.22, 0.55, 0.28], 0.0);
        if p.flower {
            m.cone(v(p.x, 0.12, p.z), 0.2 * p.s, 0.3, 6, [1.0, 0.72, 0.86], 0.6);
        }
    }
    for r in &world.reeds {
        m.cylinder(v(r.x, 0.0, r.z), 0.04, r.h, 4, [0.35, 0.48, 0.26], 0.0);
        m.cylinder(v(r.x, r.h * 0.55, r.z), 0.095, r.h * 0.3, 5, [0.45, 0.28, 0.15], 0.0);
    }

    // A little pier on the west bank (it stays on dry ground), with a rowboat pulled up beside it.
    let px = w.x - w.rx - 0.8;
    for k in 0..7 {
        m.box_(v(px - 0.3 - k as f32 * 0.62, 0.3, w.z), v(0.3, 0.05, 0.95), [0.46, 0.33, 0.22], 0.0);
    }
    for k in [0.0f32, 2.4, 4.0] {
        for sz in [-0.95f32, 0.95] {
            m.cylinder(v(px - k, 0.0, w.z + sz), 0.07, 0.55, 5, [0.30, 0.22, 0.16], 0.0);
        }
    }
    m.set_xf(v(w.x - w.rx - 1.0, 0.0, w.z + 3.2), 0.5, Vec3::ONE);
    m.ellipsoid(v(0.0, 0.25, 0.0), v(1.5, 0.3, 0.6), 3, 8, [0.45, 0.30, 0.20], 0.0);
    m.ellipsoid(v(0.0, 0.42, 0.0), v(1.25, 0.2, 0.42), 2, 8, [0.18, 0.12, 0.10], 0.0);
    m.reset_xf();
}

/// A weeping willow: thick trunk, a soft crown, and long fronds hanging all round.
fn build_willow(m: &mut Mesh, t: &Tree) {
    let g = [0.24 + 0.06 * t.tint, 0.50 + 0.08 * t.tint, 0.36 + 0.05 * t.tint];
    m.cylinder(v(t.x, 0.0, t.z), 0.36, t.h * 0.6, 6, [0.30, 0.23, 0.18], 0.0);
    m.ellipsoid(v(t.x, t.h * 0.78, t.z), v(1.55, 0.85, 1.55), 3, 8, g, 0.0);
    for k in 0..9 {
        let a = TAU * k as f32 / 9.0 + t.tint * 3.0;
        let (cx, cz) = (t.x + a.cos() * 1.35, t.z + a.sin() * 1.35);
        let c = [g[0] * 0.9, g[1] * 0.95, g[2] * 0.9];
        // Wide at the top, tapering as it hangs.
        m.frustum(v(cx, t.h * 0.78 - 1.9, cz), 0.04, 0.34, 1.9, 5, c, 0.0);
    }
}

/// The shed by the pond: planked walls, a pyramid roof, a porch, firewood and a barrel.
fn build_shed(m: &mut Mesh, s: Vec3) {
    let plank = [0.46, 0.31, 0.20];
    m.set_xf(s, 0.0, Vec3::ONE);
    m.box_(v(0.0, 0.12, 0.0), v(2.9, 0.12, 2.4), [0.33, 0.30, 0.28], 0.0); // stone footing
    m.box_(v(0.0, 1.3, 0.0), v(2.7, 1.2, 2.2), plank, 0.0);
    for k in -4..=4 {
        m.box_(v(k as f32 * 0.6, 1.3, 2.22), v(0.025, 1.2, 0.02), [0.30, 0.19, 0.12], 0.0); // plank seams
    }
    // Door, its frame, and a window (the window glows when the pond is awake).
    m.box_(v(-0.9, 1.05, 2.26), v(0.65, 1.0, 0.05), [0.27, 0.16, 0.10], 0.0);
    m.box_(v(-0.9, 2.12, 2.28), v(0.8, 0.09, 0.08), [0.30, 0.20, 0.13], 0.0);
    m.box_(v(1.35, 1.55, 2.26), v(0.5, 0.45, 0.04), [0.20, 0.14, 0.10], 0.0);
    m.box_(v(1.35, 1.55, 2.23), v(0.6, 0.55, 0.03), [0.30, 0.20, 0.13], 0.0);
    // Roof: a squat pyramid, turned to line up with the walls, plus a chimney.
    m.set_xf(s, PI / 4.0, Vec3::ONE);
    m.cone(v(0.0, 2.5, 0.0), 4.3, 1.7, 4, [0.24, 0.36, 0.34], 0.0);
    m.set_xf(s, 0.0, Vec3::ONE);
    m.box_(v(1.5, 3.4, -0.8), v(0.3, 0.7, 0.3), [0.40, 0.38, 0.40], 0.0);
    // Porch boards and a step.
    m.box_(v(0.0, 0.1, 3.05), v(1.9, 0.1, 0.8), [0.42, 0.30, 0.20], 0.0);
    // Barrel, firewood stack and a crate.
    m.cylinder(v(-3.5, 0.0, 1.4), 0.42, 0.85, 7, [0.40, 0.27, 0.17], 0.0);
    m.cylinder(v(-3.5, 0.55, 1.4), 0.44, 0.06, 7, [0.22, 0.20, 0.22], 0.0);
    for k in 0..3 {
        for j in 0..(3 - k) {
            m.cylinder(v(3.5 + j as f32 * 0.38 + k as f32 * 0.19, 0.0 + k as f32 * 0.3, -0.6), 0.17, 0.9, 5, [0.50, 0.36, 0.24], 0.0);
        }
    }
    m.box_(v(3.4, 0.35, 1.6), v(0.4, 0.35, 0.4), [0.45, 0.33, 0.22], 0.0);
    m.reset_xf();
}

fn build_dead_tree(m: &mut Mesh, t: &Tree) {
    let bark = [0.22 + 0.05 * t.tint, 0.18 + 0.03 * t.tint, 0.21 + 0.04 * t.tint];
    m.cylinder(v(t.x, 0.0, t.z), 0.26, t.h * 0.78, 5, bark, 0.0);
    m.cylinder(v(t.x, t.h * 0.78, t.z), 0.12, t.h * 0.22, 5, bark, 0.0);
    // Gnarled branches: little stepped boxes reaching up and out.
    for b in 0..4 {
        let a = TAU * b as f32 / 4.0 + t.tint * 5.0;
        let y0 = t.h * (0.42 + 0.1 * b as f32);
        for k in 1..=3 {
            let f = k as f32;
            let sz = 0.085 - 0.018 * f;
            m.box_(v(t.x + a.cos() * 0.34 * f, y0 + 0.3 * f, t.z + a.sin() * 0.34 * f), v(sz, sz * 1.6, sz), bark, 0.0);
        }
        // A sparse clump of twigs at the tip, so the tree reads as a tree from above.
        m.ellipsoid(v(t.x + a.cos() * 1.1, y0 + 1.0, t.z + a.sin() * 1.1), v(0.42, 0.22, 0.42), 2, 5, [0.17, 0.13, 0.22], 0.0);
    }
}

fn build_grave(m: &mut Mesh, g: &Grave) {
    let stone = 0.42 + 0.10 * ((g.x * 3.1 + g.z * 1.7).sin() * 0.5 + 0.5);
    let col = [stone, stone, stone + 0.07];
    m.set_xf_u(v(g.x, 0.0, g.z), g.yaw, g.s);
    // Mound of earth in front of the stone.
    m.ellipsoid(v(0.0, 0.05, 0.75), v(0.42, 0.12, 0.8), 2, 6, [0.22, 0.20, 0.20], 0.0);
    match g.kind {
        0 => {
            m.box_(v(0.0, 0.4, 0.0), v(0.38, 0.4, 0.09), col, 0.0);
            m.ellipsoid(v(0.0, 0.8, 0.0), v(0.38, 0.3, 0.09), 3, 8, col, 0.0);
        }
        1 => {
            m.box_(v(0.0, 0.55, 0.0), v(0.09, 0.55, 0.08), col, 0.0);
            m.box_(v(0.0, 0.85, 0.0), v(0.34, 0.08, 0.08), col, 0.0);
        }
        2 => {
            m.box_(v(0.0, 0.3, 0.0), v(0.52, 0.3, 0.1), col, 0.0);
            m.box_(v(0.0, 0.62, 0.0), v(0.56, 0.05, 0.12), [col[0] * 0.9, col[1] * 0.9, col[2] * 0.9], 0.0);
        }
        _ => {
            m.frustum(v(0.0, 0.0, 0.0), 0.34, 0.14, 1.35, 4, col, 0.0);
            m.cone(v(0.0, 1.35, 0.0), 0.15, 0.25, 4, col, 0.0);
        }
    }
    m.reset_xf();
}

fn build_crypt(m: &mut Mesh, world: &World) {
    let c = world.crypt;
    let stone = [0.37, 0.37, 0.44];
    m.set_xf(c, 0.0, Vec3::ONE);
    m.box_(v(0.0, 0.18, 0.0), v(3.9, 0.18, 3.1), [0.30, 0.30, 0.37], 0.0); // plinth
    m.box_(v(0.0, 1.6, 0.0), v(3.4, 1.42, 2.6), stone, 0.0); // walls
    for sx in [-1.0f32, 1.0] {
        m.box_(v(sx * 3.45, 1.7, 2.45), v(0.28, 1.5, 0.28), [0.42, 0.42, 0.50], 0.0); // front corner pillars
    }
    // Door: dark opening, frame and lintel, steps.
    m.box_(v(0.0, 1.15, 2.62), v(0.95, 1.05, 0.05), [0.06, 0.04, 0.09], 0.0);
    m.box_(v(-1.1, 1.2, 2.7), v(0.14, 1.2, 0.14), [0.46, 0.46, 0.54], 0.0);
    m.box_(v(1.1, 1.2, 2.7), v(0.14, 1.2, 0.14), [0.46, 0.46, 0.54], 0.0);
    m.box_(v(0.0, 2.5, 2.7), v(1.35, 0.14, 0.16), [0.46, 0.46, 0.54], 0.0);
    m.box_(v(0.0, 0.1, 3.35), v(1.5, 0.1, 0.5), [0.40, 0.40, 0.47], 0.0);
    // Roof: a squat pyramid, turned to line up with the walls.
    m.set_xf(c, PI / 4.0, Vec3::ONE);
    m.cone(v(0.0, 3.0, 0.0), 5.4, 2.3, 4, [0.33, 0.30, 0.43], 0.0);
    m.set_xf(c, 0.0, Vec3::ONE);
    // Spire with a little cross.
    m.cylinder(v(0.0, 5.2, 0.0), 0.07, 0.8, 5, [0.20, 0.18, 0.24], 0.0);
    m.box_(v(0.0, 5.7, 0.0), v(0.3, 0.05, 0.05), [0.20, 0.18, 0.24], 0.0);
    // Urns with a faint glow on either side of the steps.
    for sx in [-2.2f32, 2.2] {
        m.ellipsoid(v(sx, 0.55, 3.6), v(0.4, 0.45, 0.4), 3, 7, [0.40, 0.38, 0.44], 0.0);
        m.sphere(v(sx, 1.05, 3.6), 0.14, [0.7, 1.0, 0.85], 0.9);
    }
    m.reset_xf();
}

/// Gate posts with a lintel and a stub of wall each side. The door itself is dynamic.
/// The gate to Willow Pond (and the one coming back out of it) is wooden; the rest are stone.
fn build_gate(m: &mut Mesh, g: Vec3, here: Area, to: Area) {
    let wood = here == Area::Pond || to == Area::Pond;
    let stone = if wood { [0.46, 0.33, 0.22] } else { [0.40, 0.40, 0.46] };
    let dark = if wood {
        [0.36, 0.25, 0.17]
    } else if here == Area::Hollow {
        [0.30, 0.30, 0.38]
    } else {
        [0.34, 0.34, 0.40]
    };
    m.set_xf(g, 0.0, Vec3::ONE);
    for sx in [-2.7f32, 2.7] {
        m.box_(v(sx, 0.3, 0.0), v(0.8, 0.3, 0.8), dark, 0.0);
        m.box_(v(sx, 2.0, 0.0), v(0.55, 1.7, 0.55), stone, 0.0);
        m.box_(v(sx, 3.8, 0.0), v(0.72, 0.13, 0.72), dark, 0.0);
        if wood {
            m.box_(v(sx, 4.15, 0.0), v(0.3, 0.3, 0.3), stone, 0.0);
        } else {
            m.cone(v(sx, 3.93, 0.0), 0.55, 0.7, 4, stone, 0.0);
        }
    }
    m.box_(v(0.0, 3.5, 0.0), v(2.7, 0.26, 0.38), stone, 0.0);
    for sx in [-1.0f32, 1.0] {
        m.box_(v(sx * 6.5, 0.55, 0.0), v(3.0, 0.55, 0.35), dark, 0.0);
        for k in 0..5 {
            m.box_(v(sx * (4.0 + k as f32 * 1.2), 1.25, 0.0), v(0.22, 0.18, 0.28), stone, 0.0);
        }
    }
    m.reset_xf();
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

    draw_gate_doors(m, game);
    match game.area {
        Area::Hollow => draw_runes(m, game),
        Area::Pond => draw_pond(m, game),
        Area::Glade => {}
    }

    // Lantern lamps (glade only).
    for i in 0..3 {
        if game.area != Area::Glade {
            break;
        }
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
        if !game.world.has_home {
            break;
        }
        for k in 0..game.save.residents[sp].min(3) as usize {
            let a = t * 0.25 * (1.0 + 0.3 * k as f32) + sp as f32 * 1.7 + k as f32 * 2.1;
            let r = 3.4 + (sp % 4) as f32 * 0.5 + (sp / 4) as f32 * 0.25 + k as f32 * 0.45;
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
    if game.settings.shimmer {
        draw_shimmers(m, game);
    }

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
        let flies = if game.area == Area::Hollow { 22 } else { 40 };
        let fly_col = match game.area {
            Area::Hollow => [0.65, 0.95, 1.0],
            Area::Pond => [0.80, 1.0, 0.65],
            Area::Glade => [0.85, 1.0, 0.5],
        };
        for i in 0..flies {
            let f = i as f32;
            let x = c.x + 20.0 * (f * 7.31 + t * 0.07 * (1.0 + (i % 3) as f32 * 0.4)).sin();
            let z = c.z + 12.0 * (f * 3.77 + t * 0.05).cos();
            let y = 0.6 + 1.5 * (0.5 + 0.5 * (f * 2.1 + t * 0.6).sin());
            let blink = ((t * 1.3 + f).sin() * 0.5 + 0.5).powf(0.5);
            m.box_(v(x, y, z), v(0.05, 0.05, 0.05) * (0.4 + blink), fly_col, 1.0);
        }
    }
}

/// Sparkles rising from wild spirits you can sense: the closer you are, the more obvious. Spirits you
/// have never befriended also send out a slow ring on the ground, so a new face stands out.
fn draw_shimmers(m: &mut Mesh, game: &Game) {
    let t = game.time;
    let radius = game.sense_radius();
    for s in &game.spirits {
        if matches!(s.state, SpState::Follow | SpState::Leaving) || s.fade < 0.2 {
            continue;
        }
        let dist = s.pos.dist_xz(game.player.pos);
        if dist > radius {
            continue;
        }
        let d = &SPIRITS[s.def];
        let fresh = !game.save.discovered[s.def];
        // Brighten the species colour so the motes read as light, not as little coloured cubes.
        let c = [
            (d.color[0] * 0.5 + 0.5).min(1.0),
            (d.color[1] * 0.5 + 0.5).min(1.0),
            (d.color[2] * 0.5 + 0.5).min(1.0),
        ];
        let n = if fresh { 5 } else { 3 };
        for k in 0..n {
            let ph = (t * 0.5 + k as f32 / n as f32 + s.phase * 0.07).fract();
            let ang = TAU * k as f32 / n as f32 + t * 1.2;
            let rad = 0.62 * (1.0 - 0.5 * ph);
            let size = 0.11 * (ph * (1.0 - ph) * 4.0).sqrt() * s.fade;
            m.box_(v(s.pos.x + ang.cos() * rad, 0.35 + ph * 2.7, s.pos.z + ang.sin() * rad), v(size, size, size), c, 1.0);
        }
        if fresh && dist > 5.0 {
            let ph = (t * 0.6 + s.phase * 0.1).fract();
            let r = 0.7 + ph * 2.4;
            let size = 0.12 * (1.0 - ph) * s.fade;
            for j in 0..10 {
                let a = TAU * j as f32 / 10.0 + t * 0.2;
                m.box_(v(s.pos.x + a.cos() * r, 0.14, s.pos.z + a.sin() * r), v(size, size, size), c, 1.0);
            }
        }
    }
}

/// Each gate's moving parts: bars and a glowing lock while sealed, a shimmering veil once open
/// (violet into the Hollow, teal into the Pond, amber back home).
fn draw_gate_doors(m: &mut Mesh, game: &Game) {
    let t = game.time;
    for gate in &game.world.gates {
        let wood = game.area == Area::Pond || gate.to == Area::Pond;
        let bars = if wood { [0.40, 0.28, 0.18] } else { [0.14, 0.13, 0.18] };
        m.set_xf(gate.pos, 0.0, Vec3::ONE);
        // Lamps on the posts.
        for sx in [-2.7f32, 2.7] {
            let y = if wood { 4.7 } else { 4.75 };
            m.box_(v(sx, y, 0.0), v(0.14, 0.2, 0.14), [1.0, 0.85, 0.55], 0.9);
        }
        if game.gate_unlocked(gate.to) {
            // Gate leaves swung wide, and the way through glows.
            for sx in [-1.0f32, 1.0] {
                for k in 0..4 {
                    m.box_(v(sx * 2.0, 1.6, 0.25 + k as f32 * 0.42), v(0.05, 1.5, 0.05), bars, 0.0);
                }
                m.box_(v(sx * 2.0, 3.0, 0.9), v(0.05, 0.05, 0.9), bars, 0.0);
                m.box_(v(sx * 2.0, 0.5, 0.9), v(0.05, 0.05, 0.9), bars, 0.0);
            }
            let pulse = 0.55 + 0.2 * (t * 2.0).sin();
            let veil = match gate.to {
                Area::Hollow => [0.55, 0.45, 0.95],
                Area::Pond => [0.35, 0.90, 0.85],
                Area::Glade => [1.0, 0.78, 0.45],
            };
            m.box_(v(0.0, 1.7, 0.0), v(1.9, 1.7, 0.03), veil, pulse);
        } else {
            for k in -4..=4 {
                m.box_(v(k as f32 * 0.42, 1.7, 0.0), v(0.05, 1.7, 0.05), bars, 0.0);
            }
            for y in [0.35f32, 1.7, 3.1] {
                m.box_(v(0.0, y, 0.0), v(2.1, 0.07, 0.07), bars, 0.0);
            }
            // The ghostly lock.
            let glow = 0.7 + 0.25 * (t * 3.0).sin();
            m.box_(v(0.0, 1.7, 0.14), v(0.32, 0.36, 0.14), [1.0, 0.35, 0.40], glow);
            m.cylinder(v(0.0, 2.0, 0.14), 0.18, 0.28, 6, [0.85, 0.30, 0.38], glow * 0.8);
        }
        m.reset_xf();
    }
}

/// Willow Pond's moving parts: lily lamp blooms, ripples, the shed window and the queen's lotus glow.
fn draw_pond(m: &mut Mesh, game: &Game) {
    let w = &game.world;
    let t = game.time;
    let solved = game.puzzle_solved[Area::Pond.idx()];
    for i in 0..N_LAMPS {
        let l = w.lamps[i];
        if game.lamps_lit[i] {
            let pulse = 0.9 + 0.1 * (t * 2.0 + i as f32).sin();
            m.ellipsoid(v(l.x, 1.62, l.z), v(0.3, 0.26, 0.3), 3, 7, [1.0, 0.78, 0.90], pulse);
            for k in 0..6 {
                let a = TAU * k as f32 / 6.0 + t * 0.3;
                m.cone(v(l.x + a.cos() * 0.34, 1.5, l.z + a.sin() * 0.34), 0.13, 0.5, 4, [1.0, 0.70, 0.86], 0.9);
            }
            m.ellipsoid(v(l.x, 0.08, l.z), v(1.0, 0.02, 1.0), 2, 10, [1.0, 0.72, 0.88], 0.5);
        } else {
            // A closed, sleepy bud.
            m.cone(v(l.x, 1.5, l.z), 0.26, 0.55, 5, [0.42, 0.30, 0.42], 0.08);
        }
    }
    // The shed window glows once the pond is awake.
    if solved {
        let s = w.shed;
        m.box_(v(s.x + 1.35, 1.55, s.z + 2.3), v(0.46, 0.41, 0.02), [1.0, 0.85, 0.50], 0.9);
        let c = w.puzzle_spot;
        m.ellipsoid(v(c.x, 0.1, c.z), v(1.5, 0.02, 1.5), 2, 14, [1.0, 0.72, 0.88], 0.55);
    }
    // Slow ripples on the water: a light ring that grows, drawn as a disc over a water-coloured disc.
    let p = &w.ponds[0];
    for (i, (ox, oz)) in [(-5.0f32, 2.0f32), (4.0, -3.0), (7.0, 3.0)].iter().enumerate() {
        let f = (t * 0.35 + i as f32 * 0.33).fract();
        let r = 0.3 + f * 1.7;
        let emi = 0.35 * (1.0 - f);
        m.ellipsoid(v(p.x + ox, 0.105, p.z + oz), v(r, 0.01, r * 0.8), 2, 14, [0.65, 0.85, 0.95], emi + 0.12);
        m.ellipsoid(v(p.x + ox, 0.112, p.z + oz), v(r * 0.78, 0.01, r * 0.62), 2, 14, [0.18, 0.42, 0.55], 0.12);
    }
}

/// Rune stone gems, floor glyphs, and the crypt doorway once the melody is complete.
fn draw_runes(m: &mut Mesh, game: &Game) {
    let w = &game.world;
    let solved = game.runes.state == RuneState::Solved;
    for i in 0..4 {
        let q = w.runes[i];
        let c = RUNE_COLORS[i];
        let f = (game.runes.flash[i] / 0.7).clamp(0.0, 1.0);
        let idle = 0.22 + 0.08 * (game.time * 1.5 + i as f32).sin();
        let glow = (idle + f * 0.9 + if solved { 0.25 } else { 0.0 }).min(1.2);
        let rise = 0.06 * f;
        m.box_(v(q.x, 1.78 + rise, q.z), v(0.2, 0.3, 0.2), c, glow);
        if f > 0.0 {
            m.ellipsoid(v(q.x, 0.08, q.z), v(1.0 + f * 0.5, 0.02, 1.0 + f * 0.5), 2, 10, c, 0.8 * f);
        }
    }
    if solved {
        let pulse = 0.65 + 0.2 * (game.time * 2.0).sin();
        m.box_(v(w.crypt.x, 1.15, w.crypt.z + 2.68), v(0.9, 1.0, 0.03), [0.70, 1.0, 0.85], pulse);
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
        "wick" => {
            // A little wax candle on its head with a living flame.
            let flick = 1.0 + 0.15 * (phase * 4.0).sin();
            m.cylinder(v(0.0, 1.3, 0.0), 0.27, 0.45, 6, [0.96, 0.92, 0.80], 0.25);
            m.cone(v(0.0, 1.75, 0.0), 0.17, 0.55 * flick, 5, [1.0, 0.72, 0.18], 1.0);
        }
        "drip" => {
            m.cone(v(0.0, 1.2, 0.0), 0.2, 0.5, 6, c, 0.55);
            for k in 0..3 {
                let a = phase * 0.8 + k as f32 * TAU / 3.0;
                m.sphere(v(a.cos() * 0.62, 1.45 + (a * 1.7).sin() * 0.16, a.sin() * 0.62), 0.1, [0.65, 0.88, 1.0], 0.9);
            }
        }
        "gloam" => {
            let flap = (phase * 2.6).sin() * 0.25;
            let wing = [0.30, 0.22, 0.48];
            m.ellipsoid(v(-1.2, 0.7 + flap, -0.1), v(0.8, 0.1, 0.5), 2, 7, wing, 0.3);
            m.ellipsoid(v(1.2, 0.7 + flap, -0.1), v(0.8, 0.1, 0.5), 2, 7, wing, 0.3);
            for sx in [-1.0f32, 1.0] {
                m.cone(v(sx * 0.4, 1.25, 0.0), 0.1, 0.36, 4, [0.20, 0.15, 0.30], 0.0);
            }
        }
        "bobbin" => {
            // Bulging frog eyes on top of its head.
            for sx in [-1.0f32, 1.0] {
                m.sphere(v(sx * 0.38, 1.25, 0.2), 0.27, [0.95, 1.0, 0.90], 0.25);
                m.sphere(v(sx * 0.38, 1.3, 0.38), 0.13, [0.08, 0.05, 0.12], 0.0);
            }
        }
        "pondle" => {
            // A lily pad hat with a pink bud.
            m.cylinder(v(0.0, 1.25, 0.0), 0.68, 0.07, 8, [0.28, 0.62, 0.34], 0.0);
            m.cone(v(0.0, 1.3, 0.0), 0.2, 0.42, 6, [1.0, 0.72, 0.86], 0.6);
        }
        "reed" => {
            // A cattail growing out of its head.
            m.cylinder(v(0.0, 1.3, 0.0), 0.05, 0.85, 5, [0.35, 0.48, 0.26], 0.0);
            m.cylinder(v(0.0, 1.85, 0.0), 0.15, 0.5, 6, [0.45, 0.28, 0.15], 0.0);
            let flap = (phase * 2.0).sin() * 0.15;
            m.ellipsoid(v(-0.95, 0.6 + flap, -0.1), v(0.55, 0.1, 0.4), 2, 6, [0.92, 0.98, 0.94], 0.5);
            m.ellipsoid(v(0.95, 0.6 + flap, -0.1), v(0.55, 0.1, 0.4), 2, 6, [0.92, 0.98, 0.94], 0.5);
        }
        "lotus" => {
            // A lotus blossom crown: an outer ring of petals and a brighter inner ring.
            for k in 0..8 {
                let a = TAU * k as f32 / 8.0;
                m.cone(v(a.cos() * 0.62, 1.15, a.sin() * 0.62), 0.22, 0.7, 4, [1.0, 0.70, 0.84], 0.7);
            }
            for k in 0..5 {
                let a = TAU * k as f32 / 5.0 + 0.3;
                m.cone(v(a.cos() * 0.3, 1.3, a.sin() * 0.3), 0.16, 0.65, 4, [1.0, 0.92, 0.95], 1.0);
            }
        }
        "marrow" => {
            // A tiny top hat with a golden band.
            let hat = [0.17, 0.12, 0.25];
            m.cylinder(v(0.0, 1.28, 0.0), 0.78, 0.07, 8, hat, 0.0);
            m.cylinder(v(0.0, 1.33, 0.0), 0.46, 0.78, 8, hat, 0.0);
            m.cylinder(v(0.0, 1.42, 0.0), 0.48, 0.13, 8, [1.0, 0.80, 0.30], 0.8);
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
                g.save.residents = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
                g.puzzle_solved[0] = true;
                g.player.pos = g.world.glade + v(0.0, 0.0, 6.5);
                g.spawn_spirit(3, g.world.glade + v(1.0, 0.0, 0.0));
            })),
            ("glade_gate", 21.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.save.upgrades[GATE_KEY] = 1;
                g.player.pos = g.world.gate_to(Area::Hollow).pos + v(1.5, 0.0, 7.0);
            })),
            ("glade_gate_locked", 12.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.player.pos = g.world.gate_to(Area::Hollow).pos + v(1.5, 0.0, 7.0);
            })),
            ("hollow_crypt", 23.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
                g.runes.state = RuneState::Solved;
                g.puzzle_solved[1] = true;
                g.player.pos = g.world.crypt + v(-2.0, 0.0, 12.0);
                g.spawn_spirit(7, g.world.puzzle_spot + v(1.0, 0.0, 0.5));
            })),
            ("hollow_fog", 22.0, Weather::Fog, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
                g.player.pos = v(12.0, 0.0, 20.0);
                g.spawn_spirit(6, v(17.0, 0.0, 16.0));
                g.spawn_spirit(4, v(8.0, 0.0, 15.0));
            })),
            ("hollow_front", 23.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
                g.player.pos = g.world.crypt + v(0.0, 0.0, 8.0);
            })),
            ("hollow_gate", 20.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
            })),
            ("hollow_lineup", 23.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
                g.player.pos = v(6.0, 0.0, 28.0);
            })),
            ("pond_day", 12.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Pond);
                g.player.pos = v(-8.0, 0.0, 6.0);
            })),
            ("pond_shed", 13.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Pond);
                g.lamps_lit = [true, true, false, true];
                g.player.pos = g.world.shed + v(0.0, 0.0, 10.0);
            })),
            ("pond_night", 22.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Pond);
                g.lamps_lit = [true; 4];
                g.puzzle_solved[2] = true;
                g.player.pos = g.world.lamps[1] + v(0.0, 0.0, 4.0);
            })),
            ("pond_lineup", 12.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.enter_area(Area::Pond);
                g.player.pos = v(-4.0, 0.0, 22.0);
            })),
            ("pond_gate", 12.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.save.upgrades[POND_KEY] = 1;
                g.player.pos = g.world.gate_to(Area::Pond).pos + v(1.5, 0.0, -7.0);
            })),
            ("shimmer_night", 20.0, Weather::Clear, Box::new(|g: &mut Game| {
                g.player.pos = v(-6.0, 0.0, 14.0);
                g.spawn_spirit(0, v(5.0, 0.0, 12.0));
                g.spawn_spirit(0, v(-14.0, 0.0, 8.0));
            })),
            ("title_view", 19.4, Weather::Clear, Box::new(|g: &mut Game| {
                g.save.residents = [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                g.player.pos = v(0.0, 0.0, 6.0);
            })),
            ("hollow_rain", 14.0, Weather::Rain, Box::new(|g: &mut Game| {
                g.enter_area(Area::Hollow);
                g.player.pos = v(-14.0, 0.0, 22.0);
                g.spawn_spirit(5, v(-10.0, 0.0, 19.0));
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
            if name == "pond_lineup" {
                g.spirits.clear();
                for (i, d) in (8..N_SPIRITS).enumerate() {
                    let id = g.spawn_spirit(d, g.player.pos + v(-4.5 + i as f32 * 3.0, 0.0, -4.0));
                    let sp = g.spirits.iter_mut().find(|s| s.id == id).unwrap();
                    sp.fade = 1.0;
                    sp.yaw = 0.0;
                }
            }
            if name == "hollow_lineup" {
                // Every Hollow spirit in a row, regardless of weather rules, so we can inspect them.
                g.spirits.clear();
                for (i, d) in (4..N_SPIRITS).enumerate() {
                    let id = g.spawn_spirit(d, g.player.pos + v(-4.5 + i as f32 * 3.0, 0.0, -4.0));
                    let sp = g.spirits.iter_mut().find(|s| s.id == id).unwrap();
                    sp.fade = 1.0;
                    sp.yaw = 0.0;
                }
            }
            if name == "title_view" {
                // The camera the title screen uses: beside the cottage, so it sits right of centre.
                let h = g.world.home;
                g.cam = v(h.x - 9.0, 0.0, h.z + 5.0);
            }
            if name == "hollow_crypt" {
                g.runes.flash = [0.0, 0.7, 0.0, 0.4]; // mid-melody glow
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
        for d in 4..N_SPIRITS {
            g.spawn_spirit(d, v(8.0 + d as f32, 0.0, -4.0));
        }
        g.save.residents = [1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 1];
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
        // The Hollow too, with the puzzle lit up and its spirits out.
        let mut h = Game::new(Save::default(), 3);
        h.enter_area(Area::Hollow);
        h.env.hour = 23.0;
        h.runes.state = RuneState::Solved;
        h.runes.flash = [0.5, 0.0, 0.7, 0.0];
        for d in 4..N_SPIRITS {
            h.spawn_spirit(d, h.player.pos + v(d as f32, 0.0, -3.0));
        }
        for _ in 0..120 {
            h.update(STEP, &Controls::default());
        }
        let hs = build_static(&h.world);
        let mut hd = Mesh::new();
        build_dynamic(&h, &mut hd);
        for mesh in [&hs, &hd] {
            assert!(mesh.vertex_count() > 100 && mesh.vertex_count() % 3 == 0);
            assert!(mesh.data.iter().all(|x| x.is_finite()), "NaN/inf in hollow mesh");
        }
        assert!(hs.vertex_count() < 400_000, "hollow static verts: {}", hs.vertex_count());
        assert!(hd.vertex_count() < 100_000);
        assert_eq!(build_post(&h, 640, 360, 3)[11], 0.0, "no fade outside a gate walk");
        // Willow Pond, lamps half lit, queen out.
        let mut p = Game::new(Save::default(), 4);
        p.enter_area(Area::Pond);
        p.env.hour = 12.0;
        p.lamps_lit = [true, false, true, true];
        for d in 8..N_SPIRITS {
            p.spawn_spirit(d, p.player.pos + v(d as f32 - 8.0, 0.0, -3.0));
        }
        for _ in 0..120 {
            p.update(STEP, &Controls::default());
        }
        let ps = build_static(&p.world);
        let mut pd = Mesh::new();
        build_dynamic(&p, &mut pd);
        for mesh in [&ps, &pd] {
            assert!(mesh.vertex_count() > 100 && mesh.vertex_count() % 3 == 0);
            assert!(mesh.data.iter().all(|x| x.is_finite()), "NaN/inf in pond mesh");
        }
        assert!(ps.vertex_count() < 400_000, "pond static verts: {}", ps.vertex_count());
        assert!(pd.vertex_count() < 100_000, "pond dynamic verts: {}", pd.vertex_count());
        // Budgets used by the renderer (see gfx.rs): keep the per-frame upload small.
        assert!(dy.vertex_count() < 100_000, "dynamic verts: {}", dy.vertex_count());
        assert!(st.vertex_count() < 400_000, "static verts: {}", st.vertex_count());
    }
}
