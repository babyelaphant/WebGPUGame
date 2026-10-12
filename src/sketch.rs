//! The journal's drawings: one little SVG per spirit, built from layers so the page fills in as you
//! learn more about it.
//!
//!   never seen    a dashed "?" box
//!   seen          an ink silhouette (you can tell who it is by its outline)
//!   + habits      pencil outline with a face that shows how it behaves
//!   + likes       a doodle of its favorite treat in the corner
//!   + when        weather and time-of-day doodles in the top corners
//!   + friend      full colour, blush, sparkles
//!
//! Pure string building, so it is covered by native tests. Everything is drawn in a 100x100 box.

use crate::data::*;
use crate::env::Weather;

const INK: &str = "#4a3a2c";
const FACE: &str = "#2c2233";
const PAPER: &str = "#fbf3de";
const SILHOUETTE: &str = "#43364f";

fn hex(c: [f32; 3]) -> String {
    let f = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", f(c[0]), f(c[1]), f(c[2]))
}

/// Which layers of a spirit's page are filled in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layers {
    pub seen: bool,
    pub habits: bool,
    pub likes: bool,
    pub when: bool,
    pub friend: bool,
}

impl Layers {
    pub fn of(save: &Save, def: usize) -> Layers {
        Layers {
            seen: save.knows(def, Clue::Seen),
            habits: save.knows(def, Clue::Habits),
            likes: save.knows(def, Clue::Likes),
            when: save.knows(def, Clue::When),
            friend: save.knows(def, Clue::Friend),
        }
    }
}

pub fn sketch(def: usize, save: &Save) -> String {
    draw(def, Layers::of(save, def))
}

/// Squash / stretch per species so the silhouettes differ (x scale, y scale; feet stay put).
fn proportions(id: &str) -> (f32, f32) {
    match id {
        "pip" => (1.14, 0.92),
        "hush" => (0.9, 1.04),
        "luna" => (1.04, 1.06),
        "marrow" => (0.98, 1.08),
        "bobbin" => (1.1, 0.9),
        "reed" => (0.88, 1.06),
        "lotus" => (1.02, 1.04),
        _ => (1.0, 1.0),
    }
}

/// A second colour for the extra bits (stem, flame, hat band...) once the spirit is a friend.
fn accent(id: &str) -> &'static str {
    match id {
        "pip" => "#6aa84f",
        "mizzle" => "#7fc4f2",
        "hush" => "#c9b6f5",
        "luna" => "#f5d36a",
        "wick" => "#ffb84d",
        "drip" => "#5aa9c6",
        "gloam" => "#7b6bb0",
        "marrow" => "#a14848",
        "bobbin" => "#9be07a",
        "pondle" => "#4fa3d9",
        "reed" => "#9a6a3a",
        "lotus" => "#f7a8c6",
        _ => "#d9a066",
    }
}

fn unseen() -> String {
    "<svg class=\"sk\" viewBox=\"0 0 100 100\" xmlns=\"http://www.w3.org/2000/svg\">\
     <rect x=\"14\" y=\"14\" width=\"72\" height=\"72\" rx=\"12\" fill=\"none\" stroke=\"#b9a584\" stroke-width=\"2.4\" stroke-dasharray=\"5 6\" stroke-linecap=\"round\"/>\
     <text x=\"50\" y=\"66\" text-anchor=\"middle\" font-size=\"46\" font-family=\"VT323, monospace\" fill=\"#b9a584\">?</text></svg>"
        .to_string()
}

/// Everything that depends on the species: parts behind the body, parts in front, and whether it
/// has its own eyes (the frog's are bumps on its head).
struct Parts {
    behind: String,
    front: String,
    own_eyes: bool,
}

fn parts(id: &str, top: f32, fill: &str, edge: &str, acc: &str) -> Parts {
    // `fill`/`edge` style the species-specific shapes; `acc` fills the little accents.
    let st = |f: &str| format!("fill=\"{f}\" {edge}");
    let line = format!("fill=\"none\" {edge}");
    let mut behind = String::new();
    let mut front = String::new();
    let mut own_eyes = false;
    match id {
        "pip" => {
            front.push_str(&format!("<path d=\"M38 {a} C31 {b} 31 {c} 38 {d}\" {line}/>", a = top + 6.0, b = top + 18.0, c = top + 30.0, d = top + 40.0));
            front.push_str(&format!("<path d=\"M62 {a} C69 {b} 69 {c} 62 {d}\" {line}/>", a = top + 6.0, b = top + 18.0, c = top + 30.0, d = top + 40.0));
            front.push_str(&format!("<path d=\"M46 {a} L47 {b} L54 {b} L55 {a} Z\" {}/>", st(acc), a = top + 1.0, b = top - 8.0));
            front.push_str(&format!("<path d=\"M54 {a} q7 -5 9 2\" {line}/>", a = top - 5.0));
        }
        "mizzle" => {
            front.push_str(&format!("<path d=\"M50 {a} C57 {b} 59 {c} 50 {d} C41 {c} 43 {b} 50 {a} Z\" {}/>", st(acc), a = top - 13.0, b = top - 5.0, c = top, d = top + 4.0));
            front.push_str(&format!("<path d=\"M21 26 c3 4 4 6 0 9 c-4 -3 -3 -5 0 -9 Z\" {}/>", st(acc)));
            front.push_str(&format!("<path d=\"M80 18 c3 4 4 6 0 9 c-4 -3 -3 -5 0 -9 Z\" {}/>", st(acc)));
            front.push_str(&format!("<ellipse cx=\"50\" cy=\"90\" rx=\"22\" ry=\"4\" {line}/><ellipse cx=\"50\" cy=\"91\" rx=\"32\" ry=\"6\" {line} stroke-dasharray=\"6 5\"/>"));
        }
        "hush" => {
            behind.push_str(&format!("<path d=\"M32 42 C6 16 0 52 17 62 C8 76 28 86 35 68 Z\" {}/>", st(acc)));
            behind.push_str(&format!("<path d=\"M68 42 C94 16 100 52 83 62 C92 76 72 86 65 68 Z\" {}/>", st(acc)));
            front.push_str(&format!("<path d=\"M45 {a} Q40 {b} 33 {b}\" {line}/><path d=\"M55 {a} Q60 {b} 67 {b}\" {line}/>", a = top + 1.0, b = top - 10.0));
            front.push_str(&format!("<circle cx=\"33\" cy=\"{y}\" r=\"2\" {}/><circle cx=\"67\" cy=\"{y}\" r=\"2\" {}/>", st(fill), st(fill), y = top - 10.0));
        }
        "luna" => {
            behind.push_str(&format!("<circle cx=\"50\" cy=\"46\" r=\"44\" fill=\"{acc}\" fill-opacity=\"0.22\" stroke=\"{acc}\" stroke-width=\"1.8\" stroke-dasharray=\"2 6\" stroke-linecap=\"round\"/>"));
            front.push_str(&format!(
                "<path d=\"M35 {b} L38 {t} L44 {m} L50 {t0} L56 {m} L62 {t} L65 {b} Z\" {}/>",
                st(acc),
                b = top + 6.0,
                t = top - 9.0,
                m = top - 2.0,
                t0 = top - 13.0
            ));
        }
        "wick" => {
            front.push_str(&format!("<path d=\"M50 {a} C58 {b} 59 {c} 50 {d} C41 {c} 42 {b} 50 {a} Z\" {}/>", st(acc), a = top - 15.0, b = top - 7.0, c = top - 1.0, d = top + 3.0));
            front.push_str(&format!("<path d=\"M50 {a} C53 {b} 53 {c} 50 {d} C47 {c} 47 {b} 50 {a} Z\" fill=\"{PAPER}\" fill-opacity=\"0.85\"/>", a = top - 8.0, b = top - 4.0, c = top - 1.0, d = top + 1.0));
            front.push_str(&format!("<path d=\"M30 56 q-2 8 0 14\" {line}/><path d=\"M70 52 q2 9 0 15\" {line}/>"));
        }
        "drip" => {
            front.push_str(&format!("<path d=\"M42 {b} Q50 {a} 58 {b} Z\" {}/>", st(acc), a = top - 16.0, b = top + 6.0));
            for x in [34.0f32, 52.0, 67.0] {
                front.push_str(&format!("<path d=\"M{x} 90 c2.5 3.5 3.5 5.5 0 8 c-3.5 -2.5 -2.5 -4.5 0 -8 Z\" {}/>", st(acc)));
            }
        }
        "gloam" => {
            let wing = format!("<path d=\"M28 44 C16 30 8 30 1 35 C8 39 8 45 10 51 C14 47 18 49 20 55 C22 51 26 53 28 59 Z\" {}/>", st(acc));
            behind.push_str(&wing);
            behind.push_str(&format!("<g transform=\"translate(100 0) scale(-1 1)\">{wing}</g>"));
            front.push_str(&format!("<path d=\"M32 {a} L29 {t} L42 {b} Z\" {}/><path d=\"M68 {a} L71 {t} L58 {b} Z\" {}/>", st(fill), st(fill), a = top + 12.0, t = top - 6.0, b = top + 5.0));
        }
        "marrow" => {
            front.push_str(&format!("<ellipse cx=\"50\" cy=\"{y}\" rx=\"20\" ry=\"3.8\" {}/>", st(FACE), y = top + 2.0));
            front.push_str(&format!("<rect x=\"39\" y=\"{y}\" width=\"22\" height=\"{h}\" rx=\"2\" {}/>", st(FACE), y = top - 15.0, h = 17.0));
            front.push_str(&format!("<rect x=\"39\" y=\"{y}\" width=\"22\" height=\"4\" fill=\"{acc}\"/>", y = top - 4.0));
            front.push_str(&format!("<path d=\"M50 68 L38 62 L38 74 Z\" {}/><path d=\"M50 68 L62 62 L62 74 Z\" {}/><circle cx=\"50\" cy=\"68\" r=\"2.4\" {}/>", st(acc), st(acc), st(acc)));
        }
        "bobbin" => {
            own_eyes = true;
            front.push_str(&format!("<circle cx=\"37\" cy=\"{y}\" r=\"9\" {}/><circle cx=\"63\" cy=\"{y}\" r=\"9\" {}/>", st(fill), st(fill), y = top + 6.0));
            front.push_str(&format!("<circle cx=\"37\" cy=\"{y}\" r=\"4\" fill=\"{FACE}\"/><circle cx=\"63\" cy=\"{y}\" r=\"4\" fill=\"{FACE}\"/>", y = top + 6.0));
            front.push_str(&format!("<circle cx=\"38.5\" cy=\"{y}\" r=\"1.3\" fill=\"#fff\"/><circle cx=\"64.5\" cy=\"{y}\" r=\"1.3\" fill=\"#fff\"/>", y = top + 4.8));
            front.push_str(&format!("<ellipse cx=\"50\" cy=\"90\" rx=\"26\" ry=\"6\" {}/>", st(acc)));
        }
        "pondle" => {
            front.push_str(&format!("<path d=\"M42 {b} Q48 {a} 62 {c} Q54 {c} 54 {b} Z\" {}/>", st(acc), a = top - 12.0, b = top + 6.0, c = top - 2.0));
            front.push_str(&format!("<path d=\"M25 56 Q9 50 7 64 Q17 62 25 70 Z\" {}/><path d=\"M75 56 Q91 50 93 64 Q83 62 75 70 Z\" {}/>", st(acc), st(acc)));
            front.push_str(&format!("<ellipse cx=\"50\" cy=\"91\" rx=\"24\" ry=\"4\" {line}/><ellipse cx=\"50\" cy=\"91\" rx=\"34\" ry=\"6\" {line} stroke-dasharray=\"6 5\"/>"));
        }
        "reed" => {
            behind.push_str(&format!("<path d=\"M84 94 L82 28\" {line} stroke-width=\"2.6\"/><path d=\"M83 74 Q72 56 62 54\" {line}/>"));
            behind.push_str(&format!("<rect x=\"78.5\" y=\"14\" width=\"7\" height=\"19\" rx=\"3.5\" {}/>", st(acc)));
            front.push_str(&format!("<path d=\"M20 70 q6 -5 12 0 q6 5 12 0\" {line}/><path d=\"M56 76 q6 -5 12 0 q6 5 12 0\" {line}/>"));
        }
        "lotus" => {
            for a in [-62.0f32, -31.0, 0.0, 31.0, 62.0] {
                front.push_str(&format!("<ellipse cx=\"50\" cy=\"{y}\" rx=\"5.2\" ry=\"11\" transform=\"rotate({a} 50 {r})\" {}/>", st(acc), y = top - 6.0, r = top + 6.0));
            }
            front.push_str(&format!("<ellipse cx=\"50\" cy=\"90\" rx=\"28\" ry=\"6.5\" {}/>", st(if fill == SILHOUETTE { fill } else { "#8fd09a" })));
        }
        _ => {}
    }
    Parts { behind, front, own_eyes }
}

fn face(behavior: Behavior, own_eyes: bool, friend: bool) -> String {
    let mut s = String::new();
    if !own_eyes {
        // Shy spirits glance sideways; the others look straight at you.
        let dx = if behavior == Behavior::Shy { -1.6 } else { 0.0 };
        for cx in [41.0f32, 59.0] {
            s.push_str(&format!("<ellipse cx=\"{}\" cy=\"47\" rx=\"3.3\" ry=\"4.4\" fill=\"{FACE}\"/>", cx + dx));
            s.push_str(&format!("<circle cx=\"{}\" cy=\"45.4\" r=\"1.2\" fill=\"#fff\"/>", cx + dx + 1.0));
        }
    }
    match behavior {
        Behavior::Curious => s.push_str(&format!("<ellipse cx=\"50\" cy=\"59\" rx=\"2.5\" ry=\"3.1\" fill=\"{FACE}\"/>")),
        Behavior::Playful => s.push_str(&format!(
            "<path d=\"M42 56 Q50 67 58 56 Z\" fill=\"{FACE}\" stroke=\"{FACE}\" stroke-width=\"1.4\" stroke-linejoin=\"round\"/>"
        )),
        Behavior::Shy => {
            s.push_str(&format!("<path d=\"M45 59 Q50 56 55 59\" fill=\"none\" stroke=\"{FACE}\" stroke-width=\"2\" stroke-linecap=\"round\"/>"));
            s.push_str("<path d=\"M68 34 c2.6 3.4 3.4 5.4 0 7.6 c-3.4 -2.2 -2.6 -4.2 0 -7.6 Z\" fill=\"#9ad3f0\" stroke=\"#5a8fb0\" stroke-width=\"1.2\"/>");
        }
    }
    if friend {
        s.push_str("<circle cx=\"34\" cy=\"55\" r=\"4\" fill=\"#ff8fa3\" fill-opacity=\"0.55\"/><circle cx=\"66\" cy=\"55\" r=\"4\" fill=\"#ff8fa3\" fill-opacity=\"0.55\"/>");
    }
    s
}

fn sparkle(x: f32, y: f32, r: f32, fill: &str) -> String {
    format!(
        "<path d=\"M{x} {a} L{b} {y} L{x} {c} L{d} {y} Z\" fill=\"{fill}\" stroke=\"{INK}\" stroke-width=\"1\" stroke-linejoin=\"round\"/>",
        a = y - r,
        b = x + r * 0.55,
        c = y + r,
        d = x - r * 0.55
    )
}

fn weather_doodle(def: &SpiritDef, x: f32, y: f32) -> String {
    let line = format!("fill=\"none\" stroke=\"{INK}\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"");
    match def.weather {
        [Weather::Clear] => {
            let mut s = format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"4.2\" fill=\"#ffd35a\" stroke=\"{INK}\" stroke-width=\"1.4\"/>");
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                s.push_str(&format!(
                    "<path d=\"M{} {} L{} {}\" {line}/>",
                    x + a.cos() * 6.4,
                    y + a.sin() * 6.4,
                    x + a.cos() * 8.6,
                    y + a.sin() * 8.6
                ));
            }
            s
        }
        [Weather::Fog] => format!(
            "<path d=\"M{a} {y1} q3 -3 6 0 t6 0 t6 0\" {line}/><path d=\"M{a} {y2} q3 -3 6 0 t6 0 t6 0\" {line}/><path d=\"M{a} {y3} q3 -3 6 0 t6 0 t6 0\" {line}/>",
            a = x - 9.0,
            y1 = y - 5.0,
            y2 = y,
            y3 = y + 5.0
        ),
        [Weather::Rain] => format!(
            "<path d=\"M{a} {b} q-1 -7 6 -6 q3 -6 9 -1 q5 0 4 7 Z\" fill=\"#dfe9f2\" stroke=\"{INK}\" stroke-width=\"1.4\" stroke-linejoin=\"round\"/>\
             <path d=\"M{c} {d} l-1.5 4\" {line}/><path d=\"M{e} {d} l-1.5 4\" {line}/><path d=\"M{f} {d} l-1.5 4\" {line}/>",
            a = x - 9.0,
            b = y + 3.0,
            c = x - 5.0,
            d = y + 6.0,
            e = x,
            f = x + 5.0
        ),
        // Any weather at all: a little star-flower.
        _ => sparkle(x, y, 7.0, "#ffe9a8"),
    }
}

fn time_doodle(def: &SpiritDef, x: f32, y: f32) -> String {
    let (from, to) = def.hours;
    if (from, to) == (0.0, 24.0) {
        return sparkle(x, y, 6.0, "#ffe9a8");
    }
    // After dusk (or before dawn) counts as night.
    let night = from >= 17.0 || to <= 6.0;
    if night {
        format!(
            "<path d=\"M{a} {b} A8 8 0 1 0 {c} {d} A6.4 6.4 0 1 1 {a} {b} Z\" fill=\"#ffe9a8\" stroke=\"{INK}\" stroke-width=\"1.4\" stroke-linejoin=\"round\"/>\
             <circle cx=\"{sx}\" cy=\"{sy}\" r=\"1\" fill=\"{INK}\"/>",
            a = x + 2.0,
            b = y - 8.0,
            c = x + 6.0,
            d = y + 4.0,
            sx = x + 8.0,
            sy = y - 6.0
        )
    } else {
        format!(
            "<path d=\"M{a} {b} A8 8 0 0 1 {c} {b} Z\" fill=\"#ffd35a\" stroke=\"{INK}\" stroke-width=\"1.4\" stroke-linejoin=\"round\"/>\
             <path d=\"M{a2} {b} L{c2} {b}\" stroke=\"{INK}\" stroke-width=\"1.6\" stroke-linecap=\"round\"/>",
            a = x - 8.0,
            b = y + 4.0,
            c = x + 8.0,
            a2 = x - 11.0,
            c2 = x + 11.0
        )
    }
}

pub fn draw(def: usize, l: Layers) -> String {
    if !l.seen {
        return unseen();
    }
    let d = &SPIRITS[def];
    let (sx, sy) = proportions(d.id);
    let top = 82.0 - 66.0 * sy;
    let acc = accent(d.id);

    // Silhouette (only seen) -> pencil outline (habits) -> colour (friend).
    let drawn = l.habits || l.friend;
    let body_fill = if l.friend {
        hex(d.color)
    } else if l.habits {
        PAPER.to_string()
    } else {
        SILHOUETTE.to_string()
    };
    let edge = if drawn {
        format!("stroke=\"{INK}\" stroke-width=\"2.2\" stroke-linejoin=\"round\" stroke-linecap=\"round\"")
    } else {
        String::new()
    };
    let deco_fill = if l.friend {
        acc.to_string()
    } else if l.habits {
        PAPER.to_string()
    } else {
        SILHOUETTE.to_string()
    };
    let p = parts(d.id, top, &deco_fill, &edge, if l.friend { acc } else { &deco_fill });

    let mut s = String::from("<svg class=\"sk\" viewBox=\"0 0 100 100\" xmlns=\"http://www.w3.org/2000/svg\">");
    if l.friend {
        s.push_str(&format!("<circle cx=\"50\" cy=\"54\" r=\"46\" fill=\"{}\" fill-opacity=\"0.13\"/>", hex(d.color)));
    }
    // Soft ground shadow.
    s.push_str(&format!("<ellipse cx=\"50\" cy=\"90\" rx=\"24\" ry=\"3.4\" fill=\"{INK}\" fill-opacity=\"{}\"/>", if drawn { 0.16 } else { 0.28 }));
    s.push_str(&p.behind);
    s.push_str(&format!(
        "<g transform=\"translate(50 82) scale({sx} {sy}) translate(-50 -82)\"><path d=\"M24 82 L24 50 C24 28 38 16 50 16 C62 16 76 28 76 50 L76 82 q-6.5 9 -13 0 q-6.5 9 -13 0 q-6.5 9 -13 0 q-6.5 9 -13 0 Z\" fill=\"{body_fill}\" {edge}/></g>"
    ));
    if l.friend {
        // A little body shading so colour pages feel inked, not flat.
        s.push_str(&format!(
            "<path d=\"M{} 58 C{} 76 {} 80 {} 82\" fill=\"none\" stroke=\"#ffffff\" stroke-opacity=\"0.35\" stroke-width=\"3\" stroke-linecap=\"round\"/>",
            50.0 - 21.0 * sx,
            50.0 - 21.0 * sx,
            50.0 - 21.0 * sx,
            50.0 - 21.0 * sx
        ));
    }
    s.push_str(&p.front);
    if drawn {
        s.push_str(&face(d.behavior, p.own_eyes, l.friend));
    }
    if l.likes {
        // The favorite treat, doodled in the corner with a little steam.
        let tc = hex(TREATS[d.favorite].color);
        s.push_str(&format!(
            "<circle cx=\"84\" cy=\"82\" r=\"7.5\" fill=\"{tc}\" stroke=\"{INK}\" stroke-width=\"1.8\"/>\
             <path d=\"M80 79 q2 -3 5 -2\" fill=\"none\" stroke=\"#fff\" stroke-opacity=\"0.8\" stroke-width=\"1.6\" stroke-linecap=\"round\"/>\
             <path d=\"M80 70 q-2 -3 0 -6 M85 70 q-2 -3 0 -6\" fill=\"none\" stroke=\"{INK}\" stroke-opacity=\"0.5\" stroke-width=\"1.3\" stroke-linecap=\"round\"/>"
        ));
    }
    if l.when {
        s.push_str(&weather_doodle(d, 14.0, 13.0));
        s.push_str(&time_doodle(d, 87.0, 14.0));
    }
    if l.friend {
        s.push_str(&sparkle(16.0, 44.0, 6.0, "#fff1a8"));
        s.push_str(&sparkle(86.0, 52.0, 4.6, "#fff1a8"));
    }
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_layers() -> Vec<Layers> {
        let mut v = Vec::new();
        for bits in 0..32u32 {
            v.push(Layers {
                seen: bits & 1 != 0,
                habits: bits & 2 != 0,
                likes: bits & 4 != 0,
                when: bits & 8 != 0,
                friend: bits & 16 != 0,
            });
        }
        v
    }

    fn well_formed(svg: &str) -> bool {
        // Cheap structural check: balanced angle brackets and quotes, one root <svg>.
        svg.starts_with("<svg")
            && svg.ends_with("</svg>")
            && svg.matches('<').count() == svg.matches('>').count()
            && svg.matches('"').count() % 2 == 0
            && svg.matches("<svg").count() == 1
            && svg.matches("<g ").count() == svg.matches("</g>").count()
            && !svg.contains("NaN")
            && !svg.contains("inf")
    }

    #[test]
    fn every_spirit_at_every_stage_is_a_well_formed_drawing() {
        for def in 0..N_SPIRITS {
            for l in all_layers() {
                let svg = draw(def, l);
                assert!(well_formed(&svg), "{} {:?}: {}", SPIRITS[def].id, l, svg);
            }
        }
    }

    #[test]
    fn pages_fill_in_as_you_learn() {
        let def = 1; // Mizzle
        let none = Layers { seen: false, habits: false, likes: false, when: false, friend: false };
        assert!(draw(def, none).contains(">?<"), "unseen spirits are a question mark");

        let seen = Layers { seen: true, ..none };
        let silhouette = draw(def, seen);
        assert!(silhouette.contains(SILHOUETTE) && !silhouette.contains(FACE), "silhouette has no face");

        let habits = Layers { habits: true, ..seen };
        let sketch = draw(def, habits);
        assert!(sketch.contains(PAPER) && sketch.contains(FACE), "pencil outline gets a face");
        assert!(!sketch.contains(&hex(SPIRITS[def].color)), "still no colour");

        let likes = Layers { likes: true, ..habits };
        assert!(draw(def, likes).contains(&hex(TREATS[SPIRITS[def].favorite].color)), "favorite treat doodle");
        assert!(draw(def, likes).len() > sketch.len());

        let when = Layers { when: true, ..likes };
        assert!(draw(def, when).len() > draw(def, likes).len(), "weather + time doodles add to the page");

        let friend = Layers { friend: true, ..when };
        assert!(draw(def, friend).contains(&hex(SPIRITS[def].color)), "colour comes with friendship");
    }

    #[test]
    fn every_species_looks_different() {
        let l = Layers { seen: true, habits: true, likes: false, when: false, friend: true };
        let mut seen = std::collections::HashSet::new();
        for def in 0..N_SPIRITS {
            assert!(seen.insert(draw(def, l)), "{} duplicates another drawing", SPIRITS[def].id);
        }
    }

    #[test]
    fn weather_and_time_doodles_follow_the_data() {
        // Mizzle likes rain; Bobbin likes sunny days; Hush likes fog: each draws something different.
        let rain = weather_doodle(&SPIRITS[1], 14.0, 13.0);
        let clear = weather_doodle(&SPIRITS[8], 14.0, 13.0);
        let fog = weather_doodle(&SPIRITS[2], 14.0, 13.0);
        assert!(rain != clear && clear != fog && rain != fog);
        // Night spirits get a moon, day spirits a sun-on-the-horizon.
        let pip = time_doodle(&SPIRITS[0], 87.0, 14.0);
        let bobbin = time_doodle(&SPIRITS[8], 87.0, 14.0);
        assert!(pip != bobbin);
        assert!(pip.contains("A8 8 0 1 0") && bobbin.contains("A8 8 0 0 1"));
    }
}
