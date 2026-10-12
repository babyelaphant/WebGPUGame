//! Tiny dependency-free math library (just what the game needs).

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> Vec3 {
        let l = self.length();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            Vec3::ZERO
        }
    }
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
    pub fn with_y(self, y: f32) -> Vec3 {
        Vec3::new(self.x, y, self.z)
    }
    /// Distance on the ground plane (ignores height).
    pub fn dist_xz(self, o: Vec3) -> f32 {
        let dx = self.x - o.x;
        let dz = self.z - o.z;
        (dx * dx + dz * dz).sqrt()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

/// Column-major 4x4 matrix: `m[col][row]`. Matches WGSL `mat4x4<f32>` memory layout.
#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    /// Right-handed perspective projection with 0..1 depth (WebGPU clip space).
    pub fn perspective_rh(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fovy * 0.5).tan();
        Mat4 {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, far / (near - far), -1.0],
                [0.0, 0.0, near * far / (near - far), 0.0],
            ],
        }
    }

    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        let f = (target - eye).normalized();
        let s = f.cross(up).normalized();
        let u = s.cross(f);
        Mat4 {
            m: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    /// `self * rhs`
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.m[k][r] * rhs.m[c][k];
                }
                out[c][r] = sum;
            }
        }
        Mat4 { m: out }
    }

    pub fn transform_point(&self, p: Vec3) -> [f32; 4] {
        let v = [p.x, p.y, p.z, 1.0];
        let mut out = [0.0f32; 4];
        for r in 0..4 {
            for k in 0..4 {
                out[r] += self.m[k][r] * v[k];
            }
        }
        out
    }

    pub fn to_array(&self) -> [f32; 16] {
        let mut a = [0.0; 16];
        for c in 0..4 {
            for r in 0..4 {
                a[c * 4 + r] = self.m[c][r];
            }
        }
        a
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_puts_target_in_front() {
        let eye = Vec3::new(0.0, 10.0, 10.0);
        let vp = Mat4::perspective_rh(0.6, 1.5, 0.5, 100.0)
            .mul(&Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)));
        let c = vp.transform_point(Vec3::ZERO);
        let (x, y, z, w) = (c[0] / c[3], c[1] / c[3], c[2] / c[3], c[3]);
        assert!(w > 0.0);
        assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "target should be screen centre: {x} {y}");
        assert!((0.0..1.0).contains(&z), "depth in 0..1: {z}");
    }

    #[test]
    fn up_is_up_on_screen() {
        let eye = Vec3::new(0.0, 12.0, 8.0);
        let vp = Mat4::perspective_rh(0.6, 1.5, 0.5, 100.0)
            .mul(&Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)));
        // A point further from the camera (-z) should appear higher on screen (+y).
        let near = vp.transform_point(Vec3::new(0.0, 0.0, 3.0));
        let far = vp.transform_point(Vec3::new(0.0, 0.0, -3.0));
        assert!(far[1] / far[3] > near[1] / near[3]);
    }
}
