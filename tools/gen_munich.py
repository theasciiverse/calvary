"""Munich, campaign mission 26: the Order's tank line. The Watchers' Order
(player 6) builds its armour in three factories, two at the Allach works
in the north-west and one at the Moosach rail yard, and a line of new
cobras waits in the works yard. The Isar runs through the city with three
bridges; our camp is east of it, the old town west. Destroy all three
factories. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1838)  # the year Krauss built his first locomotive in Munich
noise = Noise(rng)
W, H = 120, 100

ISAR = [(84, 0), (80, 30), (78, 60), (74, 100)]
BRIDGES = [30.5, 60.5, 86.5]
CAMP = (104, 70)
ALLACH = ((8, 6), (34, 30))       # the works' yard: corners
MOOSACH = (44, 30)
ORDER_HQ = (12, 34)
OLD_TOWN = (60, 58)
FACTORIES = [(16.5, 14.5), (29.5, 12.5), (MOOSACH[0] + 0.5, MOOSACH[1] + 0.5)]

def isar(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(ISAR, ISAR[1:]))

def in_works(x, y):
    (x0, y0), (x1, y1) = ALLACH
    return x0 <= x <= x1 and y0 <= y <= y1

roads = Roads()
for n, x, y in [("camp", 100.5, 66.5), ("e1", 92.5, 40.5), ("bn", 79.5, BRIDGES[0]), ("schw", 62.5, 30.5), ("moos", 46.5, 32.5), ("allach", 30.5, 20.5),
                ("bm", 78.5, BRIDGES[1]), ("old", 62.5, 58.5), ("nymph", 36.5, 52.5), ("west", 20.5, 40.5), ("hq", 14.5, 30.5),
                ("e2", 94.5, 84.5), ("bs", 75.5, BRIDGES[2]), ("s1", 44.5, 84.5), ("s2", 14.5, 66.5)]:
    roads.node(n, x, y)
# ours to their works by three ways; the Order drives them back
roads.ours("camp", "e1", "bn", "schw", "moos", "allach", them="W")
roads.ours("camp", "bm", "old", "nymph", "west", "hq", them="W")
roads.ours("camp", "e2", "bs", "s1", "s2", "west", them="W")
roads.ours("moos", "west", them="W")
roads.ours("allach", "hq", them="W")

m = Map("munich", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("w", 6, "egyptwater4"), ("B", 0, "rome_bridge1a"),
    ("c", 0, "cromerockbed3"), ("C", 0, "cromerockbed2a"), ("k", 0, "concrete_blocks"), ("f", 0, "factoryironstone1"), ("F", 0, "factoryironstone2"),
    ("r", 0, "trackovertrackcorner"), ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1"), ("t", 0, "airportroadplain"),
    ("g", 0, "grassanddirtfull"), ("G", 0, "francegrassfull"), ("p", 0, "cromeplatform3"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    d = isar(cx, cy)
    if d < 2.6:
        if on_road and any(abs(cy - by) < 1.2 for by in BRIDGES):
            return "B"
        return "W" if d < 1.7 else "w"
    if d < 5:
        return "G"   # the Isar's green banks
    if near(CAMP, cx, cy, 3.5) or near(ORDER_HQ, cx, cy, 3.0):
        return "p"
    if on_road:
        return "t"
    if in_works(cx, cy):
        return "f" if noise(cx, cy, 3) > 0.5 else ("F" if noise(cx + 40, cy, 3) > 0.5 else "k")
    if abs(cy - 36) < 1.0 and cx < 76:
        return "r"   # the railway east from the yard
    if near(MOOSACH, cx, cy, 7):
        return "r" if int(cy) % 2 == 0 else "C"   # the rail yard's sidings
    if math.hypot(cx - OLD_TOWN[0], cy - OLD_TOWN[1]) < 13:
        return "S" if (int(cx) % 6 == 0 or int(cy) % 5 == 0) else "s"
    if math.hypot(cx - 40, cy - 52) < 6:
        return "G"   # Nymphenburg's park
    return "c" if noise(cx, cy, 6) > 0.5 else ("g" if noise(cx + 80, cy, 9) > 0.62 else "C")

def height(vx, vy):
    if isar(vx, vy) < 2.2:
        return 4
    h = 12 + noise(vx, vy, 12) * 4 * min(1.0, roads.dist(vx, vy) / 4)
    if near(CAMP, vx, vy, 6):
        h = 12
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 108.5, 76.5, 0, 1000)
m.obj("airbase", 1, 110.5, 62.5, 0, 1000)
m.obj("bank", 1, 98.5, 76.5, 0, 4000)
m.obj("moneyplane", 1, 98.5, 76.5, 0, 0)
m.obj("stank", 1, 97.5, 64.5, math.pi, 1000)
# the Order: its HQ, the three factories, their guns, the tank line
m.obj("mainbasepad", 6, *ORDER_HQ, 0, 4000)
for x, y in FACTORIES:
    m.obj("garage", 6, x, y, 0, 4000)   # the original table's garage health
m.obj("bank", 6, 6.5, 40.5, 0, 4000)
m.obj("moneyplane", 6, 6.5, 40.5, 0, 0)
m.obj("bank", 6, 22.5, 26.5, 0, 4000)
m.obj("moneyplane", 6, 22.5, 26.5, 0, 0)
for kind, x, y, hp in [("tower_missile", 34.5, 18.5, 600), ("base_launcher", 22.5, 9.5, 1000), ("turret", 48.5, 36.5, 200), ("tower_electric", 40.5, 26.5, 600),
                       ("turret", 24.5, 42.5, 200), ("supergun", 18.5, 22.5, 1200), ("supergun", 38.5, 36.5, 1200), ("turret", 64.5, 32.5, 200), ("turret", 60.5, 62.5, 200)]:
    m.obj(kind, 6, x, y, 0, hp)
# each factory's own guns
for fx, fy in FACTORIES:
    m.obj("tower_missile", 6, fx + 3.0, fy + 2.5, 0, 600)
    m.obj("turret", 6, fx - 2.5, fy + 3.0, 0, 200)
m.obj("supergun", 6, 26.5, 26.5, 0, 1200)
for k in range(5):
    m.obj("cobra_tank", 6, 12.5 + k * 2.5, 20.5, 0.0, 0)
m.roads(roads)
# the works' sheds and the city's houses
for _ in range(2500):
    x, y = rng.uniform(3, W - 3), rng.uniform(3, H - 3)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.4 or near(CAMP, x, y, 8) or near(ORDER_HQ, x, y, 5) or any(near(f, x, y, 4) for f in FACTORIES):
        continue
    if t in "fFk" and rng.random() < 0.05:
        m.obj(rng.choice(["buildwarehouseloadingdock", "silobigfull2x2"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "sS" and rng.random() < 0.07:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "building2squnderconst"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "cC" and rng.random() < 0.012:
        m.obj(rng.choice(["building2squnderconst", "greecehouses3", "buildwarehouseloadingdock"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t == "G" and rng.random() < 0.06:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Munich_Germany.mp3", ["no garage 6"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 0, 0, 20000])
