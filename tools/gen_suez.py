"""The Suez Canal, campaign mission 11: six trucks from the east bank to
the west, over three crossings where only the stubs of the old piers
are left, one cell of water between them for a bridger to span. The
Europeans (player 2) hold the west bank and shoot the decks away; the
American pursuit (player 5) comes down the east bank. Writes a .cmap to
argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(1869)  # the year the canal opened
noise = Noise(rng)
W, H = 120, 90

CANAL_X, CANAL_HALF = 60, 4
CROSSINGS = [22, 45, 68]
CAMP = (110, 48)
POST = (9.5, 44.5)
EUROPEANS = (14, 80)
AMERICANS = (108, 9)

roads = Roads()
roads.node("start", 103.5, 48.5)
roads.node("j", 90.5, 45.5)
roads.node("merge", 34.5, 45.5)
roads.node("post", *POST)
for k, y in enumerate(CROSSINGS):
    roads.node(f"e{k}", 70.5, y + 0.5)
    roads.node(f"g{k}", CANAL_X + 0.5, y + 0.5, bridge=True)
    roads.node(f"w{k}", 50.5, y + 0.5)
    roads.ours("j", f"e{k}", f"g{k}", f"w{k}", "merge")
roads.ours("start", "j")
roads.ours("merge", "post")
# the Europeans: from their camp to each crossing and over it
roads.node("eu", 20.5, 75.5)
roads.node("eu1", 36.5, 64.5)
for k in range(3):
    roads.link("E", "eu", "eu1", f"w{k}", f"g{k}", f"e{k}", "j", "start")
# the Americans: down the east bank behind the column
roads.node("am", 102.5, 14.5)
roads.node("am1", 88.5, 26.5)
roads.link("R", "am", "am1", "j", "start")
for k in range(3):
    roads.link("R", "am1", f"e{k}", f"g{k}", f"w{k}", "merge", "post")

def canal(x, y):
    return abs(x - (CANAL_X + 0.5)) < CANAL_HALF + 0.01

def pier(x, y):
    """The old piers: one row reaching out from each bank at each crossing,
    stopping a cell short of each other."""
    return any(int(y) == c for c in CROSSINGS) and canal(x, y) and int(x) != CANAL_X

m = Map("suez", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("w", 6, "egyptwater4"), ("k", 0, "cromeplatform3"),
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"), ("p", 0, "airportroadplain"), ("g", 0, "egyptgrassfull"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if pier(cx, cy):
        return "k"
    if canal(cx, cy):
        return "W" if abs(cx - (CANAL_X + 0.5)) < CANAL_HALF - 1 else "w"
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5) or near(AMERICANS, cx, cy, 3.5) or near(POST, cx, cy, 2.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "r"
    if abs(cx - (CANAL_X + 0.5)) < CANAL_HALF + 3 + 2 * noise(cx, cy, 5):
        return "g"  # the green strip the canal water keeps alive
    if noise(cx + 300, cy, 5) > 0.82:
        return "c"
    return "S" if noise(cx, cy, 9) > 0.6 else "s"

def height(vx, vy):
    d = abs(vx - (CANAL_X + 0.5))
    if d < CANAL_HALF - 0.5 and not any(abs(vy - (c + 0.5)) < 0.6 for c in CROSSINGS):
        return 0
    if d < CANAL_HALF + 1:
        return 6   # the banks and the piers, at the water's edge
    return 8 + noise(vx, vy, 11) * 12 * min(1.0, roads.dist(vx, vy) / 5) * min(1.0, (d - CANAL_HALF) / 8)

WEST = math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 106.5, 54.5, 0, 1000)
m.obj("bank", 1, 115.5, 41.5, 0, 4000)
m.obj("moneyplane", 1, 115.5, 41.5, 0, 0)
m.obj("stank", 1, 96.5, 46.5, WEST, 1000)
for k in range(6):
    m.obj("car", 1, 99.0 + k * 1.5, 47.5 + (k % 2) * 0.8, WEST, 0)
m.obj("turret", 1, POST[0] + 3, POST[1] - 3, 0, 200)
m.obj("bigwhitetent", 4, POST[0] - 3, POST[1] + 3, 0, 100)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 20.5, 79.5, 0, 1000)
m.obj("bank", 2, 8.5, 72.5, 0, 4000)
m.obj("moneyplane", 2, 8.5, 72.5, 0, 0)
for kind, x, y, hp in [("turret", 45.5, 18.5, 200), ("tower_missile", 45.5, 40.5, 600), ("turret", 45.5, 72.5, 200), ("tower_electric", 30.5, 49.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
m.obj("mainbasepad", 5, AMERICANS[0] + 4, AMERICANS[1] - 2, 0, 4000)
m.obj("garage", 5, 100.5, 10.5, 0, 1000)
m.obj("bank", 5, 114.5, 14.5, 0, 4000)
m.obj("moneyplane", 5, 114.5, 14.5, 0, 0)
m.roads(roads)
# ships waiting in the canal, palms along the strip, the old piers' huts
for y in (10.5, 33.5, 57.5, 80.5):
    m.obj("greeceship1", 4, CANAL_X + 0.5 + rng.uniform(-1, 1), y, math.pi / 2, 100)
for _ in range(1500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if tile(int(x), int(y)) == "g" and roads.dist(x, y) > 2 and rng.random() < 0.12:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
for k, y in enumerate(CROSSINGS):
    for x in (CANAL_X - CANAL_HALF - 2.5, CANAL_X + CANAL_HALF + 3.5):
        m.obj("cabin_small", 4, x, y + 3.5, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", [f"reach 1 car {POST[0]} {POST[1]} 5 4"], ["no mainbasepad 1", "lost 1 car 3"],
        build=["peon_tank", "rocket_tank", "trike", "engineer", "bridger"], money=[10000, 8000, 6000])
