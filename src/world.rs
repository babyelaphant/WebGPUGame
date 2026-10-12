//! Static layout of the first area: "Hollow Glade". Generated from a fixed seed so every
//! playthrough (and every save) sees the same forest.

use crate::math::*;
use crate::rng::Rng;
use std::f32::consts::TAU;

pub struct Collider {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

pub struct Tree {
    pub x: f32,
    pub z: f32,
    pub h: f32,
    /// 0 = spooky pine, 1 = autumn round tree
    pub kind: u8,
    pub tint: f32,
}

pub struct Rock {
    pub x: f32,
    pub z: f32,
    pub sx: f32,
    pub sy: f32,
    pub sz: f32,
    pub yaw: f32,
}

pub struct Shroom {
    pub x: f32,
    pub z: f32,
    pub s: f32,
    pub hue: u8,
}

pub struct SourceSpot {
    pub pos: Vec3,
    /// Which treat grows here (index into TREATS).
    pub treat: usize,
}

pub struct World {
    pub radius: f32,
    pub home: Vec3,
    pub glade: Vec3,
    pub lanterns: [Vec3; 3],
    pub trees: Vec<Tree>,
    pub rocks: Vec<Rock>,
    pub shrooms: Vec<Shroom>,
    pub sources: Vec<SourceSpot>,
    pub colliders: Vec<Collider>,
}

impl World {
    pub fn generate(seed: u64) -> World {
        let mut rng = Rng::new(seed);
        let radius = 54.0;
        let home = Vec3::ZERO;
        let glade = Vec3::new(24.0, 0.0, -20.0);
        let lanterns = [90.0f32, 210.0, 330.0].map(|deg| {
            let a = deg.to_radians();
            glade + Vec3::new(a.cos() * 5.5, 0.0, a.sin() * 5.5)
        });

        // (x, z, radius) areas where trees must not grow.
        let mut keepout: Vec<(f32, f32, f32)> = vec![(home.x, home.z, 9.5), (glade.x, glade.z, 10.0)];

        // Treat sources: three of each kind, spread out.
        let mut sources: Vec<SourceSpot> = Vec::new();
        for treat in 0..3 {
            let mut placed = 0;
            let mut tries = 0;
            while placed < 3 && tries < 500 {
                tries += 1;
                let a = rng.range(0.0, TAU);
                let r = rng.range(9.0, 44.0);
                let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let ok = keepout.iter().all(|&(x, z, kr)| ((p.x - x).powi(2) + (p.z - z).powi(2)).sqrt() > kr)
                    && sources.iter().all(|s| s.pos.dist_xz(p) > 8.0);
                if ok {
                    sources.push(SourceSpot { pos: p, treat });
                    placed += 1;
                }
            }
        }
        for s in &sources {
            keepout.push((s.pos.x, s.pos.z, 2.2));
        }

        // Trees scattered through the glade...
        let mut trees: Vec<Tree> = Vec::new();
        let mut tries = 0;
        while trees.len() < 170 && tries < 3000 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = 50.0 * rng.f32().sqrt();
            let (x, z) = (a.cos() * r, a.sin() * r);
            let ok = keepout.iter().all(|&(kx, kz, kr)| ((x - kx).powi(2) + (z - kz).powi(2)).sqrt() > kr)
                && trees.iter().all(|t| ((x - t.x).powi(2) + (z - t.z).powi(2)).sqrt() > 2.2);
            if ok {
                let kind = if rng.chance(0.65) { 0 } else { 1 };
                trees.push(Tree { x, z, h: rng.range(3.0, 4.6), kind, tint: rng.f32() });
            }
        }
        // ...and a thick ring at the edge so you can't wander off the map.
        for i in 0..90 {
            let a = TAU * i as f32 / 90.0 + rng.range(-0.03, 0.03);
            let r = rng.range(53.0, 57.5);
            trees.push(Tree { x: a.cos() * r, z: a.sin() * r, h: rng.range(3.6, 5.0), kind: 0, tint: rng.f32() });
        }

        let mut rocks = Vec::new();
        for _ in 0..28 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(6.0, 50.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            if keepout.iter().take(2).any(|&(kx, kz, kr)| ((x - kx).powi(2) + (z - kz).powi(2)).sqrt() < kr) {
                continue;
            }
            let s = rng.range(0.4, 1.2);
            rocks.push(Rock { x, z, sx: s * rng.range(0.9, 1.5), sy: s * rng.range(0.5, 0.9), sz: s, yaw: rng.range(0.0, TAU) });
        }

        let mut shrooms = Vec::new();
        for _ in 0..60 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(5.0, 50.0);
            shrooms.push(Shroom { x: a.cos() * r, z: a.sin() * r, s: rng.range(0.7, 1.4), hue: rng.below(3) as u8 });
        }

        let mut colliders: Vec<Collider> = trees.iter().map(|t| Collider { x: t.x, z: t.z, r: 0.5 }).collect();
        colliders.push(Collider { x: home.x, z: home.z, r: 3.3 });
        for l in &lanterns {
            colliders.push(Collider { x: l.x, z: l.z, r: 0.4 });
        }
        for r in &rocks {
            colliders.push(Collider { x: r.x, z: r.z, r: r.sx.max(r.sz) * 0.8 });
        }

        World { radius, home, glade, lanterns, trees, rocks, shrooms, sources, colliders }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_sane() {
        let a = World::generate(1031);
        let b = World::generate(1031);
        assert_eq!(a.trees.len(), b.trees.len());
        assert_eq!(a.sources.len(), 9);
        assert!(a.trees.len() > 120);
        // Nothing grows on top of the cottage or the lantern glade.
        for t in &a.trees {
            assert!(((t.x).powi(2) + (t.z).powi(2)).sqrt() > 9.0);
        }
        for l in &a.lanterns {
            assert!(l.dist_xz(a.glade) < 6.0);
        }
        // Every treat type is gatherable.
        for k in 0..3 {
            assert!(a.sources.iter().any(|s| s.treat == k));
        }
    }
}
