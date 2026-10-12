//! Time of day + weather. Drives lighting AND which spirits can appear.

use crate::math::*;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Fog,
    Rain,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Fog => "Fog",
            Weather::Rain => "Rain",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Weather::Clear => "✨",
            Weather::Fog => "🌫️",
            Weather::Rain => "🌧️",
        }
    }
    pub fn next(self) -> Weather {
        match self {
            Weather::Clear => Weather::Fog,
            Weather::Fog => Weather::Rain,
            Weather::Rain => Weather::Clear,
        }
    }
}

pub struct Environment {
    pub hour: f32,
    pub day: u32,
    pub weather: Weather,
    prev: Weather,
    blend: f32,
    timer: f32,
    /// What the weather will become at the next change (so the HUD can give a forecast).
    pub next: Weather,
    warned: bool,
    /// Set whenever the weather starts changing; the game consumes it to announce the change.
    pub changed: Option<Weather>,
    /// Set once when a change is about to happen (see FORECAST_LEAD).
    pub warning: Option<Weather>,
}

pub struct Lighting {
    pub sun_dir: Vec3,
    pub sun_col: [f32; 3],
    pub sun_i: f32,
    pub ambient: [f32; 3],
    pub fog_col: [f32; 3],
    pub fog_density: f32,
    pub rain: f32,
    pub fog_amt: f32,
    pub night: f32,
}

struct Key {
    h: f32,
    sun: [f32; 3],
    sun_i: f32,
    amb: [f32; 3],
    fog: [f32; 3],
}

// Hand-tuned palette over the day. The last key (24h) must equal the first (0h).
const KEYS: [Key; 8] = [
    Key { h: 0.0, sun: [0.55, 0.65, 1.00], sun_i: 0.95, amb: [0.24, 0.28, 0.46], fog: [0.10, 0.12, 0.24] },
    Key { h: 5.0, sun: [0.60, 0.60, 0.95], sun_i: 0.80, amb: [0.26, 0.28, 0.44], fog: [0.16, 0.16, 0.32] },
    Key { h: 7.0, sun: [1.00, 0.72, 0.52], sun_i: 0.90, amb: [0.45, 0.40, 0.50], fog: [0.70, 0.55, 0.55] },
    Key { h: 11.0, sun: [1.00, 0.95, 0.80], sun_i: 1.00, amb: [0.50, 0.55, 0.65], fog: [0.55, 0.65, 0.75] },
    Key { h: 17.0, sun: [1.00, 0.90, 0.70], sun_i: 1.00, amb: [0.48, 0.50, 0.60], fog: [0.60, 0.62, 0.70] },
    Key { h: 18.5, sun: [1.00, 0.55, 0.30], sun_i: 0.90, amb: [0.40, 0.30, 0.45], fog: [0.45, 0.28, 0.35] },
    Key { h: 20.5, sun: [0.55, 0.65, 1.00], sun_i: 0.95, amb: [0.24, 0.28, 0.46], fog: [0.10, 0.12, 0.24] },
    Key { h: 24.0, sun: [0.55, 0.65, 1.00], sun_i: 0.95, amb: [0.24, 0.28, 0.46], fog: [0.10, 0.12, 0.24] },
];

impl Environment {
    /// Real seconds for one full in-game day.
    pub const DAY_SECONDS: f32 = 480.0;
    const WEATHER_FADE: f32 = 20.0;
    /// Seconds of advance notice before the weather turns.
    pub const FORECAST_LEAD: f32 = 30.0;

    pub fn new(hour: f32) -> Environment {
        Environment {
            hour,
            day: 1,
            weather: Weather::Clear,
            prev: Weather::Clear,
            blend: 1.0,
            timer: 150.0,
            next: Weather::Fog,
            warned: false,
            changed: None,
            warning: None,
        }
    }

    /// Advances the clock + weather. Returns true when dawn (06:00) is crossed.
    pub fn update(&mut self, dt: f32, rng: &mut Rng) -> bool {
        let old = self.hour;
        self.hour += dt * 24.0 / Self::DAY_SECONDS;
        if self.hour >= 24.0 {
            self.hour -= 24.0;
            self.day += 1;
        }
        let dawn = old < 6.0 && self.hour >= 6.0;

        self.blend = (self.blend + dt / Self::WEATHER_FADE).min(1.0);
        self.timer -= dt;
        if !self.warned && self.timer <= Self::FORECAST_LEAD {
            self.warned = true;
            self.warning = Some(self.next);
        }
        if self.timer <= 0.0 {
            let w = self.next;
            self.timer = rng.range(100.0, 190.0);
            self.warned = false;
            self.set_weather(w);
            self.next = Self::pick_next(rng, w);
        }
        dawn
    }

    /// Weighted pick of what comes next, never the same as `current`.
    fn pick_next(rng: &mut Rng, current: Weather) -> Weather {
        let roll = rng.f32();
        let pick = if roll < 0.40 {
            Weather::Clear
        } else if roll < 0.70 {
            Weather::Fog
        } else {
            Weather::Rain
        };
        if pick == current {
            pick.next()
        } else {
            pick
        }
    }

    /// The weather we are about to get, once the forecast window has opened.
    pub fn forecast(&self) -> Option<Weather> {
        if self.warned {
            Some(self.next)
        } else {
            None
        }
    }

    pub fn set_weather(&mut self, w: Weather) {
        if w != self.weather {
            self.changed = Some(w);
            self.prev = self.effective();
            self.weather = w;
            self.blend = 0.0;
        }
    }

    /// The weather spirits "feel": switches halfway through the visual crossfade.
    pub fn effective(&self) -> Weather {
        if self.blend >= 0.5 {
            self.weather
        } else {
            self.prev
        }
    }

    fn weight(&self, w: Weather) -> f32 {
        let mut x = 0.0;
        if self.weather == w {
            x += self.blend;
        }
        if self.prev == w {
            x += 1.0 - self.blend;
        }
        x
    }

    pub fn phase_name(&self) -> &'static str {
        let h = self.hour;
        if (5.0..7.0).contains(&h) {
            "Dawn"
        } else if (7.0..17.0).contains(&h) {
            "Day"
        } else if (17.0..20.0).contains(&h) {
            "Dusk"
        } else {
            "Night"
        }
    }

    pub fn night_factor(&self) -> f32 {
        let h = self.hour;
        if h >= 20.5 || h < 5.0 {
            1.0
        } else if h < 7.0 {
            1.0 - smoothstep(5.0, 7.0, h)
        } else if h < 18.5 {
            0.0
        } else {
            smoothstep(18.5, 20.5, h)
        }
    }

    pub fn lighting(&self) -> Lighting {
        let h = self.hour.rem_euclid(24.0);
        let mut i = 0;
        while i + 2 < KEYS.len() && h >= KEYS[i + 1].h {
            i += 1;
        }
        let (a, b) = (&KEYS[i], &KEYS[i + 1]);
        let t = ((h - a.h) / (b.h - a.h)).clamp(0.0, 1.0);
        let mut sun_col = lerp3(a.sun, b.sun, t);
        let mut sun_i = lerp(a.sun_i, b.sun_i, t);
        let mut ambient = lerp3(a.amb, b.amb, t);
        let mut fog_col = lerp3(a.fog, b.fog, t);

        let night = self.night_factor();
        let fw = self.weight(Weather::Fog);
        let rw = self.weight(Weather::Rain);

        sun_i *= 1.0 - 0.30 * fw - 0.35 * rw;
        for c in ambient.iter_mut() {
            *c *= 1.0 - 0.06 * rw;
        }
        let gray = lerp3([0.26, 0.31, 0.44], [0.64, 0.68, 0.76], 1.0 - night);
        fog_col = lerp3(fog_col, gray, 0.40 * fw.max(rw));
        let density = 0.004 + 0.016 * fw + 0.005 * rw;
        for c in sun_col.iter_mut() {
            *c = c.clamp(0.0, 1.0);
        }

        // Sun by day, moon by night, sweeping east -> west on a tilted arc.
        let hh = if h < 6.0 { h + 24.0 } else { h };
        let p = if hh < 18.0 { (hh - 6.0) / 12.0 } else { (hh - 18.0) / 12.0 };
        let ang = p.clamp(0.0, 1.0) * std::f32::consts::PI;
        let sun_dir = Vec3::new(ang.cos() * 0.8, 0.45 + 0.55 * ang.sin(), 0.35).normalized();

        Lighting {
            sun_dir,
            sun_col,
            sun_i,
            ambient,
            fog_col,
            fog_density: density,
            rain: rw,
            fog_amt: fw,
            night,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_wrap_cleanly() {
        let (a, b) = (&KEYS[0], &KEYS[KEYS.len() - 1]);
        assert_eq!(a.sun, b.sun);
        assert_eq!(a.amb, b.amb);
        assert_eq!(b.h, 24.0);
    }

    #[test]
    fn lighting_is_finite_all_day_and_night_is_darker() {
        let mut env = Environment::new(0.0);
        let mut last_dark = 0.0;
        for step in 0..240 {
            env.hour = step as f32 * 0.1;
            let l = env.lighting();
            assert!(l.sun_dir.length() > 0.99);
            assert!(l.ambient.iter().all(|c| c.is_finite() && *c >= 0.0));
            assert!(l.fog_density > 0.0 && l.fog_density.is_finite());
            last_dark = l.ambient[0];
        }
        let _ = last_dark;
        env.hour = 12.0;
        let day = env.lighting().ambient[0];
        env.hour = 1.0;
        let night = env.lighting().ambient[0];
        assert!(day > night * 2.0);
    }

    #[test]
    fn forecast_comes_before_the_change_and_changes_are_announced() {
        let mut env = Environment::new(12.0);
        let mut rng = Rng::new(9);
        let mut warned_at = None;
        let mut changed_at = None;
        let mut warned_for = None;
        let mut t = 0.0f32;
        while t < 400.0 && changed_at.is_none() {
            env.update(1.0 / 60.0, &mut rng);
            t += 1.0 / 60.0;
            if let Some(w) = env.warning.take() {
                warned_at = Some(t);
                warned_for = Some(w);
            }
            if let Some(w) = env.changed.take() {
                changed_at = Some((t, w));
            }
        }
        let (ct, cw) = changed_at.expect("weather should change");
        let wt = warned_at.expect("should have been warned first");
        assert!(ct - wt > 25.0 && ct - wt < 31.0, "warned {}s ahead", ct - wt);
        assert_eq!(warned_for, Some(cw), "forecast must match what actually arrives");
        assert!(env.forecast().is_none(), "forecast clears once it arrives");
    }

    #[test]
    fn weather_changes_and_dawn_fires_once() {
        let mut env = Environment::new(5.9);
        let mut rng = Rng::new(3);
        let mut dawns = 0;
        let mut seen_non_clear = false;
        for _ in 0..(60 * 600) {
            if env.update(1.0 / 60.0, &mut rng) {
                dawns += 1;
            }
            if env.weather != Weather::Clear {
                seen_non_clear = true;
            }
        }
        assert!(seen_non_clear);
        assert!(dawns >= 1 && dawns <= 2, "got {dawns}");
    }
}
