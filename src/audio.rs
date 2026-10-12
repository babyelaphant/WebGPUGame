//! What the game wants to *hear*. Pure data: the game pushes `Sfx` events, the browser layer
//! forwards them to `assets/audio.js` (synthesized Web Audio, no sound files needed).

use crate::env::Weather;
use crate::game::Game;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Step,
    Pickup,
    Drop,
    Munch,
    Befriend,
    Deposit,
    Buy,
    Lantern,
    Puzzle,
    Spot,
    Flee,
    Nope,
    Ui,
    /// One of the four crypt rune stones (0..3) rings.
    Rune(u8),
    /// Passing through a gate between areas.
    Gate,
    /// The journal learned something (a pencil scratch and a tiny bell).
    Clue,
    /// The weather just started changing to this.
    Weather(Weather),
}

impl Sfx {
    /// Must match the keys of `SFX` in assets/audio.js.
    pub fn name(self) -> &'static str {
        match self {
            Sfx::Step => "step",
            Sfx::Pickup => "pickup",
            Sfx::Drop => "drop",
            Sfx::Munch => "munch",
            Sfx::Befriend => "befriend",
            Sfx::Deposit => "deposit",
            Sfx::Buy => "buy",
            Sfx::Lantern => "lantern",
            Sfx::Puzzle => "puzzle",
            Sfx::Spot => "spot",
            Sfx::Flee => "flee",
            Sfx::Nope => "nope",
            Sfx::Ui => "ui",
            Sfx::Rune(0) => "rune0",
            Sfx::Rune(1) => "rune1",
            Sfx::Rune(2) => "rune2",
            Sfx::Rune(_) => "rune3",
            Sfx::Gate => "gate",
            Sfx::Clue => "clue",
            Sfx::Weather(Weather::Clear) => "weather_clear",
            Sfx::Weather(Weather::Fog) => "weather_fog",
            Sfx::Weather(Weather::Rain) => "weather_rain",
        }
    }
}

/// The mood of the world, 0..1 each; the audio engine crossfades its ambience layers to match.
pub struct Ambience {
    pub night: f32,
    pub rain: f32,
    pub fog: f32,
}

pub fn ambience(g: &Game) -> Ambience {
    let l = g.lighting();
    Ambience { night: l.night, rain: l.rain, fog: l.fog_amt }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sfx_has_a_unique_name_that_exists_in_the_js_engine() {
        let all = [
            Sfx::Step, Sfx::Pickup, Sfx::Drop, Sfx::Munch, Sfx::Befriend, Sfx::Deposit, Sfx::Buy,
            Sfx::Lantern, Sfx::Puzzle, Sfx::Spot, Sfx::Flee, Sfx::Nope, Sfx::Ui,
            Sfx::Rune(0), Sfx::Rune(1), Sfx::Rune(2), Sfx::Rune(3), Sfx::Gate, Sfx::Clue,
            Sfx::Weather(Weather::Clear), Sfx::Weather(Weather::Fog), Sfx::Weather(Weather::Rain),
        ];
        let js = include_str!("../assets/audio.js");
        // The parameterised calls the browser layer makes besides plain Sfx names.
        for api in ["setVolumes("] {
            assert!(js.contains(api), "assets/audio.js has no {api}");
        }
        let mut seen = std::collections::HashSet::new();
        for s in all {
            assert!(seen.insert(s.name()), "duplicate name {}", s.name());
            // Each name must be defined as a key in the JS SFX table.
            let key = format!("{}(t)", s.name());
            assert!(js.contains(&key), "assets/audio.js has no SFX '{}'", s.name());
        }
    }
}
