"""Alexandria, campaign mission 14: the Eastern Harbour, three ships at
the quay, and the Europeans (player 2) holding the waterfront. Each ship
is boarded from a takeover pad on the quay; two taken and the column can
sail for Greece. A squadron of European jets and bombers is already in
the air when the battle opens. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(331)  # Alexander founded the city in 331 BC
noise = Noise(rng)
W, H = 120, 90

QUAY_Y = 27
SHIPS = [38.5, 60.5, 82.5]  # x of each ship's berth
PHAROS = (16, 12)
CAMP = (102, 76)
EUROPEANS = (18, 70)

def sea(x, y):
    # the harbour: sea north of the quay, the Pharos island off the west
    if near(PHAROS, x, y, 6):
        return False
    return y < QUAY_Y - 1 + 2 * math.sin(x / 17.0) * (x < 24 or x > 96)

roads = Roads()
for n, x, y in [("ours", 98.5, 72.5), ("j", 88.5, 62.5), ("c1", 84.5, 44.5), ("q3", SHIPS[2], QUAY_Y + 2.5), ("q2", SHIPS[1], QUAY_Y + 2.5),
                ("q1", SHIPS[0], QUAY_Y + 2.5), ("c2", 28.5, 44.5), ("eu", 22.5, 66.5), ("s1", 60.5, 66.5)]:
    roads.node(n, x, y)
roads.ours("ours", "j", "c1", "q3", "q2", "q1", "c2", "eu")
roads.ours("j", "s1", "eu")

m = Map("alexandria", W, H)
LEGEND = [
    ("W", 6, "seagreecewaterfull"), ("w", 6, "seawaterfull2"), ("k", 0, "grecdock2"), ("K", 0, "grecdock1"),
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("r", 0, "egyptlargetrackfull"), ("R", 0, "egyptroadfull"),
    ("t", 0, "egyptsandwalltexture2"), ("p", 0, "cromeplatform3"), ("g", 0, "egyptgrassfull"), ("b", 0, "seaandsandside"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if sea(cx, cy):
        return "W" if sea(cx, cy + 2) and sea(cx, cy + 3) else "w"
    if cy < QUAY_Y + 1.5 and 30 < cx < 92:
        return "K" if int(cx) % 6 == 0 else "k"   # the quay's stone
    if sea(cx, cy - 1.5):
        return "b"   # beach where there is no quay
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "r"
    if 30 < cx < 92 and QUAY_Y + 4 < cy < 60:
        # the city: streets on a grid, houses between
        return "R" if (int(cx) % 9 == 0 or int(cy) % 8 == 0) else "t"
    if noise(cx, cy, 6) > 0.7:
        return "g"
    return "S" if noise(cx, cy, 9) > 0.6 else "s"

def height(vx, vy):
    if sea(vx, vy) and sea(vx, vy + 1):
        return 0
    if vy < QUAY_Y + 2:
        return 6
    h = 8 + noise(vx, vy, 12) * 8 * min(1.0, roads.dist(vx, vy) / 5) * (0.3 if 30 < vx < 92 and vy < 60 else 1.0)
    if near(CAMP, vx, vy, 7) or near(EUROPEANS, vx, vy, 7):
        h = 9
    return h

NORTH, EAST, WEST = -math.pi / 2, 0.0, math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 96.5, 80.5, 0, 1000)
m.obj("bank", 1, 108.5, 70.5, 0, 4000)
m.obj("moneyplane", 1, 108.5, 70.5, 0, 0)
m.obj("stank", 1, 94.5, 68.5, NORTH, 1000)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 24.5, 74.5, 0, 1000)
m.obj("airbase", 2, 12.5, 62.5, 0, 1000)
m.obj("bank", 2, 10.5, 76.5, 0, 4000)
m.obj("moneyplane", 2, 10.5, 76.5, 0, 0)
# the ships, and the pads on the quay that board them
for x in SHIPS:
    m.obj("greeceship1", 4, x, QUAY_Y - 2.5, 0.0, 100)
    m.obj("takeover_pad", 0, x, QUAY_Y + 0.5, 0, 100)
# the waterfront's guns
for kind, x, y, hp in [("turret", 49.5, 31.5, 200), ("turret", 71.5, 31.5, 200), ("tower_missile", 60.5, 34.5, 600), ("tower_electric", 32.5, 36.5, 600), ("turret", 30.5, 58.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# the strike: already in the air over the city, coming east
for k, kind in enumerate(["jet", "jet", "bomber", "jet", "jet", "bomber", "bomber"]):
    m.obj(kind, 2, 40.5 + (k % 3) * 2.5, 50.5 + (k // 3) * 2.5, EAST, 0)
m.roads(roads)
m.obj("greclighthouse", 4, *PHAROS, 0, 100)
for _ in range(1600):
    x, y = rng.uniform(31, 91), rng.uniform(QUAY_Y + 5, 59)
    if tile(int(x), int(y)) == "t" and roads.dist(x, y) > 2.2 and rng.random() < 0.12:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x, y, rng.choice([0, 1.5708]), 100)
for _ in range(600):
    x, y = rng.uniform(2, W - 2), rng.uniform(QUAY_Y + 3, H - 2)
    if tile(int(x), int(y)) == "g" and roads.dist(x, y) > 2 and rng.random() < 0.12:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
for x in (34.5, 44.5, 66.5, 76.5, 88.5):
    m.obj(rng.choice(["armycratebig", "onelargecrate"]), 4, x, QUAY_Y + 0.5, rng.uniform(0, 3), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", ["own takeover_pad 1 2"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter"], money=[10000, 8000, 10000])
