"""Turin, campaign mission 21: the factory city. The Watchers' Order
(player 6) has taken the Shroud from the cathedral, and its truck is
driving it out through the city to the Alpine road in the north-west,
escorted. The Europeans (player 2) hold the Lingotto factories in the
south. Our camp is east of the Po. Kill the truck, take the Shroud,
bring it to our HQ; it is lost if the Order gets it out, or the
Europeans home. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1578)  # the year the Shroud came to Turin
noise = Noise(rng)
W, H = 120, 100

PO = [(96, 0), (92, 30), (94, 60), (90, 100)]
DORA = [(0, 20), (40, 18), (70, 22), (93, 26)]   # the Dora Riparia, into the Po
BRIDGES_PO = (40.5, 76.5)
BRIDGE_DORA = 60.5               # x where the Order's road crosses the Dora
DUOMO = (58, 44)
EXIT = (8, 12)
CAMP = (108, 70)
LINGOTTO = (40, 84)

def river_dist(pts, x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(pts, pts[1:]))

roads = Roads()
for n, x, y in [("camp", 105.5, 66.5), ("pnorth", 98.5, 40.5), ("pbn", 88.5, 40.5), ("centre", 70.5, 46.5), ("duomo", 60.5, 46.5),
                ("psouth", 98.5, 76.5), ("pbs", 86.5, 76.5), ("ling", 50.5, 80.5), ("eu", 42.5, 80.5),
                ("pal", 64.5, 32.5), ("dora", BRIDGE_DORA, 20.5), ("parco", 42.5, 12.5), ("francia", 28.5, 30.5), ("west", 14.5, 34.5), ("exit", EXIT[0] + 1.5, EXIT[1] + 2.5)]:
    roads.node(n, x, y)
# ours: over the north bridge into the centre, and over the south bridge to the Lingotto
roads.ours("camp", "pnorth", "pbn", "centre", "duomo")
roads.ours("camp", "psouth", "pbs", "ling", "eu")
roads.ours("centre", "ling")
# the Order's road out: north past Porta Palazzo, over the Dora, west
# through the old works, back down Corso Francia and out to the Alps
roads.link("W", "duomo", "pal", "dora", "parco", "francia", "west", "exit")

m = Map("turin", W, H)
LEGEND = [
    ("W", 6, "egyptwater1"), ("w", 6, "egyptwater4"), ("B", 0, "rome_bridge1a"),
    ("c", 0, "cromerockbed3"), ("C", 0, "cromerockbed2a"), ("k", 0, "concrete_blocks"), ("f", 0, "factoryironstone1"),
    ("s", 0, "rome_streets2_old"), ("S", 0, "rome_streets1"), ("t", 0, "airportroadplain"), ("p", 0, "cromeplatform3"),
    ("g", 0, "grassanddirtfull"), ("G", 0, "dirtfrancehalfgrass"), ("m", 0, "marble_small1"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    po, dora = river_dist(PO, cx, cy), river_dist(DORA, cx, cy)
    if po < 3.0:
        if any(abs(cy - by) < 1.0 for by in BRIDGES_PO):
            return "B"
        return "W" if po < 2.0 else "w"
    if dora < 1.6:
        if abs(cx - BRIDGE_DORA) < 1.0:
            return "B"
        return "W"
    if near(CAMP, cx, cy, 3.5) or near(EXIT, cx, cy, 3.0):
        return "p"
    if abs(cx - DUOMO[0]) < 3.5 and abs(cy - DUOMO[1]) < 2.5:
        return "m"
    if roads.dist(cx, cy) < 1.1:
        return "t"
    if cx > 96:
        return "G" if noise(cx, cy, 6) > 0.5 else "g"   # the hills of Superga, east of the Po
    if cy > 70 or (cy < 22 and cx < 60):
        return "f" if noise(cx, cy, 4) > 0.55 else ("k" if noise(cx + 50, cy, 3) > 0.5 else "C")   # works and yards
    if abs(cx - DUOMO[0]) < 22 and abs(cy - DUOMO[1]) < 16:
        return "S" if (int(cx) % 6 == 0 or int(cy) % 6 == 0) else "s"   # the Roman grid of the old town
    return "c" if noise(cx, cy, 5) > 0.5 else "C"

def height(vx, vy):
    if river_dist(PO, vx, vy) < 2.6 or river_dist(DORA, vx, vy) < 1.3:
        return 3
    h = 10 + noise(vx, vy, 10) * 5 * min(1.0, roads.dist(vx, vy) / 4)
    if vx > 98:
        h += (vx - 98) * 3 * noise(vx, vy, 8)   # Superga's slopes
    if near(CAMP, vx, vy, 7):
        h = 16
    return h

WEST = math.pi
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 110.5, 61.5, 0, 1000)
m.obj("airbase", 1, 113.5, 78.5, 0, 1000)
m.obj("bank", 1, 115.5, 68.5, 0, 4000)
m.obj("moneyplane", 1, 115.5, 68.5, 0, 0)
m.obj("stank", 1, 103.5, 64.5, WEST, 1000)
# the Europeans in the Lingotto
m.obj("mainbasepad", 2, *LINGOTTO, 0, 4000)
m.obj("garage", 2, 48.5, 86.5, 0, 1000)
m.obj("garage", 2, 32.5, 78.5, 0, 1000)
m.obj("bank", 2, 30.5, 90.5, 0, 4000)
m.obj("moneyplane", 2, 30.5, 90.5, 0, 0)
for kind, x, y, hp in [("turret", 56.5, 78.5, 200), ("tower_missile", 46.5, 74.5, 600), ("turret", 82.5, 72.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# the Order: the truck with the Shroud at the cathedral, its escort, the
# staging post at the Alpine road
m.obj("mainbasepad", 6, *EXIT, 0, 4000)
m.obj("car", 6, 60.5, 45.0, -math.pi / 2, 0)
m.obj("relic", 0, 60.5, 45.0, 0, 100)
for k, kind in enumerate(["electric_car", "peon_tank", "electric_car", "rocket_tank"]):
    m.obj(kind, 6, 58.5 + (k % 2) * 4, 48.5 + (k // 2) * 1.5, -math.pi / 2, 0)
for kind, x, y in [("turret", 14.5, 16.5), ("turret", 3.5, 15.5), ("tower_electric", 20.5, 30.5), ("turret", 62.5, 16.5)]:
    m.obj(kind, 6, x, y, 0, 200 if kind == "turret" else 600)
m.roads(roads)
# the city: the cathedral, blocks of the old town, the works, silos
m.obj("building_duomo", 4, DUOMO[0] - 2.5, DUOMO[1] - 1.0, 0, 100)
for _ in range(2500):
    x, y = rng.uniform(4, 94), rng.uniform(4, 96)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.4 or near(CAMP, x, y, 8) or near(EXIT, x, y, 6) or near(LINGOTTO, x, y, 6) or near(DUOMO, x, y, 5):
        continue
    if t in "sS" and rng.random() < 0.06:
        m.obj(rng.choice(["greecehouses1", "greecehouses2", "building2squnderconst"]), 4, x, y, rng.choice([0, 1.5708]), 100)
    elif t in "fkC" and rng.random() < 0.04:
        m.obj(rng.choice(["buildwarehouseloadingdock", "silobigfull2x2", "building2squnderconst"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Turin_Italy.mp3", [f"reach 1 relic {CAMP[0]} {CAMP[1]} 4 1"],
        [f"reach 6 relic {EXIT[0] + 1.5} {EXIT[1] + 2.5} 5 1", f"reach 2 relic {LINGOTTO[0]} {LINGOTTO[1]} 4 1", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 10000, 0, 5000])
