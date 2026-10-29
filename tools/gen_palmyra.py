"""Palmyra, campaign mission 8: the column's rearguard among the ruins of
Palmyra (the Great Colonnade, the Temple of Bel, the tower tombs, the
oasis), holding out while the American pursuit (player 5) closes in from
the east by three roads. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(272)  # the year Aurelian took Palmyra back
noise = Noise(rng)
W, H = 110, 90

CAMP = (34, 46)            # our camp, at the colonnade's west end
COLONNADE = ((30, 46), (70, 42))
BEL = (78, 52)             # the Temple of Bel's courtyard
OASIS = (24, 70)
AMERICANS = (100, 44)

roads = Roads()
for name, x, y in [("ours", 38.5, 50.5), ("c1", 50.5, 44.5), ("c2", 66.5, 42.5), ("east", 84.5, 42.5), ("am", 96.5, 47.5),
                   ("n1", 70.5, 22.5), ("n2", 50.5, 26.5), ("s1", 80.5, 70.5), ("s2", 56.5, 66.5), ("w", 40.5, 42.5)]:
    roads.node(name, x, y)
# ours: down the colonnade and out to the Americans' camp
roads.link("A", "ours", "w", "c1", "c2", "east", "am")
# theirs: three ways in, by the colonnade, round the north and round the
# south past the temple
roads.link("R", "am", "east", "c2", "c1", "w", "ours")
roads.link("R", "am", "east", "n1", "n2", "w")
roads.link("R", "am", "s1", "s2", "ours")

m = Map("palmyra", W, H)
LEGEND = [
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"), ("R", 0, "egyptroadfull"), ("p", 0, "cromeplatform3"),
    ("t", 0, "grectemple3"), ("T", 0, "grectemple1"), ("g", 0, "egyptgrassfull"), ("W", 6, "egyptwater1"),
]

def colonnade_dist(x, y):
    from mapgen import seg_dist
    return seg_dist(x, y, *COLONNADE[0], *COLONNADE[1])

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if near(OASIS, cx, cy, 3.2):
        return "W"  # the spring
    if near(CAMP, cx, cy, 3.5) or near(AMERICANS, cx, cy, 3.5):
        return "p"
    if abs(cx - BEL[0]) < 7 and abs(cy - BEL[1]) < 6:
        return "T" if abs(cx - BEL[0]) < 3 and abs(cy - BEL[1]) < 2 else "t"
    if colonnade_dist(cx, cy) < 1.6:
        return "R"  # the colonnaded street's paving
    if roads.dist(cx, cy) < 1.1:
        return "r"
    if near(OASIS, cx, cy, 13 + 3 * noise(cx, cy, 5)):
        return "g"
    if noise(cx + 300, cy, 5) > 0.8:
        return "c"
    return "S" if noise(cx, cy, 10) > 0.6 else "s"

def height(vx, vy):
    h = 9 + noise(vx, vy, 12) * 8 * min(1.0, roads.dist(vx, vy) / 6)
    # Jebel hills along the north edge
    if vy < 18:
        h += (18 - vy) * 2.2 * (0.6 + 0.4 * noise(vx + 40, vy, 9))
    if near(OASIS, vx, vy, 3.5):
        h = 4
    if near(CAMP, vx, vy, 7) or near(AMERICANS, vx, vy, 7):
        h = 10
    return h

NORTH, WEST = -math.pi / 2, math.pi
# our rearguard: the camp, a garage, two turrets on the colonnade, a bank
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 38.5, 54.5, 0, 1000)
m.obj("bank", 1, 26.5, 52.5, 0, 4000)
m.obj("moneyplane", 1, 26.5, 52.5, 0, 0)
m.obj("stank", 1, 44.5, 47.5, 0.0, 1000)
m.obj("turret", 1, 46.5, 40.5, 0, 200)
m.obj("turret", 1, 44.5, 58.5, 0, 200)
# the pursuit
m.obj("mainbasepad", 5, AMERICANS[0] + 3, AMERICANS[1] - 5, 0, 4000)
m.obj("garage", 5, 98.5, 50.5, 0, 1000)
m.obj("bank", 5, 104.5, 55.5, 0, 4000)
m.obj("moneyplane", 5, 104.5, 55.5, 0, 0)
for k, kind in enumerate(["peon_tank", "rocket_tank", "peon_tank", "electric_car"]):
    m.obj(kind, 5, 86.5 + (k % 2) * 1.5, 38.5 + k * 1.4, WEST, 0)
m.roads(roads)
# the Great Colonnade: columns along both sides of its street
(x0, y0), (x1, y1) = COLONNADE
L = math.hypot(x1 - x0, y1 - y0)
ux, uy = (x1 - x0) / L, (y1 - y0) / L
for k in range(int(L / 1.6)):
    t = k * 1.6
    for side in (-2.2, 2.2):
        if rng.random() < 0.8:
            x, y = x0 + ux * t - uy * side, y0 + uy * t + ux * side
            if roads.dist(x, y) > 1.2:
                m.obj("ruinsonepillar", 4, x, y, 0, 100)
# the Temple of Bel: its courtyard walls and the cella
bx, by = BEL
for k in range(-6, 7, 2):
    for (x, y, th) in [(bx + k, by - 6, 0.0), (bx + k, by + 6, 0.0), (bx - 7, by + k * 0.8, math.pi / 2), (bx + 7, by + k * 0.8, math.pi / 2)]:
        if roads.dist(x, y) > 2 and rng.random() < 0.75:
            m.obj(rng.choice(["ruinswall1", "ruinswall2"]), 4, x, y, th, 100)
m.obj("ruinstwopillar", 4, bx, by, 0, 100)
# the Valley of the Tombs, west: tower tombs standing in the sand
for _ in range(400):
    x, y = rng.uniform(3, 22), rng.uniform(20, 60)
    if roads.dist(x, y) > 3 and rng.random() < 0.06:
        m.obj("ruinsruin", 4, x, y, 0, 100)
# the oasis palms, and ruins about the old city
for _ in range(500):
    a, d = rng.uniform(0, 6.28), rng.uniform(4, 14)
    x, y = OASIS[0] + math.cos(a) * d, OASIS[1] + math.sin(a) * d
    if 1 < x < W - 1 and 1 < y < H - 1 and roads.dist(x, y) > 2 and rng.random() < 0.18:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
for _ in range(1200):
    x, y = rng.uniform(26, 90), rng.uniform(28, 64)
    if roads.dist(x, y) > 2.5 and colonnade_dist(x, y) > 3.5 and not near(CAMP, x, y, 8) and not (abs(x - bx) < 9 and abs(y - by) < 8) and rng.random() < 0.025:
        m.obj(rng.choice(["ruinsruin", "ruinswall1", "ruinstwopillar"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", ["time 300"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer"], money=[10000, 10000, 8000])
