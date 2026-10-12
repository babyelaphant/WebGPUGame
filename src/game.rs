//! All gameplay rules. No rendering, no browser: pure state + `update(dt)`, so it's testable.
//!
//! The loop:  explore -> spot spirits -> lure with treats / solve the area's puzzle
//!            -> they follow you -> bring them home -> candles -> upgrades -> new areas.
//!
//! Three areas: the Hollow Glade (lantern puzzle, cottage), Graveyard Hollow (crypt rune melody)
//! and Willow Pond (lily lamps). The glade has two gates, opened by the Gate Key and Frog Charm
//! upgrades; the other two areas lead back home.

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
const GATE_RANGE: f32 = 3.2;
const TRAVEL_SECONDS: f32 = 1.2;
/// Seconds each rune glows while the crypt plays its melody (one step = RUNE_STEP).
const RUNE_STEP: f32 = 0.9;
pub const MELODY_LEN: usize = 4;
pub const N_LAMPS: usize = 4;

/// Which upgrade opens the gate that leads to `to` (None = always open).
pub fn gate_key(to: Area) -> Option<usize> {
    match to {
        Area::Glade => None,
        Area::Hollow => Some(GATE_KEY),
        Area::Pond => Some(POND_KEY),
    }
}

/// Lily lamps are "Lights Out" on a row: pressing one flips it and its neighbours.
fn press_lamps(lit: &mut [bool; N_LAMPS], i: usize) {
    for j in i.saturating_sub(1)..=(i + 1).min(N_LAMPS - 1) {
        lit[j] = !lit[j];
    }
}

/// A fresh puzzle that is always solvable: start from "all lit" and undo a few random presses.
fn scramble_lamps(rng: &mut Rng) -> [bool; N_LAMPS] {
    loop {
        let mut lit = [true; N_LAMPS];
        for _ in 0..(2 + rng.below(3)) {
            press_lamps(&mut lit, rng.below(N_LAMPS));
        }
        if lit.iter().any(|l| !*l) {
            return lit;
        }
    }
}
/// Rune stone gem colors (also used as light colors).
pub const RUNE_COLORS: [[f32; 3]; 4] = [[1.0, 0.38, 0.38], [0.45, 0.95, 0.55], [0.45, 0.65, 1.0], [1.0, 0.85, 0.40]];

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

/// The crypt's Simon-style melody puzzle: listen at the door, then touch the stones in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuneState {
    /// Haven't listened yet (or got it wrong).
    Idle,
    /// The crypt is playing `seq[step]`. `t` counts up (negative = a short pause first).
    Playing { step: usize, t: f32, fired: bool },
    /// Waiting for the player's `n`th touch.
    Input(usize),
    Solved,
}

pub struct Runes {
    pub seq: [u8; MELODY_LEN],
    pub state: RuneState,
    /// Seconds of glow left per stone.
    pub flash: [f32; 4],
}

fn roll_melody(rng: &mut Rng) -> [u8; MELODY_LEN] {
    let mut seq = [0u8; MELODY_LEN];
    let mut prev = 9u8;
    for slot in seq.iter_mut() {
        loop {
            let n = rng.below(4) as u8;
            if n != prev {
                *slot = n;
                prev = n;
                break;
            }
        }
    }
    seq
}

/// A walk through a gate: fade out, swap areas at the midpoint, fade back in.
pub struct Travel {
    pub to: Area,
    pub t: f32,
    swapped: bool,
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
    pub area: Area,
    pub lantern_lit: [bool; 3],
    /// Per area: has its puzzle been solved today? (Resets at dawn.)
    pub puzzle_solved: [bool; N_AREAS],
    pub runes: Runes,
    /// Willow Pond: which lily lamps are glowing (all four lit = solved).
    pub lamps_lit: [bool; N_LAMPS],
    pub travel: Option<Travel>,
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
        let world = World::generate_area(Area::Glade);
        let mut rng = Rng::new(seed);
        let melody = roll_melody(&mut rng);
        let lamps = scramble_lamps(&mut rng);
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
            area: Area::Glade,
            lantern_lit: [false; 3],
            puzzle_solved: [false; N_AREAS],
            runes: Runes { seq: melody, state: RuneState::Idle, flash: [0.0; 4] },
            lamps_lit: lamps,
            travel: None,
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
            rng,
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
        self.world.has_home && self.player.pos.dist_xz(self.world.home) < HOME_ZONE
    }
    /// Is the gate that leads to `to` open? (Needs its key upgrade, if it has one.)
    pub fn gate_unlocked(&self, to: Area) -> bool {
        gate_key(to).map_or(true, |k| self.save.upgrades[k] > 0)
    }
    /// The crypt door interaction point (Hollow).
    pub fn crypt_door(&self) -> Vec3 {
        self.world.crypt + Vec3::new(0.0, 0.0, 4.4)
    }
    /// 0 = clear, 1 = black: the screen fade while walking through a gate.
    pub fn fade(&self) -> f32 {
        match &self.travel {
            None => 0.0,
            Some(t) => {
                let x = (t.t / TRAVEL_SECONDS).clamp(0.0, 1.0);
                if x < 0.4 {
                    x / 0.4
                } else if x < 0.6 {
                    1.0
                } else {
                    (1.0 - x) / 0.4
                }
            }
        }
    }
    /// Lighting for the current area: the Hollow is colder, darker and always a little misty.
    pub fn lighting(&self) -> Lighting {
        let mut l = self.env.lighting();
        if self.area == Area::Hollow {
            l.sun_i *= 0.9;
            l.ambient = [l.ambient[0] * 0.95, l.ambient[1] * 0.93, l.ambient[2] * 1.0];
            l.fog_col = lerp3(l.fog_col, [0.20, 0.17, 0.32], 0.35);
            l.fog_density += 0.006;
            l.fog_amt = l.fog_amt.max(0.22);
        } else if self.area == Area::Pond {
            // Damp, cool and a little hazy over the water.
            l.fog_col = lerp3(l.fog_col, [0.30, 0.50, 0.52], 0.22);
            l.fog_density += 0.003;
            l.fog_amt = l.fog_amt.max(0.08);
        }
        l
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
        if self.travel.is_some() {
            return;
        }
        let pp = self.player.pos;

        // 1a. Hollow: the crypt's rune stones and door.
        if self.area == Area::Hollow {
            for i in 0..4 {
                if pp.dist_xz(self.world.runes[i]) < 2.6 {
                    self.touch_rune(i);
                    return;
                }
            }
            if pp.dist_xz(self.crypt_door()) < 2.8 {
                self.listen_at_crypt();
                return;
            }
        }

        // 1a'. Pond: the lily lamps.
        if self.area == Area::Pond {
            for i in 0..N_LAMPS {
                if pp.dist_xz(self.world.lamps[i]) < 2.6 {
                    self.press_lamp(i);
                    return;
                }
            }
        }

        // 1b. A gate to another area.
        if let Some(gate) = self.world.gate_near(pp, GATE_RANGE) {
            let to = gate.to;
            if self.gate_unlocked(to) {
                self.travel = Some(Travel { to, t: 0.0, swapped: false });
                self.play(Sfx::Gate);
            } else {
                let key = gate_key(to).map(|k| UPGRADES[k].name).unwrap_or("key");
                self.play(Sfx::Nope);
                self.toast(format!("The gate is sealed by a ghostly lock. Maybe the cottage workshop can craft a {key}."));
            }
            return;
        }

        // 1c. Light a lantern in the glade (the puzzle).
        for i in 0..3 {
            if self.area == Area::Glade && !self.lantern_lit[i] && pp.dist_xz(self.world.lanterns[i]) < 2.8 {
                self.lantern_lit[i] = true;
                let lit = self.lantern_lit.iter().filter(|x| **x).count();
                if lit == 3 {
                    self.puzzle_solved[0] = true;
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

    fn press_lamp(&mut self, i: usize) {
        if self.puzzle_solved[Area::Pond.idx()] {
            self.play(Sfx::Ui);
            self.toast("The lily lamps glow softly. The pond is awake.");
            return;
        }
        press_lamps(&mut self.lamps_lit, i);
        let lit = self.lamps_lit.iter().filter(|l| **l).count();
        if lit == N_LAMPS {
            self.puzzle_solved[Area::Pond.idx()] = true;
            self.play(Sfx::Puzzle);
            self.toast("Every lily lamp glows. The pond shimmers... someone stirs below the water.");
        } else {
            self.play(Sfx::Lantern);
            self.toast(format!("The lamp flickers, and so do its neighbours ({lit}/{N_LAMPS} lit)."));
        }
    }

    fn listen_at_crypt(&mut self) {
        match self.runes.state {
            RuneState::Solved => {
                self.play(Sfx::Ui);
                self.toast("The crypt door stands open. Something old stirs inside...");
            }
            RuneState::Playing { .. } => {}
            _ => {
                self.runes.state = RuneState::Playing { step: 0, t: -0.5, fired: false };
                self.toast("The crypt hums a slow tune. Watch the stones...");
            }
        }
    }

    fn touch_rune(&mut self, i: usize) {
        self.runes.flash[i] = 0.7;
        match self.runes.state {
            RuneState::Playing { .. } => {}
            RuneState::Solved => self.play(Sfx::Rune(i as u8)),
            RuneState::Idle => {
                self.play(Sfx::Rune(i as u8));
                self.toast("The stone rings softly. Listen at the crypt door first.");
            }
            RuneState::Input(k) => {
                self.play(Sfx::Rune(i as u8));
                if self.runes.seq[k] as usize == i {
                    if k + 1 == MELODY_LEN {
                        self.runes.state = RuneState::Solved;
                        self.puzzle_solved[1] = true;
                        self.play(Sfx::Puzzle);
                        self.toast("The melody is complete. The crypt door creaks open...");
                    } else {
                        self.runes.state = RuneState::Input(k + 1);
                    }
                } else {
                    self.runes.state = RuneState::Idle;
                    self.play(Sfx::Nope);
                    self.toast("Wrong stone. The crypt falls silent. Listen again.");
                }
            }
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
        let walking = Controls::default();
        let ctl = if self.travel.is_some() { &walking } else { ctl };

        if self.env.update(dt, &mut self.rng) {
            // Dawn: the lanterns burn out, the crypt picks a new tune, the lily lamps get scrambled.
            self.lantern_lit = [false; 3];
            self.puzzle_solved = [false; N_AREAS];
            self.runes.seq = roll_melody(&mut self.rng);
            self.runes.state = RuneState::Idle;
            self.lamps_lit = scramble_lamps(&mut self.rng);
        }

        self.update_travel(dt);
        self.update_runes(dt);

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

    fn update_travel(&mut self, dt: f32) {
        let Some(tr) = &mut self.travel else { return };
        tr.t += dt;
        let (to, midpoint, done) = (tr.to, tr.t >= TRAVEL_SECONDS * 0.5 && !tr.swapped, tr.t >= TRAVEL_SECONDS);
        if midpoint {
            tr.swapped = true;
        }
        if done {
            self.travel = None;
        }
        if midpoint {
            self.enter_area(to);
        }
    }

    /// Swaps the world. Friends who follow you come along; wild spirits and dropped treats stay behind.
    pub fn enter_area(&mut self, to: Area) {
        let from = self.area;
        self.area = to;
        self.world = World::generate_area(to);
        self.sources = self
            .world
            .sources
            .iter()
            .map(|s| SourceState { pos: s.pos, treat: s.treat, ready: true, timer: 0.0 })
            .collect();
        let arrival = self.world.arrival_from(from);
        let keep = self.follow_order.clone();
        self.spirits.retain(|s| keep.contains(&s.id));
        for (i, s) in self.spirits.iter_mut().enumerate() {
            let a = i as f32 * 1.3;
            s.pos = arrival + Vec3::new(a.cos() * 1.6, 0.0, a.sin() * 1.6);
            s.state = SpState::Follow;
        }
        self.drops.clear();
        self.bursts.clear();
        self.trail.clear();
        self.player.pos = arrival;
        self.player.vel = Vec3::ZERO;
        // Face away from the gate you just came out of.
        let gate_z = self.world.gate_to(from).pos.z;
        self.player.facing = if arrival.z < gate_z { std::f32::consts::PI } else { 0.0 };
        self.cam = arrival;
        self.spawn_timer = 3.0;
        self.toast(format!("{}", to.name()));
        self.dirty = true;
    }

    fn update_runes(&mut self, dt: f32) {
        for f in &mut self.runes.flash {
            *f = (*f - dt).max(0.0);
        }
        if let RuneState::Playing { step, t, fired } = self.runes.state {
            let t = t + dt;
            let mut fired = fired;
            if !fired && t >= 0.0 {
                fired = true;
                let n = self.runes.seq[step];
                self.runes.flash[n as usize] = 0.75;
                self.play(Sfx::Rune(n));
            }
            self.runes.state = if t >= RUNE_STEP {
                if step + 1 >= MELODY_LEN {
                    self.toast("Your turn: touch the stones in the same order.");
                    RuneState::Input(0)
                } else {
                    RuneState::Playing { step: step + 1, t: 0.0, fired: false }
                }
            } else {
                RuneState::Playing { step, t, fired }
            };
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
        self.world.keep_out_of_water(&mut p.pos);

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
            if d.area != self.area.idx() || !d.active(hour, weather) || (d.needs_puzzle && !self.puzzle_solved[d.area]) {
                continue;
            }
            // One-of-a-kind spirits never come back once they live at the cottage.
            if d.unique && self.save.residents[di] > 0 {
                continue;
            }
            let alive = self.spirits.iter().filter(|s| s.def == di && s.state != SpState::Leaving).count();
            if alive >= d.max_alive || !self.rng.chance(d.spawn_chance) {
                continue;
            }
            let pos = if d.needs_puzzle {
                let g = self.world.puzzle_spot;
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

        if self.world.has_home {
            let h = self.world.home;
            out[2] = [h.x, 1.6, h.z + 3.0, 8.0];
            out[3] = [1.0, 0.78, 0.50, 0.65 * flick];
        } else {
            // The way back: a cold lamp at the gate home.
            let g = self.world.gates[0].pos;
            out[2] = [g.x, 2.0, g.z - 1.0, 9.0];
            out[3] = [0.60, 0.80, 1.0, 0.70 * flick];
        }

        if self.area == Area::Glade {
            for i in 0..3 {
                if self.lantern_lit[i] {
                    let l = self.world.lanterns[i];
                    out[4 + i * 2] = [l.x, 2.4, l.z, 8.0];
                    out[5 + i * 2] = [0.80, 0.60, 1.0, 0.85];
                }
            }
        } else if self.area == Area::Pond {
            // Up to three of the lit lily lamps.
            let mut k = 0;
            for i in 0..N_LAMPS {
                if self.lamps_lit[i] && k < 3 {
                    let l = self.world.lamps[i];
                    out[4 + k * 2] = [l.x, 1.8, l.z, 7.5];
                    out[5 + k * 2] = [1.0, 0.72, 0.88, 0.85];
                    k += 1;
                }
            }
        } else {
            // Up to three: the open crypt door, then whichever rune stones are glowing.
            let mut extra: Vec<([f32; 4], [f32; 4])> = Vec::new();
            if self.runes.state == RuneState::Solved {
                let c = self.world.crypt;
                extra.push(([c.x, 1.6, c.z + 3.6, 9.0], [0.75, 1.0, 0.85, 0.75]));
            }
            for i in 0..4 {
                let f = self.runes.flash[i];
                if f > 0.0 {
                    let q = self.world.runes[i];
                    let c = RUNE_COLORS[i];
                    extra.push(([q.x, 1.8, q.z, 8.0], [c[0], c[1], c[2], 1.1 * (f / 0.3).min(1.0)]));
                }
            }
            for (k, (a, b)) in extra.into_iter().take(3).enumerate() {
                out[4 + k * 2] = a;
                out[5 + k * 2] = b;
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
        if self.travel.is_some() {
            return None;
        }
        if self.area == Area::Hollow {
            if (0..4).any(|i| pp.dist_xz(self.world.runes[i]) < 2.6) {
                return Some(match self.runes.state {
                    RuneState::Input(k) => format!("<kbd>E</kbd> Touch the stone <small>(note {} of {})</small>", k + 1, MELODY_LEN),
                    RuneState::Solved => "<kbd>E</kbd> Ring the stone".to_string(),
                    RuneState::Playing { .. } => "<small>Listen...</small>".to_string(),
                    RuneState::Idle => "<kbd>E</kbd> Touch the stone <small>(listen at the crypt door first)</small>".to_string(),
                });
            }
            if pp.dist_xz(self.crypt_door()) < 2.8 {
                return Some(match self.runes.state {
                    RuneState::Solved => "<kbd>E</kbd> The crypt door stands open".to_string(),
                    RuneState::Playing { .. } => "<small>Watch the stones...</small>".to_string(),
                    RuneState::Input(_) => "<kbd>E</kbd> Hear the melody again".to_string(),
                    RuneState::Idle => "<kbd>E</kbd> Listen to the crypt".to_string(),
                });
            }
        }
        if self.area == Area::Pond {
            if let Some(i) = (0..N_LAMPS).find(|i| pp.dist_xz(self.world.lamps[*i]) < 2.6) {
                let lit = self.lamps_lit.iter().filter(|l| **l).count();
                return Some(if self.puzzle_solved[Area::Pond.idx()] {
                    "<kbd>E</kbd> Admire the lily lamp".to_string()
                } else {
                    format!("<kbd>E</kbd> Flip lamp {} and its neighbours <small>({lit}/{N_LAMPS} lit)</small>", i + 1)
                });
            }
        }
        if let Some(gate) = self.world.gate_near(pp, GATE_RANGE) {
            return Some(if self.gate_unlocked(gate.to) {
                format!("<kbd>E</kbd> Go through the gate to {}", gate.to.name())
            } else {
                let key = gate_key(gate.to).map(|k| UPGRADES[k].name).unwrap_or("key");
                format!("The gate is sealed. <small>Buy the {key} at the cottage workshop.</small>")
            });
        }
        for i in 0..3 {
            if self.area == Area::Glade && !self.lantern_lit[i] && pp.dist_xz(self.world.lanterns[i]) < 2.8 {
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
        assert!(g.puzzle_solved[0]);
        run(&mut g, 30.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "luna"));
        g.env.hour = 5.99;
        run(&mut g, 5.0, &still());
        assert!(!g.puzzle_solved[0] && g.lantern_lit == [false; 3]);
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

    // ------------------------------------------------------------ area 2: Graveyard Hollow

    /// Walk to the gate, buy the key if asked, and go through. Returns once the fade is over.
    fn go_through_gate(g: &mut Game, to: Area) {
        let gate = g.world.gate_to(to).pos;
        // Stand on the walkable side: north of gates on the south edge, south of the northern one.
        g.player.pos = gate + Vec3::new(0.0, 0.0, if gate.z > 30.0 { -2.0 } else { 2.0 });
        g.act(Action::Interact);
        run(g, TRAVEL_SECONDS + 0.2, &still());
    }

    fn into_hollow(seed: u64) -> Game {
        let mut g = Game::new(Save::default(), seed);
        g.save.upgrades[GATE_KEY] = 1;
        go_through_gate(&mut g, Area::Hollow);
        assert_eq!(g.area, Area::Hollow);
        g
    }

    fn into_pond(seed: u64) -> Game {
        let mut g = Game::new(Save::default(), seed);
        g.save.upgrades[POND_KEY] = 1;
        go_through_gate(&mut g, Area::Pond);
        assert_eq!(g.area, Area::Pond);
        g
    }

    #[test]
    fn gate_is_sealed_until_you_buy_the_key() {
        let mut g = Game::new(Save::default(), 30);
        g.player.pos = g.world.gate_to(Area::Hollow).pos + Vec3::new(0.0, 0.0, 2.0);
        assert!(g.interaction_hint().unwrap().contains("sealed"));
        assert!(g.interaction_hint().unwrap().contains("Gate Key"));
        g.act(Action::Interact);
        run(&mut g, 2.0, &still());
        assert_eq!(g.area, Area::Glade);
        assert!(g.toasts.iter().any(|t| t.text.contains("sealed")));
        assert!(g.sfx.contains(&Sfx::Nope));

        // Buying the key takes 40 candles at the cottage.
        g.save.candles = 100;
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 4.0);
        g.act(Action::Buy(GATE_KEY));
        assert_eq!(g.save.upgrades[GATE_KEY], 1);
        assert_eq!(g.save.candles, 60);
        g.act(Action::Buy(GATE_KEY));
        assert_eq!(g.save.upgrades[GATE_KEY], 1, "single level");

        go_through_gate(&mut g, Area::Hollow);
        assert_eq!(g.area, Area::Hollow);
        assert_eq!(g.world.area, Area::Hollow);
        assert!(g.travel.is_none() && g.fade() == 0.0);
        let back = g.world.gate_to(Area::Glade);
        assert!(g.player.pos.dist_xz(back.exit) < 0.5, "arrive at the other gate");
        assert!(g.player.pos.dist_xz(back.pos) > GATE_RANGE, "not instantly re-triggering");
        assert!(g.sfx.contains(&Sfx::Gate));
    }

    #[test]
    fn travel_fades_to_black_and_swaps_at_the_midpoint() {
        let mut g = Game::new(Save::default(), 31);
        g.save.upgrades[GATE_KEY] = 1;
        g.player.pos = g.world.gate_to(Area::Hollow).pos + Vec3::new(0.0, 0.0, 2.0);
        g.act(Action::Interact);
        let mut max_fade = 0.0f32;
        let mut swapped_at_black = false;
        for _ in 0..((TRAVEL_SECONDS + 0.1) / STEP) as usize {
            let before = g.area;
            g.update(STEP, &Controls { x: 1.0, z: 0.0, run: true }); // input is ignored mid-travel
            max_fade = max_fade.max(g.fade());
            if before != g.area {
                swapped_at_black = g.fade() > 0.95;
            }
        }
        assert!(max_fade > 0.99 && swapped_at_black, "max fade {max_fade}");
        assert_eq!(g.area, Area::Hollow);
        // The player did not walk during the fade.
        assert!(g.player.pos.dist_xz(g.world.arrival_from(Area::Glade)) < 0.6);
        // And back again.
        go_through_gate(&mut g, Area::Glade);
        assert_eq!(g.area, Area::Glade);
        assert!(g.world.has_home);
    }

    #[test]
    fn friends_follow_through_the_gate_but_wild_spirits_and_treats_stay() {
        let mut g = Game::new(Save::default(), 32);
        g.save.upgrades[GATE_KEY] = 1;
        g.env.hour = 19.0;
        let friend = g.spawn_spirit(0, g.player.pos + Vec3::new(3.0, 0.0, 3.0));
        g.selected = 0;
        g.act(Action::DropTreat);
        run(&mut g, 12.0, &still());
        assert_eq!(g.spirits.iter().find(|s| s.id == friend).unwrap().state, SpState::Follow);
        let wild = g.spawn_spirit(0, g.player.pos + Vec3::new(10.0, 0.0, 0.0));
        g.act(Action::DropTreat);
        go_through_gate(&mut g, Area::Hollow);
        assert_eq!(g.area, Area::Hollow);
        assert!(g.spirits.iter().any(|s| s.id == friend && s.state == SpState::Follow), "friend came along");
        assert!(g.spirits.iter().all(|s| s.id != wild), "wild spirit stayed behind");
        assert!(g.drops.is_empty());
        assert_eq!(g.followers().len(), 1);
        // The friend is near us, not stranded back at the old coordinates.
        assert!(g.followers()[0].pos.dist_xz(g.player.pos) < 3.0);
        // There's no cottage here: nobody gets handed over, and the shop is closed.
        run(&mut g, 3.0, &still());
        assert_eq!(g.followers().len(), 1);
        assert!(!g.near_home());
        // Back at the cottage the friend moves in.
        go_through_gate(&mut g, Area::Glade);
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 5.0);
        run(&mut g, 3.0, &still());
        assert_eq!(g.save.residents[0], 1);
    }

    #[test]
    fn each_area_has_its_own_spirits_and_weather_matters_in_both() {
        // Hollow, rain: Drip yes, Mizzle no.
        let mut g = into_hollow(33);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Rain);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "drip"));
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].area == 1), "only Hollow spirits in the Hollow");

        // Hollow, fog: Gloam.
        let mut g = into_hollow(34);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Fog);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "gloam"));

        // Hollow, clear night: Wick. Clear day: nobody.
        let mut g = into_hollow(35);
        g.env.hour = 22.0;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "wick"));
        let mut g = into_hollow(36);
        g.env.hour = 12.0;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.is_empty(), "{:?}", g.spirits.iter().map(|s| SPIRITS[s.def].id).collect::<Vec<_>>());

        // And the glade never produces Hollow spirits.
        let mut g = Game::new(Save::default(), 37);
        g.env.hour = 22.0;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].area == 0));
    }

    fn stand_at_door(g: &mut Game) {
        g.player.pos = g.crypt_door();
    }
    fn touch(g: &mut Game, rune: usize) {
        g.player.pos = g.world.runes[rune] + Vec3::new(0.0, 0.0, 1.6);
        g.act(Action::Interact);
    }

    #[test]
    fn crypt_melody_listen_repeat_solve() {
        let mut g = into_hollow(40);
        g.env.hour = 23.0;
        let seq = g.runes.seq;
        assert!(seq.windows(2).all(|w| w[0] != w[1]), "no immediate repeats");

        // Touching stones before listening does nothing useful.
        touch(&mut g, seq[0] as usize);
        assert_eq!(g.runes.state, RuneState::Idle);
        assert!(!g.puzzle_solved[1]);

        // Listen: the crypt plays each stone in order.
        g.sfx.clear();
        stand_at_door(&mut g);
        assert!(g.interaction_hint().unwrap().contains("Listen"));
        g.act(Action::Interact);
        let mut heard: Vec<u8> = Vec::new();
        for _ in 0..(8.0 / STEP) as usize {
            g.update(STEP, &still());
            for s in g.sfx.drain(..) {
                if let Sfx::Rune(n) = s {
                    heard.push(n);
                }
            }
            if matches!(g.runes.state, RuneState::Input(_)) {
                break;
            }
        }
        assert_eq!(heard, seq.to_vec(), "melody played in order");
        assert_eq!(g.runes.state, RuneState::Input(0));

        // Repeat it.
        for (k, n) in seq.iter().enumerate() {
            assert_eq!(g.runes.state, RuneState::Input(k));
            touch(&mut g, *n as usize);
        }
        assert_eq!(g.runes.state, RuneState::Solved);
        assert!(g.puzzle_solved[1] && g.sfx.contains(&Sfx::Puzzle));

        // The crypt keeper wakes up at night, and only near the crypt.
        let mut first_seen = None;
        for _ in 0..(40.0 / STEP) as usize {
            g.update(STEP, &still());
            if let Some(m) = g.spirits.iter().find(|s| SPIRITS[s.def].id == "marrow") {
                first_seen = Some(m.pos);
                break;
            }
        }
        let at = first_seen.expect("Old Marrow appears");
        assert!(at.dist_xz(g.world.puzzle_spot) < 4.0, "appears by the crypt, not somewhere random");
    }

    #[test]
    fn wrong_stone_resets_and_marrow_stays_asleep() {
        let mut g = into_hollow(41);
        g.env.hour = 23.0;
        let seq = g.runes.seq;
        stand_at_door(&mut g);
        g.act(Action::Interact);
        run(&mut g, 6.0, &still());
        assert_eq!(g.runes.state, RuneState::Input(0));
        let wrong = (0..4).find(|n| *n as u8 != seq[0]).unwrap();
        g.sfx.clear();
        touch(&mut g, wrong);
        assert_eq!(g.runes.state, RuneState::Idle);
        assert!(g.sfx.contains(&Sfx::Nope));
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].id != "marrow"));

        // Partway right then wrong also resets.
        stand_at_door(&mut g);
        g.act(Action::Interact);
        run(&mut g, 6.0, &still());
        touch(&mut g, seq[0] as usize);
        assert_eq!(g.runes.state, RuneState::Input(1));
        touch(&mut g, (seq[1] as usize + 1) % 4);
        assert_eq!(g.runes.state, RuneState::Idle);
        // Pressing the door while waiting for input replays the melody.
        stand_at_door(&mut g);
        g.act(Action::Interact);
        assert!(matches!(g.runes.state, RuneState::Playing { .. }));
    }

    #[test]
    fn dawn_makes_the_crypt_forget_and_pick_a_new_tune() {
        let mut g = into_hollow(42);
        g.runes.state = RuneState::Solved;
        g.puzzle_solved[1] = true;
        let before = g.runes.seq;
        g.env.hour = 5.99;
        run(&mut g, 5.0, &still());
        assert_eq!(g.runes.state, RuneState::Idle);
        assert!(!g.puzzle_solved[1]);
        // (a new melody is random; it must at least be a valid one)
        assert!(g.runes.seq.iter().all(|n| *n < 4));
        let _ = before;
    }

    #[test]
    fn hollow_lighting_is_moodier_and_lights_include_glowing_runes() {
        let mut g = Game::new(Save::default(), 43);
        g.env.hour = 12.0;
        let glade = g.lighting();
        g.save.upgrades[GATE_KEY] = 1;
        go_through_gate(&mut g, Area::Hollow);
        g.env.hour = 12.0;
        let hollow = g.lighting();
        assert!(hollow.ambient[0] < glade.ambient[0]);
        assert!(hollow.fog_density > glade.fog_density);
        assert!(hollow.fog_amt > 0.2, "always a little mist");
        // A glowing rune shows up as a coloured point light.
        g.runes.flash[2] = 0.7;
        let l = g.lights();
        let q = g.world.runes[2];
        assert!((0..8).any(|i| (l[i * 2][0] - q.x).abs() < 0.01 && (l[i * 2][2] - q.z).abs() < 0.01 && l[i * 2 + 1][3] > 0.5));
    }

    // ------------------------------------------------------------ one-of-a-kind spirits

    fn spirit_idx(id: &str) -> usize {
        SPIRITS.iter().position(|s| s.id == id).unwrap()
    }

    #[test]
    fn old_marrow_never_returns_once_he_lives_at_your_cottage() {
        let marrow = spirit_idx("marrow");
        assert!(SPIRITS[marrow].unique);
        // Not yet befriended: he appears when the crypt is solved at night.
        let mut g = into_hollow(60);
        g.env.hour = 23.0;
        g.runes.state = RuneState::Solved;
        g.puzzle_solved[1] = true;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| s.def == marrow));
        // While one is already around (even following you) there is never a second.
        for s in g.spirits.iter_mut().filter(|s| s.def == marrow) {
            s.state = SpState::Follow;
        }
        g.follow_order = g.spirits.iter().filter(|s| s.def == marrow).map(|s| s.id).collect();
        g.env.hour = 23.0;
        run(&mut g, 40.0, &still());
        assert_eq!(g.spirits.iter().filter(|s| s.def == marrow).count(), 1);

        // He lives at the cottage now: the crypt can be solved night after night, he stays home.
        let mut g = into_hollow(61);
        g.save.residents[marrow] = 1;
        g.env.hour = 23.0;
        g.runes.state = RuneState::Solved;
        g.puzzle_solved[1] = true;
        run(&mut g, 120.0, &still());
        assert!(g.spirits.iter().all(|s| s.def != marrow), "Marrow respawned");
        // Other spirits are not one-of-a-kind and still respawn.
        let wick = spirit_idx("wick");
        assert!(!SPIRITS[wick].unique);
        assert!(g.spirits.iter().any(|s| s.def == wick));
    }

    #[test]
    fn befriending_marrow_keeps_him_home_for_good() {
        let marrow = spirit_idx("marrow");
        let mut g = Game::new(Save::default(), 62);
        // He followed us out of the Hollow; we hand him over at the cottage.
        let id = g.spawn_spirit(marrow, g.world.home + Vec3::new(0.0, 0.0, 9.0));
        g.spirits[0].state = SpState::Follow;
        g.follow_order.push(id);
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 5.0);
        run(&mut g, 3.0, &still());
        assert_eq!(g.save.residents[marrow], 1);
        assert!(g.save.candles >= SPIRITS[marrow].reward);
        // Even if he is "alive" nowhere, back in the Hollow nothing brings him back.
        g.save.upgrades[GATE_KEY] = 1;
        go_through_gate(&mut g, Area::Hollow);
        g.env.hour = 23.0;
        g.runes.state = RuneState::Solved;
        g.puzzle_solved[1] = true;
        run(&mut g, 120.0, &still());
        assert!(g.spirits.iter().all(|s| s.def != marrow));
    }

    // ------------------------------------------------------------ area 3: Willow Pond

    #[test]
    fn frog_charm_opens_the_pond_gate_and_the_gate_key_does_not() {
        let mut g = Game::new(Save::default(), 50);
        g.save.upgrades[GATE_KEY] = 1;
        g.player.pos = g.world.gate_to(Area::Pond).pos + Vec3::new(0.0, 0.0, -2.0);
        let hint = g.interaction_hint().unwrap();
        assert!(hint.contains("sealed") && hint.contains("Frog Charm"), "{hint}");
        g.act(Action::Interact);
        run(&mut g, 2.0, &still());
        assert_eq!(g.area, Area::Glade);
        assert!(g.toasts.iter().any(|t| t.text.contains("Frog Charm")));

        g.save.candles = 100;
        g.player.pos = g.world.home + Vec3::new(0.0, 0.0, 4.0);
        g.act(Action::Buy(POND_KEY));
        assert_eq!(g.save.upgrades[POND_KEY], 1);
        assert_eq!(g.save.candles, 30);

        go_through_gate(&mut g, Area::Pond);
        assert_eq!(g.area, Area::Pond);
        assert!(g.player.pos.dist_xz(g.world.arrival_from(Area::Glade)) < 0.5);
        // The way back leads to the pond gate, not the graveyard gate.
        go_through_gate(&mut g, Area::Glade);
        assert_eq!(g.area, Area::Glade);
        let pond_gate = g.world.gate_to(Area::Pond);
        assert!(g.player.pos.dist_xz(pond_gate.exit) < 0.5, "came out of the pond gate");
        assert!(g.player.pos.dist_xz(g.world.gate_to(Area::Hollow).pos) > 30.0);
    }

    #[test]
    fn you_cannot_walk_into_the_pond() {
        let mut g = into_pond(51);
        let w = &g.world.ponds[0];
        let (cx, cz, rx) = (w.x, w.z, w.rx);
        // Start on the west shore and walk east, straight at the water, for a long while.
        g.player.pos = Vec3::new(cx - rx - 3.0, 0.0, cz + 0.5);
        run(&mut g, 12.0, &Controls { x: 1.0, z: 0.0, run: true });
        let w = &g.world.ponds[0];
        assert!(!w.contains(g.player.pos.x, g.player.pos.z, 0.0), "wading in the pond: {:?}", (g.player.pos.x, g.player.pos.z));
        assert!(g.player.pos.x > cx - rx - 3.0, "made it to the shore");
        // Same from the south.
        g.player.pos = Vec3::new(cx, 0.0, cz + 8.5 + 4.0);
        run(&mut g, 12.0, &Controls { x: 0.0, z: -1.0, run: true });
        assert!(!g.world.ponds[0].contains(g.player.pos.x, g.player.pos.z, 0.0));
        // The glade has no water, so nothing changes there.
        let mut h = Game::new(Save::default(), 52);
        run(&mut h, 1.0, &Controls { x: 1.0, z: 0.0, run: false });
        assert!(h.player.pos.x > 2.0);
    }

    #[test]
    fn pond_spirits_follow_weather_and_daylight() {
        let mut g = into_pond(53);
        g.env.hour = 12.0;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "bobbin"), "sunny day: Bobbin");
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].area == 2));

        let mut g = into_pond(54);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Rain);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "pondle"));
        assert!(g.spirits.iter().all(|s| SPIRITS[s.def].id != "bobbin"));

        let mut g = into_pond(55);
        g.env.hour = 12.0;
        g.env.set_weather(Weather::Fog);
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().any(|s| SPIRITS[s.def].id == "reed"));

        // Clear night: Bobbin is asleep.
        let mut g = into_pond(56);
        g.env.hour = 23.0;
        run(&mut g, 40.0, &still());
        assert!(g.spirits.is_empty(), "{:?}", g.spirits.iter().map(|s| SPIRITS[s.def].id).collect::<Vec<_>>());
    }

    #[test]
    fn lily_lamp_scrambles_are_always_solvable_but_never_solved() {
        let mut rng = Rng::new(99);
        for _ in 0..200 {
            let start = scramble_lamps(&mut rng);
            assert!(start.iter().any(|l| !*l), "scramble must not start solved");
            // some combination of presses lights everything
            let solvable = (0..(1u32 << N_LAMPS)).any(|mask| {
                let mut lit = start;
                for i in 0..N_LAMPS {
                    if mask >> i & 1 == 1 {
                        press_lamps(&mut lit, i);
                    }
                }
                lit.iter().all(|l| *l)
            });
            assert!(solvable, "{start:?}");
        }
        // Pressing flips the lamp and its neighbours only.
        let mut lit = [false; N_LAMPS];
        press_lamps(&mut lit, 0);
        assert_eq!(lit, [true, true, false, false]);
        press_lamps(&mut lit, 2);
        assert_eq!(lit, [true, false, true, true]);
    }

    /// Finds a press sequence that lights every lamp (brute force over the 16 subsets).
    fn lamp_solution(start: [bool; N_LAMPS]) -> Vec<usize> {
        for mask in 0..(1u32 << N_LAMPS) {
            let mut lit = start;
            let mut presses = Vec::new();
            for i in 0..N_LAMPS {
                if mask >> i & 1 == 1 {
                    press_lamps(&mut lit, i);
                    presses.push(i);
                }
            }
            if lit.iter().all(|l| *l) {
                return presses;
            }
        }
        panic!("unsolvable");
    }

    #[test]
    fn lily_lamps_solve_wakes_the_pond_queen_by_day_only_and_dawn_resets() {
        let lotus = spirit_idx("lotus");
        let mut g = into_pond(57);
        g.env.hour = 12.0;
        let before = g.lamps_lit;
        assert!(before.iter().any(|l| !*l));
        // Without the puzzle she never comes.
        run(&mut g, 40.0, &still());
        assert!(g.spirits.iter().all(|s| s.def != lotus));

        // Solve it by actually pressing the lamps in the world.
        g.sfx.clear();
        for i in lamp_solution(g.lamps_lit) {
            g.player.pos = g.world.lamps[i] + Vec3::new(0.0, 0.0, 1.6);
            assert!(g.interaction_hint().unwrap().contains("lamp"), "{:?}", g.interaction_hint());
            g.act(Action::Interact);
        }
        assert!(g.lamps_lit.iter().all(|l| *l));
        assert!(g.puzzle_solved[2] && g.sfx.contains(&Sfx::Puzzle));
        // After solving, lamps can't be un-solved by fiddling.
        g.act(Action::Interact);
        assert!(g.lamps_lit.iter().all(|l| *l));

        // Daytime: she rises from the pond.
        g.env.hour = 12.0;
        let mut seen_at = None;
        for _ in 0..(40.0 / STEP) as usize {
            g.update(STEP, &still());
            if let Some(s) = g.spirits.iter().find(|s| s.def == lotus) {
                seen_at = Some(s.pos);
                break;
            }
        }
        let at = seen_at.expect("Lady Lotus appears");
        assert!(at.dist_xz(g.world.puzzle_spot) < 4.0);

        // Night: she is asleep even though the lamps are lit.
        let mut n = into_pond(58);
        n.env.hour = 23.0;
        n.puzzle_solved[2] = true;
        run(&mut n, 60.0, &still());
        assert!(n.spirits.iter().all(|s| s.def != lotus));

        // Once she lives at the cottage she never returns.
        let mut u = into_pond(59);
        u.save.residents[lotus] = 1;
        u.env.hour = 12.0;
        u.puzzle_solved[2] = true;
        run(&mut u, 120.0, &still());
        assert!(u.spirits.iter().all(|s| s.def != lotus));

        // Dawn: lamps scrambled again, puzzle reset.
        g.env.hour = 5.99;
        run(&mut g, 5.0, &still());
        assert!(!g.puzzle_solved[2]);
        assert!(g.lamps_lit.iter().any(|l| !*l));
    }
}
