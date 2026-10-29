"""Jaffa, campaign mission 37: the landing. We come ashore on the beach
below Jaffa's old town, and all three armies are waiting: the Europeans
(player 2) from Tel Aviv in the north, the Americans (player 5) from the
south, the Watchers' Order (player 6) down the road from Jerusalem in
the east. The Order holds the old town on its hill. Take it and hold it
two minutes. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1909)  # the year Tel Aviv was founded on the dunes north of Jaffa
noise = Noise(rng)
W, H = 120, 100

CAMP = (22, 74)
JAFFA = (30, 52)               # the old town on its hill, by the port
EUROPEANS = (40, 8)
AMERICANS = (44, 94)
ORDER = (110, 52)

def shore(y):
    return 14 + (noise(3, y, 9) - 0.5) * 5 + (4 if abs(y - JAFFA[1]) < 6 else 0)

def hill(x, y):
    return max(0.0, 9 - math.hypot(x - JAFFA[0], y - JAFFA[1]))

roads = Roads()
for n, x, y in [("camp", 24.5, 70.5), ("jaffa", 30.5, 52.5), ("n1", 34.5, 34.5), ("n2", 38.5, 16.5), ("s1", 36.5, 82.5), ("s2", 42.5, 90.5),
                ("e1", 52.5, 54.5), ("e2", 74.5, 48.5), ("e3", 96.5, 52.5)]:
    roads.node(n, x, y)
roads.ours("camp", "jaffa")
roads.link("A", "jaffa", "n1", "n2")
roads.link("A", "camp", "s1", "s2")
roads.link("A", "jaffa", "e1", "e2", "e3")
# each army's own way to the old town and on to our beach
roads.link("E", "n2", "n1", "jaffa", "camp")
roads.link("R", "s2", "s1", "camp", "jaffa")
roads.link("W", "e3", "e2", "e1", "jaffa", "camp")

m = Map("jaffa", W, H)
LEGEND = [
    ("W", 6, "seagreecewaterfull"), ("b", 0, "seaandsandside"), ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"),
    ("g", 0, "egyptgrassfull"), ("t", 0, "egyptroadfull"), ("j", 0, "jerusstreetfull"), ("J", 0, "jerusslab1"),
    ("k", 0, "grecdock2"), ("p", 0, "cromeplatform3"), ("c", 0, "cromeroof"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    sh = shore(cy)
    if cx < sh - 0.8:
        return "W"
    if cx < sh + 0.6:
        return "k" if abs(cy - JAFFA[1]) < 6 else "b"
    if near(CAMP, cx, cy, 3.0) or near(EUROPEANS, cx, cy, 3.5) or near(AMERICANS, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    if hill(cx, cy) > 2:
        return "J" if noise(cx, cy, 2) > 0.55 else "j"
    if cy < 26 and cx > 26:
        return "c" if noise(cx, cy, 4) > 0.55 else "s"   # Tel Aviv
    if cx < sh + 7:
        return "s"   # the beach
    return "g" if noise(cx, cy, 5) > 0.55 else "S"   # the orange groves and the plain

def height(vx, vy):
    if vx < shore(vy) - 0.5:
        return 5
    return 10 + hill(vx, vy) * 2.2 + noise(vx, vy, 9) * 4 * min(1.0, roads.dist(vx, vy) / 3) + max(0.0, vx - 80) * 0.3

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 26.5, 80.5, 0, 1000)
m.obj("bank", 1, 30.5, 74.5, 0, 4000)
m.obj("moneyplane", 1, 30.5, 74.5, 0, 0)
m.obj("aircraftcarrier", 1, 8.5, 80.0, -math.pi / 2, 4000)
m.obj("stank", 1, 26.5, 68.5, -math.pi / 2, 1000)
for p, (x, y) in [(2, EUROPEANS), (5, AMERICANS), (6, ORDER)]:
    m.obj("mainbasepad", p, x, y, 0, 4000)
    m.obj("garage", p, x + 6.5, y + (3.5 if y < 50 else -3.5), 0, 1000)
    m.obj("bank", p, x - 5.5, y, 0, 4000)
    m.obj("moneyplane", p, x - 5.5, y, 0, 0)
m.obj("airbase", 2, 58.5, 10.5, 0, 1000)
# the Order's garrison in the old town
for kind, x, y, hp in [("tower_missile", 28.5, 48.5, 600), ("turret", 33.5, 55.5, 200), ("tower_electric", 26.5, 56.5, 600), ("turret", 35.5, 48.5, 200)]:
    m.obj(kind, 6, x, y, 0, hp)
for kind, p, x, y, hp in [("turret", 2, 36.5, 22.5, 200), ("tower_missile", 2, 46.5, 14.5, 600), ("turret", 5, 40.5, 86.5, 200), ("supergun", 5, 54.5, 92.5, 1200),
                          ("turret", 6, 80.5, 46.5, 200), ("supergun", 6, 102.5, 46.5, 1200)]:
    m.obj(kind, p, x, y, 0, hp)
m.roads(roads)
for _ in range(2400):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, EUROPEANS, AMERICANS, ORDER)) or near(JAFFA, x, y, 3):
        continue
    if t in "jJ" and rng.random() < 0.18:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t == "c" and rng.random() < 0.08:
        m.obj(rng.choice(["building2squnderconst", "buildwarehouseloadingdock"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t == "g" and rng.random() < 0.06:
        m.obj("palmtree1", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Jerusalem_Israel.mp3", [f"hold 1 {JAFFA[0]} {JAFFA[1]} 5 120"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[12000, 10000, 10000, 10000])
