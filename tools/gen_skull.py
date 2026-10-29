"""Beneath the place of the skull, campaign mission 40: under Calloway's
chamber the rock opens into caverns that are not rock any more, and at
the end of them, in a great hall, the keyhole: the thing the forty sites
were the lock for. While it stands, Nephilim come up out of it. The
Watchers' Order (player 6) makes its last stand round it, and more giants
sleep in the side caves. It is dark down here. Close the keyhole: destroy
it. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(33)  # anno Domini
noise = Noise(rng)
W, H = 110, 96

CAMP = (55, 86)
HALL = ((55, 28), (22, 14))
KEYHOLE = (55, 22)
TUNNELS = [
    [(55, 84), (30, 72), (22, 54), (36, 38)],
    [(55, 84), (56, 66), (50, 52), (55, 40)],
    [(55, 84), (82, 74), (90, 56), (74, 38)],
]
SIDE_CAVES = [((14, 66), 5), ((96, 66), 5), ((38, 58), 4), ((76, 52), 4)]
GIANTS = [(14, 66), (96, 66), (38, 58), (76, 52), (40, 26), (70, 26)]

def tunnel(x, y):
    return min(seg_dist(x, y, *a, *b) for t in TUNNELS for a, b in zip(t, t[1:]))

def hall(x, y):
    (cx, cy), (a, b) = HALL
    return 1 - math.hypot((x - cx) / a, (y - cy) / b)

def open_ground(x, y):
    wob = (noise(x, y, 4) - 0.5) * 1.6
    return (tunnel(x, y) < 2.6 + wob or hall(x, y) > 0.05 * wob or near(CAMP, x, y, 8 + wob)
            or any(math.hypot(x - cx, y - cy) < r + wob for (cx, cy), r in SIDE_CAVES))

roads = Roads()
for k, t in enumerate(TUNNELS):
    for j, (x, y) in enumerate(t[1:], 1):
        roads.node(f"t{k}{j}", x + 0.5, y + 0.5)
roads.node("camp", 55.5, 82.5)
roads.node("key", KEYHOLE[0] + 0.5, KEYHOLE[1] + 4.5)
for k, t in enumerate(TUNNELS):
    roads.ours("camp", *[f"t{k}{j}" for j in range(1, len(t))], "key", them="W")

m = Map("skull", W, H)
LEGEND = [
    ("X", 4, "jerusrockwallbig3"), ("x", 4, "jerusrockwallbig"), ("a", 0, "alien_crust"), ("A", 0, "alien_crust2"),
    ("m", 0, "moon"), ("r", 0, "jerurockfloor"), ("p", 0, "cromeplatform3"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if not open_ground(cx, cy):
        return "X" if noise(cx, cy, 3) > 0.5 else "x"
    if near(CAMP, cx, cy, 3.5):
        return "p"
    if cy > 72:
        return "r"   # still Calloway's chamber, still rock
    if hall(cx, cy) > 0:
        return "m" if noise(cx, cy, 3) > 0.6 else "A"
    return "a" if noise(cx, cy, 4) > 0.45 else ("A" if noise(cx + 9, cy, 3) > 0.5 else "r")

def height(vx, vy):
    if not open_ground(vx, vy):
        return 70 + noise(vx, vy, 4) * 60
    return 12 + noise(vx, vy, 6) * 5 - max(0.0, hall(vx, vy)) * 6

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 48.5, 88.5, 0, 1000)
m.obj("bank", 1, 61.5, 84.5, 0, 4000)
m.obj("stank", 1, 58.5, 82.5, -math.pi / 2, 1000)
m.obj("keyhole", 6, *KEYHOLE, 0, 0)
m.obj("mainbasepad", 6, 40.5, 30.5, 0, 4000)
m.obj("garage", 6, 70.5, 30.5, 0, 1000)
m.obj("bank", 6, 55.5, 34.5, 0, 4000)
for kind, x, y, hp in [("tower_electric", 46.5, 22.5, 600), ("tower_electric", 64.5, 22.5, 600), ("tower_missile", 36.5, 40.5, 600),
                       ("tower_missile", 72.5, 38.5, 600), ("turret", 55.5, 42.5, 200), ("turret", 24.5, 52.5, 200), ("turret", 86.5, 58.5, 200)]:
    m.obj(kind, 6, x, y, 0, hp)
for x, y in GIANTS:
    m.obj("nephilim", 6, x + 0.5, y + 0.5, rng.uniform(-3.1, 3.1), 0)
m.roads(roads)
# bones in the caves
for _ in range(400):
    x, y = rng.uniform(4, W - 4), rng.uniform(4, H - 4)
    if open_ground(x, y) and roads.dist(x, y) > 2.5 and not near(CAMP, x, y, 8) and not near(KEYHOLE, x, y, 5) and rng.random() < 0.08:
        m.obj(rng.choice(["giant_skull", "giant_rib", "giant_spine"]), 4, x, y, rng.uniform(-3.1, 3.1), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Jerusalem_Israel.mp3", ["no keyhole 6"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank"],
        money=[15000, 0, 0, 15000], night=True)
