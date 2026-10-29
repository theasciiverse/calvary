"""The Loire, campaign mission 25: the assault on the Order's chateau. The
Watchers' Order (player 6) holds a chateau on the north bank (the
original's `francecastle` model) inside a ring of walls and a moat, with
two gates; a supergun in its courtyard outranges our tank busters. Our
camp is across the river, with two bridges. Destroy the Order's HQ.
Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1519)  # the year work began at Chambord
noise = Noise(rng)
W, H = 120, 100

CHATEAU = (62, 36)
RING = 11.0                       # the curtain wall's radius
GATES = [(62.5, 47.5), (73.5, 36.5)]   # south and east
CAMP = (58, 88)
LOIRE = [(0, 62), (30, 66), (60, 63), (90, 68), (120, 65)]
BRIDGES = [36.5, 92.5]            # x of the two bridges

def loire(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(LOIRE, LOIRE[1:]))

def ring(x, y):
    return math.hypot(x - CHATEAU[0], y - CHATEAU[1])

def at_gate(x, y):
    return any(math.hypot(x - gx, y - gy) < 2.2 for gx, gy in GATES)

roads = Roads()
for n, x, y in [("camp", 58.5, 84.5), ("sw", 40.5, 76.5), ("bw", BRIDGES[0], 64.5), ("nw", 44.5, 54.5), ("sg", 62.5, 50.5), ("court", 62.5, 42.5),
                ("se", 82.5, 78.5), ("be", BRIDGES[1], 66.5), ("ne", 88.5, 46.5), ("eg", 76.5, 36.5), ("keep", 66.5, 36.5)]:
    roads.node(n, x, y)
# ours: over the west bridge to the south gate, or the east bridge to the east gate
roads.ours("camp", "sw", "bw", "nw", "sg", "court")
roads.ours("camp", "se", "be", "ne", "eg", "keep")
# the Order's sorties from the courtyard, out by either gate
roads.link("W", "court", "sg", "nw", "bw", "sw", "camp")
roads.link("W", "keep", "eg", "ne", "be", "se", "camp")

m = Map("loire", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("w", 6, "egyptwater4"), ("B", 0, "rome_bridge1a"), ("X", 4, "castle_walls_passages"),
    ("g", 0, "francegrassfull"), ("G", 0, "francegrassstripofrock2"), ("f", 0, "dirtplantsfrancestrips"), ("v", 0, "dirtfrancestrips"),
    ("t", 0, "francegrassroadfull"), ("c", 0, "rome_streets2_old"), ("p", 0, "cromeplatform3"), ("s", 0, "egyptsandgrassfull"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    d = loire(cx, cy)
    if d < 3.0:
        if any(abs(cx - bx) < 1.2 for bx in BRIDGES) and on_road:
            return "B"
        return "W" if d < 2.0 else "w"
    if d < 4.2:
        return "s"   # the Loire's sandbanks
    r = ring(cx, cy)
    if RING + 1.0 < r < RING + 2.6:
        return "B" if at_gate(cx, cy) and on_road else "W"   # the moat, bridged at the gates
    if RING - 0.5 < r < RING + 1.0:
        return "c" if at_gate(cx, cy) else "X"   # the curtain wall, open at the gates
    if r <= RING - 0.5:
        return "c" if noise(cx, cy, 3) > 0.35 or on_road else "g"   # the courtyard and its gardens
    if near(CAMP, cx, cy, 3.5):
        return "p"
    if on_road:
        return "t"
    if cy < 60 and noise(cx, cy, 8) > 0.55:
        return "f"   # vines on the north bank
    return "g" if noise(cx + 60, cy, 9) > 0.45 else ("G" if noise(cx, cy, 4) > 0.6 else "v")

def height(vx, vy):
    if loire(vx, vy) < 2.6:
        return 3
    r = ring(vx, vy)
    if RING + 1.0 < r < RING + 2.6:
        return 5
    h = 10 + noise(vx, vy, 12) * 6 * min(1.0, roads.dist(vx, vy) / 4)
    if RING - 0.5 < r < RING + 1.0 and not at_gate(vx, vy):
        h = 50          # the curtain wall
    elif r < RING + 1.0:
        h = 18          # the chateau's terrace
    elif vy < 60:
        h += max(0.0, 60 - vy) * 0.2
    if near(CAMP, vx, vy, 6):
        h = 10
    return h

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 52.5, 91.5, 0, 1000)
m.obj("airbase", 1, 66.5, 92.5, 0, 1000)
m.obj("bank", 1, 48.5, 84.5, 0, 4000)
m.obj("moneyplane", 1, 48.5, 84.5, 0, 0)
m.obj("stank", 1, 61.5, 82.5, NORTH, 1000)
# the Order: the chateau, its HQ and garage in the courtyard, the supergun,
# towers on the wall, a bank and a launcher
m.obj("francecastle", 4, CHATEAU[0] - 1.0, CHATEAU[1] - 3.5, 0.0, 100)
m.obj("mainbasepad", 6, CHATEAU[0] - 4, CHATEAU[1] + 2, 0, 4000)
m.obj("garage", 6, CHATEAU[0] + 4.5, CHATEAU[1] + 4.5, 0, 1000)
m.obj("bank", 6, CHATEAU[0] + 5.5, CHATEAU[1] - 5.5, 0, 4000)
m.obj("moneyplane", 6, CHATEAU[0] + 5.5, CHATEAU[1] - 5.5, 0, 0)
m.obj("supergun", 6, CHATEAU[0] - 5.5, CHATEAU[1] - 4.0, 0, 1200)
for kind, x, y, hp in [("tower_missile", 55.5, 46.5, 600), ("tower_missile", 69.5, 46.5, 600), ("tower_electric", 70.5, 29.5, 600), ("turret", 51.5, 30.5, 200),
                       ("base_launcher", 66.5, 30.5, 1000), ("turret", 40.5, 60.5, 200), ("turret", 90.5, 60.5, 200)]:
    m.obj(kind, 6, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "cobra_tank", "rocket_tank", "electric_car"]):
    m.obj(kind, 6, 60.5 + (k % 2) * 3, 40.5 + (k // 2) * 1.5, math.pi / 2, 0)
m.roads(roads)
# the formal gardens' hedges, poplars along the river, farms and vines
for _ in range(1200):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if t in "gGv" and 4.5 < loire(x, y) < 9 and roads.dist(x, y) > 2 and rng.random() < 0.25:
        m.obj("pinetree", 4, x, y, 0, 100)
    elif t in "fgv" and ring(x, y) > RING + 5 and roads.dist(x, y) > 3 and not near(CAMP, x, y, 8) and rng.random() < 0.012:
        m.obj(rng.choice(["cabin_small", "silobigfull2x2", "greecehouses2"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Turin_Italy.mp3", ["no mainbasepad 6"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 0, 0, 15000])
