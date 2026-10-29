"""The Euphrates line, campaign mission 6: six supply trucks from our camp
in the west to the forward post in the east, along the river road or the
desert road, with crossovers between them, while the enemy raids from
its base in the south-east. Writes a .cmap to argv[1]."""
import math, random, sys

random.seed(2003)
W, H = 110, 96

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

def river_y(x):
    return 21 + 3 * math.sin(x / 13.0) + 1.2 * math.sin(x / 4.7)
RIVER_HALF = 5.0

OUR_BASE = (9, 52)
THEIR_BASE = (95, 86)
POST = (100.5, 46.5)
POST_R = 5

# path nodes: A links run east, toward the post; the enemy's own branch
# from its garage joins the desert road, and its units follow E links west
nodes = []
def node(x, y):
    nodes.append([x, y, [], []])  # x, y, A links, extra E links
    return len(nodes) - 1
def a(*ids):
    for p, q in zip(ids, ids[1:]):
        nodes[p][2].append(q)

g0 = node(16.5, 54.5)
j1 = node(28.5, 50.5)
r1, x1, r2, r3 = node(40.5, 36.5), node(56.5, 33.5), node(72.5, 32.5), node(88.5, 36.5)
d1, y1, d2, d3 = node(40.5, 64.5), node(56.5, 66.5), node(72.5, 68.5), node(88.5, 62.5)
c = node(64.5, 50.5)
post = node(*POST)
a(g0, j1, r1, x1, r2, r3, post)
a(j1, d1, y1, d2, d3, post)
a(x1, c, r2)
a(y1, c, d2)
eg = node(87.5, 80.5)
e1 = node(87.5, 72.5)
nodes[eg][3].append(e1)
nodes[e1][3].append(d3)

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(nodes[p][0], nodes[p][1], nodes[q][0], nodes[q][1]) for p in range(len(nodes)) for q in nodes[p][2] + nodes[p][3]]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)
def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r

LEGEND = [
    ("W", 6, "egyptwater1"),
    ("w", 6, "egyptwater4"),
    ("m", 0, "marsh"),
    ("g", 0, "egyptgrassfull"),
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"),
    ("p", 0, "cromeplatform3"),
]

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        d = cy - river_y(cx)
        if abs(d) < RIVER_HALF - 1.2:
            ch = "W"
        elif abs(d) < RIVER_HALF:
            ch = "w"
        elif d < 0:
            ch = "m" if noise(cx, cy, 5) > 0.45 else "g"  # the far bank
        elif near(OUR_BASE, cx, cy, 3.5) or near(THEIR_BASE, cx, cy, 3.5) or near(POST, cx, cy, 2.5):
            ch = "p"
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif d < 9 + 4 * noise(cx, cy, 8):
            ch = "m" if noise(cx + 40, cy, 4) > 0.7 else "g"  # the green belt along the river
        elif d < 22 + 6 * noise(cx + 9, cy, 10):
            ch = "S"
        elif noise(cx + 300, cy, 5) > 0.8:
            ch = "c"
        else:
            ch = "s"
        row.append(ch)
    tiles.append("".join(row))

heights = []
for vx_rows in range(H + 1):
    vy = vx_rows
    row = []
    for vx in range(W + 1):
        d = abs(vy - river_y(vx))
        if d < RIVER_HALF - 1:
            h = 0
        elif d < RIVER_HALF + 1:
            h = 4
        else:
            h = 8 + noise(vx, vy, 11) * 16 * min(1.0, road_dist(vx, vy) / 5) * min(1.0, (vy - river_y(vx)) / 25)
            if near(OUR_BASE, vx, vy, 8) or near(THEIR_BASE, vx, vy, 8) or near(POST, vx, vy, 5):
                h = 9
        row.append("%02x" % int(round(max(0, min(255, h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

EAST = 0.0
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 14.5, 58.5, 0, 1000)
obj("bank", 1, 6.5, 44.5, 0, 4000)
obj("moneyplane", 1, 6.5, 44.5, 0, 0)
obj("stank", 1, 30.5, 54.5, EAST, 1000)
# the convoy, lined up on the road out of camp
for k in range(6):
    obj("car", 1, 18.5 + k * 1.6, 54.0 - k * 0.6, EAST, 0)
# the forward post: two of our turrets and its tents
obj("turret", 1, POST[0] - 2.5, POST[1] - 4.0, 0, 200)
obj("turret", 1, POST[0] + 2.5, POST[1] + 4.0, 0, 200)
for dx, dy in [(3.5, -2.5), (-3.5, 3.0)]:
    obj("bigwhitetent", 4, POST[0] + dx, POST[1] + dy, 0.0, 100)
obj("mainbasepad", 2, THEIR_BASE[0], THEIR_BASE[1], 0, 4000)
obj("garage", 2, 88.5, 84.5, 0, 1000)
obj("bank", 2, 102.5, 80.5, 0, 4000)
obj("moneyplane", 2, 102.5, 80.5, 0, 0)
# their guns: three over each road, one over the crossover
for kind, x, y, hp in [
    ("turret", 48.5, 29.5, 200),
    ("turret", 72.5, 26.5, 200),
    ("tower_missile", 86.5, 30.5, 600),
    ("turret", 48.5, 70.5, 200),
    ("tower_missile", 80.5, 65.5, 600),
    ("turret", 94.5, 56.5, 200),
    ("turret", 64.5, 56.5, 200),
    ("turret", 92.5, 78.5, 200),
]:
    obj(kind, 2, x, y, 0, hp)
# and two raiding parties already out on the roads, driving west
for x, y, kinds in [(93.5, 39.5, ["peon_tank", "peon_tank", "rocket_tank"]), (93.5, 62.5, ["peon_tank", "rocket_tank", "peon_tank"])]:
    for k, kind in enumerate(kinds):
        obj(kind, 2, x + k * 1.5, y + (k % 2), math.pi, 0)

first = len(objects)
preds = {i: [] for i in range(len(nodes))}
for p in range(len(nodes)):
    for q in nodes[p][2]:
        preds[q].append(p)
for i, (x, y, nxt, extra) in enumerate(nodes):
    al = f" A:{','.join(str(first + j) for j in nxt)}" if nxt else ""
    elinks = preds[i] + extra
    el = f" E:{','.join(str(first + j) for j in elinks)}" if elinks else ""
    obj("path_object", 1, x, y, 0, 100, al + el)

# palms along the green belt, reeds on the far bank, ruined farms
for _ in range(5000):
    x, y = random.uniform(2, W - 2), random.uniform(2, H - 2)
    ch = tiles[int(y)][int(x)]
    if road_dist(x, y) < 2.5 or near(OUR_BASE, x, y, 9) or near(THEIR_BASE, x, y, 9) or near(POST, x, y, 6):
        continue
    if ch == "g" and random.random() < 0.09:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
    elif ch == "S" and random.random() < 0.004:
        obj(random.choice(["ruinswall1", "ruinsruin", "cabin_small"]), 4, x, y, random.choice([0, 1.5708]), 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "euphrates"
width = {W}
height = {H}
music = "Al_Basrah.mp3"
win = ["reach 1 car {POST[0]} {POST[1]} {POST_R} 4"]
lose = ["no mainbasepad 1", "lost 1 car 3"]
build = ["peon_tank", "electric_car", "rocket_tank", "trike"]
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
