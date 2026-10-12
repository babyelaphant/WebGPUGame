//! CPU mesh builder for placeholder art. Everything is flat-shaded low-poly built from primitives.
//!
//! Vertex layout (10 floats): position(3) normal(3) color(3) emissive(1).
//! `emissive` 0 = fully lit, 1 = self-lit, and -1 is a special marker for procedural grass.
//!
//! Real art will arrive as glTF from Blender (milestone 2) and replace these primitives
//! one prop at a time; the vertex format is intentionally the same.

use crate::math::*;
use std::f32::consts::TAU;

pub const FLOATS_PER_VERTEX: usize = 10;

/// How to move one model part this frame (see `Mesh::add_part`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartXf {
    pub rot: [f32; 3],
    pub scale: [f32; 3],
    pub offset: [f32; 3],
}

impl PartXf {
    pub const IDENT: PartXf = PartXf { rot: [0.0; 3], scale: [1.0; 3], offset: [0.0; 3] };
}

pub struct Mesh {
    pub data: Vec<f32>,
    pos: Vec3,
    scale: Vec3,
    sin: f32,
    cos: f32,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh { data: Vec::new(), pos: Vec3::ZERO, scale: Vec3::ONE, sin: 0.0, cos: 1.0 }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.reset_xf();
    }

    pub fn vertex_count(&self) -> usize {
        self.data.len() / FLOATS_PER_VERTEX
    }

    /// Everything drawn after this call is scaled, rotated about Y by `yaw`, then moved to `pos`.
    pub fn set_xf(&mut self, pos: Vec3, yaw: f32, scale: Vec3) {
        self.pos = pos;
        self.scale = scale;
        self.sin = yaw.sin();
        self.cos = yaw.cos();
    }

    pub fn set_xf_u(&mut self, pos: Vec3, yaw: f32, s: f32) {
        self.set_xf(pos, yaw, Vec3::new(s, s, s));
    }

    pub fn reset_xf(&mut self) {
        self.set_xf(Vec3::ZERO, 0.0, Vec3::ONE);
    }

    fn xp(&self, p: Vec3) -> Vec3 {
        let s = Vec3::new(p.x * self.scale.x, p.y * self.scale.y, p.z * self.scale.z);
        Vec3::new(s.x * self.cos + s.z * self.sin, s.y, -s.x * self.sin + s.z * self.cos) + self.pos
    }

    /// Adds a loaded glTF model, moved by the current transform (scale, yaw about Y, position).
    pub fn add_model(&mut self, model: &crate::gltf::Model) {
        for v in model.data.chunks_exact(FLOATS_PER_VERTEX) {
            let p = self.xp(Vec3::new(v[0], v[1], v[2]));
            let n = Vec3::new(v[3] * self.cos + v[5] * self.sin, v[4], -v[3] * self.sin + v[5] * self.cos);
            self.data.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, v[6], v[7], v[8], v[9]]);
        }
    }

    /// Adds one animated part of a model: rotated (about its pivot, in z, then x, then y), scaled about
    /// the pivot and nudged by `xf`, then moved by the current transform like everything else.
    pub fn add_part(&mut self, part: &crate::gltf::Part, xf: &PartXf) {
        let (sx, cx) = xf.rot[0].sin_cos();
        let (sy, cy) = xf.rot[1].sin_cos();
        let (sz, cz) = xf.rot[2].sin_cos();
        let rot = |p: [f32; 3]| -> [f32; 3] {
            let (x, y) = (p[0] * cz - p[1] * sz, p[0] * sz + p[1] * cz);
            let (y, z) = (y * cx - p[2] * sx, y * sx + p[2] * cx);
            [x * cy + z * sy, y, -x * sy + z * cy]
        };
        for v in part.data.chunks_exact(FLOATS_PER_VERTEX) {
            let rel = [(v[0] - part.pivot[0]) * xf.scale[0], (v[1] - part.pivot[1]) * xf.scale[1], (v[2] - part.pivot[2]) * xf.scale[2]];
            let r = rot(rel);
            let local = Vec3::new(
                part.pivot[0] + r[0] + xf.offset[0],
                part.pivot[1] + r[1] + xf.offset[1],
                part.pivot[2] + r[2] + xf.offset[2],
            );
            let p = self.xp(local);
            let n = rot([v[3], v[4], v[5]]);
            let n = Vec3::new(n[0] * self.cos + n[2] * self.sin, n[1], -n[0] * self.sin + n[2] * self.cos);
            self.data.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, v[6], v[7], v[8], v[9]]);
        }
    }

    /// One triangle. The normal is flipped to point away from `center`, so winding never matters.
    pub fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, center: Vec3, col: [f32; 3], emi: f32) {
        let (a, b, c, center) = (self.xp(a), self.xp(b), self.xp(c), self.xp(center));
        let mut n = (b - a).cross(c - a);
        if n.length() < 1e-9 {
            return;
        }
        n = n.normalized();
        let centroid = (a + b + c) * (1.0 / 3.0);
        if n.dot(centroid - center) < 0.0 {
            n = -n;
        }
        for p in [a, b, c] {
            self.data.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, col[0], col[1], col[2], emi]);
        }
    }

    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, center: Vec3, col: [f32; 3], emi: f32) {
        self.tri(a, b, c, center, col, emi);
        self.tri(a, c, d, center, col, emi);
    }

    pub fn box_(&mut self, center: Vec3, half: Vec3, col: [f32; 3], emi: f32) {
        let c = |ix: f32, iy: f32, iz: f32| {
            Vec3::new(
                center.x + half.x * (ix * 2.0 - 1.0),
                center.y + half.y * (iy * 2.0 - 1.0),
                center.z + half.z * (iz * 2.0 - 1.0),
            )
        };
        let faces = [
            [c(1.0, 0.0, 0.0), c(1.0, 1.0, 0.0), c(1.0, 1.0, 1.0), c(1.0, 0.0, 1.0)],
            [c(0.0, 0.0, 0.0), c(0.0, 0.0, 1.0), c(0.0, 1.0, 1.0), c(0.0, 1.0, 0.0)],
            [c(0.0, 1.0, 0.0), c(0.0, 1.0, 1.0), c(1.0, 1.0, 1.0), c(1.0, 1.0, 0.0)],
            [c(0.0, 0.0, 0.0), c(1.0, 0.0, 0.0), c(1.0, 0.0, 1.0), c(0.0, 0.0, 1.0)],
            [c(0.0, 0.0, 1.0), c(1.0, 0.0, 1.0), c(1.0, 1.0, 1.0), c(0.0, 1.0, 1.0)],
            [c(0.0, 0.0, 0.0), c(0.0, 1.0, 0.0), c(1.0, 1.0, 0.0), c(1.0, 0.0, 0.0)],
        ];
        for f in faces {
            self.quad(f[0], f[1], f[2], f[3], center, col, emi);
        }
    }

    /// Tapered cylinder standing on `base` (bottom radius `r0`, top radius `r1`, height `h`).
    pub fn frustum(&mut self, base: Vec3, r0: f32, r1: f32, h: f32, seg: usize, col: [f32; 3], emi: f32) {
        let center = base + Vec3::new(0.0, h * 0.5, 0.0);
        let top = base + Vec3::new(0.0, h, 0.0);
        for i in 0..seg {
            let a0 = TAU * i as f32 / seg as f32;
            let a1 = TAU * (i + 1) as f32 / seg as f32;
            let b0 = base + Vec3::new(r0 * a0.cos(), 0.0, r0 * a0.sin());
            let b1 = base + Vec3::new(r0 * a1.cos(), 0.0, r0 * a1.sin());
            let t0 = top + Vec3::new(r1 * a0.cos(), 0.0, r1 * a0.sin());
            let t1 = top + Vec3::new(r1 * a1.cos(), 0.0, r1 * a1.sin());
            self.quad(b0, b1, t1, t0, center, col, emi);
            self.tri(top, t0, t1, center, col, emi);
        }
    }

    pub fn cylinder(&mut self, base: Vec3, r: f32, h: f32, seg: usize, col: [f32; 3], emi: f32) {
        self.frustum(base, r, r, h, seg, col, emi);
    }

    pub fn cone(&mut self, base: Vec3, r: f32, h: f32, seg: usize, col: [f32; 3], emi: f32) {
        self.frustum(base, r, 0.0, h, seg, col, emi);
    }

    pub fn ellipsoid(&mut self, center: Vec3, radii: Vec3, stacks: usize, slices: usize, col: [f32; 3], emi: f32) {
        let pt = |th: f32, ph: f32| {
            center + Vec3::new(radii.x * th.sin() * ph.cos(), radii.y * th.cos(), radii.z * th.sin() * ph.sin())
        };
        for i in 0..stacks {
            let t0 = std::f32::consts::PI * i as f32 / stacks as f32;
            let t1 = std::f32::consts::PI * (i + 1) as f32 / stacks as f32;
            for j in 0..slices {
                let p0 = TAU * j as f32 / slices as f32;
                let p1 = TAU * (j + 1) as f32 / slices as f32;
                self.quad(pt(t0, p0), pt(t0, p1), pt(t1, p1), pt(t1, p0), center, col, emi);
            }
        }
    }

    pub fn sphere(&mut self, center: Vec3, r: f32, col: [f32; 3], emi: f32) {
        self.ellipsoid(center, Vec3::new(r, r, r), 4, 6, col, emi);
    }

    /// Big flat ground plane. Emissive = -1 tells the shader to paint procedural grass on it.
    pub fn ground(&mut self, half: f32, y: f32, col: [f32; 3]) {
        let below = Vec3::new(0.0, y - 1.0, 0.0);
        self.quad(
            Vec3::new(-half, y, -half),
            Vec3::new(half, y, -half),
            Vec3::new(half, y, half),
            Vec3::new(-half, y, half),
            below,
            col,
            -1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_has_36_vertices_and_outward_normals() {
        let mut m = Mesh::new();
        m.box_(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 1.0), [1.0; 3], 0.0);
        assert_eq!(m.vertex_count(), 36);
        for v in m.data.chunks(FLOATS_PER_VERTEX) {
            let p = Vec3::new(v[0], v[1] - 1.0, v[2]);
            let n = Vec3::new(v[3], v[4], v[5]);
            assert!(n.dot(p) > 0.0, "normal should point away from box centre");
            assert!((n.length() - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn transform_moves_and_rotates() {
        let mut m = Mesh::new();
        m.set_xf_u(Vec3::new(10.0, 0.0, 0.0), std::f32::consts::FRAC_PI_2, 2.0);
        m.box_(Vec3::ZERO, Vec3::new(0.5, 0.5, 0.5), [1.0; 3], 0.0);
        let min_x = m.data.chunks(FLOATS_PER_VERTEX).map(|v| v[0]).fold(f32::MAX, f32::min);
        let max_x = m.data.chunks(FLOATS_PER_VERTEX).map(|v| v[0]).fold(f32::MIN, f32::max);
        assert!((min_x - 9.0).abs() < 1e-3 && (max_x - 11.0).abs() < 1e-3);
    }

    #[test]
    fn ground_faces_up() {
        let mut m = Mesh::new();
        m.ground(10.0, 0.0, [0.3; 3]);
        for v in m.data.chunks(FLOATS_PER_VERTEX) {
            assert!(v[4] > 0.99);
            assert_eq!(v[9], -1.0);
        }
    }
}
