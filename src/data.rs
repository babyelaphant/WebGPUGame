//! Game content tables. To add a spirit, treat or upgrade, edit ONLY this file
//! (and bump the matching `N_*` constant so the compiler checks array sizes).

use crate::env::Weather;

pub const N_TREATS: usize = 5;
pub const N_SPIRITS: usize = 12;
pub const N_UPGRADES: usize = 5;
pub const N_AREAS: usize = 3;

/// Index of the Gate Key upgrade (unlocks Graveyard Hollow).
pub const GATE_KEY: usize = 3;
/// Index of the Frog Charm upgrade (unlocks Willow Pond).
pub const POND_KEY: usize = 4;

pub struct TreatDef {
    pub id: &'static str,
    pub name: &'static str,
    pub color: [f32; 3],
}

pub static TREATS: [TreatDef; N_TREATS] = [
    TreatDef { id: "candy_corn", name: "Candy Corn", color: [1.00, 0.78, 0.20] },
    TreatDef { id: "pumpkin_cookie", name: "Pumpkin Cookie", color: [0.90, 0.50, 0.15] },
    TreatDef { id: "moon_milk", name: "Moon Milk", color: [0.75, 0.85, 1.00] },
    TreatDef { id: "soul_cake", name: "Soul Cake", color: [0.78, 0.55, 0.95] },
    TreatDef { id: "lily_jam", name: "Lily Jam", color: [1.00, 0.60, 0.78] },
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
    /// Which area it lives in: 0 = Hollow Glade, 1 = Graveyard Hollow, 2 = Willow Pond.
    pub area: usize,
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
    /// Only appears after this area's puzzle is solved (glade: lanterns, hollow: crypt runes).
    pub needs_puzzle: bool,
    /// Candles earned when you bring them home.
    pub reward: u32,
    /// One of a kind: once one lives at your cottage, no more will ever appear.
    pub unique: bool,
}

pub static SPIRITS: [SpiritDef; N_SPIRITS] = [
    SpiritDef {
        id: "pip",
        area: 0,
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
        unique: false,
    },
    SpiritDef {
        id: "mizzle",
        area: 0,
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
        unique: false,
    },
    SpiritDef {
        id: "hush",
        area: 0,
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
        unique: false,
    },
    SpiritDef {
        id: "luna",
        area: 0,
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
        unique: false,
    },
    SpiritDef {
        id: "wick",
        area: 1,
        name: "Wick",
        hint: "A stub of a candle ghost. Follows anything warm and sweet.",
        color: [1.00, 0.88, 0.60],
        size: 0.5,
        favorite: 3,
        behavior: Behavior::Curious,
        weather: &[Weather::Clear],
        hours: (18.0, 5.0),
        max_alive: 2,
        spawn_chance: 0.45,
        needs_puzzle: false,
        reward: 25,
        unique: false,
    },
    SpiritDef {
        id: "drip",
        area: 1,
        name: "Drip",
        hint: "Giggles at every raindrop. Has a weakness for spiced baking.",
        color: [0.45, 0.72, 0.78],
        size: 0.5,
        favorite: 1,
        behavior: Behavior::Playful,
        weather: &[Weather::Rain],
        hours: (0.0, 24.0),
        max_alive: 3,
        spawn_chance: 0.50,
        needs_puzzle: false,
        reward: 30,
        unique: false,
    },
    SpiritDef {
        id: "gloam",
        area: 1,
        name: "Gloam",
        hint: "A bat-winged mist sprite. Hides in the fog and hates being rushed.",
        color: [0.60, 0.50, 0.85],
        size: 0.6,
        favorite: 3,
        behavior: Behavior::Shy,
        weather: &[Weather::Fog],
        hours: (0.0, 24.0),
        max_alive: 2,
        spawn_chance: 0.40,
        needs_puzzle: false,
        reward: 40,
        unique: false,
    },
    SpiritDef {
        id: "marrow",
        area: 1,
        name: "Old Marrow, Crypt Keeper",
        hint: "Tips his tiny hat. Smells of sugar and old stone.",
        color: [0.93, 0.92, 0.82],
        size: 0.9,
        favorite: 0,
        behavior: Behavior::Curious,
        weather: &[],
        hours: (20.0, 5.0),
        max_alive: 1,
        spawn_chance: 0.50,
        needs_puzzle: true,
        reward: 90,
        unique: true,
    },
    SpiritDef {
        id: "bobbin",
        area: 2,
        name: "Bobbin",
        hint: "A bouncy little frog ghost. Loves anything sweet and floral.",
        color: [0.55, 0.90, 0.55],
        size: 0.5,
        favorite: 4,
        behavior: Behavior::Curious,
        weather: &[Weather::Clear],
        hours: (6.0, 19.0),
        max_alive: 2,
        spawn_chance: 0.45,
        needs_puzzle: false,
        reward: 35,
        unique: false,
    },
    SpiritDef {
        id: "pondle",
        area: 2,
        name: "Pondle",
        hint: "Splashes around in the rain. Can't resist warm spiced baking.",
        color: [0.45, 0.75, 0.95],
        size: 0.5,
        favorite: 1,
        behavior: Behavior::Playful,
        weather: &[Weather::Rain],
        hours: (0.0, 24.0),
        max_alive: 3,
        spawn_chance: 0.50,
        needs_puzzle: false,
        reward: 30,
        unique: false,
    },
    SpiritDef {
        id: "reed",
        area: 2,
        name: "Reed",
        hint: "A wisp of pond mist that hides in the cattails. Likes candy, hates being rushed.",
        color: [0.75, 0.88, 0.80],
        size: 0.6,
        favorite: 0,
        behavior: Behavior::Shy,
        weather: &[Weather::Fog],
        hours: (0.0, 24.0),
        max_alive: 2,
        spawn_chance: 0.40,
        needs_puzzle: false,
        reward: 45,
        unique: false,
    },
    SpiritDef {
        id: "lotus",
        area: 2,
        name: "Lady Lotus, Pond Queen",
        hint: "Rises from the water in daylight. Thirsty for something silvery.",
        color: [1.00, 0.72, 0.85],
        size: 0.9,
        favorite: 2,
        behavior: Behavior::Curious,
        weather: &[],
        hours: (7.0, 19.0),
        max_alive: 1,
        spawn_chance: 0.50,
        needs_puzzle: true,
        reward: 110,
        unique: true,
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
    UpgradeDef {
        id: "gate_key",
        name: "Gate Key",
        desc: "Opens the iron gate to Graveyard Hollow",
        costs: &[40],
    },
    UpgradeDef {
        id: "frog_charm",
        name: "Frog Charm",
        desc: "Opens the reed gate to Willow Pond",
        costs: &[70],
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
            treats: [4, 3, 3, 2, 2],
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
        s.upgrades = [2, 1, 3, 1, 1];
        s.treats = [1, 2, 3, 4, 5];
        s.residents = [0, 4, 0, 1, 2, 0, 0, 3, 1, 0, 0, 1];
        s.seen = [true, true, false, true, true, false, false, true, true, false, false, true];
        s.discovered = [false, true, false, true, true, false, false, true, false, false, false, true];
        assert_eq!(Save::parse(&s.serialize()), s);
        let g = Save::parse("lol\ncandles=abc\nupgrades=9,9,9,9,9\n===\n");
        assert_eq!(g.candles, 0);
        assert_eq!(g.upgrades, [3, 3, 3, 1, 1]);
    }

    #[test]
    fn content_is_consistent() {
        for sp in SPIRITS.iter() {
            assert!(sp.favorite < N_TREATS);
            assert!(sp.max_alive > 0);
            assert!(!sp.conditions_text().is_empty());
        }
        // Every weather type has at least one spirit IN EACH AREA, so no weather is boring anywhere.
        for area in 0..N_AREAS {
            for w in [Weather::Clear, Weather::Fog, Weather::Rain] {
                assert!(SPIRITS.iter().any(|s| s.area == area && s.weather.contains(&w)), "area {area} {w:?}");
            }
            assert!(SPIRITS.iter().any(|s| s.area == area && s.needs_puzzle), "area {area} needs a puzzle spirit");
        }
        assert!(SPIRITS.iter().all(|s| s.area < N_AREAS));
        assert_eq!(UPGRADES[GATE_KEY].id, "gate_key");
        // Soul Cake is the Hollow's own treat: somebody there must want it.
        assert!(SPIRITS.iter().any(|s| s.area == 1 && s.favorite == 3));
        // ...and Lily Jam is the Pond's.
        assert!(SPIRITS.iter().any(|s| s.area == 2 && s.favorite == 4));
        assert_eq!(UPGRADES[POND_KEY].id, "frog_charm");
        // Legendary puzzle spirits are one-of-a-kind.
        for sp in SPIRITS.iter().filter(|s| s.needs_puzzle && s.area > 0) {
            assert!(sp.unique, "{} should be unique", sp.id);
        }
    }
}
