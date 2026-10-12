//! Game content tables. To add a spirit, treat or upgrade, edit ONLY this file
//! (and bump the matching `N_*` constant so the compiler checks array sizes).

use crate::env::Weather;

pub const N_TREATS: usize = 3;
pub const N_SPIRITS: usize = 4;
pub const N_UPGRADES: usize = 3;

pub struct TreatDef {
    pub id: &'static str,
    pub name: &'static str,
    pub color: [f32; 3],
}

pub static TREATS: [TreatDef; N_TREATS] = [
    TreatDef { id: "candy_corn", name: "Candy Corn", color: [1.00, 0.78, 0.20] },
    TreatDef { id: "pumpkin_cookie", name: "Pumpkin Cookie", color: [0.90, 0.50, 0.15] },
    TreatDef { id: "moon_milk", name: "Moon Milk", color: [0.75, 0.85, 1.00] },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    /// Drifts toward you to inspect, but keeps a polite distance.
    Curious,
    /// Zig-zags around, fast.
    Playful,
    /// Bolts if you move near it. Only trusts a treat when you stand still, a bit away.
    Shy,
}

pub struct SpiritDef {
    pub id: &'static str,
    pub name: &'static str,
    /// Shown when you're close, before you know what they like.
    pub hint: &'static str,
    pub color: [f32; 3],
    pub size: f32,
    /// Index into TREATS.
    pub favorite: usize,
    pub behavior: Behavior,
    /// Empty = any weather.
    pub weather: &'static [Weather],
    /// (from, to) in hours; wraps past midnight when from > to. (0, 24) = any time.
    pub hours: (f32, f32),
    pub max_alive: usize,
    /// Chance per spawn tick (every ~2s) while conditions hold.
    pub spawn_chance: f32,
    /// Only appears after the glade's lantern puzzle is solved.
    pub needs_puzzle: bool,
    /// Candles earned when you bring them home.
    pub reward: u32,
}

pub static SPIRITS: [SpiritDef; N_SPIRITS] = [
    SpiritDef {
        id: "pip",
        name: "Pumpkin Pip",
        hint: "Sniffs hungrily at anything sweet and crunchy.",
        color: [1.00, 0.62, 0.25],
        size: 0.55,
        favorite: 0,
        behavior: Behavior::Curious,
        weather: &[Weather::Clear],
        hours: (17.0, 23.0),
        max_alive: 2,
        spawn_chance: 0.45,
        needs_puzzle: false,
        reward: 10,
    },
    SpiritDef {
        id: "mizzle",
        name: "Mizzle",
        hint: "Splashes in puddles. Smells like cold, creamy moonlight.",
        color: [0.50, 0.80, 1.00],
        size: 0.5,
        favorite: 2,
        behavior: Behavior::Playful,
        weather: &[Weather::Rain],
        hours: (0.0, 24.0),
        max_alive: 3,
        spawn_chance: 0.50,
        needs_puzzle: false,
        reward: 15,
    },
    SpiritDef {
        id: "hush",
        name: "Hush",
        hint: "A skittish fog moth. Drawn to warm, spiced baking.",
        color: [0.80, 0.70, 1.00],
        size: 0.6,
        favorite: 1,
        behavior: Behavior::Shy,
        weather: &[Weather::Fog],
        hours: (0.0, 24.0),
        max_alive: 2,
        spawn_chance: 0.40,
        needs_puzzle: false,
        reward: 20,
    },
    SpiritDef {
        id: "luna",
        name: "Luna, Lantern Queen",
        hint: "Glows with old moonlight. Hungers for something silvery.",
        color: [1.00, 0.95, 0.70],
        size: 0.85,
        favorite: 2,
        behavior: Behavior::Curious,
        weather: &[Weather::Clear],
        hours: (21.0, 4.0),
        max_alive: 1,
        spawn_chance: 0.50,
        needs_puzzle: true,
        reward: 60,
    },
];

pub struct UpgradeDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    /// Cost of each level in candles; length = max level.
    pub costs: &'static [u32],
}

pub static UPGRADES: [UpgradeDef; N_UPGRADES] = [
    UpgradeDef {
        id: "lantern",
        name: "Brighter Lantern",
        desc: "Bigger light, and treats lure spirits from farther away",
        costs: &[20, 45, 90],
    },
    UpgradeDef {
        id: "boots",
        name: "Hex Boots",
        desc: "+15% walking speed per level",
        costs: &[15, 40, 80],
    },
    UpgradeDef {
        id: "pouch",
        name: "Treat Pouch",
        desc: "+3 carrying space for each treat type",
        costs: &[15, 35, 70],
    },
];

pub fn hour_in(h: f32, (from, to): (f32, f32)) -> bool {
    if from <= to {
        h >= from && h < to
    } else {
        h >= from || h < to
    }
}

impl SpiritDef {
    pub fn active(&self, hour: f32, weather: Weather) -> bool {
        (self.weather.is_empty() || self.weather.contains(&weather)) && hour_in(hour, self.hours)
    }

    pub fn conditions_text(&self) -> String {
        let w = if self.weather.is_empty() {
            "Any weather".to_string()
        } else {
            self.weather.iter().map(|w| w.name()).collect::<Vec<_>>().join(" / ")
        };
        let t = if self.hours == (0.0, 24.0) {
            "any time".to_string()
        } else {
            format!("{}–{}", fmt_hour(self.hours.0), fmt_hour(self.hours.1))
        };
        format!("{w}, {t}")
    }
}

pub fn fmt_hour(h: f32) -> String {
    let h = h.rem_euclid(24.0);
    let hh = h.floor() as u32;
    let mm = ((h - h.floor()) * 60.0) as u32;
    format!("{hh:02}:{mm:02}")
}

/// Everything that persists between sessions.
#[derive(Clone, Debug, PartialEq)]
pub struct Save {
    pub candles: u32,
    pub upgrades: [u8; N_UPGRADES],
    pub treats: [u32; N_TREATS],
    /// How many of each spirit live at your cottage.
    pub residents: [u32; N_SPIRITS],
    /// Spotted at least once.
    pub seen: [bool; N_SPIRITS],
    /// Befriended at least once.
    pub discovered: [bool; N_SPIRITS],
}

impl Default for Save {
    fn default() -> Save {
        Save {
            candles: 0,
            upgrades: [0; N_UPGRADES],
            treats: [4, 3, 3],
            residents: [0; N_SPIRITS],
            seen: [false; N_SPIRITS],
            discovered: [false; N_SPIRITS],
        }
    }
}

fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn parse_list<T: std::str::FromStr + Copy>(s: &str, out: &mut [T]) {
    for (slot, part) in out.iter_mut().zip(s.split(',')) {
        if let Ok(v) = part.trim().parse::<T>() {
            *slot = v;
        }
    }
}

impl Save {
    pub fn serialize(&self) -> String {
        let b = |v: &[bool]| join(&v.iter().map(|x| *x as u8).collect::<Vec<_>>());
        format!(
            "v1\ncandles={}\nupgrades={}\ntreats={}\nresidents={}\nseen={}\ndiscovered={}\n",
            self.candles,
            join(&self.upgrades),
            join(&self.treats),
            join(&self.residents),
            b(&self.seen),
            b(&self.discovered)
        )
    }

    /// Tolerant parser: unknown/missing/garbled fields fall back to defaults.
    pub fn parse(text: &str) -> Save {
        let mut s = Save::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k {
                "candles" => s.candles = v.trim().parse().unwrap_or(0),
                "upgrades" => parse_list(v, &mut s.upgrades),
                "treats" => parse_list(v, &mut s.treats),
                "residents" => parse_list(v, &mut s.residents),
                "seen" | "discovered" => {
                    let mut raw = [0u8; N_SPIRITS];
                    parse_list(v, &mut raw);
                    let target = if k == "seen" { &mut s.seen } else { &mut s.discovered };
                    for (t, r) in target.iter_mut().zip(raw) {
                        *t = r != 0;
                    }
                }
                _ => {}
            }
        }
        for (lvl, up) in s.upgrades.iter_mut().zip(UPGRADES.iter()) {
            *lvl = (*lvl).min(up.costs.len() as u8);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_wrap_midnight() {
        assert!(hour_in(22.0, (21.0, 4.0)));
        assert!(hour_in(2.0, (21.0, 4.0)));
        assert!(!hour_in(12.0, (21.0, 4.0)));
        assert!(hour_in(5.0, (0.0, 24.0)));
    }

    #[test]
    fn save_roundtrip_and_garbage() {
        let mut s = Save::default();
        s.candles = 123;
        s.upgrades = [2, 1, 3];
        s.treats = [1, 2, 3];
        s.residents = [0, 4, 0, 1];
        s.seen = [true, true, false, true];
        s.discovered = [false, true, false, true];
        assert_eq!(Save::parse(&s.serialize()), s);
        let g = Save::parse("lol\ncandles=abc\nupgrades=9,9,9\n===\n");
        assert_eq!(g.candles, 0);
        assert_eq!(g.upgrades, [3, 3, 3]);
    }

    #[test]
    fn content_is_consistent() {
        for sp in SPIRITS.iter() {
            assert!(sp.favorite < N_TREATS);
            assert!(sp.max_alive > 0);
            assert!(!sp.conditions_text().is_empty());
        }
        // Every weather type has at least one spirit, so no weather is boring.
        for w in [Weather::Clear, Weather::Fog, Weather::Rain] {
            assert!(SPIRITS.iter().any(|s| s.weather.contains(&w)), "{w:?}");
        }
    }
}
