"""Al Basrah, campaign mission 1: a beachhead on the west bank of the Shatt
al-Arab. Writes a .cmap to argv[1]."""
import math, random, sys

random.seed(1048)
W, H = 100, 100

def river_x(y):
    # the Shatt runs down the east side, bending toward the south-east
    return 80 + (y - 50) * 0.10 + 3.0 * math.sin(y / 13.0)

RIVER_HALF = 7.5

# value noise for dunes
G = {}
def grid(i, j):
    if (i, j) not in G:
        G[(i, j)] = random.random()
    return G[(i, j)]
def noise(x, y, s):
    fx, fy = x / s, y / s
    i, j = int(math.floor(fx)), int(math.floor(fy))
    u, v = fx - i, fy - j
    u, v = u * u * (3 - 2 * u), v * v * (3 - 2 * v)
    a, b, c, d = grid(i, j), grid(i + 1, j), grid(i, j + 1), grid(i + 1, j + 1)
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v

# the road: the old port in the north down the bank to the docks
ROAD = [(50, 6), (52, 14), (56, 22), (60, 32), (62, 42), (63, 52), (64, 62), (64, 72), (64, 80)]
def road_dist(x, y):
    best = 1e9
    for (ax, ay), (bx, by) in zip(ROAD, ROAD[1:]):
        dx, dy = bx - ax, by - ay
        t = max(0, min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
        best = min(best, math.hypot(x - ax - t * dx, y - ay - t * dy))
    return best
# a desert track west from the middle of the road
TRACK = [(62, 46), (48, 48), (34, 44), (20, 46), (6, 44)]
def track_dist(x, y):
    best = 1e9
    for (ax, ay), (bx, by) in zip(TRACK, TRACK[1:]):
        dx, dy = bx - ax, by - ay
        t = max(0, min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
        best = min(best, math.hypot(x - ax - t * dx, y - ay - t * dy))
    return best

TOWN = (34, 4, 66, 30)     # x0 y0 x1 y1, the old town by the port
DOCKS = (57, 78, 72, 96)

def in_box(x, y, b):
    return b[0] <= x < b[2] and b[1] <= y < b[3]

LEGEND = [
    ("W", 6, "egyptwater1"),
    ("w", 6, "egyptwater4"),
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("g", 0, "egyptgrassfull"),
    ("m", 0, "marsh"),
    ("r", 0, "egyptlargetrackfull"),
    ("R", 0, "egyptroadfull"),
    ("t", 0, "egyptsandwalltexture2"),
    ("k", 0, "airportroadplain"),
    ("p", 0, "cromeplatform3"),
]

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        d = cx - river_x(cy)
        if abs(d) < RIVER_HALF - 1.5:
            ch = "W"
        elif abs(d) < RIVER_HALF:
            ch = "w"
        elif d > 0:
            ch = "m"          # the far bank: reed marsh
        elif in_box(cx, cy, DOCKS):
            ch = "p" if d > -4 else "k"
        elif road_dist(cx, cy) < 1.3:
            ch = "r"
        elif in_box(cx, cy, TOWN):
            ch = "R" if (x % 8 == 0 or y % 7 == 0) else "t"
        elif track_dist(cx, cy) < 0.9:
            ch = "r"
        elif d > -RIVER_HALF - 9:
            ch = "g"          # date groves along the bank
        else:
            n = noise(cx, cy, 9)
            ch = "S" if n > 0.68 else "s"
            if noise(cx + 300, cy, 5) > 0.82:
                ch = "c"      # shell craters from the last war
        row.append(ch)
    tiles.append("".join(row))

heights = []
for vy in range(H + 1):
    row = []
    for vx in range(W + 1):
        d = vx - river_x(vy)
        if abs(d) < RIVER_HALF - 1:
            h = 0
        elif abs(d) < RIVER_HALF + 1:
            h = 4
        elif d > 0:
            h = 7
        else:
            # the bank rises gently to the dunes in the west
            west = max(0.0, (-d - 10) / 50.0)
            dune = noise(vx, vy, 11) * 34 + noise(vx + 99, vy + 7, 4) * 6
            h = 9 + west * dune
            if in_box(vx, vy, TOWN) or in_box(vx, vy, DOCKS):
                h = 10
            r = min(road_dist(vx, vy), track_dist(vx, vy))
            if r < 3:
                h = h + (10 - h) * (1 - r / 3)
        row.append("%02x" % max(0, min(255, int(round(h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, links=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{links}")
    return len(objects) - 1

NORTH = -math.pi / 2
# ours: the HQ on the docks and the supertank beside it, facing up the road
obj("mainbasepad", 1, 65.5, 88.5, 0, 4000)
obj("stank", 1, 61.5, 82.5, NORTH, 1000)

# theirs: towers and turrets holding the road north, lighter than the
# original's 1000 (a beachhead's defences, not a base's)
for kind, x, y, hp in [
    ("turret", 60.5, 63.5, 200), ("tower_missile", 67.5, 58.5, 600),
    ("turret", 58.5, 44.5, 200), ("tower_electric", 66.5, 40.5, 600),
    ("turret", 50.5, 20.5, 200), ("tower_missile", 58.5, 16.5, 600),
    ("tower_electric", 44.5, 12.5, 600),
]:
    obj(kind, 2, x, y, 0, hp)

# the route: road nodes from the port down to the docks; their units follow
# E links (toward our HQ), ours A links (north)
nodes = [(51, 8)] + [(x + 0.5, y + 0.5) for (x, y) in ROAD[1:]] + [(64.5, 85.5)]
first = len(objects)
for i, (x, y) in enumerate(nodes):
    a = f" A:{first + i - 1}" if i > 0 else ""
    e = f" E:{first + i + 1}" if i < len(nodes) - 1 else ""
    obj("path_object", 1, x, y, 0, 100, a + e)

# the counterattack, forming up in the old port
for kind, x, y in [("peon_tank", 50.5, 6.5), ("peon_tank", 52.5, 6.5), ("rocket_tank", 51.5, 4.5), ("peon_tank", 48.5, 8.5)]:
    obj(kind, 2, x, y, math.pi / 2, 0)

# scenery: date palms along the bank, the old town, the docks, ships
for _ in range(6000):
    x, y = random.uniform(1, W - 1), random.uniform(1, H - 1)
    ch = tiles[int(y)][int(x)]
    if ch == "g" and random.random() < 0.45 and road_dist(x, y) > 2.2:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
    elif ch == "m" and random.random() < 0.12:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
for y in range(5, 30, 4):
    for x in range(36, 66, 4):
        if road_dist(x + 1, y + 1) > 2.5 and random.random() < 0.7:
            obj(random.choice(["greecehouses1", "greecehouses2", "greecehouses3"]), 4, x + 1.5, y + 1.5, random.choice([0, 1.5708, 3.1416, 4.7124]), 100)
for x, y in [(41, 26), (63, 8)]:
    obj("ruinsruin", 4, x, y, 0, 100)
for x, y in [(68.5, 82.5), (68.5, 92.5)]:
    obj("silobigfull2x2", 4, x, y, 0, 100)
for x, y in [(60.5, 92.5), (62.5, 93.5), (59.5, 94.5), (66.5, 95.0)]:
    obj(random.choice(["armycratebig", "onelargecrate"]), 4, x, y, random.uniform(0, 3), 100)
for y in [84, 93]:
    obj("greeceship1", 4, river_x(y) - 3.5, y, math.pi / 2 + 0.1, 100)
obj("greeceship1", 4, river_x(30) + 1, 30, math.pi / 2 - 0.2, 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "al-basrah"
width = {W}
height = {H}
music = "Al_Basrah.mp3"
win = ["no_units 2"]
lose = ["no mainbasepad 1"]
legend = '''
{legend}
'''
tiles = '''
{chr(10).join(tiles)}
'''
heights = '''
{chr(10).join(heights)}
'''
objects = '''
{chr(10).join(objects)}
'''
"""
open(sys.argv[1], "w").write(out)
print(len(objects), "objects")
