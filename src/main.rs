#![allow(dead_code)]
//! Spirit Collector: a cozy Halloween creature-collecting game.
//! Rust + wgpu (WebGPU), compiled to WebAssembly. No engine.
//!
//! Layout:
//!   Pure, dependency-free logic (testable natively with `cargo test`... or plain `rustc --test`):
//!     math, rng, mesh   - tiny math lib + placeholder mesh builder
//!     data              - content tables (spirits, treats, upgrades) + save format
//!     env               - day/night + weather
//!     world             - the areas' layouts
//!     settings          - player options (volume, pixel size...) + their text format
//!     sketch            - the journal's little SVG drawings of each spirit
//!     game              - all gameplay rules
//!     scene             - game state -> meshes/camera/uniforms
//!     hud               - game state -> HTML strings
//!   Browser-only (wasm32):
//!     gfx               - wgpu renderer (low-res toon pass + pixel-perfect upscale/post pass)
//!     web               - canvas, input, main loop, localStorage

mod audio;
mod data;
mod env;
mod game;
mod hud;
mod math;
mod mesh;
mod rng;
mod scene;
mod settings;
mod sketch;
mod world;

#[cfg(target_arch = "wasm32")]
mod gfx;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Spirit Collector runs in the browser. Build and serve it with:  trunk serve");
    println!("(Run the logic tests natively with:  cargo test)");
}
