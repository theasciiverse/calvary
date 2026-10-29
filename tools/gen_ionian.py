"""The Ionian crossing, campaign mission 17: the column on the Akarnanian
shore, waiting to cross to Italy. Lefkada lies off the coast, joined to
it by its causeway; the Europeans (player 2) have a garage there and a
supergun over the strait. Their HQ and airbase are out on Kefalonia,
with Ithaca between, where nothing on tracks can follow: the bombers'
first mission. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(1953)  # the year the great earthquake struck the islands
noise = Noise(rng)
W, H = 120, 100

COAST_X = 92               # the mainland's shore runs about here
LEFKADA = ((66, 26), (15, 12))
KEFALONIA = ((32, 68), (22, 15))
ITHACA = ((60, 64), (4, 9))
CAUSEWAY_Y = 17
CAMP = (106, 70)

def island(c, x, y, k=0.0):
    (cx, cy), (rx, ry) = c
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 < 1.0 + 0.25 * (noise(x, y, 6) - 0.5) - k

def land(x, y):
    if x > COAST_X + 4 * (noise(0, y, 9) - 0.5) * 2:
        return True
    if abs(y - CAUSEWAY_Y) < 1.2 and 74 < x < COAST_X + 3:
        return True
    return island(LEFKADA, x, y) or island(KEFALONIA, x, y) or island(ITHACA, x, y)

roads = Roads()
for n, x, y in [("camp", 102.5, 62.5), ("m1", 100.5, 40.5), ("cw_e", 94.5, 17.5), ("cw_w", 78.5, 17.5), ("lef1", 70.5, 22.5), ("lefg", 60.5, 25.5),
                ("air", 108.5, 79.5), ("s1", 80.5, 76.5), ("ith", 60.5, 66.5), ("kef", 39.5, 70.5), ("kefhq", 30.5, 65.5), ("lefair", 68.5, 31.5)]:
    roads.node(n, x, y)
# the ground war: along the shore and over the causeway to Lefkada
roads.ours("camp", "m1", "cw_e", "cw_w", "lef1", "lefg")
# the air routes: out over Ithaca to Kefalonia, and theirs back the same way
roads.ours("air", "s1", "ith", "kef", "kefhq")
# ours only: a second way for our aircraft, over the strait to Lefkada
roads.link("A", "air", "lefair", "lefg")

m = Map("ionian", W, H)
LEGEND = [
    ("W", 6, "seagreecewaterfull"), ("w", 6, "seagreecesmlcorner"), ("b", 0, "seaandsandside"),
    ("g", 0, "grassfullgrec"), ("d", 0, "grassdirtfullgrec"), ("r", 0, "grassrockgrec"), ("R", 0, "grassdirtrockgrec"),
    ("t", 0, "greeceroadfull"), ("p", 0, "cromeplatform3"), ("k", 0, "greececobble2"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if not land(cx, cy):
        near_land = any(land(cx + dx, cy + dy) for dx, dy in ((1.5, 0), (-1.5, 0), (0, 1.5), (0, -1.5)))
        return "w" if near_land else "W"
    if abs(cy - CAUSEWAY_Y) < 1.2 and 74 < cx < COAST_X:
        return "k"   # the causeway's stone
    if near(CAMP, cx, cy, 3.5) or near((30, 66), cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.1 and cx > 50 and land(cx - 2, cy) and land(cx + 2, cy):
        return "t"
    if not all(land(cx + dx, cy + dy) for dx, dy in ((1.5, 0), (-1.5, 0), (0, 1.5), (0, -1.5))):
        return "b"
    if noise(cx + 300, cy, 6) > 0.68:
        return "R"
    return "g" if noise(cx, cy, 8) > 0.5 else ("r" if noise(cx, cy + 50, 5) > 0.6 else "d")

def height(vx, vy):
    if not land(vx, vy):
        return 0
    edge = min(1.0, sum(land(vx + dx, vy + dy) for dx in (-2, 0, 2) for dy in (-2, 0, 2)) / 9.0)
    h = 6 + noise(vx, vy, 9) * 30 * edge * min(1.0, roads.dist(vx, vy) / 4)
    if near(CAMP, vx, vy, 7) or near((30, 66), vx, vy, 6):
        h = 10
    if abs(vy - CAUSEWAY_Y) < 2 and 74 < vx < COAST_X + 3:
        h = 6
    return h

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 104.5, 60.5, 0, 1000)
m.obj("airbase", 1, 111.5, 81.5, 0, 1000)
m.obj("bank", 1, 113.5, 66.5, 0, 4000)
m.obj("moneyplane", 1, 113.5, 66.5, 0, 0)
m.obj("stank", 1, 100.5, 66.5, NORTH, 1000)
# Lefkada: the garage, the supergun over the strait, its guards
m.obj("garage", 2, 58.5, 22.5, 0, 1000)
m.obj("supergun", 2, 74.5, 27.5, 0, 1200)
for kind, x, y, hp in [("turret", 76.5, 21.5, 200), ("turret", 64.5, 18.5, 200), ("tower_missile", 66.5, 29.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "rocket_tank", "peon_tank", "electric_car"]):
    m.obj(kind, 2, 66.5 + k * 1.5, 23.5, 0.0, 0)
# Kefalonia: the HQ, the airbase, the banks; Ithaca's missile battery
m.obj("mainbasepad", 2, 30, 66, 0, 4000)
m.obj("airbase", 2, 38.5, 74.5, 0, 1000)
m.obj("bank", 2, 22.5, 62.5, 0, 4000)
m.obj("moneyplane", 2, 22.5, 62.5, 0, 0)
m.obj("bank", 2, 26.5, 74.5, 0, 4000)
m.obj("moneyplane", 2, 26.5, 74.5, 0, 0)
for kind, x, y, hp in [("tower_missile", 42.5, 66.5, 600), ("tower_missile", 34.5, 60.5, 600), ("tower_missile", 60.5, 62.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
m.roads(roads)
m.obj("greclighthouse", 4, 54.5, 25.5, 0, 100)
for _ in range(2500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if tile(int(x), int(y)) in "gd" and roads.dist(x, y) > 2.2 and not near(CAMP, x, y, 8) and rng.random() < 0.06:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
for _ in range(500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if tile(int(x), int(y)) in "gd" and x < 80 and roads.dist(x, y) > 2.5 and rng.random() < 0.04:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Greece.mp3", ["no airbase 2", "no supergun 2"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter", "jet", "bomber"], money=[10000, 10000, 0])
