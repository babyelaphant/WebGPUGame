#!/usr/bin/env python3
"""Builds the environment models: assets/models/prop_<name>.glb (trees, rocks, graves, buildings...).

No Blender needed. Conventions (same as the spirits): +Y up, models stand on y = 0, +Z is the front.
A part whose name starts with "tint" has its colour multiplied by a colour the game picks per instance
(leaf colour, rock tone, mushroom cap...); keep those parts light/neutral. Every other part keeps its colour.
Materials with a non-zero emissive (windows, berries, glow bands) are drawn self-lit.
"""
import math, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from make_spirits import Part, G, write_glb, TAU

PI = math.pi

def hn(seed, i, j=0):
    x = math.sin(i*127.1 + j*311.7 + seed*74.7) * 43758.5453
    return x - math.floor(x)

def shade(c, k): return tuple(max(0.0, min(1.0, x*k)) for x in c)

class P(Part):
    def rock(self, c, r, stacks, slices, col, seed, amp=0.22, e=0):
        """A faceted, lumpy ellipsoid: every ring vertex is pushed in or out a little."""
        g = self.g(e)
        def pt(i, j):
            j %= slices
            k = 1 + amp*(hn(seed, i if 0 < i < stacks else 0, j if 0 < i < stacks else 0) - 0.5)*2
            th, ph = PI*i/stacks, TAU*j/slices
            return (c[0]+r[0]*k*math.sin(th)*math.cos(ph), c[1]+r[1]*k*math.cos(th), c[2]+r[2]*k*math.sin(th)*math.sin(ph))
        for i in range(stacks):
            for j in range(slices):
                shd = 0.88 + 0.12*hn(seed+3, i, j)
                g.quad(pt(i, j), pt(i, j+1), pt(i+1, j+1), pt(i+1, j), c, shade(col, shd))

def model(name, parts, out):
    return write_glb(parts, name, os.path.join(out, 'prop_%s.glb' % name))

WHITE = (1.0, 1.0, 1.0)
BARK, BARK_D = (0.27, 0.19, 0.16), (0.20, 0.14, 0.12)

# ------------------------------------------------------------------ trees (reference height 4)

def pine():
    trunk, leaves = P("body"), P("tint")
    trunk.frustum((0, 0, 0), 0.32, 0.24, 1.5, 6, BARK)
    for k, (y, r, h) in enumerate(((0.75, 1.40, 1.35), (1.45, 1.12, 1.25), (2.15, 0.84, 1.15), (2.8, 0.55, 1.0))):
        c = 0.80 + 0.07*k
        leaves.cone_to((0, y, 0), r, (0.04*k, y+h, 0.0), 8, shade(WHITE, c), rot0=0.2*k)
    return [trunk, leaves]

def round_tree():
    trunk, crown, top = P("body"), P("tint"), P("tint2")
    trunk.frustum((0, 0, 0), 0.34, 0.26, 2.2, 6, BARK)
    trunk.cone_to((0.1, 1.5, 0), 0.12, (0.9, 2.5, 0.2), 4, BARK_D)
    trunk.cone_to((-0.1, 1.4, 0), 0.12, (-0.8, 2.4, -0.2), 4, BARK_D)
    crown.ellipsoid((0, 2.72, 0), (1.35, 1.0, 1.35), shade(WHITE, 0.87), 0, 4, 7)
    crown.ellipsoid((-0.95, 2.45, 0.35), (0.75, 0.6, 0.75), shade(WHITE, 0.78), 0, 3, 6)
    crown.ellipsoid((0.85, 2.4, -0.5), (0.7, 0.55, 0.7), shade(WHITE, 0.82), 0, 3, 6)
    top.ellipsoid((0.3, 3.52, -0.15), (0.85, 0.65, 0.85), WHITE, 0, 3, 6)
    return [trunk, crown, top]

def dead_tree():
    bark = (0.24, 0.19, 0.22)
    t, tw = P("body"), P("body2")
    t.frustum((0, 0, 0), 0.30, 0.15, 3.1, 5, bark)
    t.frustum((0, 3.1, 0), 0.14, 0.04, 1.0, 5, bark)
    for b in range(4):
        a = TAU*b/4 + 0.4
        y0 = 1.7 + 0.4*b
        ca, sa = math.cos(a), math.sin(a)
        t.cone_to((ca*0.1, y0, sa*0.1), 0.11, (ca*0.65, y0+0.55, sa*0.65), 4, bark)
        t.cone_to((ca*0.5, y0+0.45, sa*0.5), 0.07, (ca*1.15, y0+1.05, sa*1.15), 4, bark)
        t.cone_to((ca*0.95, y0+0.9, sa*0.95), 0.03, (ca*1.45, y0+1.1, sa*1.45+0.2), 3, bark)
        tw.ellipsoid((ca*1.1, y0+1.0, sa*1.1), (0.42, 0.22, 0.42), (0.17, 0.13, 0.22), 0, 2, 5)
    return [t, tw]

def willow():
    trunk, crown = P("body"), P("tint")
    trunk.frustum((0, 0, 0), 0.40, 0.30, 3.0, 6, (0.30, 0.23, 0.18))
    trunk.cone_to((0.2, 2.2, 0), 0.14, (1.1, 3.4, 0.3), 4, (0.30, 0.23, 0.18))
    crown.ellipsoid((0, 3.9, 0), (1.55, 0.85, 1.55), shade(WHITE, 0.95), 0, 3, 8)
    for k in range(9):
        a = TAU*k/9
        cx, cz = math.cos(a)*1.35, math.sin(a)*1.35
        c = shade(WHITE, 0.88 + 0.07*hn(5, k))
        crown.frustum((cx, 2.1, cz), 0.04, 0.34, 1.9, 5, c)
        crown.frustum((cx*1.12, 1.4, cz*1.12), 0.02, 0.05, 0.7, 4, shade(c, 0.9))
    return [trunk, crown]

# ------------------------------------------------------------------ rocks, mushrooms, graves

def rock0():
    p = P("tint"); p.rock((0, 0.6, 0), (1, 1, 1), 4, 7, WHITE, 1); return [p]

def rock1():
    p, moss = P("tint"), P("body")
    p.rock((0, 0.6, 0), (1, 1, 1), 4, 7, WHITE, 2)
    p.rock((0.95, 0.3, 0.35), (0.45, 0.4, 0.42), 3, 6, shade(WHITE, 0.92), 3)
    moss.ellipsoid((-0.05, 1.42, 0.0), (0.58, 0.26, 0.55), (0.30, 0.44, 0.27), 0, 2, 6)
    return [p, moss]

def rock2():
    p = P("tint"); p.rock((0, 0.45, 0), (1.15, 0.7, 0.9), 3, 8, WHITE, 4, 0.18)
    p.rock((-0.9, 0.18, 0.7), (0.3, 0.22, 0.28), 2, 5, shade(WHITE, 0.9), 5)
    p.rock((0.8, 0.15, -0.8), (0.24, 0.18, 0.24), 2, 5, shade(WHITE, 0.95), 6)
    return [p]

def pebble():
    p = P("body"); p.rock((0, 0.15, 0), (0.35, 0.2, 0.3), 2, 6, (0.5, 0.48, 0.56), 7, 0.15); return [p]

def shroom():
    stem, cap, spots = P("body"), P("tint"), P("spots")
    stem.frustum((0, 0, 0), 0.06, 0.045, 0.27, 5, (0.92, 0.89, 0.80))
    cap.ellipsoid((0, 0.28, 0), (0.22, 0.11, 0.22), WHITE, 0.65, 2, 6)
    stem.frustum((0.2, 0, 0.12), 0.04, 0.03, 0.15, 5, (0.92, 0.89, 0.80))
    cap.ellipsoid((0.2, 0.16, 0.12), (0.13, 0.065, 0.13), shade(WHITE, 0.92), 0.65, 2, 6)
    for a, rr in ((0.5, 0.09), (2.2, 0.12), (4.0, 0.08), (5.3, 0.11)):
        spots.sphere((math.cos(a)*rr, 0.37, math.sin(a)*rr), 0.028, (1.0, 0.98, 0.9), 0.5, 2, 4)
    return [stem, cap, spots]

def grave(kind):
    def f():
        mound, st = P("body"), P("tint")
        mound.ellipsoid((0, 0.05, 0.75), (0.42, 0.12, 0.8), (0.22, 0.20, 0.20), 0, 2, 6)
        moss = (0.27, 0.40, 0.26)
        if kind == 0:
            st.box((0, 0.4, 0), (0.38, 0.4, 0.09), WHITE); st.ellipsoid((0, 0.8, 0), (0.38, 0.3, 0.09), WHITE, 0, 3, 8)
            st.box((0, 0.62, 0.095), (0.04, 0.2, 0.01), shade(WHITE, 0.62)); st.box((0, 0.7, 0.095), (0.13, 0.035, 0.01), shade(WHITE, 0.62))
            mound.box((0.2, 0.06, 0.14), (0.16, 0.06, 0.05), moss)
        elif kind == 1:
            st.box((0, 0.1, 0), (0.22, 0.1, 0.16), shade(WHITE, 0.92))
            st.box((0, 0.65, 0), (0.09, 0.55, 0.08), WHITE); st.box((0, 0.88, 0), (0.34, 0.08, 0.08), WHITE)
            mound.box((-0.12, 0.2, 0.1), (0.08, 0.12, 0.02), moss)
        elif kind == 2:
            st.box((0, 0.3, 0), (0.52, 0.3, 0.1), WHITE); st.box((0, 0.62, 0), (0.56, 0.05, 0.12), shade(WHITE, 0.9))
            st.box((0, 0.35, 0.105), (0.34, 0.03, 0.01), shade(WHITE, 0.62)); st.box((0, 0.24, 0.105), (0.24, 0.03, 0.01), shade(WHITE, 0.62))
            mound.box((0.38, 0.35, 0.0), (0.12, 0.3, 0.11), moss)
        else:
            st.box((0, 0.12, 0), (0.45, 0.12, 0.45), shade(WHITE, 0.88))
            st.frustum((0, 0.24, 0), 0.34, 0.14, 1.2, 4, WHITE, rot0=0.785)
            st.cone_to((0, 1.44, 0), 0.15, (0, 1.72, 0), 4, WHITE, rot0=0.785)
        return [mound, st]
    return f

# ------------------------------------------------------------------ plants for treats, the pond

def bush(ready):
    def f():
        b, parts = P("body"), []
        b.ellipsoid((0, 0.42, 0), (0.85, 0.55, 0.85), (0.16, 0.34, 0.21) if ready else (0.12, 0.20, 0.17), 0, 3, 7)
        for a, (rx, ry) in ((0.4, (0.5, 0.4)), (2.5, (0.45, 0.35)), (4.4, (0.5, 0.42))):
            col = (0.19, 0.38, 0.24) if ready else (0.14, 0.23, 0.19)
            b.ellipsoid((math.cos(a)*0.55, 0.35, math.sin(a)*0.55), (rx, ry, rx), col, 0, 2, 6)
        parts.append(b)
        if ready:
            for k in range(4):
                a = TAU*k/4 + 0.4
                c = (math.cos(a)*0.5, 0.78, math.sin(a)*0.5)
                br = P("tintberry%d" % k, c); br.sphere(c, 0.13, WHITE, 0.85, 3, 6); parts.append(br)
        return parts
    return f

def pad(flower):
    def f():
        p = P("body")
        p.frustum((0, 0, 0), 0.55, 0.53, 0.03, 8, (0.22, 0.55, 0.28))
        p.frustum((0, 0.03, 0), 0.30, 0.29, 0.005, 8, (0.28, 0.62, 0.34))
        p.cone_to((0.4, 0.03, 0.0), 0.12, (0.52, 0.03, 0.0), 3, (0.20, 0.45, 0.30))
        if flower:
            for k in range(6):
                a = TAU*k/6
                p.cone_to((math.cos(a)*0.1, 0.03, math.sin(a)*0.1), 0.1, (math.cos(a)*0.2, 0.3, math.sin(a)*0.2), 4, (1.0, 0.72, 0.86), 0.6)
            p.sphere((0, 0.14, 0), 0.07, (1.0, 0.9, 0.5), 0.8, 2, 5)
        return [p]
    return f

def reed():
    r = P("body")
    r.frustum((0, 0, 0), 0.045, 0.03, 1.0, 4, (0.35, 0.48, 0.26))
    r.frustum((0, 0.55, 0), 0.095, 0.085, 0.3, 5, (0.45, 0.28, 0.15))
    r.cone_to((0, 0.85, 0), 0.085, (0, 0.95, 0), 5, (0.45, 0.28, 0.15))
    r.cone_to((0.03, 0.05, 0), 0.05, (0.35, 0.65, 0.05), 3, (0.38, 0.55, 0.28))
    return [r]

def pier():
    p = P("body")
    for k in range(7):
        sh = 0.9 + 0.1*hn(9, k)
        p.box((-0.3 - k*0.62, 0.3, 0.0), (0.3, 0.05, 0.95), shade((0.46, 0.33, 0.22), sh))
    for k in (0.0, 2.4, 4.0):
        for sz in (-0.95, 0.95):
            p.frustum((-k, 0, sz), 0.08, 0.07, 0.62, 5, (0.30, 0.22, 0.16))
    p.box((-2.2, 0.2, -0.98), (2.4, 0.04, 0.05), (0.34, 0.25, 0.18))
    p.box((-2.2, 0.2, 0.98), (2.4, 0.04, 0.05), (0.34, 0.25, 0.18))
    return [p]

def boat():
    b = P("body")
    b.ellipsoid((0, 0.25, 0), (1.5, 0.3, 0.6), (0.45, 0.30, 0.20), 0, 3, 8)
    b.ellipsoid((0, 0.42, 0), (1.25, 0.2, 0.42), (0.18, 0.12, 0.10), 0, 2, 8)
    for x in (-0.4, 0.45):
        b.box((x, 0.46, 0), (0.08, 0.03, 0.44), (0.52, 0.38, 0.26))
    b.cone_to((-0.6, 0.5, 0.1), 0.03, (0.4, 0.6, 0.55), 3, (0.5, 0.38, 0.26))
    b.sphere((1.45, 0.3, 0.0), 0.08, (0.8, 0.7, 0.5), 0, 2, 5)
    return [b]

# ------------------------------------------------------------------ posts and stones

def rune_stone():
    s, band = P("body"), P("tint")
    s.frustum((0, 0, 0), 0.7, 0.66, 0.22, 6, (0.34, 0.34, 0.42))
    s.frustum((0, 0.22, 0), 0.55, 0.5, 1.13, 6, (0.40, 0.40, 0.48))
    band.frustum((0, 0.95, 0), 0.58, 0.58, 0.1, 6, WHITE, 0.35)
    s.frustum((0, 1.35, 0), 0.62, 0.55, 0.12, 6, (0.44, 0.44, 0.52))
    s.frustum((0, 1.47, 0), 0.5, 0.3, 0.1, 6, (0.40, 0.40, 0.48))
    return [s, band]

def lantern_post():
    s = P("body")
    s.frustum((0, 0, 0), 0.34, 0.28, 0.25, 8, (0.45, 0.45, 0.52))
    s.frustum((0, 0.25, 0), 0.08, 0.07, 1.95, 6, (0.20, 0.17, 0.22))
    s.frustum((0, 0.5, 0), 0.14, 0.14, 0.08, 6, (0.30, 0.26, 0.32))
    s.frustum((0, 1.4, 0), 0.12, 0.12, 0.06, 6, (0.30, 0.26, 0.32))
    s.frustum((0, 1.85, 0), 0.14, 0.1, 0.14, 6, (0.30, 0.26, 0.32))
    return [s]

def lily_post():
    s = P("body")
    s.frustum((0, 0, 0), 0.5, 0.45, 0.25, 7, (0.40, 0.42, 0.46))
    s.frustum((0, 0.25, 0), 0.08, 0.06, 1.25, 5, (0.22, 0.42, 0.28))
    s.frustum((0, 0.9, 0), 0.28, 0.26, 0.06, 7, (0.25, 0.55, 0.30))
    s.cone_to((0.05, 0.6, 0), 0.05, (0.45, 0.8, 0.1), 3, (0.25, 0.5, 0.3))
    s.cone_to((-0.05, 0.5, 0), 0.05, (-0.45, 0.7, -0.1), 3, (0.25, 0.5, 0.3))
    return [s]

# ------------------------------------------------------------------ buildings

def pumpkin(p, x, z, s):
    p.ellipsoid((x, 0.38*s, z), (0.5*s, 0.38*s, 0.5*s), lambda i, j: (0.95, 0.45, 0.10) if j % 2 == 0 else (0.86, 0.38, 0.07), 0.12, 3, 8)
    p.frustum((x, 0.72*s, z), 0.07*s, 0.05*s, 0.2*s, 5, (0.30, 0.45, 0.20))

def cottage():
    m, glow, pk = P("body"), P("glow"), P("pumpkins")
    wall, wood, trim = (0.58, 0.42, 0.36), (0.35, 0.22, 0.18), (0.45, 0.44, 0.50)
    m.box((0, 0.15, 0), (2.8, 0.15, 2.5), trim)
    m.box((0, 1.3, 0), (2.6, 1.3, 2.3), wall)
    for sx in (-1, 1):
        for sz in (-1, 1):
            m.box((sx*2.62, 1.3, sz*2.32), (0.1, 1.3, 0.1), wood)
    m.box((0, 2.52, 2.33), (2.7, 0.08, 0.06), wood); m.box((0, 2.52, -2.33), (2.7, 0.08, 0.06), wood)
    m.box((2.63, 2.52, 0), (0.06, 0.08, 2.4), wood); m.box((-2.63, 2.52, 0), (0.06, 0.08, 2.4), wood)
    roof = (0.38, 0.22, 0.46)
    m.cone_to((0, 2.55, 0), 4.5, (0, 5.05, 0), 4, roof, rot0=PI/4)
    for y, r, h in ((2.9, 0.0, 0.18), (3.5, 0.0, 0.18), (4.1, 0.0, 0.18)):
        r0 = 4.5*(1-(y-2.55)/2.5)+0.05; r1 = 4.5*(1-(y+h-2.55)/2.5)+0.05
        m.frustum((0, y, 0), r0, r1, h, 4, shade(roof, 1.18), rot0=PI/4)
    m.box((1.7, 3.5, -0.9), (0.36, 0.9, 0.36), (0.42, 0.40, 0.46)); m.box((1.7, 4.45, -0.9), (0.46, 0.07, 0.46), (0.25, 0.22, 0.28))
    # door, steps, knob
    m.box((0, 0.9, 2.31), (0.5, 0.9, 0.06), (0.30, 0.17, 0.12))
    m.box((-0.58, 0.95, 2.34), (0.08, 0.95, 0.07), wood); m.box((0.58, 0.95, 2.34), (0.08, 0.95, 0.07), wood); m.box((0, 1.9, 2.34), (0.66, 0.08, 0.07), wood)
    m.sphere((0.3, 0.9, 2.4), 0.05, (0.9, 0.7, 0.3), 0, 2, 5)
    m.box((0, 0.1, 2.9), (0.9, 0.1, 0.5), trim); m.box((0, 0.05, 3.45), (0.7, 0.05, 0.25), shade(trim, 0.92))
    for sx in (-1.6, 1.6):
        glow.box((sx, 1.5, 2.31), (0.42, 0.42, 0.05), (1.0, 0.80, 0.40), 1.0)
        m.box((sx, 1.5, 2.28), (0.5, 0.5, 0.04), (0.25, 0.15, 0.12))
        m.box((sx, 1.5, 2.375), (0.44, 0.025, 0.02), (0.25, 0.15, 0.12)); m.box((sx, 1.5, 2.375), (0.025, 0.44, 0.02), (0.25, 0.15, 0.12))
        for dx in (-0.68, 0.68):
            m.box((sx+dx, 1.5, 2.36), (0.16, 0.45, 0.03), (0.32, 0.45, 0.38))
        m.box((sx, 1.02, 2.45), (0.5, 0.09, 0.12), (0.40, 0.27, 0.17))
        for k, c in enumerate(((1.0, 0.6, 0.7), (1.0, 0.85, 0.4), (0.9, 0.55, 0.9), (1.0, 0.6, 0.7), (1.0, 0.85, 0.4))):
            m.sphere((sx-0.4+k*0.2, 1.14, 2.47), 0.07, c, 0.3, 2, 5)
    for sx in (-1, 1):
        glow.box((sx*2.63, 1.5, 0), (0.04, 0.4, 0.4), (1.0, 0.80, 0.40), 0.9)
        m.box((sx*2.62, 1.5, 0), (0.03, 0.46, 0.46), (0.25, 0.15, 0.12))
    for px, pz, s in ((-2.2, 3.2, 1.0), (-1.5, 3.6, 0.7), (2.3, 3.3, 0.9), (1.5, 3.8, 0.6), (3.1, 2.4, 0.7)):
        pumpkin(pk, px, pz, s)
    return [m, glow, pk]

def crypt():
    m, glow = P("body"), P("glow")
    stone, light, dark = (0.37, 0.37, 0.44), (0.46, 0.46, 0.54), (0.30, 0.30, 0.37)
    moss = (0.24, 0.34, 0.27)
    m.box((0, 0.18, 0), (3.9, 0.18, 3.1), dark)
    m.box((0, 1.6, 0), (3.4, 1.42, 2.6), stone)
    for sx in (-1, 1):
        m.box((sx*3.45, 1.7, 2.45), (0.28, 1.5, 0.28), (0.42, 0.42, 0.50))
        m.cone_to((sx*3.45, 3.2, 2.45), 0.3, (sx*3.45, 3.7, 2.45), 4, light, rot0=PI/4)
    for k in range(6):
        sh = 0.92 + 0.08*hn(11, k)
        m.box((-3.4+k*1.36, 2.7, 2.66), (0.66, 0.03, 0.02), shade(stone, sh*0.85))
    m.box((0, 1.15, 2.62), (0.95, 1.05, 0.05), (0.06, 0.04, 0.09))
    m.box((-1.1, 1.2, 2.7), (0.14, 1.2, 0.14), light); m.box((1.1, 1.2, 2.7), (0.14, 1.2, 0.14), light); m.box((0, 2.5, 2.7), (1.35, 0.14, 0.16), light)
    m.cone_to((0, 2.64, 2.7), 0.7, (0, 3.05, 2.7), 4, light, rot0=PI/4)
    m.box((0, 0.1, 3.35), (1.5, 0.1, 0.5), (0.40, 0.40, 0.47)); m.box((0, 0.05, 3.85), (1.2, 0.05, 0.25), (0.37, 0.37, 0.44))
    for sx in (-2.0, 2.0):
        glow.box((sx, 1.9, 2.64), (0.12, 0.35, 0.03), (0.6, 0.9, 0.8), 0.45)
        m.box((sx, 1.9, 2.62), (0.2, 0.43, 0.03), dark)
    for sx, h in ((-3.1, 1.3), (2.6, 0.9), (-1.6, 0.6)):
        m.box((sx, h, 2.64), (0.07, h, 0.03), moss)       # creeping vines
        m.box((sx+0.2, h*0.7, 2.64), (0.05, h*0.7, 0.03), moss)
    m.cone_to((0, 3.0, 0), 5.4, (0, 5.3, 0), 4, (0.33, 0.30, 0.43), rot0=PI/4)
    m.frustum((0, 5.2, 0), 0.07, 0.06, 0.8, 5, (0.20, 0.18, 0.24)); m.box((0, 5.7, 0), (0.3, 0.05, 0.05), (0.20, 0.18, 0.24))
    for sx in (-2.2, 2.2):
        m.ellipsoid((sx, 0.55, 3.6), (0.4, 0.45, 0.4), (0.40, 0.38, 0.44), 0, 3, 7)
        glow.sphere((sx, 1.05, 3.6), 0.14, (0.7, 1.0, 0.85), 0.9)
    return [m, glow]

def shed():
    m = P("body")
    plank = (0.46, 0.31, 0.20)
    m.box((0, 0.12, 0), (2.9, 0.12, 2.4), (0.33, 0.30, 0.28))
    m.box((0, 1.3, 0), (2.7, 1.2, 2.2), plank)
    for k in range(-4, 5):
        m.box((k*0.6, 1.3, 2.22), (0.025, 1.2, 0.02), (0.30, 0.19, 0.12))
    m.box((-0.9, 1.05, 2.26), (0.65, 1.0, 0.05), (0.27, 0.16, 0.10)); m.box((-0.9, 2.12, 2.28), (0.8, 0.09, 0.08), (0.30, 0.20, 0.13))
    m.box((1.35, 1.55, 2.26), (0.5, 0.45, 0.04), (0.20, 0.14, 0.10)); m.box((1.35, 1.55, 2.23), (0.6, 0.55, 0.03), (0.30, 0.20, 0.13))
    m.cone_to((0, 2.5, 0), 4.3, (0, 4.2, 0), 4, (0.24, 0.36, 0.34), rot0=PI/4)
    m.box((1.5, 3.4, -0.8), (0.3, 0.7, 0.3), (0.40, 0.38, 0.40))
    m.box((0, 0.1, 3.05), (1.9, 0.1, 0.8), (0.42, 0.30, 0.20))
    m.frustum((-3.5, 0, 1.4), 0.42, 0.40, 0.85, 7, (0.40, 0.27, 0.17)); m.frustum((-3.5, 0.55, 1.4), 0.44, 0.44, 0.06, 7, (0.22, 0.20, 0.22))
    for k in range(3):
        for j in range(3 - k):
            m.frustum((3.5 + j*0.38 + k*0.19, k*0.3, -0.6), 0.17, 0.17, 0.9, 5, (0.50, 0.36, 0.24))
    m.box((3.4, 0.35, 1.6), (0.4, 0.35, 0.4), (0.45, 0.33, 0.22))
    m.box((-1.9, 2.0, 2.3), (0.04, 0.14, 0.02), (0.15, 0.12, 0.12))
    return [m]

def gate(wood):
    def f():
        m, base = P("body"), P("tint")
        main = (0.46, 0.33, 0.22) if wood else (0.40, 0.40, 0.46)
        for sx in (-2.7, 2.7):
            base.box((sx, 0.3, 0), (0.8, 0.3, 0.8), WHITE)
            m.box((sx, 2.0, 0), (0.55, 1.7, 0.55), main)
            base.box((sx, 3.8, 0), (0.72, 0.13, 0.72), WHITE)
            if wood:
                m.box((sx, 4.15, 0), (0.3, 0.3, 0.3), main)
                m.box((sx, 2.0, 0.56), (0.5, 0.06, 0.03), shade(main, 0.7)); m.box((sx, 3.0, 0.56), (0.5, 0.06, 0.03), shade(main, 0.7))
            else:
                m.cone_to((sx, 3.93, 0), 0.55, (sx, 4.63, 0), 4, main, rot0=PI/4)
                m.box((sx, 2.5, 0.56), (0.3, 0.06, 0.02), shade(main, 0.8))
        m.box((0, 3.5, 0), (2.7, 0.26, 0.38), main)
        for sx in (-1, 1):
            base.box((sx*6.5, 0.55, 0), (3.0, 0.55, 0.35), WHITE)
            for k in range(5):
                m.box((sx*(4.0+k*1.2), 1.25, 0), (0.22, 0.18, 0.28), main)
        return [m, base]
    return f

MODELS = {
    "pine": pine, "round": round_tree, "dead": dead_tree, "willow": willow,
    "rock0": rock0, "rock1": rock1, "rock2": rock2, "pebble": pebble, "shroom": shroom,
    "grave0": grave(0), "grave1": grave(1), "grave2": grave(2), "grave3": grave(3),
    "bush_ready": bush(True), "bush_empty": bush(False), "pad": pad(False), "pad_flower": pad(True),
    "reed": reed, "pier": pier, "boat": boat, "rune": rune_stone, "lantern_post": lantern_post, "lily_post": lily_post,
    "cottage": cottage, "crypt": crypt, "shed": shed, "gate_stone": gate(False), "gate_wood": gate(True),
}

if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else 'assets/models'
    os.makedirs(out, exist_ok=True)
    total = 0
    for name, fn in MODELS.items():
        n = model(name, fn(), out); total += n
        print('%-13s %4d triangles' % (name, n))
