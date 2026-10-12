//! Static layouts of the three areas: "Hollow Glade" (home, lantern puzzle), "Graveyard Hollow"
//! (crypt + rune stones) and "Willow Pond" (water, shed, lily lamps). Generated from fixed seeds so
//! every playthrough (and every save) sees the same places.

use crate::math::*;
use crate::rng::Rng;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Glade,
    Hollow,
    Pond,
}

impl Area {
    pub const ALL: [Area; 3] = [Area::Glade, Area::Hollow, Area::Pond];

    pub fn idx(self) -> usize {
        match self {
            Area::Glade => 0,
            Area::Hollow => 1,
            Area::Pond => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Area::Glade => "Hollow Glade",
            Area::Hollow => "Graveyard Hollow",
            Area::Pond => "Willow Pond",
        }
    }
}

/// A gate leads to another area. `exit` is where you stand when you come out of it.
pub struct Gate {
    pub pos: Vec3,
    pub to: Area,
    pub exit: Vec3,
}

/// A pond: an ellipse you can't walk into (spirits float over it just fine).
pub struct Water {
    pub x: f32,
    pub z: f32,
    pub rx: f32,
    pub rz: f32,
}

impl Water {
    /// Is (x, z) inside the pond, with the ellipse grown by `margin`?
    pub fn contains(&self, x: f32, z: f32, margin: f32) -> bool {
        let nx = (x - self.x) / (self.rx + margin);
        let nz = (z - self.z) / (self.rz + margin);
        nx * nx + nz * nz < 1.0
    }
}

pub struct Reed {
    pub x: f32,
    pub z: f32,
    pub h: f32,
}

pub struct LilyPad {
    pub x: f32,
    pub z: f32,
    pub s: f32,
    pub flower: bool,
}

pub struct Collider {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

pub struct Tree {
    pub x: f32,
    pub z: f32,
    pub h: f32,
    /// 0 = spooky pine, 1 = autumn round tree, 2 = bare dead tree, 3 = weeping willow
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

pub struct Grave {
    pub x: f32,
    pub z: f32,
    /// 0 = round headstone, 1 = cross, 2 = slab, 3 = obelisk
    pub kind: u8,
    pub yaw: f32,
    pub s: f32,
}

pub struct SourceSpot {
    pub pos: Vec3,
    /// Which treat grows here (index into TREATS).
    pub treat: usize,
}

pub struct World {
    pub area: Area,
    pub radius: f32,
    /// The cottage (glade only). In the Hollow this just sits at the gate and `has_home` is false.
    pub home: Vec3,
    pub has_home: bool,
    pub glade: Vec3,
    pub lanterns: [Vec3; 3],
    /// Gates to other areas (the glade has two, the others lead back home).
    pub gates: Vec<Gate>,
    /// Where this area's puzzle spirit shows up.
    pub puzzle_spot: Vec3,
    /// Hollow only: the crypt and its four rune stones (in melody order 0..3).
    pub crypt: Vec3,
    pub runes: [Vec3; 4],
    pub trees: Vec<Tree>,
    pub rocks: Vec<Rock>,
    pub shrooms: Vec<Shroom>,
    pub graves: Vec<Grave>,
    /// Pond only: water, reeds, lily pads, the shed and the four lily lamps (puzzle order 0..3).
    pub ponds: Vec<Water>,
    pub reeds: Vec<Reed>,
    pub pads: Vec<LilyPad>,
    pub shed: Vec3,
    pub lamps: [Vec3; 4],
    pub sources: Vec<SourceSpot>,
    pub colliders: Vec<Collider>,
}

fn clear_of(keepout: &[(f32, f32, f32)], x: f32, z: f32) -> bool {
    keepout.iter().all(|&(kx, kz, kr)| ((x - kx).powi(2) + (z - kz).powi(2)).sqrt() > kr)
}

/// Pillars, a door line and a stub of wall around a gate.
fn gate_colliders(gate: Vec3, out: &mut Vec<Collider>) {
    for sx in [-2.7f32, 2.7] {
        out.push(Collider { x: gate.x + sx, z: gate.z, r: 0.8 });
    }
    for k in -2..=2 {
        out.push(Collider { x: gate.x + k as f32 * 0.8, z: gate.z, r: 0.5 });
    }
    for k in 0..6 {
        let dx = 3.6 + k as f32 * 1.2;
        out.push(Collider { x: gate.x + dx, z: gate.z, r: 0.7 });
        out.push(Collider { x: gate.x - dx, z: gate.z, r: 0.7 });
    }
}

impl World {
    pub fn generate_area(area: Area) -> World {
        match area {
            Area::Glade => World::generate(1031),
            Area::Hollow => World::generate_hollow(2077),
            Area::Pond => World::generate_pond(3119),
        }
    }

    /// The gate in this area that leads to `to`.
    pub fn gate_to(&self, to: Area) -> &Gate {
        self.gates.iter().find(|g| g.to == to).expect("no gate to that area")
    }

    /// The gate you are standing close enough to use, if any.
    pub fn gate_near(&self, p: Vec3, range: f32) -> Option<&Gate> {
        self.gates.iter().find(|g| p.dist_xz(g.pos) < range)
    }

    /// Where you stand after arriving here from area `from` (the exit of the gate that leads back to it).
    pub fn arrival_from(&self, from: Area) -> Vec3 {
        self.gates.iter().find(|g| g.to == from).or(self.gates.first()).map(|g| g.exit).unwrap_or(Vec3::ZERO)
    }

    /// Pushes a position out of any pond (feet stay on the shore).
    pub fn keep_out_of_water(&self, p: &mut Vec3) {
        for w in &self.ponds {
            let dx = p.x - w.x;
            let dz = p.z - w.z;
            let nx = dx / (w.rx + 0.4);
            let nz = dz / (w.rz + 0.4);
            let d2 = nx * nx + nz * nz;
            if d2 < 1.0 {
                if d2 < 1e-6 {
                    p.x = w.x + w.rx + 0.5;
                } else {
                    let k = 1.0 / d2.sqrt();
                    p.x = w.x + dx * k;
                    p.z = w.z + dz * k;
                }
            }
        }
    }

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
        let gate = Vec3::new(-20.0, 0.0, -43.0);
        let pond_gate = Vec3::new(22.0, 0.0, 45.0);
        let mut keepout: Vec<(f32, f32, f32)> =
            vec![(home.x, home.z, 9.5), (glade.x, glade.z, 10.0), (gate.x, gate.z, 8.0), (pond_gate.x, pond_gate.z, 8.0)];

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
            if keepout.iter().take(4).any(|&(kx, kz, kr)| ((x - kx).powi(2) + (z - kz).powi(2)).sqrt() < kr) {
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
        gate_colliders(gate, &mut colliders);
        gate_colliders(pond_gate, &mut colliders);

        World {
            area: Area::Glade,
            radius,
            home,
            has_home: true,
            glade,
            lanterns,
            gates: vec![
                Gate { pos: gate, to: Area::Hollow, exit: gate + Vec3::new(0.0, 0.0, 3.8) },
                Gate { pos: pond_gate, to: Area::Pond, exit: pond_gate + Vec3::new(0.0, 0.0, -3.8) },
            ],
            puzzle_spot: glade,
            crypt: Vec3::ZERO,
            runes: [Vec3::ZERO; 4],
            trees,
            rocks,
            shrooms,
            graves: Vec::new(),
            ponds: Vec::new(),
            reeds: Vec::new(),
            pads: Vec::new(),
            shed: Vec3::ZERO,
            lamps: [Vec3::ZERO; 4],
            sources,
            colliders,
        }
    }

    /// Graveyard Hollow: bare trees, tombstones, a crypt that hums a melody, and four rune stones.
    pub fn generate_hollow(seed: u64) -> World {
        let mut rng = Rng::new(seed);
        let radius = 54.0;
        let gate = Vec3::new(0.0, 0.0, 46.0);
        let crypt = Vec3::new(-6.0, 0.0, -28.0);
        let runes = [(-9.0f32, 8.5f32), (-3.0, 10.5), (3.0, 10.5), (9.0, 8.5)].map(|(dx, dz)| crypt + Vec3::new(dx, 0.0, dz));
        let puzzle_spot = crypt + Vec3::new(0.0, 0.0, 6.5);

        // (x, z, radius) where nothing else may be placed.
        let mut keepout: Vec<(f32, f32, f32)> = vec![(gate.x, gate.z, 8.0), (crypt.x, crypt.z + 4.0, 14.0)];
        // A dirt path from the gate up to the crypt door stays clear of trees and graves.
        let door = crypt + Vec3::new(0.0, 0.0, 4.4);
        for i in 0..14 {
            let p = Vec3::lerp(gate + Vec3::new(0.0, 0.0, -6.0), door, i as f32 / 13.0);
            keepout.push((p.x, p.z, 2.3));
        }

        // Soul Cake grows only here (treat 3), with a little of everything else for convenience.
        let mut sources: Vec<SourceSpot> = Vec::new();
        for treat in [3usize, 3, 3, 3, 0, 1, 2] {
            let mut tries = 0;
            while tries < 500 {
                tries += 1;
                let a = rng.range(0.0, TAU);
                let r = rng.range(10.0, 44.0);
                let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                if clear_of(&keepout, p.x, p.z) && sources.iter().all(|s| s.pos.dist_xz(p) > 9.0) {
                    sources.push(SourceSpot { pos: p, treat });
                    break;
                }
            }
        }
        for s in &sources {
            keepout.push((s.pos.x, s.pos.z, 2.2));
        }

        // Graves first (they are the stars), then dead trees around them.
        let mut graves: Vec<Grave> = Vec::new();
        let mut tries = 0;
        while graves.len() < 46 && tries < 4000 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = 47.0 * rng.f32().sqrt();
            let (x, z) = (a.cos() * r, a.sin() * r);
            let near_crypt = ((x - crypt.x).powi(2) + (z - crypt.z).powi(2)).sqrt() < 6.5;
            let near_rune = runes.iter().any(|q| ((x - q.x).powi(2) + (z - q.z).powi(2)).sqrt() < 3.0);
            let near_puzzle = ((x - puzzle_spot.x).powi(2) + (z - puzzle_spot.z).powi(2)).sqrt() < 4.0;
            let ok = !near_crypt
                && !near_rune
                && !near_puzzle
                && clear_of(&keepout, x, z)
                && graves.iter().all(|g| ((x - g.x).powi(2) + (z - g.z).powi(2)).sqrt() > 2.6);
            if ok {
                graves.push(Grave { x, z, kind: rng.below(4) as u8, yaw: rng.range(-0.25, 0.25), s: rng.range(0.85, 1.2) });
            }
        }

        let mut trees: Vec<Tree> = Vec::new();
        let mut tries = 0;
        while trees.len() < 90 && tries < 4000 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = 50.0 * rng.f32().sqrt();
            let (x, z) = (a.cos() * r, a.sin() * r);
            let near_rune = runes.iter().any(|q| ((x - q.x).powi(2) + (z - q.z).powi(2)).sqrt() < 3.5);
            let ok = !near_rune
                && clear_of(&keepout, x, z)
                && graves.iter().all(|g| ((x - g.x).powi(2) + (z - g.z).powi(2)).sqrt() > 1.8)
                && trees.iter().all(|t| ((x - t.x).powi(2) + (z - t.z).powi(2)).sqrt() > 2.6);
            if ok {
                trees.push(Tree { x, z, h: rng.range(3.0, 5.0), kind: 2, tint: rng.f32() });
            }
        }
        for i in 0..90 {
            let a = TAU * i as f32 / 90.0 + rng.range(-0.03, 0.03);
            let r = rng.range(53.0, 57.5);
            trees.push(Tree { x: a.cos() * r, z: a.sin() * r, h: rng.range(3.8, 5.4), kind: 2, tint: rng.f32() });
        }

        let mut rocks = Vec::new();
        for _ in 0..16 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(6.0, 50.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            if !clear_of(&keepout[..2], x, z) || graves.iter().any(|g| ((x - g.x).powi(2) + (z - g.z).powi(2)).sqrt() < 2.0) {
                continue;
            }
            let s = rng.range(0.4, 1.1);
            rocks.push(Rock { x, z, sx: s * rng.range(0.9, 1.5), sy: s * rng.range(0.5, 0.9), sz: s, yaw: rng.range(0.0, TAU) });
        }

        let mut shrooms = Vec::new();
        for _ in 0..36 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(5.0, 50.0);
            shrooms.push(Shroom { x: a.cos() * r, z: a.sin() * r, s: rng.range(0.7, 1.3), hue: rng.below(3) as u8 });
        }

        let mut colliders: Vec<Collider> = trees.iter().map(|t| Collider { x: t.x, z: t.z, r: 0.45 }).collect();
        for g in &graves {
            colliders.push(Collider { x: g.x, z: g.z, r: 0.5 * g.s });
        }
        // The crypt: two rows of round colliders make a solid box with the door left reachable.
        for k in -2..=2 {
            let x = crypt.x + k as f32 * 1.6;
            colliders.push(Collider { x, z: crypt.z - 1.2, r: 1.7 });
            colliders.push(Collider { x, z: crypt.z + 0.9, r: 1.7 });
        }
        for q in &runes {
            colliders.push(Collider { x: q.x, z: q.z, r: 0.55 });
        }
        for r in &rocks {
            colliders.push(Collider { x: r.x, z: r.z, r: r.sx.max(r.sz) * 0.8 });
        }
        gate_colliders(gate, &mut colliders);

        World {
            area: Area::Hollow,
            radius,
            home: gate,
            has_home: false,
            glade: Vec3::ZERO,
            lanterns: [Vec3::ZERO; 3],
            gates: vec![Gate { pos: gate, to: Area::Glade, exit: gate + Vec3::new(0.0, 0.0, -3.8) }],
            puzzle_spot,
            crypt,
            runes,
            trees,
            rocks,
            shrooms,
            graves,
            ponds: Vec::new(),
            reeds: Vec::new(),
            pads: Vec::new(),
            shed: Vec3::ZERO,
            lamps: [Vec3::ZERO; 4],
            sources,
            colliders,
        }
    }

    /// Willow Pond: a lush forest around a pond, with a little shed and four lily lamps.
    pub fn generate_pond(seed: u64) -> World {
        let mut rng = Rng::new(seed);
        let radius = 54.0;
        let gate = Vec3::new(0.0, 0.0, 46.0);
        let water = Water { x: 12.0, z: -2.0, rx: 12.0, rz: 8.5 };
        let shed = Vec3::new(-16.0, 0.0, -24.0);
        let lamps = [(-7.0f32, 10.5f32), (-2.4, 12.2), (2.4, 12.2), (7.0, 10.5)].map(|(dx, dz)| shed + Vec3::new(dx, 0.0, dz));
        // Lady Lotus rises from the middle of the water.
        let puzzle_spot = Vec3::new(water.x - 2.0, 0.0, water.z);

        let mut keepout: Vec<(f32, f32, f32)> = vec![(gate.x, gate.z, 8.0), (shed.x, shed.z, 7.0)];
        for l in &lamps {
            keepout.push((l.x, l.z, 2.0));
        }
        // A dirt path from the gate to the shed door stays clear.
        let door = shed + Vec3::new(0.0, 0.0, 3.6);
        for i in 0..14 {
            let p = Vec3::lerp(gate + Vec3::new(0.0, 0.0, -6.0), door, i as f32 / 13.0);
            keepout.push((p.x, p.z, 2.3));
        }

        // Lily Jam grows only here (treat 4), plus one of everything else.
        let mut sources: Vec<SourceSpot> = Vec::new();
        for treat in [4usize, 4, 4, 4, 0, 1, 2, 3] {
            let mut tries = 0;
            while tries < 500 {
                tries += 1;
                let a = rng.range(0.0, TAU);
                let r = rng.range(10.0, 44.0);
                let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                if clear_of(&keepout, p.x, p.z) && !water.contains(p.x, p.z, 2.5) && sources.iter().all(|s| s.pos.dist_xz(p) > 9.0) {
                    sources.push(SourceSpot { pos: p, treat });
                    break;
                }
            }
        }
        for s in &sources {
            keepout.push((s.pos.x, s.pos.z, 2.2));
        }

        // Trees: pines, willows and round trees, with a few willows leaning over the shore.
        let mut trees: Vec<Tree> = Vec::new();
        let mut tries = 0;
        while trees.len() < 130 && tries < 4000 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = 50.0 * rng.f32().sqrt();
            let (x, z) = (a.cos() * r, a.sin() * r);
            let ok = clear_of(&keepout, x, z)
                && !water.contains(x, z, 2.0)
                && trees.iter().all(|t| ((x - t.x).powi(2) + (z - t.z).powi(2)).sqrt() > 2.4);
            if ok {
                let roll = rng.f32();
                let kind = if roll < 0.34 { 0 } else if roll < 0.68 { 3 } else { 1 };
                trees.push(Tree { x, z, h: rng.range(3.0, 4.4), kind, tint: rng.f32() });
            }
        }
        for i in 0..9 {
            let a = TAU * i as f32 / 9.0 + rng.range(-0.2, 0.2);
            let (x, z) = (water.x + (water.rx + 3.6) * a.cos(), water.z + (water.rz + 3.6) * a.sin());
            let ok = clear_of(&keepout, x, z) && trees.iter().all(|t| ((x - t.x).powi(2) + (z - t.z).powi(2)).sqrt() > 2.6);
            if ok {
                trees.push(Tree { x, z, h: rng.range(3.6, 4.4), kind: 3, tint: rng.f32() });
            }
        }
        for i in 0..90 {
            let a = TAU * i as f32 / 90.0 + rng.range(-0.03, 0.03);
            let r = rng.range(53.0, 57.5);
            trees.push(Tree { x: a.cos() * r, z: a.sin() * r, h: rng.range(3.6, 5.0), kind: 0, tint: rng.f32() });
        }

        let mut rocks = Vec::new();
        for _ in 0..22 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(6.0, 50.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            if !clear_of(&keepout[..2], x, z) || water.contains(x, z, 0.8) {
                continue;
            }
            let s = rng.range(0.4, 1.1);
            rocks.push(Rock { x, z, sx: s * rng.range(0.9, 1.5), sy: s * rng.range(0.5, 0.9), sz: s, yaw: rng.range(0.0, TAU) });
        }

        let mut shrooms = Vec::new();
        for _ in 0..40 {
            let a = rng.range(0.0, TAU);
            let r = rng.range(5.0, 50.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            if water.contains(x, z, 0.6) {
                continue;
            }
            shrooms.push(Shroom { x, z, s: rng.range(0.7, 1.4), hue: rng.below(3) as u8 });
        }

        // Cattails along the shore, lily pads on the water.
        let mut reeds = Vec::new();
        for _ in 0..90 {
            let a = rng.range(0.0, TAU);
            let off = rng.range(0.15, 1.4);
            let (x, z) = (water.x + (water.rx + off) * a.cos(), water.z + (water.rz + off) * a.sin());
            if clear_of(&keepout[..2], x, z) {
                reeds.push(Reed { x, z, h: rng.range(1.0, 1.9) });
            }
        }
        let mut pads = Vec::new();
        for _ in 0..24 {
            let a = rng.range(0.0, TAU);
            let r = rng.f32().sqrt() * 0.86;
            pads.push(LilyPad {
                x: water.x + water.rx * r * a.cos(),
                z: water.z + water.rz * r * a.sin(),
                s: rng.range(0.5, 0.95),
                flower: rng.chance(0.3),
            });
        }

        let mut colliders: Vec<Collider> =
            trees.iter().map(|t| Collider { x: t.x, z: t.z, r: if t.kind == 3 { 0.6 } else { 0.5 } }).collect();
        for r in &rocks {
            colliders.push(Collider { x: r.x, z: r.z, r: r.sx.max(r.sz) * 0.8 });
        }
        // The shed: a block of round colliders, door side left open.
        for k in -1..=1 {
            let x = shed.x + k as f32 * 1.8;
            colliders.push(Collider { x, z: shed.z - 0.9, r: 1.5 });
            colliders.push(Collider { x, z: shed.z + 0.8, r: 1.5 });
        }
        for l in &lamps {
            colliders.push(Collider { x: l.x, z: l.z, r: 0.45 });
        }
        gate_colliders(gate, &mut colliders);

        World {
            area: Area::Pond,
            radius,
            home: gate,
            has_home: false,
            glade: Vec3::ZERO,
            lanterns: [Vec3::ZERO; 3],
            gates: vec![Gate { pos: gate, to: Area::Glade, exit: gate + Vec3::new(0.0, 0.0, -3.8) }],
            puzzle_spot,
            crypt: Vec3::ZERO,
            runes: [Vec3::ZERO; 4],
            trees,
            rocks,
            shrooms,
            graves: Vec::new(),
            ponds: vec![water],
            reeds,
            pads,
            shed,
            lamps,
            sources,
            colliders,
        }
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
        // Every glade treat type is gatherable (Soul Cake is a Hollow treat).
        for k in 0..3 {
            assert!(a.sources.iter().any(|s| s.treat == k));
        }
        assert!(a.has_home && a.area == Area::Glade);
        assert_eq!(a.gates.len(), 2);
        for gate in &a.gates {
            assert!(gate.pos.dist_xz(a.home) > 30.0, "gates should be a trek away");
            for t in a.trees.iter().filter(|t| (t.x * t.x + t.z * t.z).sqrt() < 52.0) {
                assert!(Vec3::new(t.x, 0.0, t.z).dist_xz(gate.pos) > 7.9, "tree in the gateway");
            }
            assert!(gate.exit.dist_xz(gate.pos) > 3.2, "exit must be outside gate-use range");
            assert!((gate.exit.x.powi(2) + gate.exit.z.powi(2)).sqrt() < a.radius - 3.0, "exit inside the map");
        }
        assert_ne!(a.gate_to(Area::Hollow).pos.dist_xz(a.gate_to(Area::Pond).pos), 0.0);
        // arriving from the Hollow puts you at the Hollow gate, from the Pond at the Pond gate
        assert_eq!(a.arrival_from(Area::Pond).x, a.gate_to(Area::Pond).exit.x);
    }

    #[test]
    fn hollow_is_a_different_place_with_everything_reachable() {
        let h = World::generate_area(Area::Hollow);
        let h2 = World::generate_area(Area::Hollow);
        assert_eq!(h.area, Area::Hollow);
        assert!(!h.has_home);
        assert_eq!(h.trees.len(), h2.trees.len());
        assert!(h.graves.len() >= 35, "graves: {}", h.graves.len());
        assert!(h.sources.iter().filter(|s| s.treat == 3).count() >= 3, "Soul Cake plants");
        let back = h.gate_to(Area::Glade);
        assert!(back.exit.dist_xz(back.pos) > 3.2);
        // Nothing solid on top of the crypt door, the rune stones, the puzzle spot or a gather point.
        let blocked = |p: Vec3, pad: f32| h.colliders.iter().any(|c| ((p.x - c.x).powi(2) + (p.z - c.z).powi(2)).sqrt() < c.r + pad);
        assert!(!blocked(h.crypt + Vec3::new(0.0, 0.0, 4.4), 0.4), "crypt door must be reachable");
        assert!(!blocked(h.arrival_from(Area::Glade), 0.4));
        for (i, q) in h.runes.iter().enumerate() {
            // the stone itself blocks, but you must be able to stand right next to it
            assert!(!blocked(*q + Vec3::new(0.0, 0.0, 1.6), 0.4), "rune {i} is walled in");
        }
        for s in &h.sources {
            assert!(!blocked(s.pos, 0.3), "plant inside a solid");
        }
        assert!((back.pos.x.powi(2) + back.pos.z.powi(2)).sqrt() < h.radius - 3.0);
    }

    #[test]
    fn pond_has_water_a_shed_and_everything_reachable() {
        let p = World::generate_area(Area::Pond);
        let p2 = World::generate_area(Area::Pond);
        assert_eq!(p.area, Area::Pond);
        assert_eq!(p.trees.len(), p2.trees.len());
        assert_eq!(p.ponds.len(), 1);
        assert!(p.trees.iter().any(|t| t.kind == 3), "willows");
        assert!(p.reeds.len() > 40 && p.pads.len() > 10);
        assert!(p.sources.iter().filter(|s| s.treat == 4).count() >= 3, "Lily Jam plants");
        let w = &p.ponds[0];
        let blocked = |q: Vec3, pad: f32| p.colliders.iter().any(|c| ((q.x - c.x).powi(2) + (q.z - c.z).powi(2)).sqrt() < c.r + pad);
        // Nothing planted in the water, and every gather point / lamp / the exit is on dry ground.
        for t in &p.trees {
            if (t.x * t.x + t.z * t.z).sqrt() < 52.0 {
                assert!(!w.contains(t.x, t.z, 0.5), "tree in the pond");
            }
        }
        for s in &p.sources {
            assert!(!w.contains(s.pos.x, s.pos.z, 1.0), "plant in the pond");
            assert!(!blocked(s.pos, 0.3));
        }
        for (i, l) in p.lamps.iter().enumerate() {
            assert!(!w.contains(l.x, l.z, 1.5), "lamp {i} too close to the water");
            assert!(!blocked(*l + Vec3::new(0.0, 0.0, 1.6), 0.4), "lamp {i} is walled in");
        }
        let back = p.gate_to(Area::Glade);
        assert!(!blocked(back.exit, 0.4) && !w.contains(back.exit.x, back.exit.z, 0.0));
        assert!(!blocked(p.shed + Vec3::new(0.0, 0.0, 4.4), 0.4), "shed door reachable");
        // The queen rises from the middle of the pond.
        assert!(w.contains(p.puzzle_spot.x, p.puzzle_spot.z, 0.0));
        // Walking into the water pushes you back out to the shore, never through to the other side.
        let mut q = Vec3::new(w.x + 3.0, 0.0, w.z + 1.0);
        p.keep_out_of_water(&mut q);
        assert!(!w.contains(q.x, q.z, 0.0), "still in the pond");
        assert!(q.dist_xz(Vec3::new(w.x + 3.0, 0.0, w.z + 1.0)) < w.rx * 1.2);
        let mut dry = Vec3::new(-30.0, 0.0, 20.0);
        p.keep_out_of_water(&mut dry);
        assert_eq!((dry.x, dry.z), (-30.0, 20.0));
    }
}
