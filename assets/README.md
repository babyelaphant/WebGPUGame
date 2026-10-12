# assets/

Copied verbatim into `dist/assets/` by Trunk. This is where Blender exports (`.glb`) will live
once the glTF loader lands (milestone 2). For now every model is a placeholder built from
primitives in `src/scene.rs`.

Planned layout:

```
assets/models/   props, trees, spirits, buildings  (.glb, low-poly, vertex colors)
assets/audio/    music + sfx
assets/ui/       concept art and UI sprites exported from Claude Design
```
