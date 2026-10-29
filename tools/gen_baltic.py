"""The Baltic crossing, campaign mission 29: six trucks over the frozen
Gulf of Finland, from the Estonian shore in the south to the Finnish
skerries in the north. Sea ice all the way (it breaks under heavy fire:
a supergun shell opens a hole, and a road with a hole in it is closed),
cut across the middle by an open lead with three ice bridges over it.
The Europeans (player 2) have superguns on two islands, battleships in
the lead and their base on the north-east shore. Our aircraft fly from a
carrier in the channel the icebreakers keep open. Writes a .cmap to
argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1944)  # the year the Gulf of Finland was last fought over
noise = Noise(rng)
W, H = 110, 120

SOUTH_SHORE = 104      # land south of this
NORTH_SHORE = 14       # land north of this
LEAD = (56, 60)        # the open lead's edges (y)
CROSSINGS = [20.5, 55.5, 90.5]   # ice bridges over the lead (x)
ISLANDS = [((55, 36), 7.0), ((92, 80), 5.5), ((18, 82), 4.0)]
CHANNEL = ((62, 106), (108, 110))   # the icebreakers' channel along the south shore
CAMP = (26, 112)
LANDING = (55.5, 7.5)
EUROPEANS = (98, 8)

def shore_y(x, base, amp):
    return base + (noise(x, base, 9) - 0.5) * amp

def in_channel(x, y):
    (x0, y0), (x1, y1) = CHANNEL
    return x0 <= x <= x1 and y0 <= y <= y1

def island(x, y):
    return max((r - math.hypot(x - cx, y - cy) + (noise(x, y, 3) - 0.5) * 2.0 for (cx, cy), r in ISLANDS), default=-1)

def in_lead(x, y):
    lo, hi = LEAD
    wob = (noise(x, 7, 6) - 0.5) * 3.0
    return lo + wob <= y <= hi + wob and not any(abs(x - c) < 2.2 for c in CROSSINGS)

roads = Roads()
for n, x, y in [("camp", 26.5, 108.5), ("fork", 40.5, 100.5),
                ("w1", 20.5, 86.5), ("w2", 20.5, 58.5), ("w3", 26.5, 30.5),
                ("m1", 50.5, 80.5), ("m2", 55.5, 58.5), ("m3", 46.5, 30.5),
                ("e1", 76.5, 90.5), ("e2", 90.5, 58.5), ("e3", 74.5, 32.5),
                ("land", *LANDING), ("eu", 94.5, 12.5)]:
    roads.node(n, x, y)
# three roads over the ice, each over its own bridge across the lead
roads.ours("camp", "fork", "w1", "w2", "w3", "land")
roads.ours("fork", "m1", "m2", "m3", "land")
roads.ours("fork", "e1", "e2", "e3", "land")
roads.ours("land", "eu")

m = Map("baltic", W, H)
LEGEND = [
    ("i", 0, "snow-mud_icepatch"), ("W", 6, "seawaterfull2"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"), ("o", 0, "snow_rocks1"),
    ("w", 0, "snowywoods1"), ("g", 0, "switzsnowgrasscorner2"), ("p", 0, "cromeplatform3"), ("d", 0, "docktopside"), ("f", 0, "finlandflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5):
        return "p"
    if near(LANDING, cx, cy, 1.0):
        return "f"   # the Finns' flag where the trucks come ashore
    if in_channel(cx, cy):
        return "W"
    if cy > shore_y(cx, SOUTH_SHORE, 4) or cy < shore_y(cx, NORTH_SHORE, 5):
        if roads.dist(cx, cy) < 1.1:
            return "d" if abs(cy - SOUTH_SHORE) < 3 or abs(cy - NORTH_SHORE) < 3 else "s"
        return "w" if noise(cx, cy, 4) > 0.6 else ("o" if noise(cx + 20, cy, 3) > 0.72 else "S")
    isl = island(cx, cy)
    if isl > 0:
        return "o" if isl < 1.2 else ("w" if noise(cx, cy, 3) > 0.55 else "s")
    if in_lead(cx, cy):
        return "W"
    return "i"

def height(vx, vy):
    if in_channel(vx, vy) or in_lead(vx, vy):
        return 6
    isl = island(vx, vy)
    if isl > 0:
        return 10 + min(isl, 5) * 3
    if vy > shore_y(vx, SOUTH_SHORE, 4) + 1 or vy < shore_y(vx, NORTH_SHORE, 5) - 1:
        return 14 + noise(vx, vy, 10) * 6
    return 8 + noise(vx, vy, 3) * 3   # pressure ridges in the ice

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 16.5, 114.5, 0, 1000)
m.obj("aircraftcarrier", 1, 84.5, 108.0, 0.0, 4000)
m.obj("bank", 1, 36.5, 114.5, 0, 4000)
m.obj("moneyplane", 1, 36.5, 114.5, 0, 0)
m.obj("stank", 1, 30.5, 108.5, NORTH, 1000)
for k in range(6):
    m.obj("car", 1, 18.5 + (k % 3) * 2.0, 106.5 + (k // 3) * 2.0, NORTH, 0)
# the Europeans: their base on the north-east shore, guns on the islands
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 104.5, 4.5, 0, 1000)
m.obj("airbase", 2, 88.5, 4.5, 0, 1000)
m.obj("bank", 2, 106.5, 12.5, 0, 4000)
m.obj("moneyplane", 2, 106.5, 12.5, 0, 0)
for kind, x, y, hp in [("supergun", 55.5, 36.5, 1200), ("tower_missile", 52.5, 40.5, 600), ("supergun", 92.5, 80.5, 1200), ("turret", 18.5, 82.5, 200),
                       ("tower_missile", 62.5, 10.5, 600), ("turret", 40.5, 14.5, 200)]:
    m.obj(kind, 2, x, y, 0, hp)
# battleships in the lead, beside the bridges
for x in (26.5, 61.5, 84.5):
    m.obj("battleship", 2, x, 58.0, 0.0, 0)
m.roads(roads)
# the shores' woods and fishing huts
for _ in range(1400):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, EUROPEANS, LANDING)):
        continue
    if t in "wsS" and rng.random() < 0.07:
        m.obj("pinetree", 4, x, y, 0, 100)
    elif t == "S" and rng.random() < 0.01:
        m.obj("cabin_small", 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Helsinki_Sweden.mp3", [f"reach 1 car {LANDING[0]} {LANDING[1]} 6 4"], ["lost 1 car 3", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 15000, 0])
