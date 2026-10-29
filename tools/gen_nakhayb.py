"""Nakhayb, campaign mission 4: the western desert, wadis cut between
mesas, and Dr. Calloway's abandoned camp in the basin where two wadis
meet. A takeover pad stands in the camp: whoever's engineer reaches it
first gets the pad's turret, and the camp. Writes a .cmap to argv[1]."""
import math, random, sys

random.seed(4204)
W, H = 100, 100

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

OUR_BASE = (46, 88)
THEIR_BASE = (50, 9)
CAMP = (50.0, 38.0)

nodes = []
def node(x, y):
    nodes.append([x, y, []])
    return len(nodes) - 1
def chain(*ids):
    for a, b in zip(ids, ids[1:]):
        nodes[a][2].append(b)

start = node(53.5, 84.5)
junction = node(50.5, 75.5)
west = [node(x, y) for x, y in [(30.5, 66.5), (21.5, 50.5), (31.5, 40.5)]]
east = [node(x, y) for x, y in [(70.5, 66.5), (79.5, 50.5), (69.5, 40.5)]]
camp = node(*CAMP)
north = [node(x, y) for x, y in [(50.5, 26.5), (55.5, 15.5)]]
chain(start, junction, *west, camp, *north)
chain(junction, *east, camp)

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(nodes[a][0], nodes[a][1], nodes[b][0], nodes[b][1]) for a in range(len(nodes)) for b in nodes[a][2]]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)
def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r

def openness(x, y):
    """How far inside the open ground (wadis and basins) a point is: > 0 is
    open, 0 is the cliff foot."""
    wadi = 5.5 + 2.5 * noise(x, y, 7) - road_dist(x, y)
    basins = max(13 - math.hypot(x - OUR_BASE[0], y - OUR_BASE[1]),
                 12 - math.hypot(x - THEIR_BASE[0], y - THEIR_BASE[1]),
                 10 + 2 * noise(x + 50, y, 5) - math.hypot(x - CAMP[0], y - CAMP[1]))
    return max(wadi, basins)

CLIFF = 1.4  # width of the cliff band, in cells

LEGEND = [
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"),
    ("p", 0, "cromeplatform3"),
    ("C", 4, "cliff1-sand_side"),
    ("m", 0, "egyptsandrockpointandpoint"),
    ("M", 0, "egyptrockyfull"),
    ("g", 0, "egyptgrassfull"),
]

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        o = openness(cx, cy)
        on_edge = x in (0, W - 1) or y in (0, H - 1)
        if on_edge:
            ch = "C"  # the map's rim: cliff all round
        elif o < -CLIFF:
            ch = "M" if noise(cx + 200, cy, 6) > 0.55 else "m"
        elif o < 0:
            ch = "C"
        elif near(OUR_BASE, cx, cy, 3.5) or near(THEIR_BASE, cx, cy, 3.5):
            ch = "p"
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif near(CAMP, cx, cy, 7) and noise(cx, cy, 2.5) > 0.5:
            ch = "g"  # the camp's spring keeps some grass alive
        elif noise(cx + 300, cy, 4) > 0.8:
            ch = "c"
        else:
            ch = "S" if noise(cx, cy, 9) > 0.6 else "s"
        row.append(ch)
    tiles.append("".join(row))

# no mesa cell may touch open ground, even corner to corner: the cliff
# between them is what stops a tank
grid_t = [list(r) for r in tiles]
for y in range(H):
    for x in range(W):
        if tiles[y][x] in "mM" and any(tiles[b][a] not in "CmM" for a in (x - 1, x, x + 1) for b in (y - 1, y, y + 1) if 0 <= a < W and 0 <= b < H):
            grid_t[y][x] = "C"
tiles = ["".join(r) for r in grid_t]

heights = []
for vy in range(H + 1):
    row = []
    for vx in range(W + 1):
        o = openness(vx, vy)
        floor = 10 + 4 * noise(vx, vy, 9)
        top = 70 + 25 * noise(vx + 80, vy, 11)
        if o >= 0:
            h = floor
        elif o <= -CLIFF:
            h = top
        else:
            h = floor + (top - floor) * (-o / CLIFF)
        row.append("%02x" % int(round(max(0, min(255, h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

NORTH = -math.pi / 2
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 53.5, 88.5, 0, 1000)
obj("bank", 1, 39.5, 85.5, 0, 4000)
obj("moneyplane", 1, 39.5, 85.5, 0, 0)
obj("stank", 1, 50.5, 80.5, NORTH, 1000)
obj("mainbasepad", 2, THEIR_BASE[0] - 6, THEIR_BASE[1], 0, 4000)
obj("garage", 2, THEIR_BASE[0] + 6, THEIR_BASE[1] + 2, 0, 1000)
obj("bank", 2, THEIR_BASE[0] - 1, THEIR_BASE[1] - 4, 0, 4000)
obj("moneyplane", 2, THEIR_BASE[0] - 1, THEIR_BASE[1] - 4, 0, 0)
# the camp: neutral until an engineer reaches it
obj("takeover_pad", 0, CAMP[0], CAMP[1], 0, 100)
# their guns: halfway up each wadi, over the camp, and at home
for kind, x, y, hp in [
    ("turret", 24.5, 54.5, 200),
    ("turret", 76.5, 54.5, 200),
    ("tower_missile", 52.5, 33.5, 600),
    ("tower_electric", 45.5, 18.5, 600),
    ("turret", 58.5, 7.5, 200),
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

# Calloway's camp: tents left standing, crates, her trench, palms at the spring
for dx, dy, th in [(-4.5, -2.5, 0.3), (4.5, -3.0, -0.2), (-3.5, 3.5, 1.4), (5.0, 3.0, 1.7)]:
    obj("bigwhitetent", 4, CAMP[0] + dx, CAMP[1] + dy, th, 100)
for dx, dy in [(-2.0, -4.0), (2.5, 4.5), (-5.5, 0.5), (6.0, 0.0)]:
    obj(random.choice(["armycratebig", "onelargecrate"]), 4, CAMP[0] + dx, CAMP[1] + dy, random.uniform(0, 3), 100)
for dx, dy in [(-7.5, -5.5), (7.0, -6.5), (-8.0, 6.0), (8.5, 5.0), (0.5, 8.0)]:
    obj("palmtree1", 4, CAMP[0] + dx, CAMP[1] + dy, random.uniform(0, 6.28), 100)
# scattered scrub palms and old walls along the wadis
for _ in range(2500):
    x, y = random.uniform(2, W - 2), random.uniform(2, H - 2)
    if tiles[int(y)][int(x)] in "sSc" and road_dist(x, y) > 2.5 and openness(x, y) > 1.5 \
            and not near(OUR_BASE, x, y, 10) and not near(THEIR_BASE, x, y, 10):
        r = random.random()
        if r < 0.012:
            obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
        elif r < 0.018:
            obj("ruinsruin", 4, x, y, 0, 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "nakhayb"
width = {W}
height = {H}
music = "Nakhayb_Iraq.mp3"
win = ["own takeover_pad 1"]
lose = ["no mainbasepad 1"]
build = ["peon_tank", "trike", "rocket_tank", "engineer"]
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
