//! A tiny, dependency-free glTF binary (.glb) reader. It turns a model exported from Blender (or made
//! by `tools/make_player.py`) into the game's own vertex format, so props can swap in one at a time.
//!
//! Supported: node hierarchy with translation / rotation / scale / matrix, triangle primitives with
//! or without indices, POSITION, NORMAL (flat normals are made when missing), COLOR_0 (vec3 or vec4,
//! float or normalized integers), the material's base colour factor (multiplied in) and its
//! emissive factor (any non-zero emissive makes that primitive self-lit).
//! Not supported (on purpose, for now): textures, skins, animations, sparse accessors.

use crate::mesh::FLOATS_PER_VERTEX;

/// Vertices in the game's layout: position(3) normal(3) colour(3) emissive(1), three per triangle.
pub struct Model {
    pub data: Vec<f32>,
    /// The same triangles split by node, so the game can animate a part by its name.
    pub parts: Vec<Part>,
}

/// One mesh node of the model. `pivot` is the node's origin in model space (where it should rotate or
/// scale around); `data` is in model space too, in the same layout as `Model::data`.
pub struct Part {
    pub name: String,
    pub pivot: [f32; 3],
    pub data: Vec<f32>,
}

impl Model {
    pub fn vertex_count(&self) -> usize {
        self.data.len() / FLOATS_PER_VERTEX
    }
}

// ---------------------------------------------------------------- minimal JSON

#[derive(Debug, Clone)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn at(&self, i: usize) -> Option<&Json> {
        match self {
            Json::Arr(a) => a.get(i),
            _ => None,
        }
    }
    fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => &[],
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn usize(&self) -> Option<usize> {
        self.num().filter(|n| *n >= 0.0).map(|n| n as usize)
    }
    fn nums(&self) -> Vec<f32> {
        self.arr().iter().filter_map(|j| j.num()).map(|n| n as f32).collect()
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("json: expected '{}' at byte {}", c as char, self.i))
        }
    }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("json: bad literal at byte {}", self.i))
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.b.get(self.i).ok_or("json: unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or("json: bad escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' | b'f' => {}
                        b'u' => {
                            let hex = self.b.get(self.i..self.i + 4).ok_or("json: bad \\u")?;
                            let code = u32::from_str_radix(std::str::from_utf8(hex).map_err(|e| e.to_string())?, 16)
                                .map_err(|e| e.to_string())?;
                            self.i += 4;
                            let ch = char::from_u32(code).unwrap_or('?');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        String::from_utf8(out).map_err(|e| e.to_string())
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.b.get(self.i).copied() {
            Some(b'{') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(items));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    let v = self.value()?;
                    items.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(items));
                        }
                        _ => return Err(format!("json: bad object at byte {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return Err(format!("json: bad array at byte {}", self.i)),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(_) => {
                let start = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|e| e.to_string())?;
                s.parse::<f64>().map(Json::Num).map_err(|_| format!("json: bad number '{}'", s))
            }
            None => Err("json: unexpected end".into()),
        }
    }
}

// ---------------------------------------------------------------- 4x4 matrices (column-major)

type M4 = [f32; 16];

const IDENT: M4 = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn mul(a: &M4, b: &M4) -> M4 {
    let mut o = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            o[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    o
}

fn trs(t: [f32; 3], q: [f32; 4], s: [f32; 3]) -> M4 {
    let [x, y, z, w] = q;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz, wx, wy, wz) = (x * y, x * z, y * z, w * x, w * y, w * z);
    [
        (1.0 - 2.0 * (yy + zz)) * s[0], (2.0 * (xy + wz)) * s[0], (2.0 * (xz - wy)) * s[0], 0.0,
        (2.0 * (xy - wz)) * s[1], (1.0 - 2.0 * (xx + zz)) * s[1], (2.0 * (yz + wx)) * s[1], 0.0,
        (2.0 * (xz + wy)) * s[2], (2.0 * (yz - wx)) * s[2], (1.0 - 2.0 * (xx + yy)) * s[2], 0.0,
        t[0], t[1], t[2], 1.0,
    ]
}

fn xform_point(m: &M4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn xform_dir(m: &M4, p: [f32; 3]) -> [f32; 3] {
    let v = [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2],
    ];
    normalize(v)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l < 1e-12 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

// ---------------------------------------------------------------- loading

struct Doc<'a> {
    json: Json,
    bin: &'a [u8],
}

impl<'a> Doc<'a> {
    /// Reads an accessor into plain floats (normalized integers become 0..1 / -1..1).
    fn floats(&self, acc_idx: usize) -> Result<(Vec<f32>, usize), String> {
        let acc = self.json.get("accessors").and_then(|a| a.at(acc_idx)).ok_or("gltf: missing accessor")?;
        let count = acc.get("count").and_then(|c| c.usize()).ok_or("gltf: accessor count")?;
        let comps = match acc.get("type") {
            Some(Json::Str(t)) => match t.as_str() {
                "SCALAR" => 1,
                "VEC2" => 2,
                "VEC3" => 3,
                "VEC4" => 4,
                other => return Err(format!("gltf: accessor type {}", other)),
            },
            _ => return Err("gltf: accessor type".into()),
        };
        let ctype = acc.get("componentType").and_then(|c| c.usize()).ok_or("gltf: componentType")?;
        let size = match ctype {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err(format!("gltf: componentType {}", ctype)),
        };
        let normalized = matches!(acc.get("normalized"), Some(Json::Bool(true)));
        let view = acc.get("bufferView").and_then(|v| v.usize()).ok_or("gltf: sparse/empty accessors are not supported")?;
        let bv = self.json.get("bufferViews").and_then(|a| a.at(view)).ok_or("gltf: missing bufferView")?;
        let base = bv.get("byteOffset").and_then(|o| o.usize()).unwrap_or(0) + acc.get("byteOffset").and_then(|o| o.usize()).unwrap_or(0);
        let stride = bv.get("byteStride").and_then(|s| s.usize()).filter(|s| *s > 0).unwrap_or(size * comps);
        let mut out = Vec::with_capacity(count * comps);
        for i in 0..count {
            for c in 0..comps {
                let at = base + i * stride + c * size;
                let raw = self.bin.get(at..at + size).ok_or("gltf: accessor runs past the buffer")?;
                let f = match ctype {
                    5126 => f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
                    5125 => u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as f32,
                    5123 => {
                        let v = u16::from_le_bytes([raw[0], raw[1]]) as f32;
                        if normalized { v / 65535.0 } else { v }
                    }
                    5122 => {
                        let v = i16::from_le_bytes([raw[0], raw[1]]) as f32;
                        if normalized { (v / 32767.0).max(-1.0) } else { v }
                    }
                    5121 => {
                        if normalized { raw[0] as f32 / 255.0 } else { raw[0] as f32 }
                    }
                    _ => {
                        if normalized { (raw[0] as i8 as f32 / 127.0).max(-1.0) } else { raw[0] as i8 as f32 }
                    }
                };
                out.push(f);
            }
        }
        Ok((out, comps))
    }

    fn node_matrix(&self, node: &Json) -> M4 {
        if let Some(m) = node.get("matrix") {
            let v = m.nums();
            if v.len() == 16 {
                let mut a = IDENT;
                a.copy_from_slice(&v);
                return a;
            }
        }
        let t = node.get("translation").map(|j| j.nums()).filter(|v| v.len() == 3).unwrap_or(vec![0.0; 3]);
        let q = node.get("rotation").map(|j| j.nums()).filter(|v| v.len() == 4).unwrap_or(vec![0.0, 0.0, 0.0, 1.0]);
        let s = node.get("scale").map(|j| j.nums()).filter(|v| v.len() == 3).unwrap_or(vec![1.0; 3]);
        trs([t[0], t[1], t[2]], [q[0], q[1], q[2], q[3]], [s[0], s[1], s[2]])
    }

    fn add_node(&self, idx: usize, parent: &M4, depth: usize, out: &mut Vec<f32>, parts: &mut Vec<Part>) -> Result<(), String> {
        if depth > 32 {
            return Err("gltf: node tree is too deep".into());
        }
        let node = self.json.get("nodes").and_then(|n| n.at(idx)).ok_or("gltf: missing node")?;
        let world = mul(parent, &self.node_matrix(node));
        if let Some(mesh_idx) = node.get("mesh").and_then(|m| m.usize()) {
            let mesh = self.json.get("meshes").and_then(|m| m.at(mesh_idx)).ok_or("gltf: missing mesh")?;
            let mut local = Vec::new();
            for prim in mesh.get("primitives").map(|p| p.arr()).unwrap_or(&[]) {
                self.add_primitive(prim, &world, &mut local)?;
            }
            let name = match node.get("name") {
                Some(Json::Str(s)) => s.clone(),
                _ => format!("part{}", idx),
            };
            out.extend_from_slice(&local);
            parts.push(Part { name, pivot: xform_point(&world, [0.0, 0.0, 0.0]), data: local });
        }
        for child in node.get("children").map(|c| c.arr()).unwrap_or(&[]) {
            if let Some(c) = child.usize() {
                self.add_node(c, &world, depth + 1, out, parts)?;
            }
        }
        Ok(())
    }

    fn add_primitive(&self, prim: &Json, world: &M4, out: &mut Vec<f32>) -> Result<(), String> {
        let mode = prim.get("mode").and_then(|m| m.usize()).unwrap_or(4);
        if mode != 4 {
            return Err(format!("gltf: only triangle lists are supported (mode {})", mode));
        }
        let attrs = prim.get("attributes").ok_or("gltf: primitive has no attributes")?;
        let (pos, _) = self.floats(attrs.get("POSITION").and_then(|a| a.usize()).ok_or("gltf: no POSITION")?)?;
        let normals = match attrs.get("NORMAL").and_then(|a| a.usize()) {
            Some(a) => Some(self.floats(a)?.0),
            None => None,
        };
        let colors = match attrs.get("COLOR_0").and_then(|a| a.usize()) {
            Some(a) => Some(self.floats(a)?),
            None => None,
        };
        let n_verts = pos.len() / 3;
        let indices: Vec<usize> = match prim.get("indices").and_then(|a| a.usize()) {
            Some(a) => self.floats(a)?.0.into_iter().map(|f| f as usize).collect(),
            None => (0..n_verts).collect(),
        };

        // Material: base colour tint and emissive strength.
        let mut tint = [1.0f32; 3];
        let mut emissive = 0.0f32;
        if let Some(m) = prim.get("material").and_then(|m| m.usize()).and_then(|m| self.json.get("materials").and_then(|a| a.at(m))) {
            if let Some(b) = m.get("pbrMetallicRoughness").and_then(|p| p.get("baseColorFactor")).map(|j| j.nums()) {
                if b.len() >= 3 {
                    tint = [b[0], b[1], b[2]];
                }
            }
            if let Some(e) = m.get("emissiveFactor").map(|j| j.nums()) {
                emissive = e.iter().cloned().fold(0.0, f32::max).clamp(0.0, 1.0);
            }
        }

        for tri in indices.chunks_exact(3) {
            if tri.iter().any(|&i| i >= n_verts) {
                return Err("gltf: index out of range".into());
            }
            let p: Vec<[f32; 3]> = tri.iter().map(|&i| xform_point(world, [pos[i * 3], pos[i * 3 + 1], pos[i * 3 + 2]])).collect();
            let flat = {
                let (a, b) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
                normalize([a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]])
            };
            for (k, &i) in tri.iter().enumerate() {
                let n = match &normals {
                    Some(n) => xform_dir(world, [n[i * 3], n[i * 3 + 1], n[i * 3 + 2]]),
                    None => flat,
                };
                let c = match &colors {
                    Some((c, comps)) => [c[i * comps] * tint[0], c[i * comps + 1] * tint[1], c[i * comps + 2] * tint[2]],
                    None => tint,
                };
                out.extend_from_slice(&[p[k][0], p[k][1], p[k][2], n[0], n[1], n[2], c[0], c[1], c[2], emissive]);
            }
        }
        Ok(())
    }
}

/// Parses a .glb file. Errors are plain strings so a bad model can fall back to the placeholder art.
pub fn parse_glb(bytes: &[u8]) -> Result<Model, String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("gltf: not a .glb file".into());
    }
    let rd = |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
    if rd(4) != 2 {
        return Err("gltf: only version 2 is supported".into());
    }
    let (mut json_bytes, mut bin): (Option<&[u8]>, &[u8]) = (None, &[]);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let len = rd(at);
        let kind = &bytes[at + 4..at + 8];
        let body = bytes.get(at + 8..at + 8 + len).ok_or("gltf: chunk runs past the end of the file")?;
        match kind {
            b"JSON" => json_bytes = Some(body),
            b"BIN\0" => bin = body,
            _ => {}
        }
        at += 8 + len + (4 - len % 4) % 4;
    }
    let json = Parser { b: json_bytes.ok_or("gltf: no JSON chunk")?, i: 0 }.value()?;
    let doc = Doc { json, bin };
    let scene = doc.json.get("scene").and_then(|s| s.usize()).unwrap_or(0);
    let roots: Vec<usize> = match doc.json.get("scenes").and_then(|s| s.at(scene)).and_then(|s| s.get("nodes")) {
        Some(n) => n.arr().iter().filter_map(|j| j.usize()).collect(),
        None => (0..doc.json.get("nodes").map(|n| n.arr().len()).unwrap_or(0)).collect(),
    };
    let mut data = Vec::new();
    let mut parts = Vec::new();
    for r in roots {
        doc.add_node(r, &IDENT, 0, &mut data, &mut parts)?;
    }
    if data.is_empty() {
        return Err("gltf: the model has no triangles".into());
    }
    Ok(Model { data, parts })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a .glb by hand: one triangle (no normals, no indices), uint8 colours, a translated node.
    fn tiny_glb(emissive: f32) -> Vec<u8> {
        let mut bin = Vec::new();
        for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&f.to_le_bytes());
        }
        bin.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]);
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"translation":[10,0,0],"children":[1]}},{{"mesh":0,"scale":[2,2,2]}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"COLOR_0":1}},"material":0}}]}}],
            "materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[1,1,1,1]}},"emissiveFactor":[{e},{e},{e}]}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}},
                         {{"bufferView":1,"componentType":5121,"normalized":true,"count":3,"type":"VEC4"}}],
            "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":12}}],
            "buffers":[{{"byteLength":48}}]}}"#,
            e = emissive
        );
        let mut js = json.into_bytes();
        while js.len() % 4 != 0 {
            js.push(b' ');
        }
        let total = 12 + 8 + js.len() + 8 + bin.len();
        let mut f = Vec::new();
        f.extend_from_slice(b"glTF");
        f.extend_from_slice(&2u32.to_le_bytes());
        f.extend_from_slice(&(total as u32).to_le_bytes());
        f.extend_from_slice(&(js.len() as u32).to_le_bytes());
        f.extend_from_slice(b"JSON");
        f.extend_from_slice(&js);
        f.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        f.extend_from_slice(b"BIN\0");
        f.extend_from_slice(&bin);
        f
    }

    #[test]
    fn reads_nodes_colours_and_flat_normals() {
        let m = parse_glb(&tiny_glb(0.0)).unwrap();
        assert_eq!(m.vertex_count(), 3);
        let v: Vec<&[f32]> = m.data.chunks(FLOATS_PER_VERTEX).collect();
        // parent translates by 10, child scales by 2: (1,0,0) -> (12,0,0)
        assert!((v[1][0] - 12.0).abs() < 1e-5 && v[1][1].abs() < 1e-5);
        assert!((v[2][1] - 2.0).abs() < 1e-5);
        // no NORMAL attribute: a flat normal is made (counter-clockwise triangle in XY faces +Z)
        assert!((v[0][5] - 1.0).abs() < 1e-5);
        // normalized u8 colours
        assert_eq!(&v[0][6..9], &[1.0, 0.0, 0.0]);
        assert_eq!(&v[2][6..9], &[0.0, 0.0, 1.0]);
        assert_eq!(v[0][9], 0.0);
        assert_eq!(parse_glb(&tiny_glb(1.0)).unwrap().data[9], 1.0);
    }

    #[test]
    fn bad_files_are_errors_not_panics() {
        assert!(parse_glb(b"nope").is_err());
        let good = tiny_glb(0.0);
        assert!(parse_glb(&good[..good.len() - 20]).is_err());
        let mut bad = good.clone();
        bad[4] = 1; // version 1
        assert!(parse_glb(&bad).is_err());
    }

    #[test]
    fn the_player_model_loads() {
        let m = parse_glb(include_bytes!("../assets/models/player.glb")).unwrap();
        assert!(m.vertex_count() > 300);
        let ys: Vec<f32> = m.data.chunks(FLOATS_PER_VERTEX).map(|v| v[1]).collect();
        let lo = ys.iter().cloned().fold(f32::MAX, f32::min);
        let hi = ys.iter().cloned().fold(f32::MIN, f32::max);
        assert!(lo > -0.01 && lo < 0.1, "feet should sit on y=0, got {}", lo);
        assert!(hi > 1.4 && hi < 2.2, "about 1.9 tall, got {}", hi);
        assert!(m.data.chunks(FLOATS_PER_VERTEX).any(|v| v[9] > 0.5), "the lantern should glow");
        for v in m.data.chunks(FLOATS_PER_VERTEX) {
            assert!((v[3] * v[3] + v[4] * v[4] + v[5] * v[5] - 1.0).abs() < 1e-3);
        }
    }
}
