//! All gameplay rules. No rendering, no browser: pure state + `update(dt)`, so it's testable.
//!
//! The loop:  explore -> spot spirits -> lure with treats / solve the lantern puzzle
//!            -> they follow you -> bring them home -> candles -> upgrades -> explore more.

use crate::audio::Sfx;
use crate::data::*;
use crate::env::*;
use crate::math::*;
use crate::rng::Rng;
use crate::world::*;
use std::collections::VecDeque;
use std::f32::consts::TAU;

pub const STEP: f32 = 1.0 / 60.0;
pub const HOME_ZONE: f32 = 6.0;
const MAX_DROPS: usize = 6;
const MAX_SPIRITS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Interact,
    DropTreat,
    NextTreat,
    PrevTreat,
    Buy(usize),
    ToggleJournal,
    ToggleDebug,
    CycleWeather,
    HourShift(f32),
}

/// Continuous input, sampled every fixed step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// -1 (left) .. 1 (right)
    pub x: f32,
    /// -1 (up the screen) .. 1 (down the screen)
    pub z: f32,
    pub run: bool,
}

pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub facing: f32,
    pub moving: bool,
    pub walk_phase: f32,
    /// Seconds spent standing still (shy spirits care about this).
    pub still_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpState {
    Wander,
    /// Heading to the treat drop with this id.
    Approach(u32),
    /// (seconds left, treat id)
    Eat(f32, u32),
    Follow,
    /// seconds left
    Flee(f32),
    Leaving,
}

#[derive(Clone, Debug)]
pub struct Spirit {
    pub id: u32,
    pub def: usize,
    pub pos: Vec3,
    pub yaw: f32,
    pub state: SpState,
    pub target: Vec3,
    pub timer: f32,
    pub age: f32,
    /// 0..1 appear/disappear scale.
    pub fade: f32,
    pub phase: f32,
}

pub struct TreatDrop {
    pub id: u32,
    pub kind: usize,
    pub pos: Vec3,
    pub ttl: f32,
}

pub struct SourceState {
    pub pos: Vec3,
    pub treat: usize,
    pub ready: bool,
    pub timer: f32,
}

pub struct Toast {
    pub text: String,
    pub ttl: f32,
}

pub struct Burst {
    pub pos: Vec3,
    pub age: f32,
    pub color: [f32; 3],
}

pub struct Game {
    pub env: Environment,
    pub world: World,
    pub player: Player,
    pub spirits: Vec<Spirit>,
    pub drops: Vec<TreatDrop>,
    pub sources: Vec<SourceState>,
    pub lantern_lit: [bool; 3],
    pub puzzle_solved: bool,
    pub save: Save,
    pub selected: usize,
    pub toasts: Vec<Toast>,
    pub bursts: Vec<Burst>,
    pub time: f32,
    pub cam: Vec3,
    pub show_journal: bool,
    pub show_debug: bool,
    /// Set when `save` changed; the web layer persists it.
    pub dirty: bool,
    pub fps: f32,
    /// Sound events since the last frame; the browser layer drains this.
    pub sfx: Vec<Sfx>,
    follow_order: Vec<u32>,
    trail: VecDeque<Vec3>,
    rng: Rng,
    spawn_timer: f32,
    deposit_cd: f32,
    next_id: u32,
    autosave: f32,
}

impl Game {
    pub fn new(save: Save, seed: u64) -> Game {
        let world = World::generate(1031);
        let sources = world
            .sources
            .iter()
            .map(|s| SourceState { pos: s.pos, treat: s.treat, ready: true, timer: 0.0 })
            .collect();
        let start = Vec3::new(0.0, 0.0, 6.0);
        Game {
            env: Environment::new(18.5),
            world,
            player: Player { pos: start, vel: Vec3::ZERO, facing: 0.0, moving: false, walk_phase: 0.0, still_time: 0.0 },
            spirits: Vec::new(),
            drops: Vec::new(),
            sources,
            lantern_lit: [false; 3],
            puzzle_solved: false,
            save,
            selected: 0,
            toasts: Vec::new(),
            bursts: Vec::new(),
            time: 0.0,
            cam: start,
            show_journal: false,
            show_debug: false,
            dirty: false,
            fps: 60.0,
            sfx: Vec::new(),
            follow_order: Vec::new(),
            trail: VecDeque::new(),
            rng: Rng::new(seed),
            spawn_timer: 3.0,
            deposit_cd: 0.0,
            next_id: 1,
            autosave: 0.0,
        }
    }

    // ---------------------------------------------------------------- derived stats

    pub fn lure_radius(&self) -> f32 {
        7.5 + 2.0 * self.save.upgrades[0] as f32
    }
    pub fn lantern_radius(&self) -> f32 {
        9.0 + 3.0 * self.save.upgrades[0] as f32
    }
    pub fn treat_cap(&self) -> u32 {
        5 + 3 * self.save.upgrades[2] as u32
    }
    pub fn speed_mul(&self) -> f32 {
        1.0 + 0.15 * self.save.upgrades[1] as f32
    }
    pub fn near_home(&self) -> bool {
        self.player.pos.dist_xz(self.world.home) < HOME_ZONE
    }
    pub fn followers(&self) -> Vec<&Spirit> {
        self.follow_order
            .iter()
            .filter_map(|id| self.spirits.iter().find(|s| s.id == *id))
            .collect()
    }

    pub fn play(&mut self, s: Sfx) {
        if self.sfx.len() < 24 {
            self.sfx.push(s);
        }
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), ttl: 4.5 });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    // ---------------------------------------------------------------- discrete actions

    pub fn act(&mut self, a: Action) {
        match a {
            Action::Interact => self.interact(),
            Action::DropTreat => self.drop_treat(),
            Action::NextTreat => {
                self.selected = (self.selected + 1) % N_TREATS;
                self.play(Sfx::Ui);
            }
            Action::PrevTreat => {
                self.selected = (self.selected + N_TREATS - 1) % N_TREATS;
                self.play(Sfx::Ui);
            }
            Action::Buy(i) => self.buy(i),
            Action::ToggleJournal => {
                self.show_journal = !self.show_journal;
                self.play(Sfx::Ui);
            }
            Action::ToggleDebug => self.show_debug = !self.show_debug,
            Action::CycleWeather => {
                let w = self.env.weather.next();
                self.env.set_weather(w);
            }
            Action::HourShift(h) => self.env.hour = (self.env.hour + h).rem_euclid(24.0),
        }
    }

    fn interact(&mut self) {
        let pp = self.player.pos;

        // 1. Light a lantern in the glade (the puzzle).
        for i in 0..3 {
            if !self.lantern_lit[i] && pp.dist_xz(self.world.lanterns[i]) < 2.8 {
                self.lantern_lit[i] = true;
                let lit = self.lantern_lit.iter().filter(|x| **x).count();
                if lit == 3 {
                    self.puzzle_solved = true;
                    self.play(Sfx::Puzzle);
                    self.toast("All three lanterns glow. The glade hums... something moonlit stirs.");
                } else {
                    self.play(Sfx::Lantern);
                    self.toast(format!("The lantern flickers to life ({lit}/3)."));
                }
                return;
            }
        }

        // 2. Gather treats from a plant.
        let mut best: Option<(usize, f32)> = None;
        for (i, s) in self.sources.iter().enumerate() {
            let d = s.pos.dist_xz(pp);
            if s.ready && d < 2.6 && best.map_or(true, |(_, b)| d < b) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            let kind = self.sources[i].treat;
            let cap = self.treat_cap();
            let have = self.save.treats[kind];
            if have >= cap {
                self.play(Sfx::Nope);
                self.toast(format!("Your pouch is full of {}.", TREATS[kind].name));
                return;
            }
            let amount = (1 + self.rng.below(2) as u32).min(cap - have);
            self.save.treats[kind] += amount;
            self.sources[i].ready = false;
            self.sources[i].timer = 50.0;
            self.dirty = true;
            self.play(Sfx::Pickup);
            self.toast(format!("+{} {}", amount, TREATS[kind].name));
        }
    }

    fn drop_treat(&mut self) {
        let kind = self.selected;
        if self.drops.len() >= MAX_DROPS {
            self.play(Sfx::Nope);
            self.toast("You've already laid out plenty of treats.");
            return;
        }
        if self.save.treats[kind] == 0 {
            self.play(Sfx::Nope);
            self.toast(format!("Out of {}. Gather more from the glowing plants.", TREATS[kind].name));
            return;
        }
        self.save.treats[kind] -= 1;
        self.dirty = true;
        let f = self.player.facing;
        let pos = self.player.pos + Vec3::new(f.sin(), 0.0, f.cos()) * 1.1;
        let id = self.alloc_id();
        self.drops.push(TreatDrop { id, kind, pos, ttl: 60.0 });
        self.play(Sfx::Drop);
    }

    fn buy(&mut self, i: usize) {
        if i >= N_UPGRADES {
            return;
        }
        if !self.near_home() {
            self.play(Sfx::Nope);
            self.toast("Upgrades are bought at your cottage.");
            return;
        }
        let up = &UPGRADES[i];
        let lvl = self.save.upgrades[i] as usize;
        if lvl >= up.costs.len() {
            self.play(Sfx::Nope);
            self.toast(format!("{} is already maxed out.", up.name));
            return;
        }
        let cost = up.costs[lvl];
        if self.save.candles < cost {
            self.play(Sfx::Nope);
            self.toast(format!("You need {cost} candles for {}.", up.name));
            return;
        }
        self.save.candles -= cost;
        self.save.upgrades[i] += 1;
        self.dirty = true;
        self.play(Sfx::Buy);
        self.toast(format!("{} upgraded to level {}!", up.name, lvl + 1));
    }

    fn alloc_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    // ---------------------------------------------------------------- fixed update

    pub fn update(&mut self, dt: f32, ctl: &Controls) {
        self.time += dt;

        if self.env.update(dt, &mut self.rng) {
            // Dawn: the lanterns burn out and the glade sleeps until next night.
            self.lantern_lit = [false; 3];
            self.puzzle_solved = false;
        }

        self.announce_weather();
        self.update_player(dt, ctl);
        self.update_trail();
        self.update_spawner(dt);
        self.update_spirits(dt);
        self.update_drops(dt);
        self.update_sources(dt);
        self.update_deposit(dt);

        // Smooth camera with a little look-ahead.
        let want = self.player.pos + self.player.vel * 0.35;
        let k = 1.0 - (-5.0 * dt).exp();
        self.cam = self.cam.lerp(want, k);

        for t in &mut self.toasts {
            t.ttl -= dt;
        }
        self.toasts.retain(|t| t.ttl > 0.0);
        for b in &mut self.bursts {
            b.age += dt;
        }
        self.bursts.retain(|b| b.age < 1.4);

        self.autosave += dt;
        if self.autosave > 20.0 {
            self.autosave = 0.0;
            self.dirty = true;
        }
    }

    /// Tells the player when the weather is about to turn and when it has started to.
    /// (Visual changes alone are easy to miss, so we also use a toast, a HUD forecast and a sound.)
    fn announce_weather(&mut self) {
        if let Some(w) = self.env.warning.take() {
            let msg = match w {
                Weather::Rain => "The air grows heavy... rain is on its way.",
                Weather::Fog => "A chill drifts in... fog is on its way.",
                Weather::Clear => "The clouds are thinning... clear skies soon.",
            };
            self.toast(format!("{} {}", w.icon(), msg));
            self.play(Sfx::Ui);
        }
        if let Some(w) = self.env.changed.take() {
            let msg = match w {
                Weather::Rain => "Rain begins to fall.",
                Weather::Fog => "Fog creeps in between the trees.",
                Weather::Clear => "The sky clears.",
            };
            self.toast(format!("{} {}", w.icon(), msg));
            self.play(Sfx::Weather(w));
        }
    }

    fn update_player(&mut self, dt: f32, ctl: &Controls) {
        let mut dir = Vec3::new(ctl.x, 0.0, ctl.z);
        if dir.length() > 1.0 {
            dir = dir.normalized();
        }
        let base = if ctl.run { 5.6 } else { 3.5 } * self.speed_mul();
        let k = 1.0 - (-14.0 * dt).exp();
        let p = &mut self.player;
        p.vel = p.vel + (dir * base - p.vel) * k;
        p.pos += p.vel * dt;

        for c in &self.world.colliders {
            let dx = p.pos.x - c.x;
            let dz = p.pos.z - c.z;
            let d = (dx * dx + dz * dz).sqrt();
            let min = c.r + 0.35;
            if d < min && d > 1e-4 {
                p.pos.x = c.x + dx / d * min;
                p.pos.z = c.z + dz / d * min;
            }
        }
        let lim = self.world.radius - 3.0;
        let len = (p.pos.x * p.pos.x + p.pos.z * p.pos.z).sqrt();
        if len > lim {
            p.pos.x *= lim / len;
            p.pos.z *= lim / len;
        }

        let speed = p.vel.length();
        p.moving = speed > 0.5;
        if p.moving {
            p.facing = p.vel.x.atan2(p.vel.z);
            p.still_time = 0.0;
        } else {
            p.still_time += dt;
        }
        let step_before = (p.walk_phase / std::f32::consts::PI).floor();
        p.walk_phase += speed * dt * 2.4;
        if p.moving && (p.walk_phase / std::f32::consts::PI).floor() != step_before && self.sfx.len() < 24 {
            self.sfx.push(Sfx::Step);
        }
    }

    fn update_trail(&mut self) {
        let pp = self.player.pos;
        let far_enough = self.trail.back().map_or(true, |b| b.dist_xz(pp) > 0.35);
        if far_enough {
            self.trail.push_back(pp);
            if self.trail.len() > 240 {
                self.trail.pop_front();
            }
        }
    }

    fn trail_point(&self, back: usize) -> Vec3 {
        let len = self.trail.len();
        if len > back {
            self.trail[len - 1 - back]
        } else {
            self.trail.front().copied().unwrap_or(self.player.pos)
        }
    }

    // ---------------------------------------------------------------- spawning

    /// Spawns a spirit of species `def` at `pos`. Public so tests (and debug tools) can use it.
    pub fn spawn_spirit(&mut self, def: usize, pos: Vec3) -> u32 {
        let id = self.alloc_id();
        let phase = self.rng.range(0.0, TAU);
        self.spirits.push(Spirit {
            id,
            def,
            pos,
            yaw: self.rng.range(0.0, TAU),
            state: SpState::Wander,
            target: pos,
            timer: 0.0,
            age: 0.0,
            fade: 0.0,
            phase,
        });
        id
    }

    fn random_spawn_point(&mut self) -> Vec3 {
        let pp = self.player.pos;
        for _ in 0..16 {
            let a = self.rng.range(0.0, TAU);
            let d = self.rng.range(16.0, 30.0);
            let p = pp + Vec3::new(a.cos() * d, 0.0, a.sin() * d);
            let len = (p.x * p.x + p.z * p.z).sqrt();
            if len < self.world.radius - 8.0 && p.dist_xz(self.world.home) > 8.0 {
                return p;
            }
        }
        let a = self.rng.range(0.0, TAU);
        Vec3::new(a.cos() * 25.0, 0.0, a.sin() * 25.0)
    }

    fn update_spawner(&mut self, dt: f32) {
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 {
            return;
        }
        self.spawn_timer = 2.0;
        if self.spirits.len() >= MAX_SPIRITS {
            return;
        }
        let weather = self.env.effective();
        let hour = self.env.hour;
        for di in 0..N_SPIRITS {
            let d = &SPIRITS[di];
            if !d.active(hour, weather) || (d.needs_puzzle && !self.puzzle_solved) {
                continue;
            }
            let alive = self.spirits.iter().filter(|s| s.def == di && s.state != SpState::Leaving).count();
            if alive >= d.max_alive || !self.rng.chance(d.spawn_chance) {
                continue;
            }
            let pos = if d.needs_puzzle {
                let g = self.world.glade;
                g + Vec3::new(self.rng.range(-2.0, 2.0), 0.0, self.rng.range(-2.0, 2.0))
            } else {
                self.random_spawn_point()
            };
            self.spawn_spirit(di, pos);
        }
    }

    // ---------------------------------------------------------------- spirit AI

    fn find_lure(&self, s: &Spirit, d: &SpiritDef, lure_r: f32, player_dist: f32, still: f32) -> Option<u32> {
        // Shy spirits only come out for a treat when you keep your distance and stand still.
        if d.behavior == Behavior::Shy && (player_dist < 4.5 || still < 0.4) {
            return None;
        }
        let mut best: Option<(f32, u32)> = None;
        for t in &self.drops {
            if t.kind != d.favorite {
                continue;
            }
            let dd = t.pos.dist_xz(s.pos);
            if dd < lure_r && best.map_or(true, |(b, _)| dd < b) {
                best = Some((dd, t.id));
            }
        }
        best.map(|(_, id)| id)
    }

    fn wander(&mut self, s: &mut Spirit, d: &SpiritDef, dist: f32, pp: Vec3, dt: f32) {
        s.timer -= dt;
        let to = (s.target - s.pos).with_y(0.0);
        if s.timer <= 0.0 || to.length() < 0.5 {
            let ang = self.rng.range(0.0, TAU);
            let r = self.rng.range(2.0, 7.0);
            let mut t = s.pos + Vec3::new(ang.cos() * r, 0.0, ang.sin() * r);
            if d.behavior == Behavior::Curious && dist < 10.0 && dist > 3.5 {
                // Curious spirits come to take a look at you.
                t = pp + Vec3::new(self.rng.range(-3.0, 3.0), 0.0, self.rng.range(-3.0, 3.0));
            }
            let lim = self.world.radius - 6.0;
            let len = (t.x * t.x + t.z * t.z).sqrt();
            if len > lim {
                t = t * (lim / len);
            }
            s.target = t;
            s.timer = self.rng.range(2.5, 5.5);
        }
        let speed = match d.behavior {
            Behavior::Curious => 1.1,
            Behavior::Playful => 1.8,
            Behavior::Shy => 0.8,
        };
        let dir = (s.target - s.pos).with_y(0.0).normalized();
        let perp = Vec3::new(-dir.z, 0.0, dir.x);
        let wiggle = if d.behavior == Behavior::Playful { (s.age * 3.0).sin() * 1.1 } else { 0.0 };
        let mut v = dir * speed + perp * wiggle;
        if d.behavior == Behavior::Curious && dist < 2.5 {
            v += (s.pos - pp).with_y(0.0).normalized() * 1.5;
        }
        s.pos += v * dt;
        if v.length() > 0.05 {
            s.yaw = v.x.atan2(v.z);
        }
    }

    fn update_spirits(&mut self, dt: f32) {
        let pp = self.player.pos;
        let moving = self.player.moving;
        let still = self.player.still_time;
        let weather = self.env.effective();
        let hour = self.env.hour;
        let lure_r = self.lure_radius();

        let mut i = 0;
        while i < self.spirits.len() {
            let mut s = self.spirits[i].clone();
            let d = &SPIRITS[s.def];
            s.age += dt;
            s.phase += dt * 2.4;
            let dist = s.pos.dist_xz(pp);

            if s.state == SpState::Leaving {
                s.fade -= dt * 0.5;
            } else {
                s.fade = (s.fade + dt * 0.7).min(1.0);
            }
            if s.fade <= 0.0 || dist > 75.0 {
                let id = s.id;
                self.follow_order.retain(|&f| f != id);
                self.spirits.remove(i);
                continue;
            }
            // Conditions ended (weather turned, night is over): wild spirits drift away.
            if !d.active(hour, weather) && !matches!(s.state, SpState::Follow | SpState::Leaving) {
                s.state = SpState::Leaving;
            }
            if dist < 9.0 && !self.save.seen[s.def] {
                self.save.seen[s.def] = true;
                self.dirty = true;
                self.toast(format!("You spot something: {}!", d.name));
                self.play(Sfx::Spot);
            }

            let st = s.state;
            match st {
                SpState::Leaving => {}
                SpState::Wander => {
                    if d.behavior == Behavior::Shy && dist < 5.5 && moving {
                        s.state = SpState::Flee(1.6);
                        self.play(Sfx::Flee);
                    } else if let Some(tid) = self.find_lure(&s, d, lure_r, dist, still) {
                        s.state = SpState::Approach(tid);
                    } else {
                        self.wander(&mut s, d, dist, pp, dt);
                    }
                }
                SpState::Approach(tid) => {
                    let tp = self.drops.iter().find(|t| t.id == tid).map(|t| t.pos);
                    match tp {
                        None => s.state = SpState::Wander,
                        Some(tp) => {
                            if d.behavior == Behavior::Shy && dist < 3.5 && moving {
                                s.state = SpState::Flee(1.6);
                        self.play(Sfx::Flee);
                            } else {
                                let to = (tp - s.pos).with_y(0.0);
                                if to.length() < 0.7 {
                                    s.state = SpState::Eat(2.4, tid);
                                    self.play(Sfx::Munch);
                                } else {
                                    let v = to.normalized() * 2.2;
                                    s.pos += v * dt;
                                    s.yaw = v.x.atan2(v.z);
                                }
                            }
                        }
                    }
                }
                SpState::Eat(t, tid) => {
                    let t = t - dt;
                    if d.behavior == Behavior::Shy && dist < 2.5 && moving {
                        s.state = SpState::Flee(1.6);
                        self.play(Sfx::Flee);
                    } else if !self.drops.iter().any(|x| x.id == tid) {
                        s.state = SpState::Wander;
                    } else if t <= 0.0 {
                        self.drops.retain(|x| x.id != tid);
                        s.state = SpState::Follow;
                        self.follow_order.push(s.id);
                        self.play(Sfx::Befriend);
                        self.bursts.push(Burst { pos: s.pos, age: 0.0, color: d.color });
                        self.toast(format!("{} likes you now! Bring them home.", d.name));
                    } else {
                        s.state = SpState::Eat(t, tid);
                    }
                }
                SpState::Follow => {
                    let idx = self.follow_order.iter().position(|&id| id == s.id).unwrap_or(0);
                    let target = self.trail_point((idx + 1) * 5);
                    let old = s.pos;
                    s.pos = s.pos.lerp(target, 1.0 - (-5.0 * dt).exp());
                    // Keep a polite gap so friends never stack on top of you (or each other's slot).
                    let gap = 1.4 + idx as f32 * 0.35;
                    let off = (s.pos - pp).with_y(0.0);
                    if off.length() < gap {
                        let dir = if off.length() > 0.01 { off.normalized() } else { Vec3::new(-(idx as f32 + 1.0).cos(), 0.0, -(idx as f32 + 1.0).sin()) };
                        s.pos = pp.with_y(s.pos.y) + dir * gap;
                    }
                    let mv = (s.pos - old).with_y(0.0);
                    if mv.length() > 0.002 {
                        s.yaw = mv.x.atan2(mv.z);
                    }
                }
                SpState::Flee(t) => {
                    let away = (s.pos - pp).with_y(0.0).normalized();
                    let v = away * 3.6;
                    s.pos += v * dt;
                    s.yaw = v.x.atan2(v.z);
                    let len = (s.pos.x * s.pos.x + s.pos.z * s.pos.z).sqrt();
                    let lim = self.world.radius - 6.0;
                    if len > lim {
                        s.pos.x *= lim / len;
                        s.pos.z *= lim / len;
                    }
                    let t = t - dt;
                    s.state = if t <= 0.0 { SpState::Wander } else { SpState::Flee(t) };
                }
            }

            self.spirits[i] = s;
            i += 1;
        }
    }

    fn update_drops(&mut self, dt: f32) {
        for t in &mut self.drops {
            t.ttl -= dt;
        }
        self.drops.retain(|t| t.ttl > 0.0);
    }

    fn update_sources(&mut self, dt: f32) {
        for s in &mut self.sources {
            if !s.ready {
                s.timer -= dt;
                if s.timer <= 0.0 {
                    s.ready = true;
                }
            }
        }
    }

    // ---------------------------------------------------------------- bringing spirits home

    fn update_deposit(&mut self, dt: f32) {
        self.deposit_cd -= dt;
        if self.deposit_cd > 0.0 || !self.near_home() {
            return;
        }
        let Some(&id) = self.follow_order.first() else { return };
        let Some(idx) = self.spirits.iter().position(|s| s.id == id) else {
            self.follow_order.remove(0);
            return;
        };
        let s = self.spirits.remove(idx);
        self.follow_order.remove(0);
        let d = &SPIRITS[s.def];
        self.save.candles += d.reward;
        self.save.residents[s.def] += 1;
        let first = !self.save.discovered[s.def];
        self.save.discovered[s.def] = true;
        self.save.seen[s.def] = true;
        self.dirty = true;
        self.deposit_cd = 0.8;
        self.bursts.push(Burst { pos: s.pos, age: 0.0, color: d.color });
        self.play(Sfx::Deposit);
        if first {
            self.toast(format!("New friend! {} moved into the cottage garden. +{} candles", d.name, d.reward));
        } else {
            self.toast(format!("{} joined the garden. +{} candles", d.name, d.reward));
        }
    }

    // ---------------------------------------------------------------- render helpers

    /// Up to 8 point lights as 16 vec4 (position+radius, color+intensity).
    pub fn lights(&self) -> [[f32; 4]; 16] {
        let mut out = [[0.0f32; 4]; 16];
        let flick = 1.0 + 0.06 * (self.time * 13.0).sin() * (self.time * 5.3).cos();
        let p = self.player.pos;
        out[0] = [p.x, 1.7, p.z, self.lantern_radius()];
        out[1] = [1.0, 0.74, 0.40, 0.80 * flick];

        let h = self.world.home;
        out[2] = [h.x, 1.6, h.z + 3.0, 8.0];
        out[3] = [1.0, 0.78, 0.50, 0.65 * flick];

        for i in 0..3 {
            if self.lantern_lit[i] {
                let l = self.world.lanterns[i];
                out[4 + i * 2] = [l.x, 2.4, l.z, 8.0];
                out[5 + i * 2] = [0.80, 0.60, 1.0, 0.85];
            }
        }
        // The 3 remaining slots go to the closest spirits, which cast a soft glow in their own color.
        let mut near: Vec<&Spirit> = self.spirits.iter().filter(|s| s.fade > 0.2).collect();
        near.sort_by(|a, b| a.pos.dist_xz(p).partial_cmp(&b.pos.dist_xz(p)).unwrap_or(std::cmp::Ordering::Equal));
        for (k, s) in near.iter().take(3).enumerate() {
            let d = &SPIRITS[s.def];
            out[10 + k * 2] = [s.pos.x, 1.2, s.pos.z, 3.5 + d.size * 3.0];
            out[11 + k * 2] = [d.color[0], d.color[1], d.color[2], 0.9 * s.fade];
        }
        out
    }

    /// One-line context for the HUD: what would [E] do right now?
    pub fn interaction_hint(&self) -> Option<String> {
        let pp = self.player.pos;
        for i in 0..3 {
            if !self.lantern_lit[i] && pp.dist_xz(self.world.lanterns[i]) < 2.8 {
                return Some("<kbd>E</kbd> Light the lantern".to_string());
            }
        }
        for s in &self.sources {
            if s.ready && s.pos.dist_xz(pp) < 2.6 {
                return Some(format!("<kbd>E</kbd> Gather {}", TREATS[s.treat].name));
            }
        }
        None
    }

    /// The closest wild spirit within earshot, for the "who's that?" line.
    pub fn nearby_spirit(&self) -> Option<&Spirit> {
        let pp = self.player.pos;
        self.spirits
            .iter()
            .filter(|s| !matches!(s.state, SpState::Follow | SpState::Leaving) && s.pos.dist_xz(pp) < 7.0)
            .min_by(|a, b| a.pos.dist_xz(pp).partial_cmp(&b.pos.dist_xz(pp)).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(g: &mut Game, seconds: f32, ctl: &Controls) {
        for _ in 0..(seconds / STEP) as usize {
            g.update(STEP, ctl);
        }
    }

    fn still() -> Controls {
        Controls::default()
    }

    #[test]
    fn player_walks_collides_and_stays_in_bounds() {
        let mut g = Game::new(Save::default(), 1);
        let start = g.player.pos;
        run(&mut g, 1.0, &Controls { x: 1.0, z: 0.0, run: false });
        assert!(g.player.pos.x > start.x + 2.0);
        run(&mut g, 60.0, &Controls { x: 1.0, z: 0.3, run: true });
        let r = (g.player.pos.x.powi(2) + g.player.pos.z.powi(2)).sqrt();
        assert!(r <= g.world.radius - 3.0 + 0.01, "left the map: {r}");
        for c in &g.world.colliders {
            let d = ((g.player.pos.x - c.x).powi(2) + (g.player.pos.z - c.z).powi(2)).sqrt();
            assert!(d >= c.r + 0.35 - 0.05, "inside a collider");
        }
    }

    #[test]
    fn weather_and_time_decide_who_spawns() {
        // Rain at midnight: Mizzle yes, Pip/Hush no.
        let mut g = Game::new(Save::default(), 5);
        g.env.hour = 0.0;
        g.env.set_weather(Weather::Rain);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "mizzle"));
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].id == "mizzle"));

        // Fog: Hush.
        let mut g = Game::new(Save::default(), 6);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Fog);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "hush"));

        // Luna never appears without the puzzle, even on a perfect clear night.
        let mut g = Game::new(Save::default(), 7);
        g.env.hour = 23.0;
        run(&mut g, 30.0, &still());
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].id != "luna"));
    }

    #[test]
    fn lantern_puzzle_summons_luna_and_dawn_resets_it() {
        let mut g = Game::new(Save::default(), 8);
        g.env.hour = 22.0;
        for i in 0..3 {
            g.player.pos = g.world.lanterns[i] + Vec3::new(0.0, 0.0, 1.5);
            g.act(Action::Interact);
        }
        assert!(g.puzzle_solved);
        run(&mut g, 30.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "luna"));
        g.env.hour = 5.99;
        run(&mut g, 5.0, &still());
        assert!(!g.puzzle_solved && g.lantern_lit == [false; 3]);
    }

    #[test]
    fn full_loop_lure_befriend_bring_home_upgrade() {
        let mut g = Game::new(Save::default(), 9);
        g.env.hour = 19.0;
        g.cam = g.player.pos;
        // Pip (index 0) loves Candy Corn (treat 0).
        let sp_pos = g.player.pos + Vec3::new(5.0, 0.0, 6.0);
        let id = g.spawn_spirit(0, sp_pos);
        g.selected = 0;
        let before = g.save.treats[0];
        g.act(Action::DropTreat);
        assert_eq!(g.save.treats[0], before - 1);
        assert_eq!(g.drops.len(), 1);

        run(&mut g, 12.0, &still());
        let s = g.spirits.iter().find(|s| s.id == id).expect("spirit still around");
        assert_eq!(s.state, SpState::Follow, "Pip should have befriended us: {:?}", s.state);
        assert!(g.drops.is_empty(), "treat was eaten");

        // Walk home; the follower is deposited automatically.
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 5.0);
        run(&mut g, 3.0, &still());
        assert!(g.spirits.iter().all(|s| s.id != id));
        assert_eq!(g.save.candles, SPIRITS[0].reward);
        assert_eq!(g.save.residents[0], 1);
        assert!(g.save.discovered[0] && g.dirty);

        // Not enough candles for the 20-candle lantern yet.
        g.act(Action::Buy(0));
        assert_eq!(g.save.upgrades[0], 0);
        // Boots cost 15: still short, then enough after another friend.
        g.save.candles = 20;
        g.act(Action::Buy(1));
        assert_eq!(g.save.upgrades[1], 1);
        assert_eq!(g.save.candles, 5);
    }

    #[test]
    fn wrong_treat_does_not_lure() {
        let mut g = Game::new(Save::default(), 10);
        g.env.hour = 19.0;
        let id = g.spawn_spirit(0, g.player.pos + Vec3::new(4.0, 0.0, 4.0));
        g.selected = 2; // Moon Milk: Pip does not care
        g.act(Action::DropTreat);
        run(&mut g, 8.0, &still());
        let s = g.spirits.iter().find(|s| s.id == id).unwrap();
        assert!(!matches!(s.state, SpState::Follow | SpState::Eat(..)));
    }

    #[test]
    fn shy_spirit_needs_you_to_stand_still_at_a_distance() {
        let mut g = Game::new(Save::default(), 11);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Fog);
        run(&mut g, 12.0, &still()); // let the fog roll in: spirits feel it halfway through the 20s crossfade
        g.spirits.clear();
        let id = g.spawn_spirit(2, g.player.pos + Vec3::new(8.0, 0.0, 0.0)); // Hush
        g.selected = 1; // Pumpkin Cookie
        g.act(Action::DropTreat);
        // Running straight at it spooks it.
        for _ in 0..(3.0 / STEP) as usize {
            let Some(s) = g.spirits.iter().find(|s| s.id == id) else { break };
            let to = (s.pos - g.player.pos).normalized();
            g.update(STEP, &Controls { x: to.x, z: to.z, run: true });
        }
        let s = g.spirits.iter().find(|s| s.id == id).expect("Hush should still be around");
        assert_ne!(s.state, SpState::Follow, "must not be befriended while we charge at it");
        assert!(matches!(s.state, SpState::Flee(_) | SpState::Wander), "{:?}", s.state);

        // Now stand still at a polite distance with a cookie set out: it comes and becomes a friend.
        g.player.vel = Vec3::ZERO;
        g.player.facing = -std::f32::consts::FRAC_PI_2; // facing -x
        let spot = g.player.pos + Vec3::new(-6.5, 0.0, 0.0);
        {
            let sp = g.spirits.iter_mut().find(|s| s.id == id).unwrap();
            sp.pos = spot;
            sp.target = spot;
            sp.state = SpState::Wander;
            sp.timer = 30.0; // keep it from wandering off on its own
        }
        g.drops.clear();
        g.act(Action::DropTreat);
        run(&mut g, 14.0, &still());
        let s = g.spirits.iter().find(|s| s.id == id).expect("Hush still around");
        assert_eq!(s.state, SpState::Follow, "{:?}", s.state);
    }

    #[test]
    fn gathering_respects_pouch_capacity_and_regrows() {
        let mut g = Game::new(Save::default(), 12);
        let src = (g.sources[0].pos, g.sources[0].treat);
        g.player.pos = src.0 + Vec3::new(0.0, 0.0, 1.0);
        g.save.treats[src.1] = g.treat_cap();
        g.act(Action::Interact);
        assert!(g.sources[0].ready, "full pouch must not consume the plant");
        g.save.treats[src.1] = 0;
        g.act(Action::Interact);
        assert!(!g.sources[0].ready && g.save.treats[src.1] >= 1);
        run(&mut g, 60.0, &still());
        assert!(g.sources[0].ready);
    }

    #[test]
    fn conditions_ending_makes_wild_spirits_leave() {
        let mut g = Game::new(Save::default(), 13);
        g.env.hour = 0.0;
        g.env.set_weather(Weather::Rain);
        run(&mut g, 30.0, &still());
        assert!(!g.spirits.is_empty());
        g.env.set_weather(Weather::Clear);
        // 15s fade + spirits fade out over ~2s; keep checking nothing panics.
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].id != "mizzle"
            || s.state == SpState::Leaving
            || g.env.weather == Weather::Rain));
    }
    #[test]
    fn actions_make_the_right_sounds() {
        let mut g = Game::new(Save::default(), 21);
        g.env.hour = 19.0;
        // dropping a treat
        g.selected = 0;
        g.act(Action::DropTreat);
        assert!(g.sfx.contains(&Sfx::Drop));
        // running out of treats complains softly
        g.sfx.clear();
        g.save.treats[1] = 0;
        g.selected = 1;
        g.act(Action::DropTreat);
        assert_eq!(g.sfx, vec![Sfx::Nope]);
        // befriending + depositing
        g.sfx.clear();
        g.drops.clear();
        g.selected = 0;
        let id = g.spawn_spirit(0, g.player.pos + Vec3::new(3.0, 0.0, 3.0));
        g.act(Action::DropTreat);
        g.sfx.clear();
        run(&mut g, 10.0, &still());
        assert!(g.sfx.contains(&Sfx::Munch) && g.sfx.contains(&Sfx::Befriend), "{:?}", g.sfx);
        g.sfx.clear();
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 5.0);
        run(&mut g, 3.0, &still());
        assert!(g.sfx.contains(&Sfx::Deposit));
        assert!(g.spirits.iter().all(|s| s.id != id));
        // footsteps only while moving
        g.sfx.clear();
        run(&mut g, 2.0, &still());
        assert!(!g.sfx.contains(&Sfx::Step));
        g.sfx.clear();
        run(&mut g, 2.0, &Controls { x: 1.0, z: 0.0, run: false });
        let steps = g.sfx.iter().filter(|s| **s == Sfx::Step).count();
        assert!((3..=8).contains(&steps), "steps in 2s of walking: {steps}");
    }

    #[test]
    fn weather_is_announced_ahead_and_when_it_arrives() {
        let mut g = Game::new(Save::default(), 22);
        // Fast-forward to just before the first forecast (first change is Fog at ~150s).
        run(&mut g, 118.0, &still());
        assert!(g.toasts.iter().all(|t| !t.text.contains("on its way")));
        run(&mut g, 4.0, &still());
        assert!(g.toasts.iter().any(|t| t.text.contains("fog is on its way")), "{:?}", g.toasts.iter().map(|t| &t.text).collect::<Vec<_>>());
        assert_eq!(g.env.forecast(), Some(Weather::Fog));
        run(&mut g, 32.0, &still());
        assert!(g.sfx.contains(&Sfx::Weather(Weather::Fog)) || g.toasts.iter().any(|t| t.text.contains("Fog creeps")));
        assert_eq!(g.env.weather, Weather::Fog);
        assert!(g.env.forecast().is_none());
    }

}
