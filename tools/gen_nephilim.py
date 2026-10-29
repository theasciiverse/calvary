"""The Nephilim Battle, campaign mission 35: Jotunheimen, the home of the
giants. A long frozen lake between the mountains, and under its ice the
Nephilim lie; the Watchers' Order (player 6) holds its rite on the north
shore, where the ground has turned to something that is not rock, and
sends its convoys over the lake to wake them. Anything that drives near
one wakes it, and it wakes the others near it. We hold the south shore
until the ice breaks. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1951)  # the year the Jotunheimen giants' road (the Sognefjellsvegen) was finished
noise = Noise(rng)
W, H = 120, 100

LAKE = ((60, 50), (48, 13))        # centre, half axes
CAMP = (60, 88)
RITE = (60, 12)                    # the Order's rite on the north shore
GIANTS = [(28, 48), (44, 56), (50, 42), (66, 54), (74, 42), (88, 56), (96, 46), (60, 62)]

def lake(x, y):
    (cx, cy), (a, b) = LAKE
    return 1 - math.hypot((x - cx) / a, (y - cy) / b) + (noise(x, y, 4) - 0.5) * 0.12

def walls(x, y):
    """The mountains round the valley: > 0 is sheer."""
    return max(abs(x - 60) - 56, abs(y - 50) - 46, 0) + max(0.0, (math.hypot((x - 60) / 58, (y - 50) / 47) - 1) * 40)

def crust(x, y):
    return math.hypot(x - RITE[0], (y - RITE[1]) * 1.5) < 14 + (noise(x, y, 3) - 0.5) * 6

roads = Roads()
for n, x, y in [("camp", 60.5, 84.5), ("sw", 34.5, 74.5), ("se", 86.5, 74.5),
                ("w1", 28.5, 60.5), ("w2", 30.5, 40.5), ("m1", 60.5, 66.5), ("m2", 60.5, 34.5), ("e1", 92.5, 60.5), ("e2", 90.5, 40.5),
                ("nw", 40.5, 24.5), ("ne", 80.5, 24.5), ("rite", 60.5, 16.5)]:
    roads.node(n, x, y)
roads.ours("camp", "sw", "w1", "w2", "nw", "rite", them="W")
roads.ours("camp", "m1", "m2", "rite", them="W")
roads.ours("camp", "se", "e1", "e2", "ne", "rite", them="W")

m = Map("nephilim", W, H)
LEGEND = [
    ("i", 0, "snow-mud_icepatch"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"), ("o", 0, "snow_rocks1"), ("C", 4, "snow_cliff1"),
    ("c", 4, "snowcliff5"), ("a", 0, "alien_crust"), ("A", 0, "alien_crust2"), ("t", 0, "switzsnowroadstraight"), ("p", 0, "cromeplatform3"),
    ("w", 0, "snowywoods1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if walls(cx, cy) > 0:
        return "C" if noise(cx, cy, 3) > 0.45 else "c"
    if lake(cx, cy) > 0:
        return "i"
    if near(CAMP, cx, cy, 3.5) or near(RITE, cx, cy, 2.5):
        return "p"
    if crust(cx, cy):
        return "A" if noise(cx, cy, 2) > 0.55 else "a"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    return "w" if cy > 70 and noise(cx, cy, 4) > 0.55 else ("o" if noise(cx + 3, cy, 3) > 0.7 else ("S" if noise(cx, cy + 7, 5) > 0.5 else "s"))

def height(vx, vy):
    if walls(vx, vy) > 0:
        return min(250, 60 + walls(vx, vy) * 30 + noise(vx, vy, 5) * 40)
    if lake(vx, vy) > 0:
        return 10 + noise(vx, vy, 3) * 2
    return 14 + noise(vx, vy, 8) * 8 * min(1.0, roads.dist(vx, vy) / 4)

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 52.5, 92.5, 0, 1000)
m.obj("airbase", 1, 70.5, 92.5, 0, 1000)
m.obj("bank", 1, 46.5, 86.5, 0, 4000)
m.obj("moneyplane", 1, 46.5, 86.5, 0, 0)
m.obj("stank", 1, 60.5, 80.5, NORTH, 1000)
for kind, x, y, hp in [("turret", 44.5, 76.5, 200), ("turret", 76.5, 76.5, 200), ("tower_missile", 60.5, 74.5, 600)]:
    m.obj(kind, 1, x, y, 0, hp)
m.obj("mainbasepad", 6, 48.5, 8.5, 0, 4000)
m.obj("garage", 6, 72.5, 8.5, 0, 1000)
m.obj("bank", 6, 86.5, 10.5, 0, 4000)
m.obj("moneyplane", 6, 86.5, 10.5, 0, 0)
for kind, x, y, hp in [("supergun", 60.5, 8.5, 1200), ("tower_electric", 44.5, 18.5, 600), ("tower_electric", 76.5, 18.5, 600), ("tower_missile", 60.5, 22.0, 600)]:
    m.obj(kind, 6, x, y, 0, hp)
for x, y in GIANTS:
    m.obj("nephilim", 6, x + 0.5, y + 0.5, rng.uniform(-3.1, 3.1), 0)
m.roads(roads)
for _ in range(1200):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, RITE)):
        continue
    if t == "w" and rng.random() < 0.25:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Norway_Nephelim_Battle.mp3", ["time 420"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[12000, 0, 0, 20000])
