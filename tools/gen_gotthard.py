"""The Gotthard Pass, campaign mission 22: six supply trucks north over
the Alps. Up from Airolo by the Tremola's switchbacks (or the old mule
path straight up), over the pass between its lakes and past the hospice,
down to Andermatt, where the Europeans (player 2) hold the basin, and out
through the Schollenen gorge over the Devil's Bridge. Snow cornices hang
over the road; a shot landing near one, from either side, sends an
avalanche across it. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1830)  # the year the first carriage road over the pass opened
noise = Noise(rng)
W, H = 120, 100

CAMP = (60, 94)
EUROPEANS = (36, 22)
EXIT = (60.5, 2.5)
LAKES = [((52, 54), 3.2), ((68, 47), 2.6)]
HOSPICE = (61, 51)
REUSS = [(66, 34), (63, 20), (62, 12), (57, 0)]

def half_width(y):
    """How far the open valley floor reaches either side of x = 60."""
    if y >= 64:
        return 18          # the slope the Tremola zigzags up
    if y >= 42:
        return 20          # the pass
    if y >= 33:
        return 9           # the way down
    if y >= 12:
        return 30 * math.sqrt(max(0.0, 1 - ((y - 22) / 11.5) ** 2)) + 4   # the Andermatt basin
    return 4               # the gorge

def edge(x, y):
    """Distance inside the valley floor (negative: into the mountain)."""
    return half_width(y) + 2.5 * (noise(x, y, 5) - 0.5) - abs(x - 60)

def reuss(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(REUSS, REUSS[1:]))

roads = Roads()
for n, x, y in [("camp", 60.5, 90.5), ("t1", 50.5, 86.5), ("t2", 69.5, 80.5), ("t3", 51.5, 74.5), ("t4", 67.5, 68.5), ("top", 60.5, 62.5),
                ("mule", 60.5, 76.5), ("pass", 60.5, 56.5), ("hosp", 58.5, 47.5), ("down", 59.5, 38.5), ("ander", 60.5, 26.5),
                ("gorge", 59.5, 11.5), ("bridge", 59.5, 5.5), ("exit", *EXIT), ("eu", 40.5, 24.5)]:
    roads.node(n, x, y)
# the Tremola, and the old mule path straight up beside it
roads.ours("camp", "t1", "t2", "t3", "t4", "top")
roads.ours("camp", "mule", "top")
roads.ours("top", "pass", "hosp", "down", "ander", "gorge", "bridge", "exit")
# the Europeans come out of Andermatt and down the way we are going up
roads.link("E", "eu", "ander")

m = Map("gotthard", W, H)
LEGEND = [
    ("s", 0, "switzsnowfull"), ("S", 0, "snow1"), ("o", 0, "snow_rocks1"), ("m", 0, "switzsnowgrasscorner2"), ("M", 0, "switzsnowgrasscorner3"),
    ("c", 4, "snowcliff5"), ("C", 4, "snowcliff1"), ("t", 0, "switzsnowroadstraight"), ("W", 6, "watrsnow1deepbutt"), ("B", 0, "rome_bridge1a"),
    ("p", 0, "cromeplatform3"), ("w", 0, "snowywoods1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    e = edge(cx, cy)
    if e < 0:
        return "c" if e > -2.5 else "C"   # rock at the foot, snow-covered crags above
    if any(near(c, cx, cy, r) for c, r in LAKES):
        return "W"
    on_road = roads.dist(cx, cy) < 1.1
    if reuss(cx, cy) < 0.7 and cy < 34:
        return "B" if on_road else "W"   # the Reuss, and the Devil's Bridge over it
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if on_road:
        return "t"
    if cy > 84 or (12 <= cy < 33):
        return "m" if noise(cx, cy, 6) > 0.45 else ("w" if noise(cx + 70, cy, 4) > 0.65 else "M")   # Airolo's and Andermatt's meadows
    return "o" if noise(cx, cy, 4) > 0.7 else ("S" if noise(cx + 30, cy, 7) > 0.5 else "s")

def floor_height(y):
    if y > 50:
        return 20 + (99 - y) * 2.1          # up from Airolo to the pass
    if y > 33:
        return 123 - (50 - y) * 2.5         # down from the pass
    return 80 - (33 - y) * 1.0              # Andermatt and the gorge

def height(vx, vy):
    e = edge(vx, vy)
    h = floor_height(vy) + noise(vx, vy, 9) * 6 * min(1.0, roads.dist(vx, vy) / 4)
    if e < 0:
        h += min(80.0, -e * 12)
    if any(near(c, vx, vy, r) for c, r in LAKES):
        h -= 6
    if near(CAMP, vx, vy, 6) or near(EUROPEANS, vx, vy, 6):
        h = floor_height(vy)
    return h

NORTH, EAST, WEST = -math.pi / 2, 0.0, math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 67.5, 93.5, 0, 1000)
m.obj("bank", 1, 52.5, 95.5, 0, 4000)
m.obj("moneyplane", 1, 52.5, 95.5, 0, 0)
m.obj("stank", 1, 57.5, 89.5, NORTH, 1000)
for k in range(6):
    m.obj("car", 1, 62.0 + (k % 3) * 1.6, 89.0 + (k // 3) * 1.4, NORTH, 0)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 44.5, 18.5, 0, 1000)
m.obj("bank", 2, 30.5, 26.5, 0, 4000)
m.obj("moneyplane", 2, 30.5, 26.5, 0, 0)
for kind, x, y, hp in [("turret", 64.5, 60.5, 200), ("tower_missile", 55.5, 44.5, 600), ("turret", 62.5, 36.5, 200), ("turret", 56.5, 14.5, 200),
                       ("tower_electric", 63.5, 9.5, 600), ("turret", 66.5, 28.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "rocket_tank", "peon_tank"]):
    m.obj(kind, 2, 50.5 + k * 1.5, 28.5, 0.0, 0)
# the cornices, each sliding across the road below it
for x, y, th in [(77.5, 78.5, WEST), (42.5, 72.5, EAST), (79.5, 52.5, WEST), (50.5, 38.5, EAST), (68.5, 40.5, WEST), (55.5, 7.5, EAST)]:
    m.obj("cornice", 0, x, y, th, 100)
m.roads(roads)
# the hospice, the inns of Airolo and Andermatt, larches on the meadows
for dx, dy in [(0, 0), (2.5, 0.5), (-1.0, 2.5)]:
    m.obj("cabin_small", 4, HOSPICE[0] + dx, HOSPICE[1] + dy, 0, 100)
for _ in range(1800):
    x, y = rng.uniform(3, W - 3), rng.uniform(3, H - 3)
    t = tile(int(x), int(y))
    if t in "mMw" and roads.dist(x, y) > 2.4 and not near(CAMP, x, y, 7) and not near(EUROPEANS, x, y, 7) and edge(x, y) > 2 and rng.random() < 0.1:
        m.obj("cabin_small" if rng.random() < 0.08 else "pinetree", 4, x, y, rng.uniform(0, 6.28), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Zurich_Switzerland.mp3", [f"reach 1 car {EXIT[0]} {EXIT[1] + 1} 5 4"], ["no mainbasepad 1", "lost 1 car 3"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank"], money=[10000, 10000, 0])
