"""Vienna, campaign mission 27: the race for the Holy Lance. It lies in the
treasury of the Hofburg, in the old city inside the Ringstrasse. We come
from the Prater in the east; the Watchers' Order (player 6) from the west
down the Mariahilfer Strasse; the Europeans (player 2) hold the north
across the Danube canal and guns on the Ring. Whoever carries the Lance
home decides it. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1452)  # the year Frederick III was crowned emperor in Rome
noise = Noise(rng)
W, H = 120, 100

CENTRE = (60, 50)
RING = 17.0                     # the Ringstrasse's radius
CANAL = [(30, 18), (60, 26), (90, 40), (120, 56)]
HOFBURG = (50, 56)
LANCE = (49.5, 57.5)
CAMP = (108, 66)
ORDER = (10, 54)
EUROPEANS = (46, 8)

def canal(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(CANAL, CANAL[1:]))

def north_of_canal(x, y):
    """Whether (x, y) lies on the far side of the canal from the old city."""
    for (ax, ay), (bx, by) in zip(CANAL, CANAL[1:]):
        if ax <= x <= bx:
            return y < ay + (by - ay) * (x - ax) / (bx - ax)
    return y < CANAL[0][1] if x < CANAL[0][0] else y < CANAL[-1][1]

def ring_dist(x, y):
    return math.hypot(x - CENTRE[0], y - CENTRE[1])

roads = Roads()
for n, x, y in [("camp", 104.5, 64.5), ("prater", 88.5, 58.5), ("re", 76.5, 52.5), ("stephan", 62.5, 50.5), ("hof", 51.5, 55.5),
                ("rse", 72.5, 64.5), ("rs", 58.5, 67.5), ("rsw", 46.5, 64.5),
                ("mariah", 30.5, 58.5), ("west", 16.5, 56.5),
                ("rn", 60.5, 33.5), ("bridge", 58.5, 25.5), ("nord", 48.5, 14.5)]:
    roads.node(n, x, y)
# ours: in across the Ring to the Stephansdom and the Hofburg, or round the
# south of the Ring; on west to the Order
roads.ours("camp", "prater", "re", "stephan", "hof")
roads.ours("camp", "rse", "rs", "rsw", "hof")
roads.ours("hof", "mariah", "west", them="W")
# the Order: in by the Mariahilfer Strasse, and on to our camp
roads.link("W", "west", "mariah", "hof")
roads.link("W", "hof", "rsw", "rs", "rse", "camp")
# the Europeans: over the canal bridge and down through the Ring
roads.ours("stephan", "rn", "bridge", "nord")

m = Map("vienna", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("B", 0, "rome_bridge1a"), ("X", 4, "castle_walls_passages"), ("c", 0, "castle"),
    ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1"), ("t", 0, "airportroadplain"), ("R", 0, "airportroad"),
    ("g", 0, "francegrassfull"), ("G", 0, "grassanddirtfull"), ("m", 0, "marble_small1"), ("p", 0, "cromeplatform3"),
    ("C", 0, "cromerockbed2a"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    on_road = roads.dist(cx, cy) < 1.1
    if canal(cx, cy) < 1.8:
        return "B" if on_road else "W"
    if near(CAMP, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    r = ring_dist(cx, cy)
    if abs(r - RING) < 1.4 and not north_of_canal(cx, cy):
        return "R"   # the Ringstrasse
    if on_road:
        return "t"
    # the Hofburg: courtyards walled round, the treasury floor marble
    if abs(cx - HOFBURG[0]) < 5 and abs(cy - HOFBURG[1]) < 4:
        if abs(cx - HOFBURG[0]) > 4 or abs(cy - HOFBURG[1]) > 3:
            return "X" if not (abs(cy - HOFBURG[1]) < 1.6 or abs(cx - HOFBURG[0]) < 1.6) else "c"
        return "m"
    if r < RING - 1.4 and not north_of_canal(cx, cy):
        if 40 < cx < 50 and 40 < cy < 53 or 52 < cx < 60 and 60 < cy < 66:
            return "g"   # the Volksgarten and the Burggarten
        return "S" if (int(cx) % 5 == 0 or int(cy) % 4 == 0) else "s"
    if 82 < cx < 110 and 40 < cy < 62:
        return "g" if noise(cx, cy, 5) > 0.35 else "G"   # the Prater
    return "C" if noise(cx, cy, 6) > 0.55 else ("s" if noise(cx + 40, cy, 4) > 0.5 else "G")

def height(vx, vy):
    if canal(vx, vy) < 1.5:
        return 4
    h = 12 + noise(vx, vy, 12) * 4 * min(1.0, roads.dist(vx, vy) / 4)
    if abs(vx - HOFBURG[0]) < 5 and abs(vy - HOFBURG[1]) < 4 and (abs(vx - HOFBURG[0]) > 4 or abs(vy - HOFBURG[1]) > 3) \
            and not (abs(vy - HOFBURG[1]) < 1.6 or abs(vx - HOFBURG[0]) < 1.6):
        h = 34   # the palace walls
    return h

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 110.5, 72.5, 0, 1000)
m.obj("airbase", 1, 113.5, 58.5, 0, 1000)
m.obj("bank", 1, 102.5, 74.5, 0, 4000)
m.obj("moneyplane", 1, 102.5, 74.5, 0, 0)
m.obj("stank", 1, 101.5, 62.5, math.pi, 1000)
m.obj("mainbasepad", 6, *ORDER, 0, 4000)
m.obj("garage", 6, 8.5, 62.5, 0, 1000)
m.obj("bank", 6, 4.5, 46.5, 0, 4000)
m.obj("moneyplane", 6, 4.5, 46.5, 0, 0)
for kind, x, y, hp in [("turret", 22.5, 52.5, 200), ("tower_missile", 26.5, 62.5, 600)]:
    m.obj(kind, 6, x, y, 0, hp)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 54.5, 10.5, 0, 1000)
m.obj("bank", 2, 38.5, 12.5, 0, 4000)
m.obj("moneyplane", 2, 38.5, 12.5, 0, 0)
for kind, x, y, hp in [("tower_missile", 62.5, 36.5, 600), ("turret", 74.5, 40.5, 200), ("turret", 44.5, 42.5, 200), ("tower_electric", 70.5, 60.5, 600),
                       ("supergun", 60.5, 30.5, 1200)]:
    m.obj(kind, 2, x, y, 0, hp)
# the treasury's guard: towers at the Hofburg's gates (towers cannot carry it off)
for kind, x, y, hp in [("turret", 44.5, 56.5, 200), ("turret", 56.5, 55.5, 200), ("tower_missile", 50.5, 50.5, 600), ("tower_electric", 50.5, 62.5, 600)]:
    m.obj(kind, 2, x, y, 0, hp)
m.obj("relic", 0, *LANCE, 0, 100)
m.roads(roads)
# the old city's houses, the Prater's trees, the suburbs
for _ in range(2600):
    x, y = rng.uniform(3, W - 3), rng.uniform(3, H - 3)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, ORDER, EUROPEANS)) or (abs(x - HOFBURG[0]) < 6 and abs(y - HOFBURG[1]) < 5):
        continue
    if t in "sS" and ring_dist(x, y) < RING - 2 and rng.random() < 0.09:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "greecehouses3", "building2squnderconst"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "gG" and rng.random() < 0.06:
        m.obj("pinetree", 4, x, y, 0, 100)
    elif t in "sC" and ring_dist(x, y) > RING + 2 and rng.random() < 0.02:
        m.obj(rng.choice(["greecehouses2", "buildwarehouseloadingdock", "building2squnderconst"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Vienna_Austria.mp3", [f"reach 1 relic {CAMP[0]} {CAMP[1]} 4 1"],
        [f"reach 6 relic {ORDER[0]} {ORDER[1]} 4 1", f"reach 2 relic {EUROPEANS[0]} {EUROPEANS[1]} 4 1", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 10000, 0, 10000])
