"""The Finnish lakes, campaign mission 31: lake country north of the
Saimaa. Two chains of lakes cross the land from edge to edge, and each of
the three roads north crosses each chain at a narrows one cell wide whose
bridge the Europeans (player 2) have blown: six bridge spots. Our base is
in the south, a Finnish detachment (player 7) in the south-east, the
Europeans' in the north past both chains. Destroy their HQ. Writes a
.cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1939)  # the Winter War
noise = Noise(rng)
W, H = 120, 100

ROADS_X = [24.5, 60.5, 96.5]
CHAINS = [66.5, 34.5]        # the lake chains' middle lines (y)
CAMP = (58, 92)
FINNS = (102, 88)
EUROPEANS = (62, 8)

def blend(x):
    """0 at a road's narrows, rising to 1 a few cells either side."""
    return max(0.0, min(1.0, (min(abs(x - rx) for rx in ROADS_X) - 2.2) / 5))

def chain_centre(x, c):
    """The chain's middle line; at each road's narrows it runs through a
    cell's middle, so the water there is one cell."""
    return c + (noise(x, c, 11) - 0.5) * 10 * blend(x)

def chain_half(x, c):
    return 0.5 + blend(x) * (2.5 + noise(x, c + 7, 6) * 6)

def in_chain(x, y):
    return any(abs(y - chain_centre(x, c)) < chain_half(x, c) for c in CHAINS)

# scattered lakes clear of the roads and the bases
LAKES = []
while len(LAKES) < 14:
    cx, cy, r = rng.uniform(8, W - 8), rng.uniform(14, H - 14), rng.uniform(2.5, 6)
    if all(abs(cx - rx) > r + 4 for rx in ROADS_X) and not any(near(b, cx, cy, r + 8) for b in (CAMP, FINNS, EUROPEANS)):
        LAKES.append(((cx, cy), r))

def in_lake(x, y):
    return any(math.hypot(x - cx, (y - cy) * 1.4) < r + (noise(x, y, 3) - 0.5) * 1.5 for (cx, cy), r in LAKES)

roads = Roads()
for k, rx in enumerate(ROADS_X):
    roads.node(f"s{k}", rx, 82.5)
    roads.node(f"b{k}a", rx, CHAINS[0], bridge=True)
    roads.node(f"m{k}", rx, 50.5)
    roads.node(f"b{k}b", rx, CHAINS[1], bridge=True)
    roads.node(f"n{k}", rx, 20.5)
for n, x, y in [("camp", 58.5, 88.5), ("eu", 62.5, 12.5), ("finn", 98.5, 86.5)]:
    roads.node(n, x, y)
for k in range(3):
    roads.ours("camp", f"s{k}", f"b{k}a", f"m{k}", f"b{k}b", f"n{k}", "eu")
roads.ours("finn", "s2")
# a cross road between the chains, along the esker
roads.ours("m0", "m1")
roads.ours("m2", "m1")

m = Map("lakes", W, H)
LEGEND = [
    ("W", 6, "watrsnow1deepbutt"), ("t", 0, "switzsnowroadstraight"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"),
    ("g", 0, "switzsnowgrasscorner2"), ("w", 0, "snowywoods1"), ("v", 0, "snowywoods2"), ("o", 0, "snow_rocks1"),
    ("p", 0, "cromeplatform3"), ("f", 0, "finlandflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if in_chain(cx, cy) or in_lake(cx, cy):
        return "W"
    if near(FINNS, cx, cy, 1.0):
        return "f"
    if near(CAMP, cx, cy, 3.5) or near(FINNS, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "t"
    n = noise(cx, cy, 5)
    return "w" if n > 0.58 else ("v" if n > 0.5 else ("g" if noise(cx + 30, cy, 4) > 0.6 else ("o" if noise(cx, cy + 30, 2) > 0.8 else "s")))

def height(vx, vy):
    if in_chain(vx, vy) or in_lake(vx, vy):
        return 6
    return 10 + noise(vx, vy, 9) * 10 * min(1.0, roads.dist(vx, vy) / 4)

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 50.5, 94.5, 0, 1000)
m.obj("airbase", 1, 68.5, 94.5, 0, 1000)
m.obj("bank", 1, 46.5, 88.5, 0, 4000)
m.obj("moneyplane", 1, 46.5, 88.5, 0, 0)
m.obj("stank", 1, 60.5, 86.5, NORTH, 1000)
m.obj("mainbasepad", 7, *FINNS, 0, 4000)
m.obj("garage", 7, 108.5, 92.5, 0, 1000)
m.obj("bank", 7, 110.5, 84.5, 0, 4000)
m.obj("moneyplane", 7, 110.5, 84.5, 0, 0)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 54.5, 6.5, 0, 1000)
m.obj("airbase", 2, 72.5, 6.5, 0, 1000)
m.obj("bank", 2, 46.5, 10.5, 0, 4000)
m.obj("moneyplane", 2, 46.5, 10.5, 0, 0)
# their guns over the far shore of each narrows
for k, rx in enumerate(ROADS_X):
    m.obj("turret", 2, rx + 3.0, CHAINS[0] - 4.5, 0, 200)
    m.obj("tower_missile", 2, rx - 3.5, CHAINS[1] - 5.0, 0, 600)
for kind, x, y, hp in [("supergun", 60.5, 26.5, 1200), ("tower_electric", 66.5, 16.5, 600), ("tower_missile", 44.5, 18.5, 600), ("turret", 80.5, 14.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# neutral banks on the esker for engineers
for x, y in [(42.5, 52.5), (78.5, 52.5)]:
    m.obj("bank", 0, x, y, 0, 4000)
m.roads(roads)
for _ in range(2200):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, FINNS, EUROPEANS)):
        continue
    if t in "wv" and rng.random() < 0.18:
        m.obj("pinetree", 4, x, y, 0, 100)
    elif t in "sg" and rng.random() < 0.006:
        m.obj(rng.choice(["cabin_small", "silobigfull2x2"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Finland.mp3", ["no mainbasepad 2"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "bridger", "helicopter", "jet", "bomber"],
        money=[10000, 15000, 0, 10000, 8000])
