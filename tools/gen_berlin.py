"""Berlin, campaign mission 28: the Order's command. Its headquarters in
the Tiergarten, between the Spree and the Landwehr canal, is nearly empty:
a garrison, a factory, guns, and the command itself (a takeover pad under
the Order's own turret). The Europeans (player 2) hold the north-east past
the Spree and come down every road. An engineer of ours into the command
wins it. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1961)  # the year the wall went up
noise = Noise(rng)
W, H = 120, 100

SPREE = [(0, 40), (28, 36), (52, 33), (74, 38), (96, 34), (120, 36)]
CANAL = [(0, 62), (30, 60), (60, 58), (90, 64), (120, 62)]
TIERGARTEN = ((22, 42), (52, 56))     # corners of the park
COMMAND = (40.5, 47.5)
ORDER = (30, 48)
CAMP = (18, 84)
EUROPEANS = (96, 14)

def river(pts, x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(pts, pts[1:]))

def in_park(x, y):
    (x0, y0), (x1, y1) = TIERGARTEN
    return x0 <= x <= x1 and y0 <= y <= y1

roads = Roads()
for n, x, y in [("camp", 20.5, 80.5), ("sw", 30.5, 70.5), ("cw", 32.5, 60.5), ("tier", 36.5, 52.5), ("cmd", 40.5, 49.5),
                ("s2", 56.5, 76.5), ("cm", 60.5, 58.5), ("pots", 56.5, 50.5),
                ("sb", 62.5, 35.5), ("alex", 78.5, 24.5), ("eu", 94.5, 18.5),
                ("e1", 90.5, 78.5), ("ce", 96.5, 64.0), ("kreuz", 98.5, 50.5), ("se", 100.5, 35.0), ("frie", 102.5, 24.5)]:
    roads.node(n, x, y)
# ours: up through the park to the command, or by the Potsdamer Platz;
# on over the Spree to the Europeans, or round by Kreuzberg
roads.ours("camp", "sw", "cw", "tier", "cmd")
roads.ours("camp", "s2", "cm", "pots", "cmd")
roads.link("A", "pots", "sb", "alex", "eu")   # the Europeans keep to Kreuzberg
roads.ours("camp", "e1", "ce", "kreuz", "se", "frie", "eu")
# the Order's garrison sorties down the park road
roads.link("W", "tier", "cw", "sw", "camp")

m = Map("berlin", W, H)
LEGEND = [
    ("W", 6, "watrsnow1deepbutt"), ("B", 0, "rome_bridge1a"), ("t", 0, "airportroadplain"), ("r", 0, "cromeroof"),
    ("R", 0, "crome2panarchbricks"), ("k", 0, "cromecemtwlk"), ("c", 0, "concrete_blocks"), ("g", 0, "switzsnowgrasscorner2"),
    ("G", 0, "switzsnowfull"), ("p", 0, "cromeplatform3"), ("w", 0, "snowywoods1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    if river(SPREE, cx, cy) < 1.7 or river(CANAL, cx, cy) < 1.1:
        return "B" if on_road else "W"
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5) or near(ORDER, cx, cy, 2.5):
        return "p"
    if on_road:
        return "t"
    if in_park(cx, cy):
        return "w" if noise(cx, cy, 4) > 0.62 else ("G" if noise(cx + 9, cy, 5) > 0.55 else "g")
    # city blocks between the streets
    if int(cx) % 9 == 0 or int(cy) % 7 == 0:
        return "k"
    return "r" if noise(cx, cy, 6) > 0.5 else ("R" if noise(cx + 50, cy, 5) > 0.5 else "c")

def height(vx, vy):
    if river(SPREE, vx, vy) < 1.4 or river(CANAL, vx, vy) < 0.8:
        return 6
    return 12 + noise(vx, vy, 14) * 3 * min(1.0, roads.dist(vx, vy) / 3)

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 12.5, 90.5, 0, 1000)
m.obj("airbase", 1, 26.5, 92.5, 0, 1000)
m.obj("bank", 1, 10.5, 78.5, 0, 4000)
m.obj("moneyplane", 1, 10.5, 78.5, 0, 0)
m.obj("stank", 1, 22.5, 78.5, NORTH, 1000)
# the Order's command: the pad under its own turret, the garrison round it
m.obj("takeover_pad", 6, *COMMAND, 0, 100)
m.obj("turret", 6, *COMMAND, 0, 200)
m.obj("mainbasepad", 6, *ORDER, 0, 4000)
m.obj("garage", 6, 46.5, 44.5, 0, 1000)
for kind, x, y, hp in [("tower_missile", 34.5, 44.5, 1000), ("tower_electric", 44.5, 52.5, 1000), ("tower_missile", 46.5, 48.5, 1000), ("tower_electric", 36.5, 54.5, 1000),
                       ("turret", 30.5, 56.5, 200), ("turret", 52.5, 48.5, 200), ("turret", 40.5, 56.5, 200), ("turret", 60.5, 54.5, 200), ("supergun", 42.5, 42.5, 1200)]:
    m.obj(kind, 6, x, y, 0, hp)
m.obj("bank", 6, 26.5, 44.5, 0, 4000)
m.obj("moneyplane", 6, 26.5, 44.5, 0, 0)
# the Europeans past the Spree
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 88.5, 10.5, 0, 1000)
m.obj("airbase", 2, 106.5, 12.5, 0, 1000)
m.obj("bank", 2, 102.5, 6.5, 0, 4000)
m.obj("moneyplane", 2, 102.5, 6.5, 0, 0)
for kind, x, y, hp in [("turret", 64.5, 31.5, 200), ("tower_missile", 80.5, 28.5, 600), ("turret", 100.5, 40.5, 200), ("tower_electric", 94.5, 26.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
# two neutral banks in the city for engineers
for x, y in [(58.5, 72.5), (92.5, 70.5)]:
    m.obj("bank", 0, x, y, 0, 4000)
m.roads(roads)
# the city's buildings, the park's trees
for _ in range(2600):
    x, y = rng.uniform(3, W - 3), rng.uniform(3, H - 3)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.2 or any(near(b, x, y, 6) for b in (CAMP, EUROPEANS, ORDER, COMMAND)):
        continue
    if t in "rRc" and rng.random() < 0.1:
        m.obj(rng.choice(["building2squnderconst", "buildwarehouseloadingdock", "greecehouses2", "silobigfull2x2"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "gGw" and rng.random() < 0.08:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Munich_Germany.mp3", ["own takeover_pad 1"], ["no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "engineer", "helicopter", "jet"],
        money=[10000, 20000, 0, 12000])
