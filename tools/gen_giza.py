"""Giza, campaign mission 13: the plateau with its three pyramids (in the
terrain itself, too steep for any tank: the original's pyramid models
are 8 cells across and 8 high) and the Sphinx, both in the limestone of
the original's own Sphinx textures. The Europeans (player 2)
dig on the plateau; below the Sphinx, in the Valley Temple, the
Watchers' Order (player 6) comes out of the ground for the first time,
at war with everyone. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near

rng = random.Random(2560)  # Khufu's pyramid, about 2560 BC
noise = Noise(rng)
W, H = 120, 100

PLATEAU_EDGE = 70          # the plateau's east scarp runs at about this x
PYRAMIDS = [((42, 34), 5.0), ((50, 48), 4.6), ((55, 62), 3.2)]   # centre, half width
SPHINX = (66, 52)
TEMPLE = (78, 54)           # the Valley Temple, below the Sphinx: the Watchers
CAMP = (106, 50)
EUROPEANS = (16, 46)

def scarp(x, y):
    return PLATEAU_EDGE + 3 * math.sin(y / 9.0)

roads = Roads()
for n, x, y in [("ours", 100.5, 51.5), ("j", 92.5, 50.5), ("causeway", 72.5, 44.5), ("top", 60.5, 40.5), ("eu1", 32.5, 44.5), ("eu", 20.5, 46.5),
                ("valley", 84.5, 60.5), ("temple", 79.5, 56.5), ("south", 64.5, 74.5), ("w1", 40.5, 70.5)]:
    roads.node(n, x, y)
roads.ours("ours", "j", "causeway", "top", "eu1", "eu")
roads.ours("j", "valley", "temple")
# the Watchers: out of the temple, to our camp, and round the south of the
# pyramids to the Europeans
roads.link("W", "temple", "valley", "j", "ours")
roads.link("W", "temple", "south", "w1", "eu1", "eu")

def pyramid_h(x, y):
    best = 0.0
    for (px, py), hw in PYRAMIDS:
        d = max(abs(x - px), abs(y - py))
        if d < hw:
            best = max(best, (hw - d) * 2.0)   # two cells of rise a cell
    return best

def in_pyramid(x, y):
    return any(max(abs(x - px), abs(y - py)) < hw for (px, py), hw in PYRAMIDS)

def sphinx(x, y):
    dx, dy = x - SPHINX[0], y - SPHINX[1]
    return (-3.5 < dx < 2.5 and abs(dy) < 1.1) or (1.5 < dx < 3.2 and abs(dy) < 1.5)

m = Map("giza", W, H)
LEGEND = [
    ("s", 0, "sand_center_1"), ("S", 0, "egyptsandgrassfull"), ("c", 0, "sandcraters"), ("r", 0, "egyptlargetrackfull"),
    ("p", 0, "cromeplatform3"), ("y", 4, "2sphinxneckmaintop"), ("Y", 4, "2sphinxneckmain4"), ("x", 4, "sphinxfacehalf6"),
    ("g", 0, "egyptgrassfull"), ("G", 0, "marsh"), ("W", 6, "egyptwater1"), ("t", 0, "egyptsandwalltexture2"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if cx > W - 6:
        return "W"   # the Nile, along the east edge
    if in_pyramid(cx, cy):
        # each face a shade: lit from the east and south
        for (px, py), hw in PYRAMIDS:
            if max(abs(cx - px), abs(cy - py)) < hw:
                return "y" if (cx - px) > abs(cy - py) or (cy - py) > abs(cx - px) else "Y"
    if sphinx(cx, cy):
        return "x"
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if abs(cx - TEMPLE[0]) < 4 and abs(cy - TEMPLE[1]) < 3:
        return "t"   # the Valley Temple's granite floor
    if roads.dist(cx, cy) < 1.1:
        return "r"
    if cx > 86:
        return "G" if noise(cx, cy, 4) > 0.6 else "g"   # the fields along the Nile
    if noise(cx + 300, cy, 5) > 0.82:
        return "c"
    return "S" if noise(cx, cy, 9) > 0.6 else "s"

def height(vx, vy):
    plateau = 22 if vx < scarp(vx, vy) - 2 else 8 if vx > scarp(vx, vy) + 2 else 8 + (scarp(vx, vy) + 2 - vx) * 3.5
    if roads.dist(vx, vy) < 2.5 and abs(vx - scarp(vx, vy)) < 3:
        plateau = min(plateau, 8 + max(0.0, scarp(vx, vy) + 3 - vx) * 2.2)   # the causeway ramps up the scarp
    if vx > W - 6:
        return 3
    h = plateau + noise(vx, vy, 10) * 4 * min(1.0, roads.dist(vx, vy) / 4)
    h += pyramid_h(vx, vy) * 20   # height bytes are 0.05 cells
    if sphinx(vx, vy):
        h += 30
    if near(CAMP, vx, vy, 7):
        h = 9
    if near(EUROPEANS, vx, vy, 7):
        h = 22
    return h

EAST, WEST = 0.0, math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 104.5, 56.5, 0, 1000)
m.obj("airbase", 1, 106.5, 42.5, 0, 1000)
m.obj("bank", 1, 110.5, 60.5, 0, 4000)
m.obj("moneyplane", 1, 110.5, 60.5, 0, 0)
m.obj("stank", 1, 96.5, 50.5, WEST, 1000)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 21.5, 51.5, 0, 1000)
m.obj("bank", 2, 10.5, 40.5, 0, 4000)
m.obj("moneyplane", 2, 10.5, 40.5, 0, 0)
for kind, x, y, hp in [("tower_missile", 58.5, 36.5, 600), ("turret", 36.5, 40.5, 200), ("turret", 64.5, 44.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# the Watchers: a garage in the Valley Temple, nothing else to show
m.obj("garage", 6, TEMPLE[0] - 1, TEMPLE[1] + 0.5, 0, 1500)
m.obj("bank", 6, TEMPLE[0] + 2.5, TEMPLE[1] - 2, 0, 4000)
m.obj("moneyplane", 6, TEMPLE[0] + 2.5, TEMPLE[1] - 2, 0, 0)
for k, kind in enumerate(["electric_car", "peon_tank", "electric_car"]):
    m.obj(kind, 6, 74.5 + k * 1.5, 60.5, EAST, 0)
m.roads(roads)
# the Europeans' dig at the foot of the great pyramid: tents, crates, bones
for dx, dy in [(-6.5, 5.5), (-8.0, 2.0)]:
    m.obj("bigwhitetent", 4, PYRAMIDS[0][0][0] + dx, PYRAMIDS[0][0][1] + dy, 0.2, 100)
m.obj("giant_rib", 4, 33.5, 39.5, 0.7, 100)
m.obj("armycratebig", 4, 35.5, 37.0, 0.3, 100)
# palms along the Nile fields; the temple's pillars
for _ in range(1500):
    x, y = rng.uniform(86, W - 7), rng.uniform(2, H - 2)
    if roads.dist(x, y) > 2.5 and not near(CAMP, x, y, 8) and rng.random() < 0.06:
        m.obj("palmtree1", 4, x, y, rng.uniform(0, 6.28), 100)
for dx in (-3.5, 3.5):
    for dy in (-2.5, 2.5):
        m.obj("ruinsonepillar", 4, TEMPLE[0] + dx, TEMPLE[1] + dy, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Cairo_Egypt.mp3", ["no garage 6"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter"], money=[10000, 6000, 10000, 5000])
