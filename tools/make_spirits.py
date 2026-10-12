#!/usr/bin/env python3
"""Builds assets/models/spirit_<id>.glb for all 12 spirits, with no dependencies (no Blender needed).

Every spirit is a handful of named parts (glTF nodes). The node's origin is the part's pivot, and the
game animates a part by its NAME (see `part_xf` in src/scene.rs): wings flap, flames flicker, crowns bob,
eyes blink. A model made in Blender follows the same rule: one object per moving part, origin where it
should pivot, named from the list in the README.

Frame: origin near the middle of the body, +Y up, the spirit faces +Z, body bottom around y = -0.95.
"""
import json, math, struct, sys, os
sys.path.insert(0, os.path.dirname(__file__))
from make_player import Group, TAU, sub, add

class G(Group):
    def ellipsoid(self, c, r, stacks, slices, color):
        """Like Group.ellipsoid, but `color` may be a function (stack, slice) -> colour (for pumpkin ribs)."""
        if not callable(color): return Group.ellipsoid(self, c, r, stacks, slices, color)
        pt = lambda th, ph: (c[0]+r[0]*math.sin(th)*math.cos(ph), c[1]+r[1]*math.cos(th), c[2]+r[2]*math.sin(th)*math.sin(ph))
        for i in range(stacks):
            t0, t1 = math.pi*i/stacks, math.pi*(i+1)/stacks
            for j in range(slices):
                p0, p1 = TAU*j/slices, TAU*(j+1)/slices
                self.quad(pt(t0,p0), pt(t0,p1), pt(t1,p1), pt(t1,p0), c, color(i, j))

class Part:
    """One animatable piece. Built in model space; written relative to `pivot`."""
    def __init__(self, name, pivot=(0, 0, 0)):
        self.name, self.pivot, self.groups = name, pivot, {}
    def g(self, e):
        if e not in self.groups: self.groups[e] = G(self.name, e)
        return self.groups[e]
    def box(self, c, h, col, e=0): self.g(e).box(c, h, col)
    def frustum(self, base, r0, r1, h, seg, col, e=0, rot0=0.0): self.g(e).frustum(base, r0, r1, h, seg, col, rot0)
    def cone_to(self, base, r, tip, seg, col, e=0, rot0=0.0): self.g(e).cone_to(base, r, tip, seg, col, rot0)
    def ellipsoid(self, c, r, col, e=0, stacks=4, slices=7): self.g(e).ellipsoid(c, r, stacks, slices, col)
    def sphere(self, c, r, col, e=0, stacks=3, slices=6): self.g(e).sphere(c, r, col, stacks, slices)

INK, BLUSH, WHITE = (0.08, 0.05, 0.12), (1.0, 0.5, 0.6), (0.97, 0.96, 0.92)
def shade(c, k): return tuple(max(0, min(1, x*k)) for x in c)
def front_z(c, r, x, y):
    """z on an ellipsoid's front surface at (x, y), or None if outside it."""
    q = 1 - (x-c[0])**2/r[0]**2 - (y-c[1])**2/r[1]**2
    return c[2] + r[2]*math.sqrt(q) if q > 0.02 else None

def ghost(p, col, c=(0, 0.4, 0), r=(0.9, 1.0, 0.9), tail=(-0.95, 0.05, 0.82), e=0.55, slices=8):
    """Round head + a tail that narrows to a point: the classic cozy ghost."""
    p.ellipsoid(c, r, col, e, 5, slices)
    base, r0, r1 = tail
    p.frustum((0, base, 0), r0, r1, c[1]-base, slices, col, e)

def face(parts, body, c, r, y=0.55, dx=0.3, er=0.16, blush=True, by=-0.25, bdx=0.52, e=0.0):
    """Two eyes (their own part, so they can blink) + blush on the body."""
    eyes = Part("eyes", (0, y, 0)); parts.append(eyes)
    for sx in (-1, 1):
        z = front_z(c, r, sx*dx, y) or 0.8
        eyes.sphere((sx*dx, y, z-0.02), er, INK)
        eyes.sphere((sx*dx+er*0.35, y+er*0.4, z+er*0.7), er*0.28, WHITE, 0.6, 2, 4)
        if blush:
            zb = front_z(c, r, sx*bdx, y+by) or 0.6
            body.sphere((sx*bdx, y+by, zb-0.04), 0.1, BLUSH, 0.4)

# -------------------------------------------------------------------------------------- spirits

def pip():
    col = (1.0, 0.62, 0.25); rib = (0.92, 0.50, 0.17)
    body, parts = Part("squish", (0, -0.85, 0)), []
    parts.append(body)
    c, r = (0, 0.15, 0), (1.0, 0.82, 1.0)
    body.ellipsoid(c, r, lambda i, j: col if j % 2 == 0 else rib, 0.5, 5, 10)
    body.frustum((0, -0.95, 0), 0.05, 0.40, 0.45, 8, col, 0.5)
    face(parts, body, c, r, y=0.3, dx=0.3, bdx=0.56, by=-0.28)
    stem = Part("sway", (0, 0.9, 0)); parts.append(stem)
    stem.frustum((0, 0.88, 0), 0.12, 0.09, 0.38, 5, (0.30, 0.60, 0.25))
    stem.ellipsoid((0.27, 1.12, 0), (0.30, 0.05, 0.12), (0.36, 0.70, 0.30), 0.1, 2, 6)
    stem.sphere((0.55, 1.1, 0), 0.07, (0.36, 0.70, 0.30))
    return parts

def mizzle():
    col = (0.50, 0.80, 1.0)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, -0.05, 0), (0.88, 0.80, 0.88)
    body.ellipsoid(c, r, col, 0.55, 5, 8)
    face(parts, body, c, r, y=0.0, dx=0.3, by=-0.3)
    body.ellipsoid((-0.38, 0.4, 0.5), (0.14, 0.2, 0.08), (0.85, 0.95, 1.0), 0.9, 2, 5)  # shine
    tip = Part("sway", (0, 0.55, 0)); parts.append(tip)
    tip.cone_to((0, 0.5, 0), 0.5, (0, 1.4, 0), 8, col, 0.55)
    cloud = Part("bob", (0, 2.0, 0)); parts.append(cloud)
    wh = (0.93, 0.96, 1.0)
    for x, y, rr in ((-0.35, 1.95, 0.3), (0.0, 2.08, 0.4), (0.38, 1.96, 0.32), (0.05, 1.9, 0.34)):
        cloud.sphere((x, y, 0), rr, wh, 0.5, 3, 6)
    for k, x in enumerate((-0.25, 0.05, 0.3)):
        d = Part("dripfall%d" % k, (x, 1.6, 0.05*k)); parts.append(d)
        d.ellipsoid((x, 1.6, 0.05*k), (0.06, 0.1, 0.06), (0.55, 0.85, 1.0), 0.9, 2, 5)
    return parts

def hush():
    col = (0.80, 0.70, 1.0); wing = (0.92, 0.86, 1.0)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.35, 0), (0.66, 0.8, 0.66)
    ghost(body, col, c, r, (-0.95, 0.05, 0.62))
    body.ellipsoid((0, 0.05, 0), (0.8, 0.25, 0.8), (0.97, 0.95, 1.0), 0.6, 3, 8)  # fluffy collar
    face(parts, body, c, r, y=0.42, dx=0.27, er=0.2, by=-0.28, bdx=0.46)
    for sx, s in ((-1, "l"), (1, "r")):
        up = Part("flap_" + s, (sx*0.4, 0.45, -0.1)); parts.append(up)
        up.ellipsoid((sx*1.15, 0.75, -0.15), (0.8, 0.12, 0.55), wing, 0.6, 3, 8)
        up.ellipsoid((sx*1.3, 0.87, -0.15), (0.24, 0.1, 0.24), (0.62, 0.48, 0.92), 0.7, 2, 6)  # eyespot
        lo = Part("flaplow_" + s, (sx*0.35, 0.15, -0.15)); parts.append(lo)
        lo.ellipsoid((sx*0.85, 0.0, -0.3), (0.55, 0.1, 0.38), shade(wing, 0.94), 0.6, 3, 7)
        an = Part("twitch_" + s, (sx*0.2, 1.05, 0.1)); parts.append(an)
        an.cone_to((sx*0.2, 1.05, 0.1), 0.05, (sx*0.55, 1.65, 0.1), 5, (0.95, 0.9, 1.0), 0.6)
        an.sphere((sx*0.56, 1.67, 0.1), 0.08, (1, 0.9, 1), 0.9, 2, 5)
    return parts

def luna():
    col = (1.0, 0.95, 0.70)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.5, 0), (0.78, 0.92, 0.78)
    body.ellipsoid(c, r, col, 0.55, 5, 8)
    body.frustum((0, -1.0, 0), 1.0, 0.62, 1.4, 10, col, 0.55)       # a flared gown
    body.frustum((0, -1.0, 0), 1.03, 1.0, 0.1, 10, (1.0, 0.82, 0.38), 0.8)  # golden hem
    face(parts, body, c, r, y=0.55, dx=0.28, by=-0.25)
    crown = Part("bob", (0, 1.5, 0)); parts.append(crown)
    crown.frustum((0, 1.35, 0), 0.5, 0.5, 0.12, 10, (1.0, 0.82, 0.38), 1.0)
    for k in range(5):
        a = TAU*k/5
        crown.cone_to((math.cos(a)*0.42, 1.47, math.sin(a)*0.42), 0.13, (math.cos(a)*0.48, 1.95, math.sin(a)*0.48), 4, (1.0, 0.88, 0.45), 1.0)
        crown.sphere((math.cos(a)*0.48, 1.97, math.sin(a)*0.48), 0.06, (1, 1, 0.9), 1.0, 2, 4)
    halo = Part("spin", (0, 2.3, 0)); parts.append(halo)
    for k in range(12):
        a = TAU*k/12
        halo.box((math.cos(a)*0.85, 2.3, math.sin(a)*0.85), (0.07, 0.04, 0.07), (1.0, 0.97, 0.8), 1.0)
    return parts

def wick():
    col = (0.96, 0.90, 0.76)
    body, parts = Part("body"), []
    parts.append(body)
    body.frustum((0, -0.9, 0), 0.76, 0.76, 1.85, 10, col, 0.45)
    body.frustum((0, -0.95, 0), 0.98, 0.80, 0.28, 10, (0.92, 0.84, 0.64), 0.4)   # melted pool
    body.ellipsoid((0, 0.95, 0), (0.77, 0.12, 0.77), (1.0, 0.96, 0.82), 0.55, 2, 10)  # rounded top
    for x, z, h in ((0.55, 0.52, 0.5), (-0.7, 0.3, 0.38), (0.1, -0.76, 0.42)):
        body.ellipsoid((x, 0.8-h*0.45, z), (0.12, h, 0.12), (1.0, 0.95, 0.8), 0.5, 3, 5)   # wax drips
    face(parts, body, (0, 0.1, 0), (0.76, 0.9, 0.76), y=0.3, dx=0.28, by=-0.25, bdx=0.5)
    fl = Part("flicker", (0, 1.0, 0)); parts.append(fl)
    fl.frustum((0, 0.98, 0), 0.04, 0.04, 0.22, 4, (0.2, 0.15, 0.12))
    fl.cone_to((0, 1.15, 0), 0.22, (0, 1.9, 0), 6, (1.0, 0.70, 0.16), 1.0)
    fl.ellipsoid((0, 1.35, 0), (0.12, 0.2, 0.12), (1.0, 0.96, 0.62), 1.0, 3, 6)
    return parts

def drip():
    col = (0.45, 0.72, 0.78)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.1, 0), (0.95, 0.85, 0.95)
    body.ellipsoid(c, r, col, 0.55, 5, 8)
    for k in range(6):
        a = TAU*k/6 + 0.3
        body.sphere((math.cos(a)*0.68, -0.72, math.sin(a)*0.68), 0.26, col, 0.55, 3, 6)   # melted hem
    body.frustum((0, -0.82, 0), 0.55, 0.62, 0.1, 8, col, 0.55)
    face(parts, body, c, r, y=0.2, dx=0.32, by=-0.25, bdx=0.58)
    body.ellipsoid((-0.45, 0.62, 0.55), (0.2, 0.1, 0.1), (0.85, 0.97, 1.0), 0.9, 2, 5)  # shine
    curl = Part("sway", (0, 0.85, 0)); parts.append(curl)
    curl.cone_to((0, 0.8, 0), 0.26, (0.18, 1.45, 0.05), 6, col, 0.6)
    curl.sphere((0.2, 1.48, 0.05), 0.07, (0.7, 0.92, 1.0), 0.9, 2, 5)
    orb = Part("spin", (0, 0.2, 0)); parts.append(orb)
    for k in range(3):
        a = TAU*k/3
        orb.ellipsoid((math.cos(a)*1.35, 0.2+0.25*math.sin(a*2), math.sin(a)*1.35), (0.11, 0.15, 0.11), (0.65, 0.88, 1.0), 0.9, 2, 5)
    for k, a in enumerate((0.9, 3.6)):
        x, z = math.cos(a)*0.75, math.sin(a)*0.75
        d = Part("dripfall%d" % k, (x, -0.7, z)); parts.append(d)
        d.ellipsoid((x, -0.78, z), (0.07, 0.11, 0.07), (0.65, 0.9, 1.0), 0.9, 2, 5)
    return parts

def gloam():
    col = (0.60, 0.50, 0.85); dark = (0.30, 0.22, 0.48)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.4, 0), (0.8, 0.9, 0.8)
    ghost(body, col, c, r, (-0.95, 0.05, 0.74), 0.4)
    face(parts, body, c, r, y=0.5, dx=0.28, er=0.14, by=-0.3, bdx=0.5)
    for sx in (-1, 1):
        body.cone_to((sx*0.1, 0.1, 0.78), 0.045, (sx*0.1, -0.05, 0.79), 4, WHITE, 0.4)   # little fangs
        ear = Part("twitch_" + ("l" if sx < 0 else "r"), (sx*0.4, 1.1, 0)); parts.append(ear)
        ear.cone_to((sx*0.4, 1.1, 0), 0.2, (sx*0.55, 1.65, 0), 4, dark)
        w = Part("bat_" + ("l" if sx < 0 else "r"), (sx*0.65, 0.65, -0.1)); parts.append(w)
        w.ellipsoid((sx*1.4, 0.5, -0.2), (0.82, 0.06, 0.5), shade(dark, 1.15), 0.3, 2, 8)
        w.cone_to((sx*0.65, 0.66, -0.1), 0.06, (sx*2.15, 0.98, -0.2), 4, dark)     # wing bone
        for k, (x, z) in enumerate(((1.7, 0.2), (1.9, -0.2), (2.0, -0.55))):
            w.cone_to((sx*(x-0.3), 0.45, z-0.1), 0.05, (sx*x, 0.35, z+0.2), 4, dark)  # fingers
    return parts

def marrow():
    col = (0.93, 0.92, 0.82)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.55, 0), (0.85, 1.0, 0.85)
    ghost(body, col, c, r, (-0.95, 0.05, 0.8), 0.55)
    face(parts, body, c, r, y=0.62, dx=0.3, er=0.12, blush=False)
    for sx in (-1, 1):
        zb = front_z(c, r, sx*0.3, 0.85) or 0.8
        body.box((sx*0.3, 0.85, zb-0.03), (0.15, 0.03, 0.04), (0.55, 0.55, 0.6))      # bushy brows
        zm = front_z(c, r, sx*0.14, 0.3) or 0.8
        body.ellipsoid((sx*0.15, 0.3, zm-0.03), (0.17, 0.06, 0.08), (0.99, 0.99, 0.96), 0.5, 2, 5)  # moustache
    zb = front_z(c, r, 0, -0.1) or 0.8
    body.box((-0.19, -0.1, zb), (0.14, 0.1, 0.05), (0.72, 0.15, 0.2)); body.box((0.19, -0.1, zb), (0.14, 0.1, 0.05), (0.72, 0.15, 0.2))
    body.box((0, -0.1, zb+0.02), (0.06, 0.06, 0.05), (0.55, 0.1, 0.15))                # bowtie
    hat = Part("tip", (-0.7, 1.5, 0)); parts.append(hat)
    dk = (0.17, 0.12, 0.25)
    hat.frustum((0, 1.46, 0), 0.8, 0.8, 0.07, 10, dk)
    hat.frustum((0, 1.5, 0), 0.47, 0.44, 0.78, 10, dk)
    hat.frustum((0, 1.6, 0), 0.5, 0.5, 0.14, 10, (1.0, 0.80, 0.30), 0.8)
    return parts

def bobbin():
    col = (0.55, 0.90, 0.55)
    body, parts = Part("squish", (0, -0.85, 0)), []
    parts.append(body)
    c, r = (0, -0.05, 0), (1.1, 0.8, 0.95)
    body.ellipsoid(c, r, col, 0.5, 5, 9)
    body.ellipsoid((0, -0.2, 0.4), (0.7, 0.5, 0.6), (0.82, 1.0, 0.72), 0.6, 4, 8)         # belly
    body.box((0, -0.02, 0.93), (0.5, 0.025, 0.03), INK)                                      # wide mouth
    for sx in (-1, 1):
        body.ellipsoid((sx*0.55, -0.82, 0.5), (0.3, 0.09, 0.36), shade(col, 0.85), 0.4, 2, 6)   # feet
        zb = front_z(c, r, sx*0.78, -0.05) or 0.6
        body.sphere((sx*0.78, -0.05, zb-0.05), 0.1, BLUSH, 0.4)
    eyes = Part("eyes", (0, 0.75, 0.3)); parts.append(eyes)
    for sx in (-1, 1):
        eyes.sphere((sx*0.46, 0.75, 0.25), 0.3, (0.96, 1.0, 0.92), 0.3, 3, 6)
        eyes.sphere((sx*0.46, 0.78, 0.5), 0.14, INK, 0, 2, 5)
        eyes.sphere((sx*0.5, 0.84, 0.58), 0.045, WHITE, 0.7, 2, 4)
    th = Part("pulse", (0, -0.4, 0.8)); parts.append(th)
    th.sphere((0, -0.42, 0.78), 0.26, (0.9, 1.0, 0.72), 0.6, 3, 6)
    return parts

def pondle():
    col = (0.45, 0.75, 0.95)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.3, 0), (0.85, 0.95, 0.8)
    ghost(body, col, c, r, (-0.95, 0.05, 0.75))
    face(parts, body, c, r, y=0.4, dx=0.3, by=-0.25)
    for sx, s in ((-1, "l"), (1, "r")):
        f = Part("fin_" + s, (sx*0.8, 0.05, 0)); parts.append(f)
        f.ellipsoid((sx*1.15, 0.05, 0), (0.45, 0.09, 0.3), (0.62, 0.86, 1.0), 0.6, 2, 7)
        w = Part("twitch_" + s, (sx*0.25, 0.1, 0.72)); parts.append(w)
        w.cone_to((sx*0.25, 0.1, 0.72), 0.04, (sx*0.7, -0.1, 0.95), 4, (0.35, 0.6, 0.8), 0.3)
    tail = Part("tail", (0, -0.8, 0)); parts.append(tail)
    tail.ellipsoid((0, -1.05, 0), (0.12, 0.3, 0.5), (0.62, 0.86, 1.0), 0.6, 3, 6)
    pad = Part("sway", (0, 1.2, 0)); parts.append(pad)
    pad.frustum((0.0, 1.17, 0), 0.72, 0.7, 0.08, 10, (0.28, 0.62, 0.34), 0.1)
    pad.cone_to((0.1, 1.24, 0), 0.2, (0.1, 1.68, 0), 6, (1.0, 0.72, 0.86), 0.6)
    pad.cone_to((-0.1, 1.24, 0.05), 0.16, (-0.1, 1.55, 0.05), 6, (1.0, 0.82, 0.92), 0.7)
    return parts

def reed():
    col = (0.75, 0.88, 0.80)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.5, 0), (0.62, 1.05, 0.62)
    ghost(body, col, c, r, (-0.95, 0.05, 0.55))
    face(parts, body, c, r, y=0.65, dx=0.24, er=0.14, by=-0.25, bdx=0.42)
    st = Part("sway", (0, 1.45, 0)); parts.append(st)
    st.frustum((0, 1.4, 0), 0.06, 0.04, 1.0, 5, (0.35, 0.48, 0.26))
    st.frustum((0, 1.95, 0), 0.16, 0.16, 0.55, 8, (0.45, 0.28, 0.15))
    st.cone_to((0, 2.5, 0), 0.16, (0, 2.78, 0), 6, (0.45, 0.28, 0.15))
    for sx, s in ((-1, "l"), (1, "r")):
        lf = Part("sway2_" + s, (sx*0.15, 1.3, 0)); parts.append(lf)
        lf.cone_to((sx*0.15, 1.3, 0), 0.09, (sx*0.85, 2.05, 0), 5, (0.40, 0.62, 0.34))
        wg = Part("fin_" + s, (sx*0.5, 0.6, -0.05)); parts.append(wg)
        wg.ellipsoid((sx*1.0, 0.65, -0.1), (0.55, 0.08, 0.4), (0.92, 0.98, 0.94), 0.5, 2, 7)
    return parts

def lotus():
    col = (1.0, 0.72, 0.85)
    body, parts = Part("body"), []
    parts.append(body)
    c, r = (0, 0.4, 0), (0.9, 1.0, 0.9)
    ghost(body, col, c, r)
    face(parts, body, c, r, y=0.5, dx=0.3, by=-0.25)
    outer = Part("bloom", (0, 1.15, 0)); parts.append(outer)
    for k in range(8):
        a = TAU*k/8
        outer.cone_to((math.cos(a)*0.4, 1.1, math.sin(a)*0.4), 0.22, (math.cos(a)*0.85, 1.85, math.sin(a)*0.85), 4, (1.0, 0.70, 0.84), 0.7)
    inner = Part("bloom2", (0, 1.25, 0)); parts.append(inner)
    for k in range(5):
        a = TAU*k/5 + 0.3
        inner.cone_to((math.cos(a)*0.22, 1.25, math.sin(a)*0.22), 0.16, (math.cos(a)*0.4, 1.95, math.sin(a)*0.4), 4, (1.0, 0.93, 0.96), 1.0)
    fl = Part("spin", (0, 0.5, 0)); parts.append(fl)
    for k in range(3):
        a = TAU*k/3 + 0.5
        fl.sphere((math.cos(a)*1.5, 0.7+0.3*k*0.4, math.sin(a)*1.5), 0.09, (1.0, 0.9, 0.95), 1.0, 2, 5)
    return parts

SPIRITS = {"pip": pip, "mizzle": mizzle, "hush": hush, "luna": luna, "wick": wick, "drip": drip,
           "gloam": gloam, "marrow": marrow, "bobbin": bobbin, "pondle": pondle, "reed": reed, "lotus": lotus}

# -------------------------------------------------------------------------------------- glb writer

def write_glb(parts, name, path):
    bin_data = bytearray(); views, accs, meshes, nodes, mats, mat_of = [], [], [], [], [], {}
    def add_view(data):
        while len(bin_data) % 4: bin_data.append(0)
        views.append({"buffer": 0, "byteOffset": len(bin_data), "byteLength": len(data)})
        bin_data.extend(data); return len(views)-1
    def material(e):
        if e not in mat_of:
            m = {"name": "glow" if e else "matte", "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0.0, "roughnessFactor": 1.0}}
            if e: m["emissiveFactor"] = [e, e, e]
            mats.append(m); mat_of[e] = len(mats)-1
        return mat_of[e]
    tris = 0
    for p in parts:
        prims = []
        px, py, pz = p.pivot
        for e, g in sorted(p.groups.items()):
            if not g.pos: continue
            n = len(g.pos); tris += n // 3
            def acc(vals, minmax=False):
                flat = [x for v in vals for x in v]
                v = add_view(struct.pack('<%df' % len(flat), *flat))
                a = {"bufferView": v, "componentType": 5126, "count": n, "type": "VEC3"}
                if minmax:
                    a["min"] = [min(q[i] for q in vals) for i in range(3)]; a["max"] = [max(q[i] for q in vals) for i in range(3)]
                accs.append(a); return len(accs)-1
            rel = [(q[0]-px, q[1]-py, q[2]-pz) for q in g.pos]
            prims.append({"attributes": {"POSITION": acc(rel, True), "NORMAL": acc(g.nrm), "COLOR_0": acc(g.col)}, "material": material(e), "mode": 4})
        if not prims: continue
        meshes.append({"name": p.name, "primitives": prims})
        nodes.append({"name": p.name, "mesh": len(meshes)-1, "translation": [px, py, pz]})
    root = {"name": name, "children": list(range(1, len(nodes)+1))}
    doc = {"asset": {"version": "2.0", "generator": "tools/make_spirits.py"}, "scene": 0, "scenes": [{"nodes": [0]}],
           "nodes": [root] + nodes, "meshes": meshes, "materials": mats, "accessors": accs, "bufferViews": views,
           "buffers": [{"byteLength": len(bin_data)}]}
    js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
    bin_data += b'\0' * (-len(bin_data) % 4)
    with open(path, 'wb') as f:
        f.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + 8 + len(bin_data)))
        f.write(struct.pack('<I4s', len(js), b'JSON')); f.write(js)
        f.write(struct.pack('<I4s', len(bin_data), b'BIN\0')); f.write(bin_data)
    return tris

if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else 'assets/models'
    os.makedirs(out, exist_ok=True)
    for name, fn in SPIRITS.items():
        print('%-7s %4d triangles' % (name, write_glb(fn(), name, os.path.join(out, 'spirit_%s.glb' % name))))
