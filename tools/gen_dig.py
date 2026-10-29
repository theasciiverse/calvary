"""The dig, campaign mission 39: Calloway's trench under Golgotha. It cuts
down into the hill in a zigzag, walled in rock, and the Watchers' Order
(player 6) brought the trench walls down on it in five places: rubble
only an engineer can clear. Their guns stand in alcoves along the trench,
their base is north of the hill, and their convoys come round it at our
camp. Get the supertank down to the chamber at the bottom. Writes a .cmap
to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(2047)  # the year Calloway dug here
noise = Noise(rng)
W, H = 120, 100

CAMP = (60, 92)
HILL = ((60, 48), (36, 37))           # centre, half axes
TRENCH = [(60, 86), (60, 80), (42, 70), (76, 60), (44, 48), (72, 36), (58, 24)]
CHAMBER = (58, 22)
ORDER = (60, 6)
# one pile in the middle of each leg down from the mouth of the trench
RUBBLE = [((ax + bx) / 2, (ay + by) / 2) for (ax, ay), (bx, by) in zip(TRENCH[1:], TRENCH[2:])]
ALCOVES = [(46, 76), (80, 65), (40, 52), (77, 34), (50, 30)]

def trench(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(TRENCH, TRENCH[1:]))

def hill(x, y):
    (cx, cy), (a, b) = HILL
    return 1 - math.hypot((x - cx) / a, (y - cy) / b)

def alcove(x, y):
    return any(math.hypot(x - ax, y - ay) < 2.4 for ax, ay in ALCOVES)

def open_ground(x, y):
    """Where something can stand inside the hill: the trench, the alcoves
    off it, the chamber."""
    return trench(x, y) < 1.6 or alcove(x, y) or math.hypot(x - CHAMBER[0], y - CHAMBER[1]) < 3.2 or any(
        seg_dist(x, y, ax, ay, *min(TRENCH, key=lambda p: math.hypot(p[0] - ax, p[1] - ay))) < 1.0 for ax, ay in ALCOVES)

roads = Roads()
for k, (x, y) in enumerate(TRENCH):
    roads.node(f"t{k}", x + 0.5, y + 0.5)
for n, x, y in [("camp", 60.5, 90.5), ("eu", 60.5, 10.5), ("w1", 20.5, 30.5), ("w2", 16.5, 70.5), ("e1", 100.5, 30.5), ("e2", 104.5, 72.5)]:
    roads.node(n, x, y)
roads.link("A", "camp", *[f"t{k}" for k in range(len(TRENCH))])
roads.link("W", "eu", "w1", "w2", "camp")
roads.link("W", "eu", "e1", "e2", "camp")
roads.link("A", "camp", "w2", "w1", "eu")

m = Map("dig", W, H)
LEGEND = [
    ("X", 4, "jerusrockwallbig"), ("x", 4, "jerusrockwallbig3"), ("r", 0, "jerurockfloor"), ("l", 0, "jerusslab1"),
    ("g", 0, "jerusgrassidesand"), ("s", 0, "sand_center_1"), ("t", 0, "egyptroadfull"), ("p", 0, "cromeplatform3"),
    ("o", 0, "egyptgrassfull"), ("a", 0, "alien_crust2"), ("c", 0, "sandcraters"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if hill(cx, cy) > 0:
        if math.hypot(cx - CHAMBER[0], cy - CHAMBER[1]) < 3.2:
            return "a"   # the chamber floor, not rock any more
        if open_ground(cx, cy):
            return "r" if noise(cx, cy, 2) > 0.3 else "l"
        return "X" if noise(cx, cy, 3) > 0.5 else "x"
    if near(CAMP, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    if cy > 80:
        return "c" if noise(cx, cy, 3) > 0.7 else ("o" if noise(cx, cy, 5) > 0.55 else "g")
    return "g" if noise(cx, cy, 6) > 0.5 else "s"

def height(vx, vy):
    hl = hill(vx, vy)
    if hl > 0:
        # the trench steps down toward the chamber; the rock stands over it
        if open_ground(vx, vy):
            depth = 1 - (vy - 22) / 60
            return 16 - depth * 8
        return 26 + hl * 60
    return 16 + noise(vx, vy, 9) * 5 * min(1.0, roads.dist(vx, vy) / 3)

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 52.5, 94.5, 0, 1000)
m.obj("airbase", 1, 70.5, 94.5, 0, 1000)
m.obj("bank", 1, 46.5, 88.5, 0, 4000)
m.obj("moneyplane", 1, 46.5, 88.5, 0, 0)
m.obj("stank", 1, 64.5, 90.5, 0.0, 1000)
for x, y in RUBBLE:
    m.obj("rubble", 4, x, y, 0.0, 100)
# the Order: guns in the alcoves, the base north of the hill
for (x, y), kind in zip(ALCOVES, ["turret", "tower_missile", "tower_electric", "turret", "tower_missile"]):
    m.obj(kind, 6, x + 0.5, y + 0.5, 0, 200 if kind == "turret" else 600)
m.obj("mainbasepad", 6, *ORDER, 0, 4000)
m.obj("garage", 6, 68.5, 6.5, 0, 1000)
m.obj("airbase", 6, 50.5, 6.5, 0, 1000)
m.obj("bank", 6, 76.5, 10.5, 0, 4000)
m.obj("moneyplane", 6, 76.5, 10.5, 0, 0)
for kind, x, y, hp in [("supergun", 76.5, 6.5, 1200), ("turret", 18.5, 48.5, 200), ("turret", 102.5, 50.5, 200)]:
    m.obj(kind, 6, x, y, 0, hp)
m.obj("digsite", 4, CHAMBER[0] + 0.5, CHAMBER[1] + 2.5, 0.0, 100)
m.roads(roads)
for _ in range(1500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, ORDER)) or hill(x, y) > -0.05:
        continue
    if t in "og" and rng.random() < 0.05:
        m.obj("palmtree1", 4, x, y, 0, 100)
    elif t == "s" and rng.random() < 0.01:
        m.obj(rng.choice(["bigwhitetent", "armycratebig"]), 4, x, y, 0.0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Jerusalem_Israel.mp3", [f"reach 1 stank {CHAMBER[0]} {CHAMBER[1]} 3 1"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[12000, 0, 0, 15000])
