"""Pompeii, campaign mission 19, at night: the ruined town under a fall of
ash, its walls and street grid, Via dell'Abbondanza across it, the
amphitheatre at the east end. Calloway's casket lies in the Villa of the
Mysteries outside the Herculaneum Gate; the Europeans (player 2) camp
south-west of the town and race us for it. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(1748)  # the year the excavations began
noise = Noise(rng)
W, H = 120, 100

TOWN = ((60, 52), (30, 20))        # the walled oval: centre, radii
GATES = {"herc": (37.5, 39.5), "sarno": (89.5, 52.5), "stab": (60.5, 71.5)}
VILLA = (24.5, 22.5)
CAMP = (108, 56)
EUROPEANS = (14, 80)
FORUM = (46, 52)
AMPHI = (82, 63)

def in_town(x, y, k=0.0):
    (cx, cy), (rx, ry) = TOWN
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 < 1.0 - k

roads = Roads()
for n, x, y in [("camp", 104.5, 56.5), ("sarno", 92.5, 52.5), ("abb", 70.5, 50.5), ("forum", 48.5, 50.5), ("herc", *GATES["herc"]),
                ("villa", 24.5, 25.5), ("w1", 12.5, 44.5), ("eu", 16.5, 76.5), ("s1", 96.5, 76.5), ("stab", 60.5, 74.5), ("w2", 34.5, 76.5)]:
    roads.node(n, x, y)
# through the town by Via dell'Abbondanza, out by the Herculaneum Gate past
# the villa; and round the south outside the walls
roads.ours("camp", "sarno", "abb", "forum", "herc", "villa", "w1", "eu")
roads.ours("camp", "s1", "stab", "w2", "eu")

m = Map("pompeii", W, H)
LEGEND = [
    ("a", 0, "lavasmlcorner"), ("A", 0, "lavarockside2"), ("c", 0, "cromerockbed3"), ("C", 0, "cromerockbed2a"),
    ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1a"), ("t", 0, "rome_roads"), ("f", 0, "marble_small1"),
    ("g", 0, "grassburncraters3"), ("p", 0, "cromeplatform3"), ("v", 0, "rome_streets2d"),
]

def street(x, y):
    # the grid: Via dell'Abbondanza east-west, the cross streets north-south
    return abs(y - 50.5) < 1.0 or int(x) % 8 == 0 or int(y) % 7 == 0

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if abs(cx - VILLA[0]) < 4 and abs(cy - VILLA[1]) < 3:
        return "v"   # the villa's floors
    if abs(cx - FORUM[0]) < 5 and abs(cy - FORUM[1]) < 3:
        return "f"
    if roads.dist(cx, cy) < 1.1:
        return "t"
    if in_town(cx, cy):
        return "s" if street(cx, cy) else ("S" if noise(cx, cy, 3) > 0.5 else "C")
    # ash over everything outside, a few burnt fields showing through
    if noise(cx, cy, 7) > 0.7:
        return "g"
    return "a" if noise(cx + 300, cy, 5) > 0.45 else ("A" if noise(cx, cy + 90, 4) > 0.6 else "c")

def height(vx, vy):
    h = 9 + noise(vx, vy, 10) * 8 * min(1.0, roads.dist(vx, vy) / 4)
    if in_town(vx, vy, -0.1):
        h = 14 + noise(vx, vy, 12) * 3   # the town's low hill
    if near(CAMP, vx, vy, 7) or near(EUROPEANS, vx, vy, 7):
        h = 10
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 109.5, 63.5, 0, 1000)
m.obj("bank", 1, 113.5, 50.5, 0, 4000)
m.obj("moneyplane", 1, 113.5, 50.5, 0, 0)
m.obj("stank", 1, 101.5, 58.5, math.pi, 1000)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 20.5, 84.5, 0, 1000)
m.obj("bank", 2, 7.5, 74.5, 0, 4000)
m.obj("moneyplane", 2, 7.5, 74.5, 0, 0)
# their guns: over the villa, the gate, the south road
for kind, x, y, hp in [("turret", 30.5, 20.5, 200), ("turret", 18.5, 27.5, 200), ("tower_electric", 40.5, 34.5, 600), ("turret", 46.5, 78.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# a patrol in the town, well away from the villa
for k, kind in enumerate(["peon_tank", "electric_car", "peon_tank"]):
    m.obj(kind, 2, 60.5 + k * 1.5, 46.5, 0.0, 0)
m.obj("relic", 0, *VILLA, 0, 100)
m.roads(roads)
# the town walls, broken at the gates
(cx, cy), (rx, ry) = TOWN
for k in range(72):
    a = k / 72 * 2 * math.pi
    x, y = cx + rx * math.cos(a), cy + ry * math.sin(a)
    if all(math.hypot(x - gx, y - gy) > 3 for gx, gy in GATES.values()) and rng.random() < 0.8:
        m.obj("brickwall", 4, x, y, a + math.pi / 2, 100)
# houses: broken walls along the blocks; the forum's columns; the amphitheatre
for _ in range(2500):
    x, y = rng.uniform(31, 89), rng.uniform(33, 71)
    if in_town(x, y, 0.1) and not street(x, y) and roads.dist(x, y) > 1.8 and abs(x - FORUM[0]) > 6 and math.hypot(x - AMPHI[0], y - AMPHI[1]) > 6 and rng.random() < 0.05:
        m.obj(rng.choice(["brickwall", "brickwall", "ruinsruin"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for dx in (-4.5, -2.5, -0.5, 1.5, 3.5):
    for dy in (-2.5, 2.5):
        m.obj("ruinsonepillar", 4, FORUM[0] + dx, FORUM[1] + dy, 0, 100)
for k in range(16):
    a = k / 16 * 2 * math.pi
    m.obj("brickwall", 4, AMPHI[0] + 5 * math.cos(a), AMPHI[1] + 3.5 * math.sin(a), a + math.pi / 2, 100)
# the villa: its walls and the Europeans' dig beside it
for dx, dy, th in [(-3.5, 0, 1.5708), (3.5, 0, 1.5708), (0, -2.5, 0), (-2, 2.5, 0), (2, 2.5, 0)]:
    m.obj("brickwall", 4, VILLA[0] + dx, VILLA[1] + dy, th, 100)
m.obj("bigwhitetent", 4, 31.5, 25.5, 0.2, 100)
m.obj("armycratebig", 4, 29.5, 28.0, 0.5, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Naples_Italy.mp3", [f"reach 1 relic {CAMP[0]} {CAMP[1]} 4 1"],
        [f"reach 2 relic {EUROPEANS[0]} {EUROPEANS[1]} 4 1", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter", "jet", "bomber"], money=[10000, 10000, 0], night=True)
