#!/usr/bin/env python3
"""Builds assets/models/player.glb with no dependencies (no Blender needed).

The game's glTF loader (src/gltf.rs) reads any .glb that follows the same conventions, so a model
made in Blender can replace this one: see "Making models in Blender" in the README.

Conventions: +Y up, the model faces +Z, feet on y = 0, about 1.9 units tall (the game scales it 1.2x).
Colours are per-vertex (COLOR_0). A material with a non-zero emissiveFactor is drawn self-lit.
"""
import json, math, struct, sys

TAU = math.tau

def sub(a, b): return (a[0]-b[0], a[1]-b[1], a[2]-b[2])
def add(a, b): return (a[0]+b[0], a[1]+b[1], a[2]+b[2])
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def dot(a, b): return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
def norm(a):
    l = math.sqrt(dot(a, a)) or 1.0
    return (a[0]/l, a[1]/l, a[2]/l)

class Group:
    """Triangles that share one material (lit or glowing)."""
    def __init__(self, name, emissive):
        self.name, self.emissive, self.pos, self.nrm, self.col = name, emissive, [], [], []

    def tri(self, a, b, c, center, color):
        n = cross(sub(b, a), sub(c, a))
        if dot(n, n) < 1e-12: return
        n = norm(n)
        cen = ((a[0]+b[0]+c[0])/3, (a[1]+b[1]+c[1])/3, (a[2]+b[2]+c[2])/3)
        if dot(n, sub(cen, center)) < 0: n = (-n[0], -n[1], -n[2])
        # Wind counter-clockwise when seen from outside, as glTF expects.
        if dot(cross(sub(b, a), sub(c, a)), n) < 0: b, c = c, b
        for p in (a, b, c):
            self.pos.append(p); self.nrm.append(n); self.col.append(color)

    def quad(self, a, b, c, d, center, color):
        self.tri(a, b, c, center, color); self.tri(a, c, d, center, color)

    def box(self, c, h, color):
        P = lambda ix, iy, iz: (c[0]+h[0]*(ix*2-1), c[1]+h[1]*(iy*2-1), c[2]+h[2]*(iz*2-1))
        for f in ([P(1,0,0),P(1,1,0),P(1,1,1),P(1,0,1)], [P(0,0,0),P(0,0,1),P(0,1,1),P(0,1,0)],
                  [P(0,1,0),P(0,1,1),P(1,1,1),P(1,1,0)], [P(0,0,0),P(1,0,0),P(1,0,1),P(0,0,1)],
                  [P(0,0,1),P(1,0,1),P(1,1,1),P(0,1,1)], [P(0,0,0),P(0,1,0),P(1,1,0),P(1,0,0)]):
            self.quad(*f, c, color)

    def frustum(self, base, r0, r1, h, seg, color, rot0=0.0):
        center = (base[0], base[1]+h/2, base[2]); top = (base[0], base[1]+h, base[2])
        for i in range(seg):
            a0, a1 = TAU*i/seg + rot0, TAU*(i+1)/seg + rot0
            b0 = (base[0]+r0*math.cos(a0), base[1], base[2]+r0*math.sin(a0))
            b1 = (base[0]+r0*math.cos(a1), base[1], base[2]+r0*math.sin(a1))
            t0 = (top[0]+r1*math.cos(a0), top[1], top[2]+r1*math.sin(a0))
            t1 = (top[0]+r1*math.cos(a1), top[1], top[2]+r1*math.sin(a1))
            self.quad(b0, b1, t1, t0, center, color)
            self.tri(top, t0, t1, center, color)

    def cone_to(self, base, r, tip, seg, color, rot0=0.0):
        """A cone whose point can lean: base circle (flat) at `base`, point at `tip`."""
        center = ((base[0]+tip[0])/2, (base[1]+tip[1])/2, (base[2]+tip[2])/2)
        for i in range(seg):
            a0, a1 = TAU*i/seg + rot0, TAU*(i+1)/seg + rot0
            b0 = (base[0]+r*math.cos(a0), base[1], base[2]+r*math.sin(a0))
            b1 = (base[0]+r*math.cos(a1), base[1], base[2]+r*math.sin(a1))
            self.tri(b0, b1, tip, center, color)
            self.tri(base, b0, b1, center, color)

    def ellipsoid(self, c, r, stacks, slices, color):
        pt = lambda th, ph: (c[0]+r[0]*math.sin(th)*math.cos(ph), c[1]+r[1]*math.cos(th), c[2]+r[2]*math.sin(th)*math.sin(ph))
        for i in range(stacks):
            t0, t1 = math.pi*i/stacks, math.pi*(i+1)/stacks
            for j in range(slices):
                p0, p1 = TAU*j/slices, TAU*(j+1)/slices
                self.quad(pt(t0,p0), pt(t0,p1), pt(t1,p1), pt(t1,p0), c, color)

    def sphere(self, c, r, color, stacks=3, slices=6):
        self.ellipsoid(c, (r, r, r), stacks, slices, color)

def build():
    body, glow = Group("PlayerBody", 0.0), Group("LanternGlow", 1.0)

    cloak, cloak_dark = (0.36, 0.20, 0.54), (0.27, 0.15, 0.43)
    hood = (0.24, 0.12, 0.40)
    skin, ink, blush = (0.90, 0.75, 0.64), (0.10, 0.06, 0.12), (1.0, 0.55, 0.62)
    gold, boot, leather = (0.88, 0.62, 0.26), (0.20, 0.12, 0.10), (0.50, 0.30, 0.18)
    scarf = (0.92, 0.50, 0.20)

    # boots peeking out under the cloak
    for sx in (-1, 1):
        body.box((sx*0.15, 0.07, 0.10), (0.10, 0.07, 0.16), boot)
    # cloak: wide at the hem, trimmed in gold, with a darker back panel
    body.frustum((0, 0.04, 0), 0.60, 0.30, 0.98, 8, cloak)
    body.frustum((0, 0.04, 0), 0.62, 0.60, 0.07, 8, gold)
    body.cone_to((0, 0.30, -0.28), 0.34, (0, 0.04, -0.62), 6, cloak_dark)  # a little swish at the back
    # sleeves and hands
    body.ellipsoid((-0.40, 0.72, 0.06), (0.10, 0.24, 0.10), 3, 6, cloak)
    body.ellipsoid((0.40, 0.78, 0.18), (0.10, 0.20, 0.12), 3, 6, cloak)
    body.sphere((-0.40, 0.50, 0.10), 0.07, skin)
    body.sphere((0.42, 0.64, 0.30), 0.07, skin)
    # satchel on the left hip, with a strap across the chest
    body.box((-0.42, 0.46, -0.12), (0.09, 0.12, 0.08), leather)
    body.box((-0.42, 0.55, -0.12), (0.095, 0.03, 0.085), (0.38, 0.22, 0.13))
    body.box((0.0, 0.80, 0.20), (0.03, 0.30, 0.03), leather)
    # scarf with a tail
    body.frustum((0, 0.94, 0), 0.34, 0.30, 0.14, 8, scarf)
    body.box((0.20, 0.80, -0.26), (0.07, 0.17, 0.03), scarf)
    # head: face, hood, a pointy tip that flops backwards, and a bobble
    body.ellipsoid((0, 1.28, 0.05), (0.27, 0.27, 0.27), 3, 6, skin)
    body.ellipsoid((0, 1.38, -0.08), (0.37, 0.33, 0.34), 3, 7, hood)
    body.cone_to((0, 1.52, -0.10), 0.27, (0.0, 1.70, -0.62), 6, hood)
    body.sphere((0.0, 1.70, -0.64), 0.075, gold, 2, 5)
    body.sphere((-0.10, 1.30, 0.28), 0.05, ink)
    body.sphere((0.10, 1.30, 0.28), 0.05, ink)
    body.sphere((-0.19, 1.22, 0.24), 0.04, blush)
    body.sphere((0.19, 1.22, 0.24), 0.04, blush)

    # lantern on a short chain from the right hand
    body.frustum((0.42, 0.56, 0.30), 0.012, 0.012, 0.08, 4, (0.15, 0.12, 0.15))
    body.box((0.42, 0.535, 0.30), (0.13, 0.025, 0.13), (0.15, 0.12, 0.15))
    body.box((0.42, 0.28, 0.30), (0.12, 0.02, 0.12), (0.15, 0.12, 0.15))
    glow.box((0.42, 0.40, 0.30), (0.10, 0.12, 0.10), (1.0, 0.78, 0.40))
    for sx in (-1, 1):
        for sz in (-1, 1):
            body.box((0.42+sx*0.10, 0.40, 0.30+sz*0.10), (0.012, 0.125, 0.012), (0.15, 0.12, 0.15))
    return [body, glow]

def write_glb(groups, path):
    bin_data = bytearray(); views, accs, prims, mats = [], [], [], []
    def add_view(data):
        while len(bin_data) % 4: bin_data.append(0)
        views.append({"buffer": 0, "byteOffset": len(bin_data), "byteLength": len(data)})
        bin_data.extend(data); return len(views)-1
    for g in groups:
        if not g.pos: continue
        n = len(g.pos)
        def acc(vals, comps, kind, with_minmax=False):
            flat = [x for v in vals for x in v]
            v = add_view(struct.pack('<%df' % len(flat), *flat))
            a = {"bufferView": v, "componentType": 5126, "count": n, "type": kind}
            if with_minmax:
                a["min"] = [min(p[i] for p in vals) for i in range(3)]
                a["max"] = [max(p[i] for p in vals) for i in range(3)]
            accs.append(a); return len(accs)-1
        attrs = {"POSITION": acc(g.pos, 3, "VEC3", True), "NORMAL": acc(g.nrm, 3, "VEC3"), "COLOR_0": acc(g.col, 3, "VEC3")}
        mat = {"name": g.name, "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0.0, "roughnessFactor": 1.0}}
        if g.emissive: mat["emissiveFactor"] = [g.emissive] * 3
        mats.append(mat)
        prims.append({"attributes": attrs, "material": len(mats)-1, "mode": 4})
    doc = {"asset": {"version": "2.0", "generator": "tools/make_player.py"},
           "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"name": "Player", "mesh": 0}],
           "meshes": [{"name": "Player", "primitives": prims}], "materials": mats,
           "accessors": accs, "bufferViews": views, "buffers": [{"byteLength": len(bin_data)}]}
    js = json.dumps(doc, separators=(',', ':')).encode()
    js += b' ' * (-len(js) % 4)
    bin_data += b'\0' * (-len(bin_data) % 4)
    total = 12 + 8 + len(js) + 8 + len(bin_data)
    with open(path, 'wb') as f:
        f.write(struct.pack('<4sII', b'glTF', 2, total))
        f.write(struct.pack('<I4s', len(js), b'JSON')); f.write(js)
        f.write(struct.pack('<I4s', len(bin_data), b'BIN\0')); f.write(bin_data)
    return sum(len(g.pos) for g in groups) // 3

if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else 'assets/models/player.glb'
    print(write_glb(build(), out), 'triangles ->', out)
