"""Chartres, campaign mission 24: the cathedral on its hill above the Eure
and the wheat of the Beauce. The Veil lies in the cathedral, and we hold
the town. The Watchers' Order (player 6) comes for it from the east, the
Europeans (player 2) from the north-west over the Eure, and they fight
each other on the way. Hold out six minutes without either carrying the
Veil home. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1194)  # the fire that spared the Veil, and the cathedral rebuilt after it
noise = Noise(rng)
W, H = 110, 100

CATHEDRAL = (58, 46)
VEIL = (57.5, 46.5)
HILL = ((58, 50), 16)
CAMP = (66, 58)
WATCHERS = (102, 54)
EUROPEANS = (12, 14)
EURE = [(28, 0), (32, 30), (26, 60), (30, 100)]

def eure(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(EURE, EURE[1:]))

roads = Roads()
for n, x, y in [("close", 58.5, 50.5), ("camp", 64.5, 55.5), ("e1", 74.5, 50.5), ("e2", 88.5, 48.5), ("wb", 98.5, 52.5),
                ("se1", 72.5, 66.5), ("se2", 90.5, 64.5), ("nw1", 48.5, 38.5), ("eb", 36.5, 30.5), ("ww", 24.5, 28.5), ("eu", 16.5, 18.5),
                ("sw1", 46.5, 60.5), ("sb", 34.5, 70.5), ("sw2", 18.5, 64.5)]:
    roads.node(n, x, y)
# ours: out of the town to both of them; they come back the same ways
roads.ours("camp", "close", "e1", "e2", "wb")
roads.ours("camp", "se1", "se2", "wb")
roads.ours("camp", "close", "nw1", "eb", "ww", "eu")
roads.ours("camp", "sw1", "sb", "sw2", "eu")
# the Order's own way in from the east
roads.link("W", "wb", "e2", "e1", "close")
roads.link("W", "wb", "se2", "se1", "camp")

m = Map("chartres", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("B", 0, "rome_bridge1a"), ("g", 0, "francegrassfull"), ("G", 0, "francegrassstripofrock2"),
    ("f", 0, "dirtplantsfrancestrips"), ("F", 0, "dirtfrancestrips"), ("t", 0, "francegrassroadfull"),
    ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1"), ("m", 0, "marble_small1"), ("p", 0, "cromeplatform3"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    if eure(cx, cy) < 1.7:
        return "B" if on_road else "W"
    if near(CAMP, cx, cy, 3.0) or near(WATCHERS, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if abs(cx - CATHEDRAL[0]) < 7 and abs(cy - CATHEDRAL[1]) < 5:
        return "m"   # the cathedral close
    if on_road:
        return "t"
    (hx, hy), hr = HILL
    if math.hypot(cx - hx, cy - hy) < hr - 2:
        return "S" if (int(cx) % 5 == 0 or int(cy) % 5 == 0) and noise(cx, cy, 4) > 0.3 else "s"   # the old town
    if eure(cx, cy) < 5:
        return "g" if noise(cx, cy, 4) > 0.4 else "G"   # water meadows
    # the Beauce: long fields, ploughed or standing
    band = int((cx * 0.8 + cy * 0.25) / 7)
    return "f" if (band + int(noise(cx, cy, 20) * 3)) % 2 else "F"

def height(vx, vy):
    (hx, hy), hr = HILL
    d = math.hypot(vx - hx, vy - hy)
    h = 10 + noise(vx, vy, 14) * 4 + max(0.0, hr - d) * 2.2
    if eure(vx, vy) < 1.5:
        h = 4
    if near(WATCHERS, vx, vy, 6) or near(EUROPEANS, vx, vy, 6):
        h = 10
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 70.5, 60.5, 0, 1000)
m.obj("bank", 1, 62.5, 62.5, 0, 4000)
m.obj("moneyplane", 1, 62.5, 62.5, 0, 0)
m.obj("stank", 1, 61.5, 54.5, -math.pi / 2, 1000)
for kind, x, y, hp in [("turret", 66.5, 44.5, 200), ("turret", 50.5, 42.5, 200), ("turret", 52.5, 54.5, 200)]:
    m.obj(kind, 1, x, y, 0, hp)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 20.5, 10.5, 0, 1000)
m.obj("bank", 2, 8.5, 22.5, 0, 4000)
m.obj("moneyplane", 2, 8.5, 22.5, 0, 0)
m.obj("mainbasepad", 6, *WATCHERS, 0, 4000)
m.obj("garage", 6, 100.5, 62.5, 0, 1000)
m.obj("bank", 6, 105.5, 44.5, 0, 4000)
m.obj("moneyplane", 6, 105.5, 44.5, 0, 0)
m.obj("cathedral", 4, *CATHEDRAL, 0.0, 100)
m.obj("relic", 0, *VEIL, 0, 100)
m.roads(roads)
# the town's houses round the cathedral, farms out on the plain
(hx, hy), hr = HILL
for _ in range(1500):
    x, y = rng.uniform(hx - hr, hx + hr), rng.uniform(hy - hr, hy + hr)
    if math.hypot(x - hx, y - hy) < hr - 2.5 and roads.dist(x, y) > 1.8 and not (abs(x - CATHEDRAL[0]) < 8 and abs(y - CATHEDRAL[1]) < 6) and not near(CAMP, x, y, 6) and rng.random() < 0.08:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for _ in range(40):
    x, y = rng.uniform(4, W - 4), rng.uniform(4, H - 4)
    if math.hypot(x - hx, y - hy) > hr + 4 and roads.dist(x, y) > 3 and eure(x, y) > 4 and not any(near(b, x, y, 8) for b in (WATCHERS, EUROPEANS)):
        m.obj(rng.choice(["silobigfull2x2", "cabin_small", "buildwarehouseloadingdock"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for _ in range(600):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if 1.8 < eure(x, y) < 5 and roads.dist(x, y) > 2 and rng.random() < 0.2:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Turin_Italy.mp3", ["time 360"],
        [f"reach 6 relic {WATCHERS[0]} {WATCHERS[1]} 4 1", f"reach 2 relic {EUROPEANS[0]} {EUROPEANS[1]} 4 1", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank"], money=[10000, 10000, 0, 10000])
