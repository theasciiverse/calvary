"""Cyprus, campaign mission 36: staging for the last voyage. The Americans
(player 5), still hunting the deserter, hold the south coast: their HQ at
Limassol harbour, an airbase on the Akrotiri peninsula, guns at Kolossi
castle. We land at Paphos in the west with the Finns (player 7) who
sailed with us. Between, the Troodos mountains come down to the sea; a
coast road and a hill road. Take Limassol: destroy their HQ. Writes a
.cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1571)  # the year Famagusta fell
noise = Noise(rng)
W, H = 130, 90

CAMP = (14, 58)
FINNS = (14, 40)
LIMASSOL = (104, 50)
AKROTIRI = (90, 76)
KOLOSSI = (78, 54)
TROODOS = ((60, 8), 34)

def coast(x):
    """The shore's y: sea south of it; the Akrotiri peninsula sticks out."""
    y = 66 + (noise(x, 1, 9) - 0.5) * 5 - max(0.0, x - 100) * 0.25
    y += max(0.0, 9 - abs(x - 90) * 0.7) * 1.6
    return y

def mountain(x, y):
    (cx, cy), r = TROODOS
    return max(0.0, r - math.hypot(x - cx, (y - cy) * 1.3))

roads = Roads()
for n, x, y in [("camp", 16.5, 54.5), ("paphos", 26.5, 58.5), ("c1", 44.5, 62.5), ("c2", 64.5, 60.5), ("kol", 78.5, 58.5), ("lim", 100.5, 52.5),
                ("h1", 34.5, 46.5), ("h2", 52.5, 40.5), ("h3", 72.5, 40.5), ("h4", 90.5, 42.5), ("akr", 90.5, 70.5), ("finn", 18.5, 44.5)]:
    roads.node(n, x, y)
roads.ours("camp", "paphos", "c1", "c2", "kol", "lim", them="R")
roads.ours("paphos", "h1", "h2", "h3", "h4", "lim", them="R")
roads.ours("kol", "akr", them="R")
roads.ours("finn", "h1", them="R")

m = Map("cyprus", W, H)
LEGEND = [
    ("W", 6, "seagreecewaterfull"), ("b", 0, "seaandsandside"), ("g", 0, "grassfullgrec"), ("d", 0, "grassdirtfullgrec"),
    ("r", 0, "grassrockgrec"), ("R", 0, "grassdirtrockgrec"), ("C", 4, "cliff1-top"), ("t", 0, "greeceroadfull"),
    ("p", 0, "cromeplatform3"), ("k", 0, "greececobble2"), ("K", 0, "grecdock2"), ("f", 0, "finlandflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if cy > coast(cx) + 0.8:
        return "W"
    if cy > coast(cx) - 0.6:
        return "K" if near(LIMASSOL, cx, cy, 10) else "b"
    if near(FINNS, cx, cy, 1.0):
        return "f"
    if near(CAMP, cx, cy, 3.5) or near(FINNS, cx, cy, 3.0) or near(LIMASSOL, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    mt = mountain(cx, cy)
    if mt > 18:
        return "C"
    if near(LIMASSOL, cx, cy, 11) or near((24, 60), cx, cy, 5):
        return "k"   # the towns
    if mt > 8:
        return "R" if noise(cx, cy, 3) > 0.5 else "r"
    return "g" if noise(cx, cy, 5) > 0.5 else "d"

def height(vx, vy):
    if vy > coast(vx) + 0.5:
        return 5
    return 10 + mountain(vx, vy) * 4.2 * min(1.0, roads.dist(vx, vy) / 5) + noise(vx, vy, 7) * 3

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 8.5, 52.5, 0, 1000)
m.obj("airbase", 1, 8.5, 64.5, 0, 1000)
m.obj("bank", 1, 22.5, 50.5, 0, 4000)
m.obj("moneyplane", 1, 22.5, 50.5, 0, 0)
m.obj("stank", 1, 20.5, 56.5, 0.0, 1000)
m.obj("mainbasepad", 7, *FINNS, 0, 4000)
m.obj("garage", 7, 22.5, 38.5, 0, 1000)
m.obj("bank", 7, 8.5, 34.5, 0, 4000)
m.obj("moneyplane", 7, 8.5, 34.5, 0, 0)
m.obj("mainbasepad", 5, *LIMASSOL, 0, 4000)
m.obj("garage", 5, 112.5, 48.5, 0, 1000)
m.obj("garage", 5, 104.5, 40.5, 0, 1000)
m.obj("airbase", 5, *AKROTIRI, 0, 1000)
m.obj("bank", 5, 116.5, 56.5, 0, 4000)
m.obj("moneyplane", 5, 116.5, 56.5, 0, 0)
m.obj("francecastle", 4, KOLOSSI[0], KOLOSSI[1] - 4, 0.0, 100)
for kind, x, y, hp in [("tower_missile", 80.5, 54.5, 600), ("turret", 74.5, 56.5, 200), ("supergun", 92.5, 46.5, 1200), ("tower_electric", 96.5, 56.5, 600),
                       ("turret", 66.5, 42.5, 200), ("tower_missile", 86.5, 40.5, 600), ("turret", 94.5, 66.5, 200), ("turret", 108.5, 58.5, 200)]:
    m.obj(kind, 5, x, y, 0, hp)
m.obj("battleship", 5, 106.5, 64.0, 0.0, 0)
for x, y in [(48.5, 56.5), (60.5, 36.5)]:
    m.obj("bank", 0, x, y, 0, 4000)
m.roads(roads)
for _ in range(1800):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, FINNS, LIMASSOL, KOLOSSI)):
        continue
    if t == "k" and rng.random() < 0.15:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "gd" and rng.random() < 0.02:
        m.obj(rng.choice(["palmtree1", "pinetree"]), 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Greece.mp3", ["no mainbasepad 5"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "bridger", "helicopter", "jet", "bomber"],
        money=[12000, 0, 20000, 0, 10000])
