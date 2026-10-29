"""Ur, campaign mission 3: the desert round the ziggurat of Ur, where
Dr. Calloway dug first. Our camp in the south-west, theirs in the
north-east, neutral banks along both roads, and the dig at the
ziggurat's south foot to hold. Writes a .cmap to argv[1]."""
import math, random, sys

random.seed(1922)  # Woolley's first season at Ur
W, H = 110, 100

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

OUR_BASE = (16, 88)
THEIR_BASE = (92, 10)
ZIG = (56, 38)            # the ziggurat's centre
DIG = (56.0, 52.0)        # Calloway's dig, at its south foot
DIG_R = 4.0
# terraces: (half width, half height, height byte, tile char)
TERRACES = [(11, 8, 50, "1"), (7, 5, 90, "2"), (4, 3, 130, "3")]
STAIR_HALF = 1.5          # the stair up the south face

def terrace(x, y):
    """Index of the highest terrace (x, y) stands on, or -1."""
    k = -1
    for i, (hw, hh, _, _) in enumerate(TERRACES):
        if abs(x - ZIG[0]) < hw and abs(y - ZIG[1]) < hh:
            k = i
    return k

def on_stair(x, y):
    return abs(x - ZIG[0]) < STAIR_HALF and ZIG[1] < y < ZIG[1] + TERRACES[0][1] + 3

# path nodes, our garage to theirs
nodes = []
def node(x, y):
    nodes.append([x, y, []])
    return len(nodes) - 1
def chain(*ids):
    for a, b in zip(ids, ids[1:]):
        nodes[a][2].append(b)

start = node(27.5, 84.5)
junction = node(35.5, 78.5)
# the west road: past the west bank, round the ziggurat's west side, past
# the north bank
w = [node(x, y) for x, y in [(30.5, 66.5), (30.5, 52.5), (32.5, 38.5), (38.5, 24.5), (52.5, 19.5)]]
# the dig road: through the dig, east of the ziggurat past the east bank
d = [node(x, y) for x, y in [(46.5, 66.5), (DIG[0], DIG[1]), (68.5, 52.5), (80.5, 46.5), (80.5, 30.5)]]
merge = node(74.5, 18.5)
end = node(86.5, 14.5)
chain(start, junction, *w, merge, end)
chain(junction, *d, merge)

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(nodes[a][0], nodes[a][1], nodes[b][0], nodes[b][1]) for a in range(len(nodes)) for b in nodes[a][2]]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)
def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r

BANKS = [(33.5, 50.5), (83.5, 44.5), (55.5, 16.5)]

LEGEND = [
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"),
    ("p", 0, "cromeplatform3"),
    ("1", 0, "scrapstuccowbrick2"),
    ("2", 0, "scrapstuccowbrickplain"),
    ("3", 0, "scrapstuccowbrickplain2"),
    ("t", 0, "cromstepswide"),
    ("d", 0, "egyptsandwalltexture2"),
]

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        k = terrace(cx, cy)
        if on_stair(cx, cy) and k >= 0:
            ch = "t"
        elif k >= 0:
            ch = TERRACES[k][3]
        elif near(DIG, cx, cy, DIG_R + 1.5):
            # the dig: trenches in a grid, spoil between
            ch = "d" if (x % 3 == 0 or y % 3 == 0) else "c"
        elif near(OUR_BASE, cx, cy, 8) or near(THEIR_BASE, cx, cy, 8):
            ch = "p" if (near(OUR_BASE, cx, cy, 3.5) or near(THEIR_BASE, cx, cy, 3.5)) else "S"
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif noise(cx + 300, cy, 5) > 0.8:
            ch = "c"
        else:
            ch = "S" if noise(cx, cy, 10) > 0.62 else "s"
        row.append(ch)
    tiles.append("".join(row))

heights = []
for vy in range(H + 1):
    row = []
    for vx in range(W + 1):
        h = 10 + noise(vx, vy, 13) * 14 * min(1.0, (road_dist(vx, vy) / 6))
        if near(OUR_BASE, vx, vy, 9) or near(THEIR_BASE, vx, vy, 9):
            h = 10
        if near(DIG, vx, vy, DIG_R + 1):
            h = 7  # dug down
        # the terraces: steep enough (two cells a cell) to stop a tank
        k = -1
        for i, (hw, hh, _, _) in enumerate(TERRACES):
            if abs(vx - ZIG[0]) <= hw and abs(vy - ZIG[1]) <= hh:
                k = i
        if k >= 0:
            h = TERRACES[k][2]
        if abs(vx - ZIG[0]) <= STAIR_HALF and ZIG[1] - TERRACES[2][1] < vy <= ZIG[1] + TERRACES[0][1] + 3:
            # the stair: from the plain at its foot to the top, evenly
            foot, top = ZIG[1] + TERRACES[0][1] + 3, ZIG[1] + TERRACES[2][1]
            f = max(0.0, min(1.0, (foot - vy) / (foot - top)))
            h = max(h if k < 0 else 0, 10 + f * (TERRACES[2][2] - 10)) if vy > top else TERRACES[2][2]
        row.append("%02x" % int(round(max(0, min(255, h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

NORTH = -math.pi / 2
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 24.5, 87.5, 0, 1000)
obj("bank", 1, 10.5, 83.5, 0, 4000)
obj("moneyplane", 1, 10.5, 83.5, 0, 0)
obj("stank", 1, 30.5, 81.5, NORTH, 1000)
obj("mainbasepad", 2, THEIR_BASE[0], THEIR_BASE[1], 0, 4000)
obj("garage", 2, 85.5, 10.5, 0, 1000)
obj("bank", 2, 98.5, 16.5, 0, 4000)
obj("moneyplane", 2, 98.5, 16.5, 0, 0)
# neutral banks along the roads, for engineers to take
for x, y in BANKS:
    obj("bank", 0, x, y, 0, 4000)
# their guns: on the ziggurat's lowest terrace over the dig, and at home
for kind, x, y, hp in [
    ("turret", ZIG[0] - 6.5, ZIG[1] + 7.0, 200),
    ("turret", ZIG[0] + 6.5, ZIG[1] + 7.0, 200),
    ("tower_missile", 78.5, 20.5, 600),
    ("turret", 92.5, 16.5, 200),
]:
    obj(kind, 2, x, y, 0, hp)

first = len(objects)
preds = {i: [] for i in range(len(nodes))}
for a in range(len(nodes)):
    for b in nodes[a][2]:
        preds[b].append(a)
for i, (x, y, nxt) in enumerate(nodes):
    a = f" A:{','.join(str(first + j) for j in nxt)}" if nxt else ""
    e = f" E:{','.join(str(first + j) for j in preds[i])}" if preds[i] else ""
    obj("path_object", 1, x, y, 0, 100, a + e)

# Calloway's camp at the dig: tents, crates, the broken walls she uncovered
for dx, dy in [(-5.5, 5.5), (5.5, 5.5)]:
    obj("bigwhitetent", 4, DIG[0] + dx, DIG[1] + dy, 0, 100)
for dx, dy in [(-3.5, 6.5), (4.0, 7.0), (-3.0, -0.5)]:
    obj(random.choice(["armycratebig", "onelargecrate"]), 4, DIG[0] + dx, DIG[1] + dy, random.uniform(0, 3), 100)
for dx, dy in [(-2.0, 1.5), (2.5, 2.5), (-1.0, 3.5)]:
    obj("ruinsruin", 4, DIG[0] + dx, DIG[1] + dy, 0, 100)
# the old city: ruined walls scattered over the plain
for _ in range(1500):
    x, y = random.uniform(3, W - 3), random.uniform(3, H - 3)
    if tiles[int(y)][int(x)] in "sSc" and road_dist(x, y) > 3 and not near(OUR_BASE, x, y, 11) and not near(THEIR_BASE, x, y, 11) \
            and near(ZIG, x, y, 34) and random.random() < 0.05:
        obj("ruinsruin", 4, x, y, 0, 100)
# palms where the old canal ran, and at the camps
for _ in range(400):
    x = random.uniform(2, W - 2)
    y = 72 + 6 * math.sin(x / 11) + random.uniform(-2, 2)
    if tiles[int(y)][int(x)] in "sS" and road_dist(x, y) > 2.5 and random.random() < 0.15:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
for p in (OUR_BASE, THEIR_BASE):
    for _ in range(10):
        a, dd = random.uniform(0, 6.28), random.uniform(8.5, 11)
        x, y = p[0] + math.cos(a) * dd, p[1] + math.sin(a) * dd
        if 1 < x < W - 1 and 1 < y < H - 1 and road_dist(x, y) > 2.5:
            obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "ur"
width = {W}
height = {H}
music = "Nakhayb.mp3"
win = ["hold 1 {DIG[0]} {DIG[1]} {DIG_R} 90"]
lose = ["no mainbasepad 1"]
build = ["peon_tank", "engineer", "rocket_tank"]
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
