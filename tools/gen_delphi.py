"""Delphi, campaign mission 16: the sanctuary on the slope of Parnassus,
under the sheer Phaedriades cliffs. The Europeans (player 2) hold the
heights with three superguns (the original's `supergun`: a mortar lobbed
on an arc, 10000 damage over three cells, one shell every 20 s): one at
the stadium under the cliffs, one at the Marmaria below the sanctuary,
one on the western spur. Our camp is down in the Pleistos valley among
the olives. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(548)  # the year the old temple of Apollo burned
noise = Noise(rng)
W, H = 120, 100

CAMP = (16, 86)
EUROPEANS = (104, 34)
TEMPLE = (56, 42)          # the temple terrace
THEATRE = (50, 33)
STADIUM = ((40, 25), (60, 25))
MARMARIA = (82, 57)
SPUR = (27, 47)
SPRING = (66, 18)          # the Castalian spring, in the gorge between the cliffs
CLIFF_Y = (11, 17)         # the Phaedriades' band of rock

GUNS = [(44.5, 23.5), (85.5, 56.5), (27.5, 46.5)]

roads = Roads()
for n, x, y in [("camp", 18.5, 82.5), ("v1", 36.5, 82.5), ("sw1", 50.5, 72.5), ("sw2", 60.5, 62.5), ("sw3", 50.5, 52.5), ("temple", 56.5, 46.5),
                ("theatre", 50.5, 37.5), ("stadium", 50.5, 27.5), ("stad_e", 70.5, 26.5), ("eu1", 90.5, 30.5), ("eu", 101.5, 36.5),
                ("v2", 66.5, 80.5), ("marm", 78.5, 60.5), ("e2", 92.5, 46.5), ("w1", 24.5, 66.5), ("w2", 33.5, 50.5), ("w3", 42.5, 40.5)]:
    roads.node(n, x, y)
# the Sacred Way, zigzagging up through the sanctuary and on east under the cliffs
roads.ours("camp", "v1", "sw1", "sw2", "sw3", "temple", "theatre", "stadium", "stad_e", "eu1", "eu")
# the valley road east, and up past the Marmaria
roads.ours("v1", "v2", "marm", "e2", "eu")
# up the west side, past the spur
roads.ours("camp", "w1", "w2", "w3", "theatre")

def cliff(x, y):
    """The Phaedriades: a band of sheer rock, split by the spring's gorge."""
    top = CLIFF_Y[0] + 1.5 * noise(x, 0, 6)
    return 26 < x < 96 and top < y < CLIFF_Y[1] and abs(x - SPRING[0]) > 2.2

def above_cliffs(x, y):
    return 26 < x < 96 and y <= CLIFF_Y[0] + 1.5 * noise(x, 0, 6) and abs(x - SPRING[0]) > 2.2

def stadium(x, y):
    return seg_dist(x, y, *STADIUM[0], *STADIUM[1]) < 2.4

m = Map("delphi", W, H)
LEGEND = [
    ("g", 0, "grassfullgrec"), ("d", 0, "grassdirtfullgrec"), ("r", 0, "grassrockgrec"), ("R", 0, "grassdirtrockgrec"),
    ("c", 4, "cliff1"), ("C", 4, "cliff1-top"), ("t", 0, "greeceroadfull"), ("s", 0, "greecestreetfull"),
    ("m", 0, "marble_small1"), ("T", 0, "grectemple1"), ("w", 6, "seagreecewaterfull"), ("p", 0, "cromeplatform3"),
    ("k", 0, "greecetrackfullgrass"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if near(SPRING, cx, cy, 1.6):
        return "w"
    if cliff(cx, cy):
        return "c"
    if above_cliffs(cx, cy):
        return "C"
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if abs(cx - TEMPLE[0]) < 4 and abs(cy - TEMPLE[1]) < 2:
        return "T"
    if abs(cx - TEMPLE[0]) < 9 and abs(cy - TEMPLE[1]) < 5 or near(THEATRE, cx, cy, 4) or near(MARMARIA, cx, cy, 3):
        return "m"   # the sanctuary's terraces, the theatre's seats, the tholos
    if stadium(cx, cy):
        return "k"
    if roads.dist(cx, cy) < 1.1:
        return "t"
    if cy > 70:
        return "g" if noise(cx, cy, 5) > 0.45 else "d"   # the olive groves of the valley
    if noise(cx + 300, cy, 6) > 0.7:
        return "R"
    return "r" if noise(cx, cy, 9) > 0.55 else "d"

def height(vx, vy):
    # the slope of Parnassus, rising from the valley to the cliffs
    h = 8 + max(0.0, 78 - vy) * 0.9 + noise(vx, vy, 10) * 10 * min(1.0, roads.dist(vx, vy) / 5)
    if near(SPUR, vx, vy, 6):
        h += (6 - math.hypot(vx - SPUR[0], vy - SPUR[1])) * 4   # the western spur
    if abs(vx - TEMPLE[0]) < 9 and abs(vy - TEMPLE[1]) < 5:
        h = 8 + (78 - TEMPLE[1]) * 0.9      # the temple terrace, level
    if stadium(vx, vy):
        h = 8 + (78 - 25) * 0.9
    if above_cliffs(vx, vy) or cliff(vx, vy):
        h = 8 + (78 - CLIFF_Y[1]) * 0.9 + 60
    if near(CAMP, vx, vy, 7):
        h = 10
    if near(EUROPEANS, vx, vy, 7):
        h = 8 + (78 - EUROPEANS[1]) * 0.9
    return h

NORTH, EAST = -math.pi / 2, 0.0
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 22.5, 90.5, 0, 1000)
m.obj("airbase", 1, 8.5, 80.5, 0, 1000)
m.obj("bank", 1, 8.5, 92.5, 0, 4000)
m.obj("moneyplane", 1, 8.5, 92.5, 0, 0)
m.obj("stank", 1, 22.5, 80.5, NORTH, 1000)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 108.5, 40.5, 0, 1000)
m.obj("bank", 2, 112.5, 28.5, 0, 4000)
m.obj("moneyplane", 2, 112.5, 28.5, 0, 0)
for x, y in GUNS:
    m.obj("supergun", 2, x, y, 0, 1200)
for kind, x, y, hp in [("turret", 48.5, 22.5, 200), ("turret", 82.5, 60.5, 200), ("turret", 30.5, 44.5, 200), ("tower_missile", 61.5, 40.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "rocket_tank", "peon_tank"]):
    m.obj(kind, 2, 47.5 + k * 1.5, 35.5, math.pi / 2, 0)
m.roads(roads)
# the temple's columns, the treasuries along the Sacred Way, the tholos
for dx in (-3.5, -1.2, 1.2, 3.5):
    for dy in (-1.5, 1.5):
        m.obj("ruinsonepillar", 4, TEMPLE[0] + dx, TEMPLE[1] + dy, 0, 100)
for x, y in [(54.5, 56.5), (58.5, 53.5), (47.5, 49.5)]:
    m.obj(rng.choice(["ruinsruin", "ruinstwopillar"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for a in range(0, 360, 60):
    m.obj("ruinsonepillar", 4, MARMARIA[0] + 2 * math.cos(math.radians(a)), MARMARIA[1] + 2 * math.sin(math.radians(a)), 0, 100)
for _ in range(2000):
    x, y = rng.uniform(2, W - 2), rng.uniform(68, H - 2)
    if roads.dist(x, y) > 2.2 and not near(CAMP, x, y, 8) and rng.random() < 0.1:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Greece.mp3", ["no supergun 2"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter", "jet"], money=[10000, 10000, 0])
