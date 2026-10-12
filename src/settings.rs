//! Player-facing options (volume, pixel size, hints...). Pure data + a tolerant `key=value` text
//! format, so it can be unit-tested natively. The menu itself (assets/menu.js) edits the same text
//! format and keeps it in localStorage; the game just re-parses it whenever it changes.

/// Volume steps run 0..=10.
pub const VOL_MAX: u8 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub master: u8,
    pub music: u8,
    pub sfx: u8,
    pub ambience: u8,
    /// 5..=15 where 10 = normal (50%..150%). Applied in CSS by the menu.
    pub brightness: u8,
    /// 0 small, 1 normal, 2 large. Applied in CSS by the menu.
    pub ui_scale: u8,
    /// 0 chunky, 1 normal, 2 fine: how big one game pixel is.
    pub pixel: u8,
    /// 0 off, 1 subtle, 2 full: how strong the rain streaks and fog banks look.
    pub weather_fx: u8,
    pub vignette: bool,
    /// Tutorial-style hint lines (the "press E" prompts always stay).
    pub hints: bool,
    /// Sparkles over nearby spirits + the little compass hint.
    pub shimmer: bool,
    pub show_fps: bool,
    /// false = hold Shift to run, true = Shift toggles running.
    pub run_toggle: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            master: VOL_MAX,
            music: VOL_MAX,
            sfx: VOL_MAX,
            ambience: VOL_MAX,
            brightness: 10,
            ui_scale: 1,
            pixel: 1,
            weather_fx: 2,
            vignette: true,
            hints: true,
            shimmer: true,
            show_fps: false,
            run_toggle: false,
        }
    }
}

fn num(v: &str, lo: u8, hi: u8, fallback: u8) -> u8 {
    v.trim().parse::<u8>().map(|x| x.clamp(lo, hi)).unwrap_or(fallback)
}

fn flag(v: &str, fallback: bool) -> bool {
    match v.trim() {
        "1" | "true" | "on" => true,
        "0" | "false" | "off" => false,
        _ => fallback,
    }
}

impl Settings {
    /// Target height (in game pixels) of the low-res scene. The renderer rounds the screen-to-game
    /// pixel ratio to a whole number, so every game pixel stays a perfect square.
    pub fn pixel_target_h(&self) -> f32 {
        match self.pixel {
            0 => 240.0,
            2 => 480.0,
            _ => 320.0,
        }
    }

    /// Multiplier for rain streaks and fog banks in the post pass.
    pub fn weather_mul(&self) -> f32 {
        match self.weather_fx {
            0 => 0.0,
            1 => 0.5,
            _ => 1.0,
        }
    }

    pub fn vignette_amt(&self) -> f32 {
        if self.vignette {
            0.42
        } else {
            0.0
        }
    }

    /// Volume steps as 0..1.
    pub fn vol(step: u8) -> f32 {
        step.min(VOL_MAX) as f32 / VOL_MAX as f32
    }

    pub fn serialize(&self) -> String {
        let b = |x: bool| x as u8;
        format!(
            "master={}\nmusic={}\nsfx={}\nambience={}\nbrightness={}\nui_scale={}\npixel={}\nweather_fx={}\nvignette={}\nhints={}\nshimmer={}\nfps={}\nrun_toggle={}\n",
            self.master,
            self.music,
            self.sfx,
            self.ambience,
            self.brightness,
            self.ui_scale,
            self.pixel,
            self.weather_fx,
            b(self.vignette),
            b(self.hints),
            b(self.shimmer),
            b(self.show_fps),
            b(self.run_toggle)
        )
    }

    /// Tolerant: unknown keys, missing keys and garbage all fall back to the defaults.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "master" => s.master = num(v, 0, VOL_MAX, s.master),
                "music" => s.music = num(v, 0, VOL_MAX, s.music),
                "sfx" => s.sfx = num(v, 0, VOL_MAX, s.sfx),
                "ambience" => s.ambience = num(v, 0, VOL_MAX, s.ambience),
                "brightness" => s.brightness = num(v, 5, 15, s.brightness),
                "ui_scale" => s.ui_scale = num(v, 0, 2, s.ui_scale),
                "pixel" => s.pixel = num(v, 0, 2, s.pixel),
                "weather_fx" => s.weather_fx = num(v, 0, 2, s.weather_fx),
                "vignette" => s.vignette = flag(v, s.vignette),
                "hints" => s.hints = flag(v, s.hints),
                "shimmer" => s.shimmer = flag(v, s.shimmer),
                "fps" => s.show_fps = flag(v, s.show_fps),
                "run_toggle" => s.run_toggle = flag(v, s.run_toggle),
                _ => {}
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_defaults() {
        let d = Settings::default();
        assert_eq!(Settings::parse(&d.serialize()), d);
        let s = Settings {
            master: 3,
            music: 0,
            sfx: 10,
            ambience: 5,
            brightness: 12,
            ui_scale: 2,
            pixel: 0,
            weather_fx: 1,
            vignette: false,
            hints: false,
            shimmer: false,
            show_fps: true,
            run_toggle: true,
        };
        assert_eq!(Settings::parse(&s.serialize()), s);
        assert_eq!(Settings::parse(""), d);
    }

    #[test]
    fn garbage_and_out_of_range_are_tamed() {
        let s = Settings::parse("master=99\nmusic=abc\nbrightness=0\nui_scale=9\npixel=7\nweather_fx=-1\nhints=maybe\nwhat\n===\nzzz=1\nfps=on\n");
        assert_eq!(s.master, VOL_MAX);
        assert_eq!(s.music, Settings::default().music);
        assert_eq!(s.brightness, 5);
        assert_eq!(s.ui_scale, 2);
        assert_eq!(s.pixel, 2);
        assert_eq!(s.weather_fx, Settings::default().weather_fx);
        assert!(s.hints, "unreadable flag keeps its default");
        assert!(s.show_fps);
    }

    #[test]
    fn derived_values_make_sense() {
        let mut s = Settings::default();
        assert_eq!(s.pixel_target_h(), 320.0);
        s.pixel = 0;
        assert!(s.pixel_target_h() < 320.0);
        s.pixel = 2;
        assert!(s.pixel_target_h() > 320.0);
        s.weather_fx = 0;
        assert_eq!(s.weather_mul(), 0.0);
        s.weather_fx = 1;
        assert!(s.weather_mul() > 0.0 && s.weather_mul() < 1.0);
        s.weather_fx = 2;
        assert_eq!(s.weather_mul(), 1.0);
        assert_eq!(Settings::vol(0), 0.0);
        assert_eq!(Settings::vol(VOL_MAX), 1.0);
        s.vignette = false;
        assert_eq!(s.vignette_amt(), 0.0);
    }
}
