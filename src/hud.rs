//! HUD content as HTML strings. The browser layer (web.rs) just assigns them to DOM elements,
//! only touching the DOM when a string actually changed. Keeping this pure makes it testable
//! and lets us restyle the whole UI from CSS (or Claude Design mockups) without touching Rust.

use crate::data::*;
use crate::game::*;

fn hex(c: [f32; 3]) -> String {
    let f = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", f(c[0]), f(c[1]), f(c[2]))
}

fn dot(c: [f32; 3]) -> String {
    let h = hex(c);
    format!("<i class=\"dot\" style=\"color:{h};background:{h}\"></i>")
}

pub fn build(g: &Game) -> Vec<(&'static str, String)> {
    vec![
        ("hud-clock", clock(g)),
        ("hud-candles", format!("🕯️ {}", g.save.candles)),
        ("hud-followers", followers(g)),
        ("hud-toasts", g.toasts.iter().map(|t| format!("<div>{}</div>", t.text)).collect()),
        ("hud-prompt", prompt(g)),
        ("hud-hotbar", hotbar(g)),
        ("hud-shop", shop(g)),
        ("hud-journal", if g.show_journal { journal(g) } else { String::new() }),
        ("hud-debug", if g.show_debug { debug(g) } else { String::new() }),
    ]
}

fn clock(g: &Game) -> String {
    let forecast = match g.env.forecast() {
        Some(w) => format!(" · <span class=\"fc\">{} {} approaching</span>", w.icon(), w.name()),
        None => String::new(),
    };
    format!(
        "<b>{}</b>{} · {} {}<br><small>Day {}{}</small>",
        fmt_hour(g.env.hour),
        g.env.phase_name(),
        g.env.weather.icon(),
        g.env.weather.name(),
        g.env.day,
        forecast
    )
}

fn followers(g: &Game) -> String {
    let f = g.followers();
    if f.is_empty() {
        return String::new();
    }
    let mut s = String::from("Following you<br>");
    for sp in f {
        let d = &SPIRITS[sp.def];
        s.push_str(&format!("<span class=\"f\">{} {}</span>", dot(d.color), d.name));
    }
    s
}

fn hotbar(g: &Game) -> String {
    let cap = g.treat_cap();
    let mut s = String::new();
    for (i, t) in TREATS.iter().enumerate() {
        let sel = if i == g.selected { " sel" } else { "" };
        s.push_str(&format!(
            "<div class=\"slot{sel}\">{} {} <b>{}/{}</b></div>",
            dot(t.color),
            t.name,
            g.save.treats[i],
            cap
        ));
    }
    s
}

fn prompt(g: &Game) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(h) = g.interaction_hint() {
        lines.push(h);
    }
    if let Some(sp) = g.nearby_spirit() {
        let d = &SPIRITS[sp.def];
        lines.push(format!("<b>{}</b><br><small>{}</small>", d.name, d.hint));
    }
    if g.time < 40.0 && lines.is_empty() {
        lines.push(
            "<small>Gather treats from the glowing plants (<kbd>E</kbd>), set one down (<kbd>Space</kbd>) \
             and wait nearby for hungry spirits.</small>"
                .to_string(),
        );
    }
    let n = g.followers().len();
    if n > 0 && !g.near_home() {
        lines.push(format!("<small>{n} friend{} following. Bring them to the cottage!</small>", if n == 1 { "" } else { "s" }));
    }
    lines.join("<br>")
}

fn shop(g: &Game) -> String {
    if !g.near_home() {
        return String::new();
    }
    let mut s = String::from("<h3>Cottage workshop</h3>");
    for (i, up) in UPGRADES.iter().enumerate() {
        let lvl = g.save.upgrades[i] as usize;
        let price = match up.costs.get(lvl) {
            Some(c) => format!("<b>{c} 🕯️</b>"),
            None => "<b>MAX</b>".to_string(),
        };
        s.push_str(&format!(
            "<div class=\"up\"><kbd>{}</kbd><span>{} <small>Lv {}/{} · {}</small></span>{}</div>",
            i + 1,
            up.name,
            lvl,
            up.costs.len(),
            up.desc,
            price
        ));
    }
    s
}

fn journal(g: &Game) -> String {
    let mut s = String::from("<h3>Spirit journal <small>(J to close)</small></h3>");
    for (i, d) in SPIRITS.iter().enumerate() {
        let body = if g.save.discovered[i] {
            format!(
                "<b>{}</b> ×{}<small>Likes {} · Appears: {}</small><small>{}</small>",
                d.name,
                g.save.residents[i],
                TREATS[d.favorite].name,
                d.conditions_text(),
                if d.needs_puzzle { "Wakes only when the glade's three lanterns are lit." } else { d.hint }
            )
        } else if g.save.seen[i] {
            format!("<b>{}</b><small>{}</small><small>Befriend it to learn more.</small>", d.name, d.hint)
        } else {
            "<b>???</b><small>Not yet seen.</small>".to_string()
        };
        s.push_str(&format!("<div class=\"entry\">{}{}</div>", dot(d.color), body));
    }
    s
}

fn debug(g: &Game) -> String {
    format!(
        "fps {:.0}\nhour {:.2}  day {}\nweather {} (spirits feel {})\nspirits {}  drops {}\nplayer {:.1}, {:.1}\n\nF2 weather · [ ] hour −/+1",
        g.fps,
        g.env.hour,
        g.env.day,
        g.env.weather.name(),
        g.env.effective().name(),
        g.spirits.len(),
        g.drops.len(),
        g.player.pos.x,
        g.player.pos.z
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_builds_for_every_panel_state() {
        let mut g = Game::new(Save::default(), 1);
        g.show_journal = true;
        g.show_debug = true;
        g.save.seen = [true, true, false, false];
        g.save.discovered = [true, false, false, false];
        g.player.pos = g.world.home;
        let h = build(&g);
        let get = |k: &str| h.iter().find(|(id, _)| *id == k).unwrap().1.clone();
        assert!(get("hud-shop").contains("Brighter Lantern"));
        assert!(get("hud-journal").contains("Pumpkin Pip") && get("hud-journal").contains("???"));
        assert!(get("hud-hotbar").contains("Candy Corn"));
        assert!(get("hud-clock").contains("18:30"));
        assert!(!get("hud-clock").contains("approaching"));
        assert!(!get("hud-debug").is_empty());
    }
}
