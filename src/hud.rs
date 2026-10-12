//! HUD content as HTML strings. The browser layer (web.rs) just assigns them to DOM elements,
//! only touching the DOM when a string actually changed. Keeping this pure makes it testable
//! and lets us restyle the whole UI from CSS (or Claude Design mockups) without touching Rust.

use crate::data::*;
use crate::game::*;
use crate::sketch;
use crate::world::Area;

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
        ("hud-book", book_badge(g)),
        ("hud-sense", sense(g)),
        ("hud-fps", if g.settings.show_fps { format!("{:.0} fps", g.fps) } else { String::new() }),
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
        "<b>{}</b>{} · {} {}<br><small>{} · Day {}{}</small>",
        fmt_hour(g.env.hour),
        g.env.phase_name(),
        g.env.weather.icon(),
        g.env.weather.name(),
        g.area.name(),
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
    if g.travel.is_some() {
        return String::new();
    }
    if let Some(h) = g.interaction_hint() {
        lines.push(h);
    }
    if let Some(sp) = g.nearby_spirit() {
        let d = &SPIRITS[sp.def];
        lines.push(format!("<b>{}</b><br><small>{}</small>", d.name, d.hint));
    }
    if g.settings.hints && g.time < 40.0 && lines.is_empty() {
        lines.push(if g.area == Area::Hollow {
            "<small>Soul Cake grows here. Some spirits only come out in the right weather, and the crypt hums a tune...</small>"
                .to_string()
        } else if g.area == Area::Pond {
            "<small>Lily Jam grows here. Spirits keep daytime hours by the water, and the lamps by the shed look like a puzzle...</small>"
                .to_string()
        } else {
            "<small>Gather treats from the glowing plants (<kbd>E</kbd>), set one down (<kbd>Space</kbd>) \
             and wait nearby for hungry spirits. A soft chime means one is close.</small>"
                .to_string()
        });
    }
    let n = g.followers().len();
    if g.settings.hints && n > 0 && !g.near_home() {
        let way = if g.area == Area::Hollow { "Bring them back through the gate to the cottage!" } else { "Bring them to the cottage!" };
        lines.push(format!("<small>{n} friend{} following. {way}</small>", if n == 1 { "" } else { "s" }));
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

/// "7/60" and a nudge when there is something new to read.
fn book_badge(g: &Game) -> String {
    let (got, all) = g.save.journal_progress();
    if g.journal_unread > 0 && !g.show_journal {
        format!("<span class=\"newnote\">📖 {got}/{all} · new note! <kbd>J</kbd></span>")
    } else {
        format!("📖 {got}/{all}")
    }
}

const ARROWS: [&str; 8] = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
const COMPASS: [&str; 8] = ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"];

/// Which of 8 directions on screen (up the screen = north) a unit vector points to.
pub fn compass_index(dir: crate::math::Vec3) -> usize {
    let angle = dir.x.atan2(-dir.z); // 0 = up, clockwise
    ((angle / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize
}

/// The compass hint for a spirit you can sense but not yet see.
fn sense(g: &Game) -> String {
    let Some(sn) = g.sense else { return String::new() };
    if !g.settings.shimmer || g.show_journal || g.travel.is_some() || sn.dist < 8.0 {
        return String::new();
    }
    let i = compass_index(sn.dir);
    let adj = if !sn.fresh {
        "A familiar shimmer"
    } else if sn.strength < 0.33 {
        "A faint shimmer"
    } else if sn.strength < 0.66 {
        "A shimmer"
    } else {
        "A bright shimmer"
    };
    format!("<span class=\"arrow\">{}</span> {} {} to the {}", ARROWS[i], dot(SPIRITS[sn.def].color), adj, COMPASS[i])
}

fn stars(n: usize) -> String {
    format!("{}{}", "★".repeat(n), "☆".repeat(N_CLUES - n))
}

fn behavior_note(b: Behavior) -> &'static str {
    match b {
        Behavior::Curious => "Curious: drifts over to take a look at you.",
        Behavior::Playful => "Playful: zig-zags about, quick on its feet.",
        Behavior::Shy => "Shy: bolts if you rush it. Stand still, a little way off.",
    }
}

fn note(known: bool, got: String, todo: &str) -> String {
    if known {
        format!("<li class=\"got\">{got}</li>")
    } else {
        format!("<li class=\"todo\">{todo}</li>")
    }
}

/// One spirit's page: its sketch, a row of stars and the notes you have written so far.
fn card(g: &Game, i: usize) -> String {
    let d = &SPIRITS[i];
    let sv = &g.save;
    if !sv.knows(i, Clue::Seen) {
        return format!(
            "<div class=\"card unseen\"><div class=\"sketch\">{}</div><b class=\"nm\">???</b><div class=\"stars\">{}</div>\
             <ul class=\"notes\"><li class=\"todo\">Not seen yet. Listen for soft chimes and look for a shimmer.</li></ul></div>",
            sketch::sketch(i, sv),
            stars(0)
        );
    }
    let mut notes = String::new();
    notes.push_str(&format!("<li class=\"got hint\">{}</li>", d.hint));
    notes.push_str(&note(sv.knows(i, Clue::Habits), behavior_note(d.behavior).to_string(), "Watch it for a while to see how it behaves."));
    notes.push_str(&note(
        sv.knows(i, Clue::Likes),
        format!("Favorite: {} {}", dot(TREATS[d.favorite].color), TREATS[d.favorite].name),
        "Put a treat out and see what it comes running for.",
    ));
    notes.push_str(&note(sv.knows(i, Clue::When), format!("Appears: {}", d.conditions_text()), "Watch it drift away to learn when it comes out."));
    let home = if sv.knows(i, Clue::Friend) {
        let mut t = format!("At your cottage ×{} · +{} 🕯️", sv.residents[i], d.reward);
        if d.needs_puzzle {
            t.push_str(match d.area {
                0 => "<br>Wakes only when the glade's three lanterns are lit.",
                1 => "<br>Wakes only when the crypt's melody is repeated on the rune stones.",
                _ => "<br>Rises in daylight, once all four lily lamps by the shed glow.",
            });
        }
        if d.unique {
            t.push_str("<br><em>One of a kind: once it lives at your cottage, it never returns.</em>");
        }
        t
    } else {
        String::new()
    };
    notes.push_str(&note(sv.knows(i, Clue::Friend), home, "Win it over and bring it home."));
    let done = if sv.pages(i) == N_CLUES { " complete" } else { "" };
    format!(
        "<div class=\"card{done}\"><div class=\"sketch\">{}</div><b class=\"nm\">{}</b><div class=\"stars\">{}</div><ul class=\"notes\">{notes}</ul></div>",
        sketch::sketch(i, sv),
        d.name,
        stars(sv.pages(i))
    )
}

fn journal(g: &Game) -> String {
    let (got, all) = g.save.journal_progress();
    let page = g.journal_page.min(N_AREAS - 1);
    let mut tabs = String::new();
    for (a, area) in Area::ALL.iter().enumerate() {
        let (mut n, mut tot) = (0, 0);
        for (i, d) in SPIRITS.iter().enumerate() {
            if d.area == a {
                n += g.save.pages(i);
                tot += N_CLUES;
            }
        }
        let on = if a == page { " on" } else { "" };
        tabs.push_str(&format!("<span class=\"tab{on}\">{} <small>{n}/{tot}</small></span>", area.name()));
    }
    let mut cards = String::new();
    for i in 0..N_SPIRITS {
        if SPIRITS[i].area == page {
            cards.push_str(&card(g, i));
        }
    }
    format!(
        "<div class=\"book-head\"><h3>Spirit journal</h3><span class=\"prog\">{got} / {all} notes</span>\
         <span class=\"keys\"><kbd>A</kbd> <kbd>D</kbd> turn the page · <kbd>J</kbd> close</span></div>\
         <div class=\"tabs\">{tabs}</div><div class=\"cards\">{cards}</div>"
    )
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
    use crate::math::Vec3;

    #[test]
    fn hud_builds_for_every_panel_state() {
        let mut g = Game::new(Save::default(), 1);
        g.show_journal = true;
        g.show_debug = true;
        g.save.seen = [true, true, false, false, true, false, false, false, false, false, false, false];
        g.save.discovered = [true, false, false, false, true, false, false, false, false, false, false, false];
        g.player.pos = g.world.home;
        let h = build(&g);
        let get = |k: &str| h.iter().find(|(id, _)| *id == k).unwrap().1.clone();
        assert!(get("hud-shop").contains("Brighter Lantern"));
        assert!(get("hud-journal").contains("Pumpkin Pip") && get("hud-journal").contains("???"));
        assert!(get("hud-book").contains("📖"));
        assert!(get("hud-hotbar").contains("Candy Corn"));
        assert!(get("hud-clock").contains("18:30"));
        assert!(!get("hud-clock").contains("approaching"));
        assert!(!get("hud-debug").is_empty());
        assert!(get("hud-shop").contains("Gate Key") && get("hud-journal").contains("Graveyard Hollow"));
        assert!(get("hud-clock").contains("Hollow Glade"));
        assert!(get("hud-hotbar").contains("Soul Cake"));
    }

    #[test]
    fn hollow_hud_has_no_shop_and_shows_gate_and_rune_prompts() {
        let mut g = Game::new(Save::default(), 2);
        g.save.upgrades[GATE_KEY] = 1;
        g.enter_area(Area::Hollow);
        let get = |g: &Game, k: &str| build(g).iter().find(|(id, _)| *id == k).unwrap().1.clone();
        assert!(get(&g, "hud-clock").contains("Graveyard Hollow"));
        assert!(get(&g, "hud-shop").is_empty());
        g.player.pos = g.world.gate_to(Area::Glade).pos + Vec3::new(0.0, 0.0, -2.0);
        assert!(get(&g, "hud-prompt").contains("Hollow Glade"), "{}", get(&g, "hud-prompt"));
        g.player.pos = g.crypt_door();
        assert!(get(&g, "hud-prompt").contains("Listen to the crypt"));
        g.player.pos = g.world.runes[1] + Vec3::new(0.0, 0.0, 1.5);
        assert!(get(&g, "hud-prompt").contains("Touch the stone"));
        g.save.seen = [true; N_SPIRITS];
        g.save.discovered = [true; N_SPIRITS];
        g.show_journal = true;
        g.journal_page = 1;
        assert!(get(&g, "hud-journal").contains("Old Marrow") && get(&g, "hud-journal").contains("One of a kind"));
        assert!(!get(&g, "hud-journal").contains("Lady Lotus"), "pages show one area at a time");
        g.act(Action::JournalPage(1));
        assert!(get(&g, "hud-journal").contains("Willow Pond") && get(&g, "hud-journal").contains("Lady Lotus"));
    }

    #[test]
    fn pond_hud_shows_lamp_prompt_and_both_glade_gates() {
        let mut g = Game::new(Save::default(), 3);
        g.save.upgrades[POND_KEY] = 1;
        let get = |g: &Game, k: &str| build(g).iter().find(|(id, _)| *id == k).unwrap().1.clone();
        g.player.pos = g.world.gate_to(Area::Pond).pos + Vec3::new(0.0, 0.0, -2.0);
        assert!(get(&g, "hud-prompt").contains("Willow Pond"));
        g.player.pos = g.world.gate_to(Area::Hollow).pos + Vec3::new(0.0, 0.0, 2.0);
        assert!(get(&g, "hud-prompt").contains("Gate Key"));
        g.player.pos = g.world.home;
        assert!(get(&g, "hud-shop").contains("Frog Charm"));
        g.enter_area(Area::Pond);
        assert!(get(&g, "hud-clock").contains("Willow Pond"));
        g.player.pos = g.world.lamps[2] + Vec3::new(0.0, 0.0, 1.5);
        assert!(get(&g, "hud-prompt").contains("lamp 3"), "{}", get(&g, "hud-prompt"));
    }

    #[test]
    fn compass_hint_points_at_the_shimmer_and_respects_the_options() {
        use crate::math::Vec3;
        let mut g = Game::new(Save::default(), 4);
        let get = |g: &Game, k: &str| build(g).iter().find(|(id, _)| *id == k).unwrap().1.clone();
        assert!(get(&g, "hud-sense").is_empty(), "nothing nearby");
        let sn = |dir: Vec3, dist: f32, fresh: bool| Sense { def: 1, dist, dir, strength: 0.5, fresh };
        g.sense = Some(sn(Vec3::new(0.0, 0.0, -1.0), 14.0, true));
        let up = get(&g, "hud-sense");
        assert!(up.contains("north") && !up.contains("north-"), "{up}");
        assert!(up.contains("↑") && up.contains("A shimmer"));
        g.sense = Some(sn(Vec3::new(1.0, 0.0, 1.0).normalized(), 14.0, true));
        assert!(get(&g, "hud-sense").contains("south-east") && get(&g, "hud-sense").contains("↘"));
        g.sense = Some(sn(Vec3::new(-1.0, 0.0, 0.0), 14.0, false));
        assert!(get(&g, "hud-sense").contains("familiar") && get(&g, "hud-sense").contains("west"));
        // Close enough to see: the "who's that?" panel takes over.
        g.sense = Some(sn(Vec3::new(-1.0, 0.0, 0.0), 5.0, true));
        assert!(get(&g, "hud-sense").is_empty());
        // Options.
        g.sense = Some(sn(Vec3::new(-1.0, 0.0, 0.0), 14.0, true));
        g.settings.shimmer = false;
        assert!(get(&g, "hud-sense").is_empty());
        for (dir, want) in [((0.0, -1.0), 0), ((1.0, -1.0), 1), ((1.0, 0.0), 2), ((0.0, 1.0), 4), ((-1.0, 1.0), 5), ((-1.0, -1.0), 7)] {
            assert_eq!(compass_index(Vec3::new(dir.0, 0.0, dir.1).normalized()), want);
        }
    }

    #[test]
    fn hint_lines_can_be_turned_off_but_interaction_prompts_stay() {
        let mut g = Game::new(Save::default(), 5);
        let get = |g: &Game, k: &str| build(g).iter().find(|(id, _)| *id == k).unwrap().1.clone();
        g.player.pos = g.world.home + Vec3::new(30.0, 0.0, 30.0);
        assert!(get(&g, "hud-prompt").contains("Gather treats"), "tutorial text shows early on");
        g.settings.hints = false;
        assert!(get(&g, "hud-prompt").is_empty());
        if let Some(src) = g.sources.first() {
            g.player.pos = src.pos;
            assert!(get(&g, "hud-prompt").contains("Gather"), "E prompts are not hints");
        }
        g.settings.show_fps = true;
        assert!(get(&g, "hud-fps").contains("fps"));
        g.settings.show_fps = false;
        assert!(get(&g, "hud-fps").is_empty());
    }

    #[test]
    fn journal_pages_show_what_you_know_and_hint_at_the_rest() {
        let mut g = Game::new(Save::default(), 6);
        let page = |g: &Game| build(g).iter().find(|(id, _)| *id == "hud-journal").unwrap().1.clone();
        g.show_journal = true;
        let blank = page(&g);
        assert!(blank.contains("???") && blank.contains("0 / 60 notes") && blank.contains("<svg"), "four '?' boxes to start");
        assert_eq!(blank.matches("class=\"card unseen\"").count(), 4);

        g.save.learn(1, Clue::Seen);
        let seen = page(&g);
        assert!(seen.contains("Mizzle") && seen.contains("Watch it for a while") && seen.contains("Put a treat out"));
        assert!(!seen.contains("Favorite:"));

        g.save.learn(1, Clue::Likes);
        g.save.learn(1, Clue::Habits);
        g.save.learn(1, Clue::When);
        let most = page(&g);
        assert!(most.contains("Favorite:") && most.contains("Moon Milk") && most.contains("Playful") && most.contains("Appears: Rain"));
        assert!(most.contains("Win it over and bring it home"));
        assert!(!most.contains("complete"));
        assert!(most.contains("★★★★☆"));

        g.save.residents[1] = 2;
        g.save.learn(1, Clue::Friend);
        let done = page(&g);
        assert!(done.contains("At your cottage ×2") && done.contains("★★★★★") && done.contains("card complete"));

        // The badge nudges you toward new notes until you open the book.
        g.show_journal = false;
        g.journal_unread = 2;
        let badge = build(&g).iter().find(|(id, _)| *id == "hud-book").unwrap().1.clone();
        assert!(badge.contains("new note") && badge.contains("/60"));
        g.journal_unread = 0;
        let badge = build(&g).iter().find(|(id, _)| *id == "hud-book").unwrap().1.clone();
        assert!(!badge.contains("new note"));
    }

    /// Writes the HUD panels for a few journal states to $DUMP_DIR so they can be screenshotted in a real
    /// browser: `DUMP_DIR=/some/dir cargo test dump_journal -- --ignored`.
    #[test]
    #[ignore]
    fn dump_journal_html() {
        let dir = std::env::var("DUMP_DIR").expect("set DUMP_DIR");
        let mut g = Game::new(Save::default(), 7);
        g.show_journal = true;
        // A mid-game save: a bit of everything.
        let sv = &mut g.save;
        for i in 0..N_SPIRITS {
            sv.seen[i] = i != 3 && i != 7 && i != 11;
        }
        sv.learn(0, Clue::Friend);
        sv.residents[0] = 3;
        sv.learn(1, Clue::Habits);
        sv.learn(1, Clue::Likes);
        sv.learn(2, Clue::Habits);
        sv.learn(2, Clue::When);
        sv.learn(4, Clue::Friend);
        sv.residents[4] = 1;
        sv.learn(5, Clue::Habits);
        sv.learn(5, Clue::Likes);
        sv.learn(5, Clue::When);
        sv.learn(6, Clue::Habits);
        sv.learn(8, Clue::Friend);
        sv.residents[8] = 2;
        sv.learn(9, Clue::Likes);
        sv.learn(10, Clue::Habits);
        sv.learn(10, Clue::Likes);
        sv.learn(10, Clue::When);
        for page in 0..3 {
            g.journal_page = page;
            let h = build(&g);
            let get = |k: &str| h.iter().find(|(id, _)| *id == k).unwrap().1.clone();
            std::fs::write(format!("{dir}/journal_{page}.html"), get("hud-journal")).unwrap();
        }
        // Everything learned, to see the finished look (and the long Marrow / Lotus notes).
        for i in 0..N_SPIRITS {
            g.save.learn(i, Clue::Friend);
            g.save.residents[i] = 1;
        }
        g.journal_page = 1;
        let h = build(&g);
        std::fs::write(format!("{dir}/journal_full.html"), h.iter().find(|(id, _)| *id == "hud-journal").unwrap().1.clone()).unwrap();
    }
}
