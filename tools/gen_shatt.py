"""Shatt al-Arab marshes, campaign mission 2: reed beds cut by three
channels between our landing in the south-west and the enemy's levee base in
the north-east. Two causeways cross every channel where the old bridges were
blown; each crossing is a bridge spot a bridger can span. Writes a .cmap to
argv[1]."""
import math, random, sys

random.seed(2048)
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

# the three channels, west to east across the whole map; each crossing is
# narrowed to one cell (the bridger's road spans one cell) for a few cells
CHANNELS = [(72, 0.0), (50, 2.1), (28, 4.4)]
def channel_y(k, x):
    base, ph = CHANNELS[k]
    return base + 3.0 * math.sin(x / 9.0 + ph) + 1.4 * math.sin(x / 4.3 + 2 * ph)
HALF = 1.3

# the causeways cross channel k at these x (cell columns)
WEST = [26, 21, 30]
EAST = [70, 79, 74]
def crossing_row(k, x):
    return int(math.floor(channel_y(k, x + 0.5)))

def water(x, y):
    """Channel water at cell (x, y)?"""
    for k in range(3):
        narrows = [lx for lx in (WEST[k], EAST[k]) if abs(x - lx) <= 1]
        if narrows:
            if y == crossing_row(k, narrows[0]):
                return True
        elif abs(y + 0.5 - channel_y(k, x + 0.5)) < HALF:
            return True
    return False

OUR_BASE = (18, 88)
THEIR_BASE = (90, 11)

# path nodes, our garage to theirs: (name, x, y, bridge)
nodes = []
def node(x, y, bridge=False):
    nodes.append([x, y, bridge, []])
    return len(nodes) - 1
def link(a, b):
    nodes[a][3].append(b)

start = node(25.5, 84.5)
junction = node(31.5, 81.5)
link(start, junction)
merge = node(62.5, 17.5)
for lane in (WEST, EAST):
    prev = junction
    for k, lx in enumerate(lane):
        r = crossing_row(k, lx)
        cx = lx + 0.5
        south, gap, north = node(cx, r + 3.5), node(cx, r + 0.5, True), node(cx, r - 2.5)
        link(prev, south)
        link(south, gap)
        link(gap, north)
        prev = north
    link(prev, merge)
end = node(84.5, 14.5)
link(merge, end)

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(nodes[a][0], nodes[a][1], nodes[b][0], nodes[b][1]) for a in range(len(nodes)) for b in nodes[a][3]]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)

def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r

LEGEND = [
    ("W", 6, "egyptwater1"),
    ("m", 0, "marsh"),
    ("c", 0, "marsh_craters1"),
    ("S", 0, "egyptsandgrassfull"),
    ("r", 0, "francegrassroadfull"),
    ("p", 0, "cromeplatform3"),
]

def lagoon(x, y):
    """Open water in the reed beds, away from the causeways and bases."""
    if road_dist(x, y) < 5 or near(OUR_BASE, x, y, 14) or near(THEIR_BASE, x, y, 14):
        return False
    return noise(x, y, 7) > 0.8

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        if water(x, y) or lagoon(cx, cy):
            ch = "W"
        elif near(OUR_BASE, cx, cy, 9) or near(THEIR_BASE, cx, cy, 10):
            ch = "p" if (near(OUR_BASE, cx, cy, 3.5) or near(THEIR_BASE, cx, cy, 3.5)) else "S"
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif noise(cx + 500, cy, 4) > 0.8:
            ch = "c"
        else:
            ch = "m"
        row.append(ch)
    tiles.append("".join(row))

heights = []
for vy in range(H + 1):
    row = []
    for vx in range(W + 1):
        # a vertex is low if any cell touching it is water
        wet = any(tiles[y][x] == "W" for y in (vy - 1, vy) for x in (vx - 1, vx) if 0 <= x < W and 0 <= y < H)
        if wet:
            h = 3
        else:
            h = 7 + noise(vx, vy, 6) * 3
            if road_dist(vx, vy) < 2:
                h = 9
            if near(OUR_BASE, vx, vy, 9) or near(THEIR_BASE, vx, vy, 10):
                h = 11
        row.append("%02x" % int(round(h)))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

NORTH = -math.pi / 2
# ours: the landing in the south-west
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 24.5, 87.5, 0, 1000)
# a bank and its money plane, so a spent war chest refills; the enemy has
# only what it brought
obj("bank", 1, 11.5, 83.5, 0, 4000)
obj("moneyplane", 1, 11.5, 83.5, 0, 0)
obj("stank", 1, 28.5, 82.5, NORTH, 1000)
# theirs: the levee base in the north-east
obj("mainbasepad", 2, THEIR_BASE[0], THEIR_BASE[1], 0, 4000)
obj("garage", 2, 85.5, 11.5, 0, 1000)
# towers on the far banks: lighter at the middle channel, heavier at the last
for kind, x, y, hp in [
    ("turret", WEST[1] + 3.5, crossing_row(1, WEST[1]) - 3.5, 200),
    ("turret", EAST[1] - 2.5, crossing_row(1, EAST[1]) - 3.5, 200),
    ("tower_missile", WEST[2] + 4.5, crossing_row(2, WEST[2]) - 4.5, 600),
    ("tower_electric", EAST[2] - 3.5, crossing_row(2, EAST[2]) - 4.5, 600),
    ("turret", 58.5, 21.5, 200),
    ("turret", 92.5, 16.5, 200),
    ("tower_missile", 95.5, 7.5, 600),
]:
    obj(kind, 2, x, y, 0, hp)
# their pickets, already waiting at the last channel
for x, y in [(WEST[2] + 1.5, crossing_row(2, WEST[2]) - 3.5), (EAST[2] - 0.5, crossing_row(2, EAST[2]) - 3.5)]:
    obj("peon_tank", 2, x, y, math.pi / 2, 0)

# the path network: our units follow A links north, theirs E links south
first = len(objects)
preds = {i: [] for i in range(len(nodes))}
for a in range(len(nodes)):
    for b in nodes[a][3]:
        preds[b].append(a)
for i, (x, y, bridge, nxt) in enumerate(nodes):
    a = f" A:{','.join(str(first + j) for j in nxt)}" if nxt else ""
    e = f" E:{','.join(str(first + j) for j in preds[i])}" if preds[i] else ""
    obj("path_object", 1, x, y, 0, 100, a + e + (" bridge" if bridge else ""))

# scenery: reed huts (the marsh Arabs' mudhif) on dry islands, palms by the
# bases, wrecked boats in the channels' banks
placed = 0
for _ in range(4000):
    x, y = random.uniform(2, W - 2), random.uniform(2, H - 2)
    ch = tiles[int(y)][int(x)]
    if ch not in "mc" or road_dist(x, y) < 3:
        continue
    wet = sum(tiles[int(y + dy)][int(x + dx)] == "W" for dx in (-2, 0, 2) for dy in (-2, 0, 2) if 0 <= int(x + dx) < W and 0 <= int(y + dy) < H)
    if wet >= 2 and random.random() < 0.05:
        obj("cabin_small", 4, x, y, random.choice([0, 1.5708]), 100)
    elif random.random() < 0.035:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
for p, r in [(OUR_BASE, 11), (THEIR_BASE, 12)]:
    for _ in range(14):
        a, d = random.uniform(0, 6.28), random.uniform(r - 3, r)
        x, y = p[0] + math.cos(a) * d, p[1] + math.sin(a) * d
        if 1 < x < W - 1 and 1 < y < H - 1 and tiles[int(y)][int(x)] != "W" and road_dist(x, y) > 2.5:
            obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
for x, y in [(13.5, 84.5), (21.5, 92.5), (95.5, 14.5), (87.5, 6.5)]:
    obj(random.choice(["armycratebig", "onelargecrate"]), 4, x, y, random.uniform(0, 3), 100)
obj("bigwhitetent", 4, 12.5, 91.5, 0, 100)
obj("bigwhitetent", 4, 96.5, 10.5, 0, 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "shatt-al-arab"
width = {W}
height = {H}
music = "Al_Basrah.mp3"
win = ["no mainbasepad 2"]
lose = ["no mainbasepad 1"]
build = ["peon_tank", "bridger"]
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
print(len(objects), "objects,", sum(n[2] for n in nodes), "bridge spots")
