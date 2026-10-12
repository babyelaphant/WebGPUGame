# untitled-01 (working title: Spirit Collector)

A cozy Halloween creature-collecting game. You're a ghost collector exploring an enchanted forest:
find spirits, win them over with treats or little puzzles, bring them home, spend the candles they
earn you on upgrades, and push deeper into the woods.

**Stack:** Rust + [wgpu](https://wgpu.rs) (WebGPU) compiled to WebAssembly. No engine, no three.js.
Looks like a chunky low-res "A Short Hike" scene (toon lighting, pixel-perfect upscale) with weather
done in shaders.

> UI: the "Candlelit" look (pixel fonts Press Start 2P + VT323, gold double-line windows) lives in the CSS of index.html and assets/menu.css.

## Run it (Windows / macOS / Linux)

One-time setup:

```
# 1. Install Rust: https://rustup.rs   (then restart your terminal)
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

Every time:

```
trunk serve          # builds, serves on http://127.0.0.1:8080, auto-reloads on save
```

Use a recent desktop **Chrome or Edge** (WebGPU, hardware acceleration on).
`trunk serve --release` for a faster, optimized build.

> The first build downloads and compiles wgpu, so it takes a few minutes. After that it's quick.

## Controls

| Key | Action |
| --- | --- |
| WASD / arrows | Move |
| Shift | Run |
| E | Interact: gather treats, light lanterns, use gates, touch crypt stones |
| Space | Put the selected treat on the ground |
| Q / R (or Tab) | Switch treat |
| 1 / 2 / 3 / 4 / 5 | Buy upgrade (at the cottage) |
| J | Spirit journal (A / D or the arrow keys turn the page) |
| Esc | Pause menu: resume, settings, back to the title screen |
| M | Mute / unmute sound |
| F1 | Debug overlay |
| F2 | Cycle weather *(debug)* |
| [ and ] | Hour -1 / +1 *(debug)* |

## How it plays

1. **Explore.** Glowing plants grow treats (Candy Corn, Pumpkin Cookie, Moon Milk; Soul Cake grows in Graveyard Hollow and Lily Jam at Willow Pond).
2. **Find spirits.** Who shows up depends on the **time of day and weather**:

   | Spirit | Appears | Likes | Personality |
   | --- | --- | --- | --- |
   | Pumpkin Pip | Clear evenings (17:00-23:00) | Candy Corn | Curious |
   | Mizzle | Rain, any time | Moon Milk | Playful |
   | Hush | Fog, any time | Pumpkin Cookie | Shy: stand still, a few steps back |
   | Luna, Lantern Queen | Clear nights (21:00-04:00) | Moon Milk | Only after you light the 3 lanterns in the far glade |

   **Graveyard Hollow** (beyond the iron gate, once you buy the **Gate Key** at the cottage) has its own spirits,
   and weather and time of day matter all over again:

   | Spirit | Appears | Likes | Personality |
   | --- | --- | --- | --- |
   | Wick | Clear nights (18:00-05:00) | Soul Cake | Curious |
   | Drip | Rain, any time | Pumpkin Cookie | Playful |
   | Gloam | Fog, any time | Soul Cake | Shy |
   | Old Marrow, Crypt Keeper | Nights (20:00-05:00) | Candy Corn | Only after you repeat the crypt's melody |

   **Willow Pond** (the wooden reed gate at the south edge of the glade, opened by the **Frog Charm**) is a bright
   forest around a pond, with a shed, a little pier and a rowboat. Its spirits keep daytime hours:

   | Spirit | Appears | Likes | Personality |
   | --- | --- | --- | --- |
   | Bobbin | Clear days (06:00-19:00) | Lily Jam | Curious |
   | Pondle | Rain, any time | Pumpkin Cookie | Playful |
   | Reed | Fog, any time | Candy Corn | Shy |
   | Lady Lotus, Pond Queen | Daylight, after the lily lamps | Moon Milk | Curious |

   **The lily lamp puzzle:** four lamps stand in a row in front of the shed. Pressing <kbd>E</kbd> at a lamp flips it
   *and its neighbours*; light all four. Every dawn the lamps are scrambled again (always solvable). You can't wade
   into the pond, so walk around it.

   **One of a kind:** Old Marrow and Lady Lotus never reappear once they live at your cottage.

   **The crypt puzzle:** stand at the crypt door and press <kbd>E</kbd> to hear the melody (four rune stones light up
   and ring in order), then touch the stones in the same order. A new tune is picked every dawn.
   Friends who follow you come through the gate with you, but they can only be handed over at the cottage.

   **Finding them.** You don't have to stumble onto spirits any more. Anything wild within about 18 steps of you
   (more with a Brighter Lantern) gives itself away:

   - **sparkles** rising from the spirit (new faces also send out a slow ring on the ground), and
   - a small **compass hint** under the clock ("A faint shimmer to the north-east"). The dot is the spirit's colour.

   All of it can be switched off in Settings -> Audio / Gameplay.

3. **Lure or solve.** Set down the treat they like and wait. Shy ones bolt if you walk at them.
4. **Bring them home.** Befriended spirits follow you. Walk into the cottage garden to hand them over.
5. **Earn and upgrade.** Candles buy a brighter lantern (bigger light + lure range), faster boots, a bigger treat pouch.

Progress saves automatically in your browser (localStorage).

## The journal

Press <kbd>J</kbd>. There's one page per area and a little drawing for every spirit that **fills in as you learn
about it**. There are five notes per spirit (60 in all), and you learn them by playing:

| Note | You learn it by... | What the sketch gains |
| --- | --- | --- |
| Seen | getting close enough to spot it | an ink silhouette and its name |
| Habits | watching it for about 8 seconds | pencil outline + a face that shows its personality |
| Favorite treat | seeing it come running for a treat | a doodle of the treat |
| When it appears | seeing it fade away when the weather or hour changes | weather + time-of-day doodles |
| Friend | bringing it home | full colour, blush and sparkles |

A new note plays a pencil scratch and a toast, and the clock bar shows "new note!" until you open the book.
Befriending a spirit teaches you all of it at once. Unfound spirits are dashed "?" boxes.

## Title screen and settings

The title screen ("untitled-01" for now: change `GAME_NAME` in `assets/menu.js`) shows the glade at golden hour with
the camera drifting past your cottage. **Continue / Play**, **New game** (asks first) and **Settings**.
Press <kbd>Esc</kbd> in the game for the pause menu; **Save & back to title** saves your progress.

Settings (keyboard or mouse, saved in your browser):

| Tab | Options |
| --- | --- |
| Audio | master / music / sound effects / ambience volume, sound on/off |
| Video | fullscreen, pixel size (Chunky / Normal / Fine), brightness, weather effects (Off / Subtle / Full), dark screen edges, interface size, show FPS |
| Gameplay | hints & tutorial text, spirit shimmer & compass, hold-or-toggle running, reset to defaults |
| Controls | the key list (rebinding is on the to-do list) |

## Code map

```
src/
  main.rs      crate root + module map
  data.rs      <- spirits, treats, upgrades live here. Add content by editing tables.
  env.rs       day/night + weather -> lighting
  world.rs     the three areas: Hollow Glade, Graveyard Hollow, Willow Pond (seeded, deterministic)
  game.rs      all gameplay rules + tests (no rendering, no browser)
  scene.rs     game state -> placeholder meshes, camera, light/post uniforms
  hud.rs       game state -> HTML for the DOM HUD (and the journal pages)
  sketch.rs    the journal's SVG drawings, layered by what you've learned
  settings.rs  player options + their text format (shared with assets/menu.js)
  math.rs, rng.rs, mesh.rs   tiny dependency-free helpers
  gltf.rs      .glb model loader (Blender exports) into the mesh vertex format
  tools/       make_player.py, make_spirits.py: build the .glb models without Blender
  gfx.rs       wgpu renderer           (browser only)
  web.rs       canvas/input/loop/save  (browser only)
  shaders/     scene.wgsl (toon + lights + fog), post.wgsl (pixel upscale, rain, fog, vignette)
index.html     canvas + CSS HUD + journal styling
assets/
  audio.js     synthesized sound (effects, ambience, music)
  menu.js/css  title screen, pause menu, settings (plain DOM; the Rust side talks to it via window.gameMenu)
```

Everything except `gfx.rs` and `web.rs` has **zero dependencies**, so the game rules are unit-tested
natively:

```
cargo test
```

### Adding a spirit

Add an entry to `SPIRITS` in `src/data.rs` (bump `N_SPIRITS`). Pick its area (0 glade, 1 hollow), treat, weather, hours and
personality. For a custom look, add a case in `draw_spirit` in `src/scene.rs`.

## Sound

Everything is synthesized live with the Web Audio API in `assets/audio.js`, so there are no audio files:
soft music-box bells, a slow pentatonic melody (brighter by day, mellower at night), wind, crickets at
night, rain, a low fog pad, plus effects for footsteps, gathering, treats, befriending, handing spirits
over, upgrades, lanterns, and weather changes. The Rust side only emits events (`src/audio.rs`).
The browser starts audio after your first key press. **M** mutes (remembered between sessions).

## Weather

Weather is gentle on purpose, so it is announced instead of hammering you visually: about 30 seconds
before a change you get a forecast in the clock ("Rain approaching") and a toast, then a second toast
and a chime when it arrives, followed by a 20 second visual crossfade. Planning around the forecast is
part of the game: each weather brings different spirits.

## Art pipeline (placeholder now, Blender later)

All models are currently built from primitives in `scene.rs`, which is deliberate: the game is
playable end to end with no art blockers. Plan:

- **Models:** Blender -> export **glTF binary (.glb)**, low-poly with vertex colors. `src/gltf.rs` is a
  small dependency-free loader that turns a .glb into the same vertex format `mesh.rs` uses, so props
  swap in one at a time. The player (`assets/models/player.glb`) is the first one; if a model fails
  to load, the old primitive version is drawn instead. Free CC0 packs (Kenney, Quaternius) also load.
- **Making models in Blender:** model with +Z up as usual and face the model toward Blender's **-Y**
  (the exporter turns that into the game's forward). Feet on the ground at z = 0, about 1.9 m tall for
  the player. Paint colour as a **Color Attribute** (vertex colors); a material with **Emission** > 0 is drawn
  self-lit (lantern flames, windows). Export: File > Export > glTF 2.0 > **.glb**, with
  *Attributes > Vertex Color* (Active) and *Apply Modifiers* on, then drop the file in `assets/models/`.
  No textures, skins or animations yet.
- **Without Blender:** `python3 tools/make_player.py`, `make_spirits.py` and `make_props.py` regenerate the
  player, the 12 spirit models (`assets/models/spirit_<id>.glb`) and the environment (`prop_<name>.glb`: trees,
  rocks, mushrooms, graves, bushes, cottage, crypt, shed, gates, pond props) from code.
- **Environment props:** each is placed by `put()` in `scene.rs`. A part whose name starts with `tint` has its colour
  multiplied by a per-instance colour (leaf colour, rock tone, mushroom cap, berries), so one model gives many looks.
  If a prop file is missing or broken the old primitive version is drawn instead.
- **Idle animation:** a model is split into parts (one Blender object / glTF node each, origin = pivot) and the
  game animates a part by its **name** (`part_xf_opt` in `scene.rs`): `flap_l/_r`, `flaplow_l/_r` (moth wings),
  `bat_l/_r`, `fin_l/_r`, `twitch_l/_r` (ears, whiskers), `flicker` (flames), `sway`, `sway2_l/_r`, `bob`,
  `spin` (orbiting lights), `bloom`, `bloom2`, `dripfall0..2`, `tip` (hat), `eyes` (blink), `squish`, `pulse`,
  `tail`. A part named `body` stays still. Add a rule there to invent a new motion.
- **2D / UI:** concept sheets for spirits, journal and menu mockups are a good fit for Claude Design;
  the HUD is plain HTML/CSS (`index.html`, `hud.rs`) so mockups translate directly.

## Roadmap

- [x] Milestone 1: one glade, day/night, clear/fog/rain, 4 spirits, treat lures, lantern puzzle, cottage, 3 upgrades, saves
- [x] glTF loader + player model
- [x] 12 spirit models with idle animations
- [x] Environment props (trees, rocks, mushrooms, graves, bushes, buildings, pond) as models
- [ ] Replace the lamps, gate doors and water with models too
- [x] Audio: synthesized cozy soundscape (see below)
- [x] Second area, gated by an upgrade: Graveyard Hollow, 4 new spirits, a rune-melody puzzle
- [x] Third area: Willow Pond with water, a shed, 4 spirits and a lily-lamp puzzle
- [x] Finding spirits: nearby shimmer + compass hint
- [x] Journal that fills in with sketches as you learn about each spirit
- [x] Title screen + settings menu
- [ ] More spirits
- [ ] Key rebinding, gamepad support
