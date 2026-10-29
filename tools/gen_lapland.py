"""Lapland, campaign mission 32: the polar night over the fells. Night
(the ground lit only round our own) and fog of war (nothing seen until
something of ours has been near). The Watchers' Order (player 6) has two
camps somewhere in the north, each with a factory, sending its convoys
south over the frozen lakes; three roads lead out from our camp and we do
not know which ends where. Reindeer herds graze by the lakes and bolt from
the fighting. Destroy both factories. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1898)  # the year the Lapland reindeer herders' cooperatives were founded
noise = Noise(rng)
W, H = 130, 110

CAMP = (20, 98)
ORDER_EAST = (108, 22)
ORDER_NORTH = (52, 10)
FELLS = [((40, 40), 12), ((86, 56), 14), ((112, 84), 10), ((70, 28), 8), ((16, 60), 9)]
LAKES = [((62, 76), (12, 5)), ((94, 34), (7, 10)), ((36, 22), (10, 5)), ((22, 38), (5, 7))]

def fell(x, y):
    return max(0.0, max(r - math.hypot(x - cx, y - cy) for (cx, cy), r in FELLS) / 1.0)

def lake(x, y):
    """Inside a frozen lake: > 0."""
    return max(1 - math.hypot((x - cx) / a, (y - cy) / b) + (noise(x, y, 3) - 0.5) * 0.25 for (cx, cy), (a, b) in LAKES)

roads = Roads()
for n, x, y in [("camp", 22.5, 94.5), ("j1", 34.5, 86.5), ("a1", 52.5, 78.5), ("a2", 72.5, 76.5), ("a3", 96.5, 70.5), ("a4", 106.5, 48.5),
                ("a5", 96.5, 36.5), ("oe", 104.5, 24.5),
                ("b1", 28.5, 70.5), ("b2", 30.5, 50.5), ("b3", 26.5, 32.5), ("b4", 36.5, 22.5), ("on", 50.5, 14.5),
                ("c1", 56.5, 56.5), ("c2", 62.5, 40.5), ("c3", 80.5, 30.5),
                ("lavvu", 6.5, 54.5)]:
    roads.node(n, x, y)
# three ways out of camp, and none says where it goes: east over the big
# lake to the east camp; north past the Pallas fell over the small lakes to
# the north camp; and up the middle, which meets the east road; a branch
# west ends at a herders' camp
roads.ours("camp", "j1", "a1", "a2", "a3", "a4", "a5", "oe", them="W")
roads.ours("camp", "j1", "b1", "b2", "b3", "b4", "on", them="W")
roads.ours("a1", "c1", "c2", "c3", "a5", them="W")
roads.ours("b1", "lavvu")

m = Map("lapland", W, H)
LEGEND = [
    ("i", 0, "snow-mud_icepatch"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"), ("o", 0, "snow_rocks1"), ("O", 0, "snow_rocks2"),
    ("w", 0, "snowywoods1"), ("v", 0, "snowywoods2"), ("m", 0, "snow1-moguls"), ("t", 0, "switzsnowroadstraight"), ("p", 0, "cromeplatform3"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if near(CAMP, cx, cy, 3.5) or near(ORDER_EAST, cx, cy, 3.0) or near(ORDER_NORTH, cx, cy, 3.0):
        return "p"
    if lake(cx, cy) > 0:
        return "i"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    f = fell(cx, cy)
    if f > 5:
        return "O" if noise(cx, cy, 2) > 0.6 else ("m" if f > 8 else "o")   # the bare fell tops
    if f > 2:
        return "S" if noise(cx, cy, 3) > 0.5 else "s"
    return "w" if noise(cx, cy, 5) > 0.45 else ("v" if noise(cx + 9, cy, 4) > 0.5 else "s")

def height(vx, vy):
    if lake(vx, vy) > 0:
        return 8
    return 10 + fell(vx, vy) * 3.2 * min(1.0, roads.dist(vx, vy) / 4) + noise(vx, vy, 7) * 3

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 12.5, 102.5, 0, 1000)
m.obj("airbase", 1, 30.5, 104.5, 0, 1000)
m.obj("bank", 1, 10.5, 92.5, 0, 4000)
m.obj("moneyplane", 1, 10.5, 92.5, 0, 0)
m.obj("stank", 1, 26.5, 94.5, NORTH, 1000)
# the Order: two camps, each a factory under guns; one HQ, one bank
m.obj("mainbasepad", 6, 116.5, 14.5, 0, 4000)
m.obj("bank", 6, 120.5, 22.5, 0, 4000)
m.obj("moneyplane", 6, 120.5, 22.5, 0, 0)
for (gx, gy), guns in [(ORDER_EAST, [("tower_missile", 3.5, 5.0), ("turret", -4.0, 3.0), ("tower_electric", -2.0, 7.5)]),
                       (ORDER_NORTH, [("tower_missile", -4.0, 4.0), ("turret", 4.5, 3.5), ("supergun", 0.0, 7.0)])]:
    m.obj("garage", 6, gx + 0.5, gy + 0.5, 0, 1000)
    for kind, dx, dy in guns:
        m.obj(kind, 6, gx + dx + 0.5, gy + dy + 0.5, 0, {"turret": 200, "supergun": 1200}.get(kind, 600))
# the herds, by the lakes
for (hx, hy) in [(48, 70), (80, 66), (100, 42), (30, 28), (14, 48)]:
    for k in range(rng.randint(5, 8)):
        m.obj("reindeer", 4, hx + rng.uniform(-2.5, 2.5), hy + rng.uniform(-2.5, 2.5), rng.uniform(-3.1, 3.1), 100)
# the herders' camp: tents at the end of the west road
for dx, dy in [(-1.5, -1.0), (1.0, -2.0), (0.0, 1.5)]:
    m.obj("bigwhitetent", 4, 6.5 + dx, 54.5 + dy - 3.0, 0.0, 100)
m.roads(roads)
for _ in range(3200):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 1.8 or any(near(b, x, y, 6) for b in (CAMP, ORDER_EAST, ORDER_NORTH)):
        continue
    if t in "wv" and rng.random() < 0.22:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Finland.mp3", ["no garage 6"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 0, 0, 15000], night=True, fog=True)
