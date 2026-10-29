"""Helsinki, campaign mission 30: the harbour city, where the Finns join
Fisher. The Finnish army (player 7, our allies) holds the Senate Square
and its own factory; the Europeans (player 2) hold Katajanokka across the
North Harbour, a battleship in the South Harbour, a supergun on the
fortress island of Suomenlinna and an airfield in Kallio. We land in the
west. The South Harbour is frozen: an ice road crosses it to Katajanokka.
Destroy the Europeans' HQ; neither ours nor the Finns' may fall. Writes a
.cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1812)  # the year Helsinki became the capital
noise = Noise(rng)
W, H = 120, 100

CAMP = (16, 58)
SENATE = (58, 42)
KATAJANOKKA = (90, 48)
KALLIO = (88, 16)
SUOMENLINNA = ((86, 86), 6.0)
SOUTH_HARBOUR = ((64, 56), (84, 74))     # frozen
NORTH_HARBOUR = [(70, 34), (82, 38), (96, 36), (120, 34)]

def land(x, y):
    """Distance inside the shore (negative: out at sea)."""
    coast = 62 + (noise(x, 3, 10) - 0.5) * 6 + max(0.0, x - 70) * 0.25
    d = coast - y
    (sx, sy), sr = SUOMENLINNA
    isl = sr - math.hypot(x - sx, y - sy) + (noise(x, y, 3) - 0.5) * 2
    return max(d, isl)

def in_south_harbour(x, y):
    (x0, y0), (x1, y1) = SOUTH_HARBOUR
    return x0 <= x <= x1 and y0 <= y <= y1 and land(x, y) < 0

def north_harbour(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(NORTH_HARBOUR, NORTH_HARBOUR[1:]))

roads = Roads()
for n, x, y in [("camp", 18.5, 54.5), ("ruo", 30.5, 50.5), ("mann", 42.5, 40.5), ("sen", 58.5, 44.5), ("kbr", 76.5, 44.5), ("kata", 88.5, 50.5),
                ("espa", 44.5, 56.5), ("tori", 62.5, 58.5), ("ice", 74.5, 62.5),
                ("toolo", 34.5, 26.5), ("kal", 60.5, 20.5), ("kall", 84.5, 20.5), ("ns", 92.5, 30.5)]:
    roads.node(n, x, y)
# ours: by the Mannerheimintie to the Senate and over the bridge; by the
# Esplanadi and the ice of the South Harbour; or round the north by Kallio
roads.ours("camp", "ruo", "mann", "sen", "kbr", "kata")
roads.ours("camp", "espa", "tori", "ice", "kata")
roads.ours("ruo", "toolo", "kal", "kall", "ns", "kata")
roads.ours("sen", "tori")

m = Map("helsinki", W, H)
LEGEND = [
    ("W", 6, "seawaterfull2"), ("i", 0, "snow-mud_icepatch"), ("B", 0, "rome_bridge1a"), ("t", 0, "airportroadplain"),
    ("r", 0, "cromeroof"), ("R", 0, "crome2panarchbricks"), ("k", 0, "cromecemtwlk"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"),
    ("w", 0, "snowywoods1"), ("o", 0, "snow_rocks1"), ("p", 0, "cromeplatform3"), ("d", 0, "docktopside"), ("f", 0, "finlandflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    if north_harbour(cx, cy) < 2.2:
        return "B" if on_road else "W"
    if land(cx, cy) < 0:
        if in_south_harbour(cx, cy):
            return "i"
        return "W"
    if near(SENATE, cx, cy, 1.0):
        return "f"
    if near(CAMP, cx, cy, 3.5) or near(KATAJANOKKA, cx, cy, 3.0) or near(SENATE, cx, cy, 3.5) or near(KALLIO, cx, cy, 3.0):
        return "p"
    if on_road:
        return "t"
    if land(cx, cy) < 1.5:
        return "d" if 40 < cx < 92 and cy > 50 else "o"   # quays in the city, rocks elsewhere
    (sx, sy), _ = SUOMENLINNA
    if math.hypot(cx - sx, cy - sy) < 8:
        return "o" if noise(cx, cy, 2) > 0.5 else "s"
    if cx < 24 or cy < 12 or (26 < cx < 40 and 26 < cy < 40):
        return "w" if noise(cx, cy, 4) > 0.55 else "s"   # the parks and the woods round the city
    if int(cx) % 7 == 0 or int(cy) % 6 == 0:
        return "k"
    return "r" if noise(cx, cy, 5) > 0.5 else ("R" if noise(cx + 40, cy, 4) > 0.5 else "S")

def height(vx, vy):
    l = land(vx, vy)
    if l < 0 or north_harbour(vx, vy) < 1.8:
        return 6 if not in_south_harbour(vx, vy) else 7
    return 10 + min(l, 6) * 0.6 + noise(vx, vy, 12) * 4 * min(1.0, roads.dist(vx, vy) / 3)

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 12.5, 46.5, 0, 1000)
m.obj("airbase", 1, 8.5, 50.5, 0, 1000)
m.obj("bank", 1, 24.5, 46.5, 0, 4000)
m.obj("moneyplane", 1, 24.5, 46.5, 0, 0)
m.obj("stank", 1, 22.5, 54.5, 0.0, 1000)
# the Finns at the Senate Square
m.obj("mainbasepad", 7, *SENATE, 0, 4000)
m.obj("garage", 7, 52.5, 38.5, 0, 1000)
m.obj("bank", 7, 64.5, 38.5, 0, 4000)
m.obj("moneyplane", 7, 64.5, 38.5, 0, 0)
for kind, x, y, hp in [("turret", 66.5, 46.5, 200), ("tower_missile", 54.5, 48.5, 600), ("turret", 62.5, 30.5, 200)]:
    m.obj(kind, 7, x, y, 0, hp)
# the Europeans: Katajanokka, the Kallio airfield, the harbours
m.obj("mainbasepad", 2, *KATAJANOKKA, 0, 4000)
m.obj("garage", 2, 96.5, 52.5, 0, 1000)
m.obj("airbase", 2, *KALLIO, 0, 1000)
m.obj("bank", 2, 100.5, 44.5, 0, 4000)
m.obj("moneyplane", 2, 100.5, 44.5, 0, 0)
m.obj("garage", 2, 96.5, 16.5, 0, 1000)
for kind, x, y, hp in [("supergun", 86.5, 86.5, 1200), ("tower_missile", 82.5, 52.5, 600), ("tower_electric", 84.5, 44.5, 600), ("turret", 78.5, 40.5, 200),
                       ("turret", 72.5, 22.5, 200), ("tower_missile", 90.5, 26.5, 600), ("turret", 100.5, 56.5, 200), ("tower_electric", 76.5, 60.0, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
m.obj("battleship", 2, 60.5, 70.5, 0.0, 0)
m.roads(roads)
# the city's buildings and the woods
for _ in range(2600):
    x, y = rng.uniform(3, W - 3), rng.uniform(3, H - 3)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.2 or any(near(b, x, y, 6) for b in (CAMP, SENATE, KATAJANOKKA, KALLIO)):
        continue
    if t in "rRS" and rng.random() < 0.1:
        m.obj(rng.choice(["building2squnderconst", "greecehouses2", "greecehouses3", "buildwarehouseloadingdock"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "ws" and rng.random() < 0.08:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Helsinki_Sweden.mp3", ["no mainbasepad 2"], ["no mainbasepad 1", "no mainbasepad 7"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 20000, 0, 10000, 10000])
