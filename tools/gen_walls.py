"""The walls of Jerusalem, campaign mission 38: we hold the Old City
inside its walls, and every army comes at its gates: the Europeans
(player 2) from the north to the Damascus Gate, the Americans (player 5)
from the west to the Jaffa Gate, the Watchers' Order (player 6) from the
Mount of Olives to the Lions' Gate, with a supergun on the mount. The
holy places are no-fire zones: three of our shots landing on them and
the mission is lost, however it is going. Hold seven minutes. Writes a
.cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1538)  # the year Suleiman's walls were finished
noise = Noise(rng)
W, H = 120, 100

CITY = ((40, 30), (80, 70))     # inside the walls
GATES = {"jaffa": (40, 54), "damascus": (60, 30), "lions": (80, 44), "zion": (54, 70)}
CAMP = (48, 60)                 # by the Citadel, inside the Jaffa Gate
EUROPEANS = (60, 6)
AMERICANS = (8, 52)
ORDER = (110, 46)
OLIVES = ((100, 44), 10)
TEMPLE_MOUNT = (71, 52)
SEPULCHRE = (52, 44)

def wall_d(x, y):
    """Distance from the wall line (the city's edge)."""
    (x0, y0), (x1, y1) = CITY
    inside = x0 <= x <= x1 and y0 <= y <= y1
    d = min(abs(x - x0), abs(x - x1), abs(y - y0), abs(y - y1)) if inside else max(x0 - x, x - x1, y0 - y, y - y1, 0)
    return d

def at_gate(x, y):
    return any(math.hypot(x - gx, y - gy) < 1.8 for gx, gy in GATES.values())

def in_city(x, y):
    (x0, y0), (x1, y1) = CITY
    return x0 < x < x1 and y0 < y < y1

roads = Roads()
for n, x, y in [("camp", 48.5, 56.5), ("jg", 40.5, 54.5), ("dg", 60.5, 30.5), ("lg", 80.5, 44.5), ("zg", 54.5, 70.5),
                ("w1", 24.5, 52.5), ("am", 12.5, 52.5), ("n1", 60.5, 18.5), ("eu", 60.5, 10.5), ("e1", 94.5, 44.5), ("or", 106.5, 46.5),
                ("mid", 60.5, 46.5), ("s1", 54.5, 84.5)]:
    roads.node(n, x, y)
# ours: out of every gate, to whoever is beyond it; they come back in
roads.ours("camp", "jg", "w1", "am", them="R")
roads.ours("camp", "mid", "dg", "n1", "eu")
roads.ours("mid", "lg", "e1", "or", them="W")
roads.ours("camp", "zg", "s1")

m = Map("walls", W, H)
LEGEND = [
    ("X", 4, "jerusrockwallbig"), ("x", 4, "jerusrockwallbig2"), ("j", 0, "jerusstreetfull"), ("J", 0, "jerusstreetfull2"),
    ("l", 0, "jerusslab1"), ("L", 0, "jerusslab2"), ("r", 0, "jerurockfloor"), ("g", 0, "jerusgrassidesand"), ("s", 0, "sand_center_1"),
    ("t", 0, "egyptroadfull"), ("p", 0, "cromeplatform3"), ("o", 0, "egyptgrassfull"), ("f", 0, "israelflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if wall_d(cx, cy) < 1.0 and not at_gate(cx, cy):
        return "X" if noise(cx, cy, 2) > 0.5 else "x"
    if near(CAMP, cx, cy, 1.0):
        return "f"
    if near(CAMP, cx, cy, 3.0) or near(EUROPEANS, cx, cy, 3.5) or near(AMERICANS, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5):
        return "p"
    if in_city(cx, cy):
        if abs(cx - TEMPLE_MOUNT[0]) < 7 and abs(cy - TEMPLE_MOUNT[1]) < 9:
            return "L" if noise(cx, cy, 2) > 0.5 else "l"   # the esplanade
        if roads.dist(cx, cy) < 1.0 or int(cx) % 5 == 0 or int(cy) % 4 == 0:
            return "j"
        return "J" if noise(cx, cy, 3) > 0.5 else "l"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    (ox, oy), r = OLIVES
    if math.hypot(cx - ox, cy - oy) < r:
        return "o" if noise(cx, cy, 3) > 0.45 else "r"   # olive groves on the mount
    return "g" if noise(cx, cy, 6) > 0.5 else ("r" if noise(cx + 7, cy, 4) > 0.6 else "s")

def height(vx, vy):
    (ox, oy), r = OLIVES
    h = 14 + noise(vx, vy, 10) * 6 * min(1.0, roads.dist(vx, vy) / 3) + max(0.0, r - math.hypot(vx - ox, vy - oy)) * 2.5
    if wall_d(vx, vy) < 1.0 and not at_gate(vx, vy):
        h = 44
    elif in_city(vx, vy):
        h = 18 + (3 if abs(vx - TEMPLE_MOUNT[0]) < 7 and abs(vy - TEMPLE_MOUNT[1]) < 9 else 0)
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 44.5, 64.5, 0, 1000)
m.obj("bank", 1, 46.5, 36.5, 0, 4000)
m.obj("moneyplane", 1, 46.5, 36.5, 0, 0)
m.obj("stank", 1, 52.5, 58.5, 0.0, 1000)
# our guns at the gates, clear of the holy places
for kind, x, y, hp in [("turret", 43.5, 50.5, 200), ("turret", 64.5, 33.5, 200), ("tower_missile", 76.5, 40.5, 600), ("turret", 57.5, 66.5, 200)]:
    m.obj(kind, 1, x, y, 0, hp)
# the holy places
m.obj("templebigfull3x4", 4, *TEMPLE_MOUNT, 0.0, 100)
for x, y in [SEPULCHRE, (62.5, 60.5), (56.5, 36.5), (74.5, 64.5)]:
    m.obj("jerustemple1", 4, x, y, 0.0, 100)
# the wall pieces along the top of the walls
(x0, y0), (x1, y1) = CITY
for k in range(int(x0) + 1, int(x1), 2):
    for y in (y0, y1):
        if not at_gate(k, y):
            m.obj("jeruswallfull1", 4, k + 0.5, y + 0.5, 0.0, 100)
for k in range(int(y0) + 1, int(y1), 2):
    for x in (x0, x1):
        if not at_gate(x, k):
            m.obj("jeruswallfull2", 4, x + 0.5, k + 0.5, math.pi / 2, 100)
for p, (x, y) in [(2, EUROPEANS), (5, AMERICANS), (6, ORDER)]:
    m.obj("mainbasepad", p, x, y, 0, 4000)
    m.obj("garage", p, x + (6.5 if p != 6 else -2.5), y + (4.5 if p != 5 else 8.5), 0, 1000)
    m.obj("bank", p, x - 5.5, y - 2.5, 0, 4000)
    m.obj("moneyplane", p, x - 5.5, y - 2.5, 0, 0)
m.obj("airbase", 2, 74.5, 8.5, 0, 1000)
for kind, p, x, y, hp in [("supergun", 6, 98.5, 40.5, 1200), ("tower_electric", 6, 92.5, 50.5, 600), ("turret", 2, 56.5, 16.5, 200),
                          ("tower_missile", 5, 20.5, 46.5, 600), ("turret", 5, 26.5, 58.5, 200)]:
    m.obj(kind, p, x, y, 0, hp)
m.roads(roads)
for _ in range(2600):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 1.8 or any(near(b, x, y, 6) for b in (CAMP, EUROPEANS, AMERICANS, ORDER)):
        continue
    if abs(x - TEMPLE_MOUNT[0]) < 8 and abs(y - TEMPLE_MOUNT[1]) < 10 or near(SEPULCHRE, x, y, 3):
        continue
    if t in "jJl" and in_city(x, y) and wall_d(x, y) > 2 and rng.random() < 0.12:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t == "o" and rng.random() < 0.12:
        m.obj("palmtree1", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Jerusalem_Israel.mp3", ["time 420"], ["no mainbasepad 1", "harm 1 3"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[12000, 12000, 12000, 12000])
