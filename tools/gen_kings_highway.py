"""The King's Highway, campaign mission 9: six trucks south down the old
road through the Jordanian highlands. The highway passes Kerak, whose
castle the Europeans (player 2) hold; the desert road to the east is
longer and the American pursuit (player 5) raids it. Wadi Mujib cuts
across both, crossable only where the roads climb through it. The road
west to Jerusalem is closed by a rockfall. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1188)  # the year Saladin took Kerak
noise = Noise(rng)
W, H = 90, 150

CAMP = (44, 10)
DEST = (46.5, 140.5)
KERAK = (34, 82)            # the castle, west of the highway
AMERICANS = (82, 9)
MUJIB_Y = 46                 # the canyon's line

def mujib_y(x):
    return MUJIB_Y + 4 * math.sin(x / 11.0) + 1.5 * math.sin(x / 4.1)

roads = Roads()
pts = {
    "start": (44.5, 16.5), "j1": (45.5, 28.5),
    # the King's Highway: down through the wadi, past Kerak
    "k1": (40.5, mujib_y(40) - 5), "k2": (41.5, mujib_y(41) + 5), "k3": (42.5, 66.5), "kerak": (42.5, 82.5), "k4": (44.5, 100.5),
    # the desert road, east
    "d1": (70.5, mujib_y(70) - 5), "d2": (71.5, mujib_y(71) + 5), "d3": (74.5, 72.5), "d4": (70.5, 100.5),
    "x": (57.5, 86.5),          # a crossover between them, south of Kerak's guns
    "j2": (47.5, 118.5), "dest": DEST,
    "am": (78.5, 14.5), "am1": (66.5, 22.5),
    "eu": (32.5, 84.5),
}
for n, (x, y) in pts.items():
    roads.node(n, x, y)
roads.link("A", "start", "j1", "k1", "k2", "k3", "kerak", "k4", "j2", "dest")
roads.link("A", "j1", "d1", "d2", "d3", "d4", "j2")
roads.link("A", "k3", "x", "d4")
roads.link("A", "d3", "x", "k4")
# the Americans come down behind the column, by either road
roads.link("R", "am", "am1", "j1", "k1", "k2", "k3", "kerak", "k4", "j2", "dest")
roads.link("R", "am1", "d1", "d2", "d3", "d4", "j2")
# Kerak's garrison sallies onto the highway, north and south
roads.link("E", "eu", "kerak", "k3", "k2", "k1", "j1", "start")
roads.link("E", "kerak", "k4", "j2", "dest")

def canyon(x, y):
    """Inside Wadi Mujib, away from where the roads climb through."""
    return abs(y - mujib_y(x)) < 3.2 and roads.dist(x, y) > 2.2

def rim(x, y):
    return abs(y - mujib_y(x)) < 4.6 and roads.dist(x, y) > 2.2

m = Map("kings-highway", W, H)
LEGEND = [
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"), ("p", 0, "cromeplatform3"), ("C", 4, "cliff1-sand_side"),
    ("m", 0, "egyptsandrockpointandpoint"), ("w", 6, "egyptwater1"), ("k", 4, "castle_walls_tall"), ("K", 0, "castle"),
    ("g", 0, "egyptgrassfull"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if x in (0, W - 1) or y in (0, H - 1):
        return "C"
    if canyon(cx, cy):
        return "w" if abs(cy - mujib_y(cx)) < 1.0 else "m"   # the river at the canyon floor
    if rim(cx, cy):
        return "C"
    # Kerak: a walled square with a gate on the highway side
    dx, dy = abs(cx - KERAK[0]), abs(cy - KERAK[1])
    if max(dx, dy) < 7:
        on_wall = max(dx, dy) > 5.6
        gate = cx > KERAK[0] and dy < 1.6
        return "k" if on_wall and not gate else "K"
    if near(CAMP, cx, cy, 3.5) or near(AMERICANS, cx, cy, 3.5) or near(DEST, cx, cy, 2.5):
        return "p"
    if roads.dist(cx, cy) < 1.1:
        return "r"
    # the closed road west, toward Jerusalem
    if seg_dist(cx, cy, 45.5, 28.5, 3, 24) < 0.9 and cx > 14:
        return "r"
    if noise(cx + 300, cy, 5) > 0.8:
        return "c"
    if cx < 30 and noise(cx, cy + 50, 7) > 0.62:
        return "g"   # the highlands' scrub on the west, toward the Dead Sea
    return "S" if noise(cx, cy, 10) > 0.6 else "s"

def height(vx, vy):
    h = 14 + noise(vx, vy, 14) * 14 * min(1.0, roads.dist(vx, vy) / 6)
    d = abs(vy - mujib_y(vx))
    if d < 5 and roads.dist(vx, vy) > 2:
        h = 3 + d * 2.5   # down into the canyon
    if max(abs(vx - KERAK[0]), abs(vy - KERAK[1])) < 8:
        h = 24             # the castle's spur
    if near(CAMP, vx, vy, 7) or near(AMERICANS, vx, vy, 7) or near(DEST, vx, vy, 5):
        h = 14
    return h

SOUTH = math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 50.5, 8.5, 0, 1000)
m.obj("bank", 1, 36.5, 7.5, 0, 4000)
m.obj("moneyplane", 1, 36.5, 7.5, 0, 0)
m.obj("stank", 1, 46.5, 22.5, SOUTH, 1000)
for k in range(6):
    m.obj("car", 1, 44.5 + (k % 2) * 0.8, 15.0 - k * 1.5, SOUTH, 0)
# the post at the road's end, ours
m.obj("turret", 1, DEST[0] - 3, DEST[1] - 2, 0, 200)
m.obj("bigwhitetent", 4, DEST[0] + 3, DEST[1] + 1, 0, 100)
# Kerak's garrison
m.obj("mainbasepad", 2, KERAK[0] - 2, KERAK[1] - 2, 0, 4000)
m.obj("garage", 2, KERAK[0] - 3, KERAK[1] + 3, 0, 1000)
m.obj("bank", 2, KERAK[0] + 2, KERAK[1] - 3, 0, 4000)
m.obj("moneyplane", 2, KERAK[0] + 2, KERAK[1] - 3, 0, 0)
for kind, x, y, hp in [("tower_missile", KERAK[0] + 5, KERAK[1] - 5, 600), ("tower_electric", KERAK[0] + 5, KERAK[1] + 5, 600), ("turret", 44.5, 58.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# the pursuit
m.obj("mainbasepad", 5, AMERICANS[0] + 3, AMERICANS[1] - 3, 0, 4000)
m.obj("garage", 5, 76.5, 10.5, 0, 1000)
m.obj("bank", 5, 86.5, 14.5, 0, 4000)
m.obj("moneyplane", 5, 86.5, 14.5, 0, 0)
for k, kind in enumerate(["peon_tank", "trike", "rocket_tank"]):
    m.obj(kind, 5, 74.5 + k * 1.4, 75.5 + k, SOUTH, 0)   # already raiding the desert road
m.roads(roads)
# the rockfall on the road west: boulders across it
for k in range(7):
    m.obj(rng.choice(["ruinsruin", "ruinswall2"]), 4, 13.5 + rng.uniform(-1, 1), 25.0 + k * 0.4 - 1.2, rng.uniform(0, 3), 100)
for _ in range(2500):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if t in "sSgc" and roads.dist(x, y) > 2.5 and not rim(x, y) and not near(CAMP, x, y, 9) and rng.random() < 0.012:
        m.obj(rng.choice(["palmtree1", "ruinsruin", "cabin_small"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", ["reach 1 car 46.5 140.5 5 4"], ["no mainbasepad 1", "lost 1 car 3"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer"], money=[10000, 7000, 7000])
