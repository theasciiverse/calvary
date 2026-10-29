"""Babylon, campaign mission 5: the ruined city on the Euphrates, and east
of it the second site, a pit where the giants' bones lie in the open. The
enemy has ringed the pit with electric towers. Writes a .cmap to
argv[1]."""
import math, random, sys

random.seed(539)  # the year Babylon fell to Cyrus, BC
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

def river_x(y):
    return 15 + 4 * math.sin(y / 14.0) + 1.5 * math.sin(y / 5.3)
RIVER_HALF = 4.5

OUR_BASE = (92, 88)
THEIR_BASE = (60, 9)
PIT = (75.0, 46.0)
PIT_R = 6.5
CITY = (28, 26, 58, 82)   # x0 y0 x1 y1, the old city's mound
GATE = (40.5, 28.5)       # the Ishtar Gate, at the city's north end

nodes = []
def node(x, y):
    nodes.append([x, y, []])
    return len(nodes) - 1
def chain(*ids):
    for a, b in zip(ids, ids[1:]):
        nodes[a][2].append(b)

start = node(85.5, 84.5)
junction = node(78.5, 75.5)
# through the city: west to the Processional Way, north out of the gate
city = [node(x, y) for x, y in [(60.5, 72.5), (40.5, 66.5), (40.5, 46.5), GATE, (48.5, 20.5)]]
# straight through the bone pit and its ring of towers
pit = [node(x, y) for x, y in [(75.5, 58.5), PIT, (74.5, 32.5)]]
merge = node(60.5, 19.5)
end = node(66.5, 13.5)
chain(start, junction, *city, merge, end)
chain(junction, *pit, merge)

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(nodes[a][0], nodes[a][1], nodes[b][0], nodes[b][1]) for a in range(len(nodes)) for b in nodes[a][2]]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)
def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r
def in_city(x, y):
    return CITY[0] <= x < CITY[2] and CITY[1] <= y < CITY[3]

LEGEND = [
    ("W", 6, "egyptwater1"),
    ("w", 6, "egyptwater4"),
    ("g", 0, "egyptgrassfull"),
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"),
    ("R", 0, "egyptroadfull"),
    ("b", 0, "scrapstuccowbrick2"),
    ("B", 0, "scrapstuccowbrickplain"),
    ("p", 0, "cromeplatform3"),
]

tiles = []
for y in range(H):
    row = []
    for x in range(W):
        cx, cy = x + 0.5, y + 0.5
        d = cx - river_x(cy)
        if abs(d) < RIVER_HALF - 1.2:
            ch = "W"
        elif abs(d) < RIVER_HALF:
            ch = "w"
        elif abs(d) < RIVER_HALF + 5:
            ch = "g"  # green banks
        elif near(OUR_BASE, cx, cy, 3.5) or near(THEIR_BASE, cx, cy, 3.5):
            ch = "p"
        elif near(PIT, cx, cy, PIT_R + 1):
            ch = "c"
        elif in_city(cx, cy) and abs(cx - GATE[0]) < 1.6 and cy > GATE[1] - 1:
            ch = "R"  # the Processional Way
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif in_city(cx, cy):
            # a street grid of baked brick under the sand
            on_street = (x - CITY[0]) % 9 in (0, 1) or (y - CITY[1]) % 8 in (0, 1)
            ch = ("B" if noise(cx, cy, 3) > 0.5 else "b") if not on_street and noise(cx + 70, cy, 4) > 0.45 else "S"
        elif noise(cx + 300, cy, 5) > 0.8:
            ch = "c"
        else:
            ch = "S" if noise(cx, cy, 10) > 0.6 else "s"
        row.append(ch)
    tiles.append("".join(row))

heights = []
for vy in range(H + 1):
    row = []
    for vx in range(W + 1):
        d = abs(vx - river_x(vy))
        if d < RIVER_HALF - 1:
            h = 0
        elif d < RIVER_HALF + 1:
            h = 4
        else:
            h = 9 + noise(vx, vy, 12) * 10 * min(1.0, road_dist(vx, vy) / 5)
            if in_city(vx, vy):
                h += 5  # the tell: a city's worth of rubble
            if near(PIT, vx, vy, PIT_R + 1.5):
                # the pit: dug down, with a ramp in from north and south
                f = max(0.0, 1 - (math.hypot(vx - PIT[0], vy - PIT[1]) - PIT_R + 1.5) / 3)
                h -= 7 * min(1.0, f)
        row.append("%02x" % int(round(max(0, min(255, h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

NORTH = -math.pi / 2
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 84.5, 88.5, 0, 1000)
obj("bank", 1, 99.5, 83.5, 0, 4000)
obj("moneyplane", 1, 99.5, 83.5, 0, 0)
obj("stank", 1, 86.5, 80.5, NORTH, 1000)
obj("mainbasepad", 2, THEIR_BASE[0] - 6, THEIR_BASE[1], 0, 4000)
obj("garage", 2, THEIR_BASE[0] + 6, THEIR_BASE[1] + 2, 0, 1000)
obj("bank", 2, THEIR_BASE[0], THEIR_BASE[1] - 5, 0, 4000)
obj("moneyplane", 2, THEIR_BASE[0], THEIR_BASE[1] - 5, 0, 0)
# the ring of electric towers round the pit: the objective
for k in range(4):
    a = math.pi / 4 + k * math.pi / 2
    obj("tower_electric", 2, PIT[0] + math.cos(a) * 9.5, PIT[1] + math.sin(a) * 9.5, 0, 600)
# and their base's own guns
for kind, x, y, hp in [("tower_missile", 70.5, 19.5, 600), ("turret", 48.5, 15.5, 200), ("turret", 44.5, 31.5, 200)]:
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

# the bones: a skull at the pit's north end, the spine running south from
# it, ribs curving off both sides
obj("giant_skull", 4, PIT[0] + 0.5, PIT[1] - 4.0, NORTH, 100)
obj("giant_spine", 4, PIT[0] + 0.5, PIT[1] + 0.5, NORTH, 100)
for k in range(5):
    obj("giant_ribs", 4, PIT[0] + 0.5, PIT[1] - 1.8 + k * 0.85, NORTH, 100)
obj("giant_rib", 4, PIT[0] - 4.5, PIT[1] + 4.5, 2.4, 100)  # one scattered off the rest
for dx, dy in [(-5.5, 6.0), (6.0, 6.5)]:
    obj("bigwhitetent", 4, PIT[0] + dx, PIT[1] + dy, 0.2, 100)
# the city: walls along the old streets, the gate's two towers
for _ in range(2000):
    x, y = random.uniform(CITY[0] + 1, CITY[2] - 1), random.uniform(CITY[1] + 1, CITY[3] - 1)
    if road_dist(x, y) > 2.5 and abs(x - GATE[0]) > 2.5 and random.random() < 0.05:
        horizontal = random.random() < 0.5
        gx = CITY[0] + round((x - CITY[0]) / 9) * 9 + 0.5 if not horizontal else x
        gy = CITY[1] + round((y - CITY[1]) / 8) * 8 + 0.5 if horizontal else y
        if road_dist(gx, gy) > 2:
            obj(random.choice(["ruinswall1", "ruinswall2", "brickwall"]), 4, gx, gy, 0.0 if horizontal else math.pi / 2, 100)
for dx in (-2.2, 2.2):
    obj("ruinstwopillar", 4, GATE[0] + dx, GATE[1], 0, 100)
    obj("brickwall", 4, GATE[0] + dx * 2.2, GATE[1], 0.0, 100)
# palms along the river
for _ in range(600):
    y = random.uniform(2, H - 2)
    x = river_x(y) + random.choice([-1, 1]) * random.uniform(RIVER_HALF + 0.8, RIVER_HALF + 5)
    if 1 < x < W - 1 and random.random() < 0.25:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "babylon"
width = {W}
height = {H}
music = "Nakhayb_Iraq.mp3"
win = ["no tower_electric 2"]
lose = ["no mainbasepad 1"]
build = ["peon_tank", "electric_car", "rocket_tank", "engineer"]
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
