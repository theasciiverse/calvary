"""Nineveh, campaign mission 7: the ruined walls of Nineveh on the
Tigris, another of the giants' sites inside them, and Fisher's column
camped beside it after refusing the order to raze it. The Americans
(player 5, the second hostile army) come from the south, the Europeans
(player 2) from the north-east; each army's road runs through the site
and on to the other's base, so they fight each other there too. Writes a
.cmap to argv[1]."""
import math, random, sys

random.seed(612)  # the year BC Nineveh fell
W, H = 120, 100

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
    return 11 + 3 * math.sin(y / 15.0) + 1.2 * math.sin(y / 6.1)
RIVER_HALF = 5.0

SITE = (60.0, 48.0)
SITE_R = 5.0
OUR_BASE = (35, 52)
AMERICANS = (60, 91)
EUROPEANS = (102, 12)
WALLS = (30, 24, 92, 76)  # x0 y0 x1 y1: Nineveh's circuit, in ruins

# path nodes and each army's links
pos = {}
def node(name, x, y):
    pos[name] = (x, y)
pos_list = [
    ("ours", 41.5, 57.5), ("w1", 47.5, 50.5), ("s", SITE[0] + 0.5, SITE[1] + 0.5),
    ("n1", 76.5, 36.5), ("n2", 90.5, 22.5), ("eu", 96.5, 16.5),
    ("a1", 62.5, 72.5), ("am", 66.5, 88.5),
]
for name, x, y in pos_list:
    node(name, x, y)
A = [("ours", "w1"), ("w1", "s"), ("s", "n1"), ("n1", "n2"), ("n2", "eu"), ("s", "a1"), ("a1", "am")]
# the Europeans: to the site, then on to the Americans or west to our camp
E = [("eu", "n2"), ("n2", "n1"), ("n1", "s"), ("s", "a1"), ("a1", "am"), ("s", "w1"), ("w1", "ours")]
# the Americans: to the site, then on to the Europeans or west to our camp
R = [("am", "a1"), ("a1", "s"), ("s", "n1"), ("n1", "n2"), ("n2", "eu"), ("s", "w1"), ("w1", "ours")]
names = [n for n, _, _ in pos_list]

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
SEGS = [(*pos[p], *pos[q]) for p, q in A]
def road_dist(x, y):
    return min(seg_dist(x, y, *s) for s in SEGS)
def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r
def on_walls(x, y, w=1.2):
    x0, y0, x1, y1 = WALLS
    inside = x0 - w < x < x1 + w and y0 - w < y < y1 + w
    return inside and (abs(x - x0) < w or abs(x - x1) < w or abs(y - y0) < w or abs(y - y1) < w)

LEGEND = [
    ("W", 6, "egyptwater1"),
    ("w", 6, "egyptwater4"),
    ("g", 0, "egyptgrassfull"),
    ("s", 0, "sand_center_1"),
    ("S", 0, "egyptsandgrassfull"),
    ("c", 0, "sandcraters"),
    ("r", 0, "egyptlargetrackfull"),
    ("b", 0, "scrapstuccowbrick2"),
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
        elif abs(d) < RIVER_HALF + 6:
            ch = "g"
        elif near(OUR_BASE, cx, cy, 3) or near(AMERICANS, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
            ch = "p"
        elif near(SITE, cx, cy, SITE_R + 1):
            ch = "c"
        elif road_dist(cx, cy) < 1.1:
            ch = "r"
        elif on_walls(cx, cy) and noise(cx, cy, 3) > 0.3:
            ch = "b"  # the wall's footing, baked brick
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
            if on_walls(vx, vy, 2.0) and road_dist(vx, vy) > 3:
                h += 10  # the walls' ruined bank
            # Kuyunjik, the mound the city stood on, round the site
            m = math.hypot(vx - SITE[0], vy - SITE[1])
            h += 12 * max(0.0, 1 - m / 22) if m > SITE_R + 2 else 0
            if near(OUR_BASE, vx, vy, 6) or near(AMERICANS, vx, vy, 8) or near(EUROPEANS, vx, vy, 8):
                h = 10
        row.append("%02x" % int(round(max(0, min(255, h)))))
    heights.append("".join(row))

objects = []
def obj(kind, player, x, y, theta=0.0, health=0, extra=""):
    objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}{extra}")
    return len(objects) - 1

NORTH = -math.pi / 2
# Fisher's column, camped by the site
obj("mainbasepad", 1, OUR_BASE[0], OUR_BASE[1], 0, 4000)
obj("garage", 1, 41.5, 60.5, 0, 1000)
obj("bank", 1, 31.5, 45.5, 0, 4000)
obj("moneyplane", 1, 31.5, 45.5, 0, 0)
obj("stank", 1, 49.5, 52.5, 0.0, 1000)
# the Americans: the base we came from, and a company that was ours this
# morning, camped by the south gate
obj("mainbasepad", 5, AMERICANS[0] - 6, AMERICANS[1], 0, 4000)
obj("garage", 5, AMERICANS[0] + 6, AMERICANS[1] - 2, 0, 1000)
obj("bank", 5, AMERICANS[0], AMERICANS[1] + 5, 0, 4000)
obj("moneyplane", 5, AMERICANS[0], AMERICANS[1] + 5, 0, 0)
obj("turret", 5, AMERICANS[0] - 1.5, AMERICANS[1] - 7.5, 0, 200)
for k, kind in enumerate(["peon_tank", "peon_tank", "rocket_tank"]):
    obj(kind, 5, 58.5 + k * 1.6, 79.5 + (k % 2), NORTH, 0)
# the Europeans, in the north-east
obj("mainbasepad", 2, EUROPEANS[0] + 6, EUROPEANS[1] - 2, 0, 4000)
obj("garage", 2, EUROPEANS[0] - 4, EUROPEANS[1] + 3, 0, 1000)
obj("bank", 2, EUROPEANS[0] + 10, EUROPEANS[1] + 6, 0, 4000)
obj("moneyplane", 2, EUROPEANS[0] + 10, EUROPEANS[1] + 6, 0, 0)
obj("tower_missile", 2, 86.5, 26.5, 0, 600)
for k, kind in enumerate(["peon_tank", "electric_car", "peon_tank"]):
    obj(kind, 2, 74.5 + k * 1.5, 30.5 + (k % 2), math.pi / 2, 0)

first = len(objects)
idx = {n: first + k for k, n in enumerate(names)}
for n in names:
    links = ""
    for tag, edges in (("A", A), ("E", E), ("R", R)):
        nxt = [idx[q] for p, q in edges if p == n]
        if nxt:
            links += f" {tag}:{','.join(map(str, nxt))}"
    obj("path_object", 1, pos[n][0], pos[n][1], 0, 100, links)

# the site: another of them, face up under the mound
obj("giant_skull", 4, SITE[0] - 0.5, SITE[1] - 3.5, NORTH + 0.3, 100)
obj("giant_spine", 4, SITE[0] + 0.3, SITE[1] + 0.8, NORTH + 0.3, 100)
for k in range(4):
    obj("giant_ribs", 4, SITE[0] + 0.0 + k * 0.25, SITE[1] - 1.2 + k * 0.85, NORTH + 0.3, 100)
obj("giant_rib", 4, SITE[0] + 4.0, SITE[1] + 3.5, 0.6, 100)
# the walls: a broken circuit of wall segments, gaps where the roads run
x0, y0, x1, y1 = WALLS
for t in range(0, 2 * ((x1 - x0) + (y1 - y0)), 2):
    if t < x1 - x0:
        x, y, th = x0 + t, y0, 0.0
    elif t < (x1 - x0) + (y1 - y0):
        x, y, th = x1, y0 + t - (x1 - x0), math.pi / 2
    elif t < 2 * (x1 - x0) + (y1 - y0):
        x, y, th = x1 - (t - (x1 - x0) - (y1 - y0)), y1, 0.0
    else:
        x, y, th = x0, y1 - (t - 2 * (x1 - x0) - (y1 - y0)), math.pi / 2
    if road_dist(x, y) > 3 and random.random() < 0.6:
        obj(random.choice(["ruinswall1", "ruinswall2", "brickwall"]), 4, x + 0.5, y + 0.5, th, 100)
for gx, gy in [(x0 + 18.5, y0), (x1, y0 + 30.5), (x0 + 30.5, y1)]:
    obj("ruinstwopillar", 4, gx, gy, 0, 100)
# palms along the Tigris; ruins scattered inside the walls
for _ in range(700):
    y = random.uniform(2, H - 2)
    x = river_x(y) + random.choice([-1, 1]) * random.uniform(RIVER_HALF + 0.8, RIVER_HALF + 6)
    if 1 < x < W - 1 and random.random() < 0.25:
        obj("palmtree1", 4, x, y, random.uniform(0, 6.28), 100)
for _ in range(1500):
    x, y = random.uniform(x0 + 2, x1 - 2), random.uniform(y0 + 2, y1 - 2)
    if road_dist(x, y) > 3 and not near(SITE, x, y, SITE_R + 3) and not near(OUR_BASE, x, y, 9) and random.random() < 0.03:
        obj("ruinsruin", 4, x, y, 0, 100)
for dx, dy in [(-4.5, 4.5), (5.5, -1.5)]:
    obj("bigwhitetent", 4, OUR_BASE[0] + dx, OUR_BASE[1] + dy, 0.3, 100)

legend = "\n".join(f"{c} {f} {n}" for c, f, n in LEGEND)
out = f"""name = "nineveh"
width = {W}
height = {H}
music = "Nakhayb_Iraq.mp3"
win = ["hold 1 {SITE[0]} {SITE[1]} {SITE_R} 120"]
lose = ["no mainbasepad 1"]
build = ["peon_tank", "electric_car", "rocket_tank", "trike", "engineer"]
money = [10000, 5000, 5000]
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
