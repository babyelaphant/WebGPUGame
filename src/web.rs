//! Browser glue: canvas, keyboard, requestAnimationFrame loop (fixed 60 Hz simulation),
//! HUD DOM updates, and localStorage saves.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::data::Save;
use crate::game::{Action, Controls, Game, STEP};
use crate::gfx::Renderer;
use crate::math::Vec3;
use crate::mesh::Mesh;
use crate::settings::Settings;
use crate::{audio, hud, scene};

const SAVE_KEY: &str = "spirit-collector-save-v1";

#[derive(Default)]
struct InputState {
    down: HashSet<String>,
    pressed: Vec<String>,
}

pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("{info}");
        web_sys::console::error_1(&JsValue::from_str(&msg));
        show_error(&format!("The game crashed: {msg}"));
    }));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run().await {
            web_sys::console::error_1(&JsValue::from_str(&e));
            show_error(&e);
        }
    });
}

fn show_error(msg: &str) {
    if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("error")) {
        el.set_text_content(Some(msg));
        let _ = el.set_attribute("style", "display:block");
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn load_save() -> Save {
    storage()
        .and_then(|s| s.get_item(SAVE_KEY).ok().flatten())
        .map(|t| Save::parse(&t))
        .unwrap_or_default()
}

fn write_save(save: &Save) {
    if let Some(s) = storage() {
        let _ = s.set_item(SAVE_KEY, &save.serialize());
    }
}

/// A one-line summary of a save for the title screen's Continue button.
fn save_summary(save: &Save) -> String {
    let friends: u32 = save.residents.iter().sum();
    format!("🕯️ {} · {} friend{}", save.candles, friends, if friends == 1 { "" } else { "s" })
}

fn new_seed() -> u64 {
    (js_sys::Date::now() as u64) ^ 0xC0FFEE
}

fn controls(down: &HashSet<String>) -> Controls {
    let any = |codes: &[&str]| codes.iter().any(|c| down.contains(*c));
    let axis = |neg: &[&str], pos: &[&str]| (any(pos) as i32 - any(neg) as i32) as f32;
    Controls {
        x: axis(&["KeyA", "ArrowLeft"], &["KeyD", "ArrowRight"]),
        z: axis(&["KeyW", "ArrowUp"], &["KeyS", "ArrowDown"]),
        run: any(&["ShiftLeft", "ShiftRight"]),
    }
}

fn map_key(code: &str) -> Option<Action> {
    Some(match code {
        "KeyE" | "Enter" => Action::Interact,
        "Space" => Action::DropTreat,
        "KeyR" | "Tab" => Action::NextTreat,
        "KeyQ" => Action::PrevTreat,
        "Digit1" => Action::Buy(0),
        "Digit2" => Action::Buy(1),
        "Digit3" => Action::Buy(2),
        "Digit4" => Action::Buy(3),
        "Digit5" => Action::Buy(4),
        "KeyJ" => Action::ToggleJournal,
        "F1" => Action::ToggleDebug,
        "F2" => Action::CycleWeather,
        "BracketLeft" => Action::HourShift(-1.0),
        "BracketRight" => Action::HourShift(1.0),
        _ => return None,
    })
}

/// While the journal is open, left/right turn its pages.
fn map_journal_key(code: &str) -> Option<Action> {
    Some(match code {
        "ArrowLeft" | "KeyA" | "KeyQ" => Action::JournalPage(-1),
        "ArrowRight" | "KeyD" | "KeyE" => Action::JournalPage(1),
        _ => return None,
    })
}

/// Thin bridge to assets/audio.js (`window.gameAudio`). If audio isn't available the game
/// just runs silently.
struct AudioBridge {
    obj: JsValue,
    sfx: Option<js_sys::Function>,
    ambience: Option<js_sys::Function>,
}

impl AudioBridge {
    fn new(window: &web_sys::Window) -> AudioBridge {
        let obj = js_sys::Reflect::get(window, &JsValue::from_str("gameAudio")).unwrap_or(JsValue::UNDEFINED);
        let get = |name: &str| {
            js_sys::Reflect::get(&obj, &JsValue::from_str(name))
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        };
        let sfx = get("sfx");
        let ambience = get("ambience");
        AudioBridge { obj, sfx, ambience }
    }

    fn play(&self, name: &str) {
        if let Some(f) = &self.sfx {
            let _ = f.call1(&self.obj, &JsValue::from_str(name));
        }
    }

    fn set_ambience(&self, a: &audio::Ambience) {
        if let Some(f) = &self.ambience {
            let _ = f.call3(
                &self.obj,
                &JsValue::from_f64(a.night as f64),
                &JsValue::from_f64(a.rain as f64),
                &JsValue::from_f64(a.fog as f64),
            );
        }
    }
}

/// Thin bridge to assets/menu.js (`window.gameMenu`): the title screen, pause menu and settings live
/// there. If it is missing the game simply never opens a menu.
struct MenuBridge {
    obj: JsValue,
}

impl MenuBridge {
    fn new(window: &web_sys::Window) -> MenuBridge {
        let obj = js_sys::Reflect::get(window, &JsValue::from_str("gameMenu")).unwrap_or(JsValue::UNDEFINED);
        MenuBridge { obj }
    }

    fn func(&self, name: &str) -> Option<js_sys::Function> {
        js_sys::Reflect::get(&self.obj, &JsValue::from_str(name)).ok()?.dyn_into::<js_sys::Function>().ok()
    }

    fn call0(&self, name: &str) -> Option<JsValue> {
        self.func(name)?.call0(&self.obj).ok()
    }

    fn is_open(&self) -> bool {
        self.call0("isOpen").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn on_title(&self) -> bool {
        self.call0("onTitle").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    /// "continue" | "new" | "resume" | "quit", or "" when nothing is waiting.
    fn take_action(&self) -> String {
        self.call0("takeAction").and_then(|v| v.as_string()).unwrap_or_default()
    }

    fn settings_version(&self) -> f64 {
        self.call0("settingsVersion").and_then(|v| v.as_f64()).unwrap_or(0.0)
    }

    fn settings_text(&self) -> String {
        self.call0("settingsText").and_then(|v| v.as_string()).unwrap_or_default()
    }

    fn open_pause(&self) {
        let _ = self.call0("openPause");
    }

    fn ready(&self) {
        let _ = self.call0("ready");
    }

    fn set_has_save(&self, has: bool, summary: &str) {
        if let Some(f) = self.func("setHasSave") {
            let _ = f.call2(&self.obj, &JsValue::from_bool(has), &JsValue::from_str(summary));
        }
    }
}

fn request_frame(cb: &Closure<dyn FnMut(f64)>) {
    if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

async fn run() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document")?;

    let gpu = js_sys::Reflect::get(&window.navigator(), &JsValue::from_str("gpu")).unwrap_or(JsValue::UNDEFINED);
    if gpu.is_undefined() || gpu.is_null() {
        return Err("WebGPU isn't available in this browser. Please use a recent desktop Chrome or Edge \
                    (or another WebGPU-enabled browser) with hardware acceleration turned on."
            .to_string());
    }

    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("gpu")
        .ok_or("Missing <canvas id=\"gpu\">")?
        .dyn_into()
        .map_err(|_| "#gpu is not a canvas".to_string())?;

    let mut game = Game::new(load_save(), new_seed());
    let static_mesh = scene::build_static(&game.world);
    let mut renderer = Renderer::new(canvas, &static_mesh.data).await?;
    let mut dyn_mesh = Mesh::new();
    let mut shown_area = game.area;

    // ---- input
    let input = Rc::new(RefCell::new(InputState::default()));
    {
        let inp = input.clone();
        let on_down = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            let code = e.code();
            if matches!(
                code.as_str(),
                "Space" | "Tab" | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "F1" | "F2" | "F3"
            ) {
                e.prevent_default();
            }
            if e.repeat() {
                return;
            }
            let mut s = inp.borrow_mut();
            if s.down.insert(code.clone()) {
                s.pressed.push(code);
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        window
            .add_event_listener_with_callback("keydown", on_down.as_ref().unchecked_ref())
            .map_err(|_| "Couldn't attach keyboard handler".to_string())?;
        on_down.forget();

        let inp = input.clone();
        let on_up = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            inp.borrow_mut().down.remove(&e.code());
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        window
            .add_event_listener_with_callback("keyup", on_up.as_ref().unchecked_ref())
            .map_err(|_| "Couldn't attach keyboard handler".to_string())?;
        on_up.forget();

        // Losing focus must not leave a key "stuck" down.
        let inp = input.clone();
        let on_blur = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            inp.borrow_mut().down.clear();
        }) as Box<dyn FnMut(web_sys::Event)>);
        window
            .add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref())
            .map_err(|_| "Couldn't attach blur handler".to_string())?;
        on_blur.forget();
    }

    let audio_bridge = AudioBridge::new(&window);

    // ---- title screen / pause menu / settings (assets/menu.js)
    let menu = MenuBridge::new(&window);
    let had_save = storage().and_then(|s| s.get_item(SAVE_KEY).ok().flatten()).is_some();
    menu.set_has_save(had_save, &save_summary(&game.save));
    menu.ready();
    let mut settings_ver = -1.0f64;
    let mut run_latched = false;

    // ---- HUD cache: only touch the DOM when a panel's HTML actually changed
    let mut hud_cache: HashMap<&'static str, (Option<web_sys::Element>, String)> = HashMap::new();

    // ---- main loop
    let slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let slot_for_start = slot.clone();

    let mut last_ms = 0.0f64;
    let mut acc = 0.0f32;
    let mut frame_no: u64 = 0;
    let mut save_cooldown = 0.0f32;

    *slot_for_start.borrow_mut() = Some(Closure::wrap(Box::new(move |now_ms: f64| {
        let dt = if last_ms == 0.0 { 1.0 / 60.0 } else { ((now_ms - last_ms) / 1000.0) as f32 };
        let dt = dt.clamp(0.0, 0.1);
        last_ms = now_ms;
        frame_no += 1;
        if dt > 0.0 {
            game.fps += (1.0 / dt - game.fps) * 0.05;
        }

        // ---- options and menu requests
        let ver = menu.settings_version();
        if ver != settings_ver {
            settings_ver = ver;
            game.settings = Settings::parse(&menu.settings_text());
            renderer.set_pixel_target(game.settings.pixel_target_h());
        }
        loop {
            let act = menu.take_action();
            if act.is_empty() {
                break;
            }
            match act.as_str() {
                // A fresh game: forget the save, keep the options.
                "new" => {
                    let settings = game.settings.clone();
                    write_save(&Save::default());
                    game = Game::new(Save::default(), new_seed());
                    game.settings = settings;
                }
                // Back to the title screen: save, then rebuild the world so the title idles in a clean glade.
                "quit" => {
                    let settings = game.settings.clone();
                    let save = game.save.clone();
                    write_save(&save);
                    menu.set_has_save(true, &save_summary(&save));
                    game = Game::new(save, new_seed());
                    game.settings = settings;
                }
                _ => {}
            }
        }
        let menu_open = menu.is_open();

        // Discrete actions once per frame; held movement every fixed step.
        let (mut ctl, pressed) = {
            let mut s = input.borrow_mut();
            (controls(&s.down), std::mem::take(&mut s.pressed))
        };
        if menu_open {
            // The menu owns the keyboard: nothing reaches the game, and held keys are forgotten.
            input.borrow_mut().down.clear();
            ctl = Controls::default();
        } else {
            for code in pressed {
                if code == "Escape" {
                    if game.show_journal {
                        game.act(Action::ToggleJournal);
                    } else {
                        menu.open_pause();
                    }
                    continue;
                }
                if (code == "ShiftLeft" || code == "ShiftRight") && game.settings.run_toggle {
                    run_latched = !run_latched;
                }
                let action = if game.show_journal { map_journal_key(&code).or_else(|| map_key(&code)) } else { map_key(&code) };
                if let Some(a) = action {
                    game.act(a);
                }
            }
            if game.show_journal {
                ctl = Controls::default(); // reading, not walking
            }
            if game.settings.run_toggle {
                if ctl.x == 0.0 && ctl.z == 0.0 {
                    run_latched = false;
                }
                ctl.run = run_latched;
            }
        }

        if menu_open {
            acc = 0.0;
            if menu.on_title() {
                // Title screen: the world idles while the camera drifts slowly beside the cottage.
                game.time += dt;
                let h = game.world.home;
                let t = game.time;
                game.cam = Vec3::new(h.x - 9.0 + (t * 0.11).sin() * 2.5, 0.0, h.z + 5.0 + (t * 0.07).cos() * 1.5);
            }
        } else {
            acc += dt;
            let mut steps = 0;
            while acc >= STEP && steps < 5 {
                game.update(STEP, &ctl);
                acc -= STEP;
                steps += 1;
            }
            if steps == 5 {
                acc = 0.0; // fell far behind (tab was hidden): don't spiral
            }
        }

        // Sound: one-shot events every frame, world mood a few times a second.
        for s in game.sfx.drain(..) {
            audio_bridge.play(s.name());
        }
        if frame_no % 6 == 1 {
            audio_bridge.set_ambience(&audio::ambience(&game));
        }

        // Draw. (Walking through a gate swaps the static world while the screen is black.)
        if game.area != shown_area {
            shown_area = game.area;
            renderer.set_static(&scene::build_static(&game.world).data);
        }
        renderer.resize_if_needed();
        let (lw, lh, scale) = renderer.low_res();
        dyn_mesh.clear();
        scene::build_dynamic(&game, &mut dyn_mesh);
        let globals = scene::build_globals(&game, lw as f32 / lh as f32);
        let post = scene::build_post(&game, lw, lh, scale);
        renderer.render(&globals, &post, &dyn_mesh.data, scene::clear_color(&game));

        // HUD at ~15 Hz is plenty.
        if frame_no % 4 == 0 {
            for (id, html) in hud::build(&game) {
                let entry = hud_cache
                    .entry(id)
                    .or_insert_with(|| (web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id(id)), String::new()));
                if entry.1 != html {
                    if let Some(el) = &entry.0 {
                        el.set_inner_html(&html);
                    }
                    entry.1 = html;
                }
            }
        }

        // Persist progress (at most every 2 s).
        save_cooldown -= dt;
        if game.dirty && save_cooldown <= 0.0 {
            game.dirty = false;
            save_cooldown = 2.0;
            write_save(&game.save);
        }

        if let Some(cb) = slot.borrow().as_ref() {
            request_frame(cb);
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(cb) = slot_for_start.borrow().as_ref() {
        request_frame(cb);
    }
    Ok(())
}
