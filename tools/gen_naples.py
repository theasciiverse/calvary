"""Naples, campaign mission 18: the Bay of Naples under Vesuvius, which is
stirring. Lava runs down three channels from the crater (one across the
coast road) and burns what drives into it; the crater throws volcanic
bombs onto the cone. The Europeans (player 2) hold Naples and dig at
Herculaneum on the coast below the volcano: the site to hold. Writes a
.cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(79)  # the eruption that buried Herculaneum and Pompeii
noise = Noise(rng)
W, H = 120, 100

CRATER = (70, 42)
CONE = 22                  # the cone's radius
HERCULANEUM = (46.5, 63.5)
CAMP = (106, 70)
NAPLES = (20, 34)
FLOWS = [
    [CRATER, (62, 56), (57, 68), (55, 79)],   # south-west, across the coast road
    [CRATER, (82, 54), (89, 64)],             # south-east
    [CRATER, (64, 30), (59, 22)],             # north-west
]

def sea(x, y):
    return y > 46 + 0.5 * x + 3 * (noise(x, 0, 7) - 0.5)

def flow_dist(x, y):
    return min(seg_dist(x, y, *a, *b) for f in FLOWS for a, b in zip(f, f[1:]))

def flow_width(x, y):
    return 1.1 + 0.6 * noise(x, y, 4)

roads = Roads()
for n, x, y in [("camp", 104.5, 72.5), ("c1", 86.5, 80.5), ("c2", 66.5, 75.5), ("herc", 47.5, 61.5), ("c3", 34.5, 50.5), ("naples", 24.5, 40.5),
                ("n1", 100.5, 44.5), ("n2", 86.5, 18.5), ("n3", 56.5, 12.5), ("n4", 36.5, 22.5)]:
    roads.node(n, x, y)
roads.ours("camp", "c1", "c2", "herc", "c3", "naples")
roads.ours("camp", "n1", "n2", "n3", "n4", "naples")

m = Map("naples", W, H)
LEGEND = [
    ("W", 6, "seagreecewaterfull"), ("w", 6, "seagreecesmlcorner"), ("b", 0, "seaandsandside"),
    ("L", 2, "lavafull"), ("l", 2, "lavaside"), ("a", 0, "lavasmlcorner"), ("A", 0, "lavarockside2"), ("k", 0, "cromerockbed2"),
    ("g", 0, "grassfullgrec"), ("d", 0, "grassdirtfullgrec"), ("v", 0, "dirtplantsfrancestrips"),
    ("t", 0, "rome_roads"), ("s", 0, "rome_streets2"), ("S", 0, "rome_streets1"), ("p", 0, "cromeplatform3"), ("m", 0, "marble_small1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if sea(cx, cy):
        return "w" if not sea(cx, cy - 2) else "W"
    if sea(cx, cy + 1.5):
        return "b"
    r = math.hypot(cx - CRATER[0], cy - CRATER[1])
    if r < 3.2:
        return "L"   # the crater's floor
    fd = flow_dist(cx, cy)
    if fd < flow_width(cx, cy):
        return "L" if fd < flow_width(cx, cy) - 0.6 else "l"
    if near(CAMP, cx, cy, 3.5) or near(NAPLES, cx, cy, 3.5):
        return "p"
    if abs(cx - HERCULANEUM[0]) < 4 and abs(cy - HERCULANEUM[1]) < 3:
        return "m"   # the excavated town's floors
    if roads.dist(cx, cy) < 1.1:
        return "t"
    if cx < 40 and cy < 50 and math.hypot(cx - NAPLES[0], cy - NAPLES[1]) < 17:
        return "S" if (int(cx) % 7 == 0 or int(cy) % 6 == 0) else "s"   # the city's streets
    if r < CONE:
        return "k" if noise(cx, cy, 4) > 0.72 else ("A" if r < CONE * 0.55 or noise(cx, cy, 6) > 0.6 else "a")
    if noise(cx, cy, 7) > 0.62:
        return "v"   # vineyards on the volcanic soil
    return "g" if noise(cx + 200, cy, 9) > 0.45 else "d"

def height(vx, vy):
    if sea(vx, vy) and sea(vx, vy - 1):
        return 0
    r = math.hypot(vx - CRATER[0], vy - CRATER[1])
    h = 8 + noise(vx, vy, 10) * 8 * min(1.0, roads.dist(vx, vy) / 4)
    if r < CONE:
        h += (CONE - r) * 7.0
    if r < 3.5:
        h -= (3.5 - r) * 12    # the crater
    if near(CAMP, vx, vy, 7) or near(NAPLES, vx, vy, 6):
        h = 10
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 108.5, 63.5, 0, 1000)
m.obj("airbase", 1, 112.5, 76.5, 0, 1000)
m.obj("bank", 1, 113.5, 68.5, 0, 4000)
m.obj("moneyplane", 1, 113.5, 68.5, 0, 0)
m.obj("stank", 1, 101.5, 70.5, math.pi, 1000)
m.obj("mainbasepad", 2, *NAPLES, 0, 4000)
m.obj("garage", 2, 27.5, 31.5, 0, 1000)
m.obj("airbase", 2, 12.5, 24.5, 0, 1000)
m.obj("bank", 2, 10.5, 40.5, 0, 4000)
m.obj("moneyplane", 2, 10.5, 40.5, 0, 0)
for kind, x, y, hp in [("turret", 42.5, 58.5, 200), ("turret", 51.5, 60.5, 200), ("tower_missile", 40.5, 64.5, 600), ("turret", 60.5, 16.5, 200), ("tower_electric", 30.5, 44.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "electric_car", "rocket_tank"]):
    m.obj(kind, 2, 44.5 + k * 1.5, 61.5, 0.0, 0)
m.obj("volcano", 0, *CRATER, 0, 100)
m.roads(roads)
# the dig at Herculaneum
m.obj("bigwhitetent", 4, 42.5, 66.0, 0.3, 100)
m.obj("armycratebig", 4, 50.5, 66.0, 0.1, 100)
for x, y in [(44.0, 62.0), (49.0, 61.5), (45.0, 65.0)]:
    m.obj(rng.choice(["ruinsruin", "ruinstwopillar", "ruinsonepillar"]), 4, x, y, rng.choice([0, 1.5708]), 100)
m.obj("giant_rib", 4, 47.5, 65.5, 0.4, 100)
# Naples' houses, palms along the bay
for _ in range(1500):
    x, y = rng.uniform(4, 40), rng.uniform(18, 52)
    if tile(int(x), int(y)) == "s" and roads.dist(x, y) > 2 and not near(NAPLES, x, y, 5) and rng.random() < 0.15:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for _ in range(1500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if tile(int(x), int(y)) in "gd" and roads.dist(x, y) > 2 and not near(CAMP, x, y, 8) and rng.random() < 0.05:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Naples_Italy.mp3", [f"hold 1 {HERCULANEUM[0]} {HERCULANEUM[1]} 4 120"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter", "jet", "bomber"], money=[10000, 10000, 0])
