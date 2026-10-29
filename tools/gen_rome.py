"""Rome, campaign mission 20: a siege of the city's hills, won by holding
the Palatine. The Tiber down
the west, the Aurelian Walls round the centre, the seven hills raised
over the valleys between them, the Colosseum and the Circus Maximus. The
Europeans (player 2) hold the hills: their HQ on the Palatine, their
garage on the Esquiline, guns on the Capitoline, the Aventine and the
Caelian, a supergun on the Quirinal. Our camp is outside the walls on the
Via Appia. Tank busters and cobras join the column. Writes a .cmap to
argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(753)  # ab urbe condita
noise = Noise(rng)
W, H = 120, 100

HILLS = {  # centre, radius, height bytes over the valleys
    "palatine": ((62, 58), 6.5, 50), "capitoline": ((50, 45), 4.5, 50), "aventine": ((50, 72), 6.5, 40),
    "caelian": ((78, 66), 5.5, 36), "esquiline": ((84, 42), 7.5, 36), "viminal": ((74, 30), 5.0, 30), "quirinal": ((60, 26), 6.0, 40),
}
WALLS = ((70, 50), (34, 38))         # the Aurelian Walls: an oval, centre and radii
GATES = [(98.5, 80.5), (103.5, 46.5), (52.5, 13.5)]   # Appia, Maggiore, Salaria
COLOSSEUM = (72, 56)
CIRCUS = ((52, 64), (66, 64))       # the Circus Maximus, between Palatine and Aventine
CAMP = (108, 90)
TIBER = [(44, 0), (40, 18), (32, 34), (36, 50), (30, 66), (36, 84), (32, 100)]

def tiber_dist(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(TIBER, TIBER[1:]))

def hill(x, y):
    """Height of the hills here, and which one."""
    best, which = 0.0, None
    for name, ((cx, cy), r, h) in HILLS.items():
        d = math.hypot(x - cx, y - cy) + 1.5 * (noise(x, y, 5) - 0.5)
        if d < r:
            v = h * min(1.0, (r - d) / 2.5)   # steep sides, flat tops
            if v > best:
                best, which = v, name
    return best, which

roads = Roads()
for n, x, y in [("camp", 105.5, 87.5), ("appia", *GATES[0]), ("caelv", 86.5, 74.5), ("colo", 72.5, 62.5), ("palup", 64.5, 60.5),
                ("circus", 58.5, 66.5), ("forum", 56.5, 52.5), ("pal", 62.5, 55.5),
                ("e1", 110.5, 60.5), ("magg", *GATES[1]), ("esq", 88.5, 42.5), ("vim", 72.5, 36.5), ("sub", 66.5, 44.5)]:
    roads.node(n, x, y)
# the Via Appia in by the Porta San Sebastiano, past the Colosseum and up the Palatine
roads.ours("camp", "appia", "caelv", "colo", "palup", "pal")
# round by the Circus Maximus and the Forum
roads.ours("colo", "circus", "forum", "pal")
# in by the Porta Maggiore, over the Esquiline and down through the Subura
roads.ours("camp", "e1", "magg", "esq", "vim", "sub", "forum")
# theirs: from the Esquiline down to the Palatine, to hold it
roads.link("E", "esq", "vim", "sub", "forum", "pal")

def walls(x, y):
    (cx, cy), (rx, ry) = WALLS
    return abs(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 - 1.0)

m = Map("rome", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("w", 6, "egyptwater4"), ("B", 0, "rome_bridge1a"),
    ("g", 0, "grassanddirtfull"), ("G", 0, "dirtplantsfrancestrips"), ("d", 0, "dirtfrancehalfgrass"),
    ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1"), ("t", 0, "rome_roads"), ("m", 0, "marble_small1"),
    ("p", 0, "cromeplatform3"), ("r", 0, "rome_streets2d"), ("k", 0, "cromerockbed3"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    td = tiber_dist(cx, cy)
    if td < 3.2:
        if any(abs(cy - by) < 1.0 for by in (40.5, 70.5)):
            return "B"   # the old bridges
        return "W" if td < 2.2 else "w"
    if near(CAMP, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "t"
    h, which = hill(cx, cy)
    if which and h > 30:
        return "m" if which in ("palatine", "capitoline") and noise(cx, cy, 3) > 0.55 else ("k" if noise(cx, cy, 4) > 0.7 else "g")
    if seg_dist(cx, cy, *CIRCUS[0], *CIRCUS[1]) < 2.5:
        return "r"   # the Circus's long track
    if walls(cx, cy) < 1.0:
        # inside the walls: the city's streets, ruins between
        grid = (int(cx) % 7 == 0 and noise(cx + 40, cy, 5) > 0.35) or (int(cy) % 6 == 0 and noise(cx, cy + 40, 5) > 0.35)
        return "S" if grid else ("s" if noise(cx, cy, 3) > 0.4 else "d")
    return "G" if noise(cx, cy, 7) > 0.6 else ("g" if noise(cx + 90, cy, 9) > 0.45 else "d")

def height(vx, vy):
    if tiber_dist(vx, vy) < 2.6:
        return 2
    h = 10 + noise(vx, vy, 11) * 6 * min(1.0, roads.dist(vx, vy) / 4)
    hh, _ = hill(vx, vy)
    if roads.dist(vx, vy) < 2.0:
        hh *= 0.8   # roads climb in cuttings
    h += hh
    if near(CAMP, vx, vy, 7):
        h = 10
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 112.5, 82.5, 0, 1000)
m.obj("airbase", 1, 114.5, 94.5, 0, 1000)
m.obj("bank", 1, 101.5, 95.5, 0, 4000)
m.obj("moneyplane", 1, 101.5, 95.5, 0, 0)
m.obj("stank", 1, 103.5, 86.5, math.pi, 1000)
m.obj("mainbasepad", 2, 61.5, 57.5, 0, 4000)
m.obj("garage", 2, 85.5, 40.5, 0, 1000)
m.obj("bank", 2, 50.5, 45.5, 0, 4000)
m.obj("moneyplane", 2, 50.5, 45.5, 0, 0)
m.obj("bank", 2, 60.5, 24.5, 0, 4000)
m.obj("moneyplane", 2, 60.5, 24.5, 0, 0)
m.obj("supergun", 2, 63.5, 27.5, 0, 1200)
for kind, x, y, hp in [("tower_missile", 52.5, 70.5, 600), ("tower_electric", 47.5, 74.5, 600), ("turret", 78.5, 67.5, 200), ("turret", 75.5, 64.5, 200),
                       ("tower_missile", 52.5, 42.5, 600), ("turret", 67.5, 60.5, 200), ("turret", 58.5, 61.5, 200), ("turret", 90.5, 46.5, 200),
                       ("tower_electric", 99.5, 48.5, 600), ("turret", 95.5, 78.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
m.roads(roads)
# the Aurelian Walls, broken at the gates and where the Tiber runs
(cx, cy), (rx, ry) = WALLS
for k in range(110):
    a = k / 110 * 2 * math.pi
    x, y = cx + rx * math.cos(a), cy + ry * math.sin(a)
    if 1 < x < W - 1 and 1 < y < H - 1 and tiber_dist(x, y) > 4 and all(math.hypot(x - gx, y - gy) > 3 for gx, gy in GATES) and roads.dist(x, y) > 1.8:
        m.obj("brickwall", 4, x, y, a + math.pi / 2, 100)
# the Colosseum's oval, the Forum's columns, ruins in the valleys
for k in range(18):
    a = k / 18 * 2 * math.pi
    m.obj("brickwall", 4, COLOSSEUM[0] + 4.2 * math.cos(a), COLOSSEUM[1] + 3.2 * math.sin(a), a + math.pi / 2, 100)
for dx in (-3, -1, 1, 3):
    m.obj("ruinsonepillar", 4, 54.5 + dx, 50.0, 0, 100)
    m.obj("ruinstwopillar", 4, 55.5 + dx, 55.0, 1.5708, 100)
for _ in range(1500):
    x, y = rng.uniform(38, 102), rng.uniform(14, 86)
    if walls(x, y) < 0.9 and roads.dist(x, y) > 2 and hill(x, y)[0] < 20 and tiber_dist(x, y) > 4 and math.hypot(x - COLOSSEUM[0], y - COLOSSEUM[1]) > 6 and rng.random() < 0.04:
        m.obj(rng.choice(["ruinsruin", "ruinstwopillar", "brickwall", "greecehouses2"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for _ in range(800):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if walls(x, y) > 1.15 and roads.dist(x, y) > 2.5 and tiber_dist(x, y) > 4 and not near(CAMP, x, y, 8) and rng.random() < 0.08:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Rome_Italy.mp3", ["hold 1 62.5 57.5 4 120"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 15000, 0])
