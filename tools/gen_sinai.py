"""Sinai, campaign mission 10: the column crosses the desert westward
with almost nothing in its war chest and no bank of its own; every bank
on the way is neutral, and only the money planes, once an engineer has
taken a bank, bring anything home. The Europeans (player 2) hold the
Mitla Pass between two ridges in the west. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(1956)  # the paratroopers' drop at the Mitla Pass
noise = Noise(rng)
W, H = 140, 80

CAMP = (128, 42)
EUROPEANS = (12, 40)
PASS_X = (30, 44)          # the pass runs between these x, through the ridges
BANKS = [(98.5, 22.5), (88.5, 60.5), (62.5, 40.5)]
SPRING = (58.5, 47.5)

def ridge(x, y):
    """The two ridges of the pass: north and south of a corridor that
    narrows to five cells."""
    if not (PASS_X[0] <= x <= PASS_X[1]):
        return False
    mid = 40 + 3 * math.sin(x / 5.0)
    half = 2.5 + 0.18 * abs(x - (PASS_X[0] + PASS_X[1]) / 2) ** 1.4
    return abs(y - mid) > half

roads = Roads()
pts = {"start": (122.5, 44.5), "j": (112.5, 42.5),
       "n1": (98.5, 25.5), "n2": (78.5, 28.5), "s1": (88.5, 57.5), "s2": (72.5, 52.5),
       "oasis": (62.5, 43.5), "east_mouth": (48.5, 40.5), "pass": (37.5, 40.5), "west_mouth": (26.5, 40.5), "eu": (16.5, 43.5)}
for n, p in pts.items():
    roads.node(n, *p)
roads.ours("start", "j", "n1", "n2", "oasis", "east_mouth", "pass", "west_mouth", "eu")
roads.ours("j", "s1", "s2", "oasis")

m = Map("sinai", W, H)
LEGEND = [
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"), ("p", 0, "cromeplatform3"), ("C", 4, "cliff1-sand_side"),
    ("m", 0, "egyptsandrockpointandpoint"), ("g", 0, "egyptgrassfull"), ("W", 6, "egyptwater1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if x in (0, W - 1) or y in (0, H - 1):
        return "C"
    if ridge(cx, cy):
        inner = ridge(cx - 1.2, cy) and ridge(cx + 1.2, cy) and ridge(cx, cy - 1.2) and ridge(cx, cy + 1.2)
        return "m" if inner else "C"
    if near(SPRING, cx, cy, 2.0):
        return "W"  # the oasis spring by the middle bank
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "r"
    if near(BANKS[2], cx, cy, 8 + 2 * noise(cx, cy, 4)):
        return "g"
    if noise(cx + 300, cy, 5) > 0.82:
        return "c"
    return "S" if noise(cx, cy, 9) > 0.58 else "s"

def height(vx, vy):
    # long dunes running north to south, flattened along the roads
    dune = (0.5 + 0.5 * math.sin(vx / 3.3 + 2 * noise(vx, vy, 13))) * 10 * noise(vx + 7, vy, 8)
    h = 8 + dune * min(1.0, roads.dist(vx, vy) / 5)
    if ridge(vx, vy):
        h = 60 + 30 * noise(vx, vy + 30, 6)
    if near(CAMP, vx, vy, 7) or near(EUROPEANS, vx, vy, 7):
        h = 9
    if near(SPRING, vx, vy, 2.5):
        h = 4
    return h

WEST = math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 126.5, 48.5, 0, 1000)
m.obj("stank", 1, 118.5, 42.5, WEST, 1000)
# the only money in the desert: neutral banks, a money plane at each, ours
# once an engineer takes the bank
for x, y in BANKS:
    m.obj("bank", 0, x, y, 0, 4000)
m.obj("moneyplane", 1, CAMP[0] - 3, CAMP[1] - 4, 0, 0)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 18.5, 46.5, 0, 1000)
m.obj("bank", 2, 8.5, 32.5, 0, 4000)
m.obj("moneyplane", 2, 8.5, 32.5, 0, 0)
# the pass's guns, and a battery at its eastern mouth
for kind, x, y, hp in [("tower_missile", 37.5, 31.5, 600), ("tower_missile", 37.5, 49.5, 600), ("turret", 47.5, 36.5, 200), ("turret", 47.5, 45.5, 200), ("turret", 22.5, 38.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# a rocket company out in the dunes, coming east
for k, kind in enumerate(["rocket_tank", "rocket_tank", "peon_tank"]):
    m.obj(kind, 2, 70.5 + k * 1.5, 30.5 + k, 0.0, 0)
m.roads(roads)
for _ in range(1600):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    if tile(int(x), int(y)) == "g" and roads.dist(x, y) > 2 and rng.random() < 0.3:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
    elif tile(int(x), int(y)) in "sS" and roads.dist(x, y) > 3 and rng.random() < 0.004:
        m.obj(rng.choice(["ruinsruin", "armycratebig"]), 4, x, y, rng.uniform(0, 3), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", ["no mainbasepad 2"], ["no mainbasepad 1"],
        build=["peon_tank", "rocket_tank", "trike", "engineer"], money=[4000, 8000, 10000])
