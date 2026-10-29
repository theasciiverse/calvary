"""The fjord, campaign mission 33: one road east along a ledge on the
fjord's north side, sheer mountain above it and a cliff down to the water
below. Three streams fall across the ledge to the fjord; the Watchers'
Order (player 6) blew their bridges. The Order holds the fjord head with
its own supertank, a supergun and two battleships lying under the ledge.
A supertank's mission: destroy their HQ. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1905)  # the year Norway went its own way
noise = Noise(rng)
W, H = 140, 64

LEDGE = (17.0, 27.0)        # the ledge between the mountain and the cliff (y)
FJORD = (29.0, 45.0)        # the water (y), from the sea to the head
HEAD_X = 112                # the fjord head: open valley east of this
MOUTH_X = 20                # our landing: open ground west of this
STREAMS = [36.5, 66.5, 96.5]
CAMP = (10, 20)
ORDER = (126, 30)

def mountain(x, y):
    """Sheer ground: north of the ledge, the cliff under it, the south side."""
    if x > HEAD_X:
        return y < 8 + noise(x, y, 6) * 4 or y > 54 - noise(x, y, 6) * 4
    if x < MOUTH_X:
        return y < 8 or (FJORD[1] < y)
    top = LEDGE[0] + (noise(x, 3, 5) - 0.5) * 2
    return y < top or (LEDGE[1] < y < FJORD[0]) or y > FJORD[1]

def water(x, y):
    if x <= HEAD_X + 4 and FJORD[0] <= y <= FJORD[1] - (max(0.0, x - HEAD_X) * 2):
        return True
    return any(abs(x - s) < 0.5 for s in STREAMS) and y < FJORD[0] and x < HEAD_X

roads = Roads()
nodes = [("camp", 12.5, 22.5), ("r1", 24.5, 22.5), ("s1", STREAMS[0], 22.5, True), ("r2", 50.5, 22.5), ("s2", STREAMS[1], 22.5, True),
         ("r3", 80.5, 22.5), ("s3", STREAMS[2], 22.5, True), ("r4", 108.5, 22.5), ("head", 118.5, 26.5), ("hq", 124.5, 30.5)]
for n, x, y, *b in nodes:
    roads.node(n, x, y, bridge=bool(b))
roads.ours(*[n[0] for n in nodes], them="W")

m = Map("fjord", W, H)
LEGEND = [
    ("W", 6, "watrsnow1deepbutt"), ("C", 4, "snow_cliff1"), ("c", 4, "snowcliff5"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"),
    ("o", 0, "snow_rocks1"), ("w", 0, "snowywoods1"), ("t", 0, "switzsnowroadstraight"), ("p", 0, "cromeplatform3"), ("n", 0, "norwayflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    if water(cx, cy):
        return "W"
    if mountain(cx, cy):
        return "C" if noise(cx, cy, 3) > 0.45 else "c"
    if near(CAMP, cx, cy, 1.0):
        return "n"
    if near(CAMP, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    return "w" if noise(cx, cy, 4) > 0.6 else ("o" if noise(cx + 7, cy, 3) > 0.7 else ("S" if noise(cx, cy + 9, 5) > 0.5 else "s"))

def height(vx, vy):
    if water(vx, vy) and not any(abs(vx - s) < 0.6 for s in STREAMS):
        return 4
    if vx < HEAD_X and vy < LEDGE[0] - 1:
        return min(250, 60 + (LEDGE[0] - vy) * 14)          # the mountain wall
    if vx < HEAD_X and LEDGE[1] + 0.5 < vy < FJORD[0]:
        return 30 - (vy - LEDGE[1]) * 10                     # the cliff down to the water
    if vx < HEAD_X and vy > FJORD[1]:
        return min(250, 40 + (vy - FJORD[1]) * 16)          # the far side
    if vx >= HEAD_X and (vy < 10 or vy > 52):
        return 80 + noise(vx, vy, 4) * 60
    return 26 + noise(vx, vy, 6) * 4 * min(1.0, roads.dist(vx, vy) / 3)

m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 6.5, 26.5, 0, 1000)
m.obj("airbase", 1, 14.5, 12.5, 0, 1000)
m.obj("bank", 1, 4.5, 14.5, 0, 4000)
m.obj("moneyplane", 1, 4.5, 14.5, 0, 0)
m.obj("stank", 1, 16.5, 22.5, 0.0, 1000)
# the Order at the fjord head: HQ, factory, its own supertank, the supergun
m.obj("mainbasepad", 6, *ORDER, 0, 4000)
m.obj("garage", 6, 130.5, 40.5, 0, 1000)
m.obj("bank", 6, 132.5, 22.5, 0, 4000)
m.obj("moneyplane", 6, 132.5, 22.5, 0, 0)
m.obj("stank", 6, 120.5, 30.5, math.pi, 1000)
for kind, x, y, hp in [("supergun", 122.5, 18.5, 1200), ("tower_missile", 116.5, 22.5, 600), ("tower_electric", 118.5, 36.5, 600),
                       ("turret", 44.5, 19.5, 200), ("tower_missile", 74.5, 19.5, 600), ("turret", 88.5, 25.5, 200), ("tower_electric", 104.5, 19.5, 600)]:
    m.obj(kind, 6, x, y, 0, hp)
for x in (52.5, 84.5):
    m.obj("battleship", 6, x, 30.5, 0.0, 0)
m.roads(roads)
for _ in range(1200):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 1.8 or any(near(b, x, y, 6) for b in (CAMP, ORDER)):
        continue
    if t in "wsS" and rng.random() < 0.15:
        m.obj("pinetree", 4, x, y, 0, 100)

m.write(sys.argv[1], LEGEND, tile, height, "Norway_Nephelim_Battle.mp3", ["no mainbasepad 6"], ["no mainbasepad 1"],
        build=["peon_tank", "rocket_tank", "engineer", "bridger", "helicopter"], money=[10000, 0, 0, 15000])
