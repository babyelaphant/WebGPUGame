# Spirit Collector

A cozy Halloween creature-collecting game. You're a ghost collector exploring an enchanted forest:
find spirits, win them over with treats or little puzzles, bring them home, spend the candles they
earn you on upgrades, and push deeper into the woods.

**Stack:** Rust + [wgpu](https://wgpu.rs) (WebGPU) compiled to WebAssembly. No engine, no three.js.
Looks like a chunky low-res "A Short Hike" scene (toon lighting, pixel-perfect upscale) with weather
done in shaders.

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
| J | Spirit journal |
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

3. **Lure or solve.** Set down the treat they like and wait. Shy ones bolt if you walk at them.
4. **Bring them home.** Befriended spirits follow you. Walk into the cottage garden to hand them over.
5. **Earn and upgrade.** Candles buy a brighter lantern (bigger light + lure range), faster boots, a bigger treat pouch.

Progress saves automatically in your browser (localStorage).

## Code map

```
src/
  main.rs      crate root + module map
  data.rs      <- spirits, treats, upgrades live here. Add content by editing tables.
  env.rs       day/night + weather -> lighting
  world.rs     the three areas: Hollow Glade, Graveyard Hollow, Willow Pond (seeded, deterministic)
  game.rs      all gameplay rules + tests (no rendering, no browser)
  scene.rs     game state -> placeholder meshes, camera, light/post uniforms
  hud.rs       game state -> HTML for the DOM HUD
  math.rs, rng.rs, mesh.rs   tiny dependency-free helpers
  gfx.rs       wgpu renderer           (browser only)
  web.rs       canvas/input/loop/save  (browser only)
  shaders/     scene.wgsl (toon + lights + fog), post.wgsl (pixel upscale, rain, fog, vignette)
index.html     canvas + CSS HUD
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

- **Models:** Blender -> export **glTF binary (.glb)**, low-poly with vertex colors. A small glTF
  loader (milestone 2) will turn each file into the same vertex format `mesh.rs` already uses, so
  props swap in one at a time. Free CC0 packs (Kenney, Quaternius) are a good stopgap.
- **2D / UI:** concept sheets for spirits, journal and menu mockups are a good fit for Claude Design;
  the HUD is plain HTML/CSS (`index.html`, `hud.rs`) so mockups translate directly.

## Roadmap

- [x] Milestone 1: one glade, day/night, clear/fog/rain, 4 spirits, treat lures, lantern puzzle, cottage, 3 upgrades, saves
- [ ] glTF loader + first Blender props
- [x] Audio: synthesized cozy soundscape (see below)
- [x] Second area, gated by an upgrade: Graveyard Hollow, 4 new spirits, a rune-melody puzzle
- [x] Third area: Willow Pond with water, a shed, 4 spirits and a lily-lamp puzzle
- [ ] More spirits, journal with sketches
- [ ] Gamepad support, settings menu
