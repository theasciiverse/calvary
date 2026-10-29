"""Svalbard, campaign mission 34: the vault under the mountain. Calloway's
last letters lie in a vault cut into the mountain north of the frozen
Adventfjorden. Our camp is at Longyearbyen on the south shore, the
Europeans (player 2) east of us on the same shore, the Watchers' Order
(player 6) on the north side; we and the Europeans each have a road over
the fjord ice to the vault, and ice breaks under fire. Fog of war.
Whoever carries the letters home keeps them. Writes a .cmap to argv[1]."""
import math, random, sys
from mapgen import Map, Noise, Roads, near, seg_dist

rng = random.Random(1596)  # the year Barentsz sighted Svalbard
noise = Noise(rng)
W, H = 120, 100

FJORD = [(0, 58), (40, 52), (80, 48), (120, 40)]
FJORD_HALF = 7.0
VAULT = (58, 22)                  # the vault's chamber
LETTERS = (58.5, 21.5)
CAMP = (18, 84)
EUROPEANS = (102, 82)
ORDER = (104, 14)
MOUNTAIN = ((58, 16), 16.0)      # the mountain the vault is cut into

def fjord(x, y):
    return min(seg_dist(x, y, *a, *b) for a, b in zip(FJORD, FJORD[1:]))

def mountain(x, y):
    """Height of the mountain above the plain (0 off it); the vault's
    passage is cut level from the south."""
    (cx, cy), r = MOUNTAIN
    d = math.hypot(x - cx, (y - cy) * 1.3)
    h = max(0.0, r - d) * 9
    if abs(x - VAULT[0]) < 1.6 and VAULT[1] - 1.5 < y < cy + r * 0.8:
        return 0.0       # the passage
    if math.hypot(x - VAULT[0], y - VAULT[1]) < 2.2:
        return 0.0       # the chamber
    return h

roads = Roads()
for n, x, y in [("camp", 20.5, 80.5), ("lyb", 30.5, 66.5), ("ice_w", 38.5, 52.5), ("nw", 44.5, 40.5), ("gate", 58.5, 34.5), ("vault", 58.5, 23.5),
                ("eu", 100.5, 78.5), ("se", 88.5, 64.5), ("ice_e", 82.5, 48.5), ("ne", 72.5, 38.5),
                ("ord", 102.5, 18.5), ("n1", 86.5, 26.5), ("n2", 72.5, 30.5)]:
    roads.node(n, x, y)
roads.ours("camp", "lyb", "ice_w", "nw", "gate", "vault")
roads.ours("vault", "gate", "ne", "ice_e", "se", "eu")          # on to the Europeans
roads.link("E", "eu", "se", "ice_e", "ne", "gate", "vault")     # the Europeans' own way in
roads.link("W", "ord", "n1", "n2", "gate", "vault")             # the Order's
roads.link("W", "gate", "nw", "ice_w", "lyb", "camp")           # and on to us
roads.link("A", "gate", "n2", "n1", "ord")                      # ours, on to the Order

m = Map("svalbard", W, H)
LEGEND = [
    ("i", 0, "snow-mud_icepatch"), ("s", 0, "switzsnowfull"), ("S", 0, "snow1"), ("o", 0, "snow_rocks1"), ("O", 0, "snow_rocks2"),
    ("m", 0, "muddysnow"), ("C", 4, "snow_cliff1"), ("v", 4, "cromewallbigbrcks5"), ("V", 0, "cromecemtwlk"), ("t", 0, "switzsnowroadstraight"),
    ("p", 0, "cromeplatform3"), ("k", 0, "concrete_blocks"), ("n", 0, "norwayflag"),
]

def tile(x, y):
    cx, cy = x + 0.5, y + 0.5
    mt = mountain(cx, cy)
    in_cut = mt == 0 and math.hypot(cx - MOUNTAIN[0][0], (cy - MOUNTAIN[0][1]) * 1.3) < MOUNTAIN[1]
    if in_cut:
        return "V"       # the passage and the chamber, concrete
    if mt > 0:
        near_cut = abs(cx - VAULT[0]) < 2.6 and cy > VAULT[1] - 2.5
        if near_cut:
            return "v"   # the vault's walls
        return "C" if mt > 40 else ("O" if mt > 15 else "o")
    if fjord(cx, cy) < FJORD_HALF:
        return "i"
    if near(CAMP, cx, cy, 1.0):
        return "n"
    if near(CAMP, cx, cy, 3.5) or near(EUROPEANS, cx, cy, 3.5) or near(ORDER, cx, cy, 3.5):
        return "p"
    if roads.dist(cx, cy) < 1.0:
        return "t"
    if (24 < cx < 36 and 70 < cy < 80) or (86 < cx < 96 and 8 < cy < 16):
        return "k" if noise(cx, cy, 3) > 0.5 else "m"   # Longyearbyen's houses, the old mine at the Order's camp
    return "m" if noise(cx, cy, 5) > 0.7 else ("o" if noise(cx + 5, cy, 3) > 0.75 else ("S" if noise(cx, cy + 11, 6) > 0.5 else "s"))

def height(vx, vy):
    if fjord(vx, vy) < FJORD_HALF:
        return 8 + noise(vx, vy, 3) * 2
    base = 12 + noise(vx, vy, 10) * 6 * min(1.0, roads.dist(vx, vy) / 4)
    return min(250, base + mountain(vx, vy))

NORTH = -math.pi / 2
m.obj("mainbasepad", 1, *CAMP, 0, 4000)
m.obj("garage", 1, 12.5, 90.5, 0, 1000)
m.obj("airbase", 1, 28.5, 90.5, 0, 1000)
m.obj("bank", 1, 10.5, 78.5, 0, 4000)
m.obj("moneyplane", 1, 10.5, 78.5, 0, 0)
m.obj("stank", 1, 24.5, 78.5, NORTH, 1000)
m.obj("mainbasepad", 2, *EUROPEANS, 0, 4000)
m.obj("garage", 2, 110.5, 88.5, 0, 1000)
m.obj("airbase", 2, 112.5, 74.5, 0, 1000)
m.obj("bank", 2, 94.5, 90.5, 0, 4000)
m.obj("moneyplane", 2, 94.5, 90.5, 0, 0)
m.obj("mainbasepad", 6, *ORDER, 0, 4000)
m.obj("garage", 6, 112.5, 22.5, 0, 1000)
m.obj("bank", 6, 92.5, 12.5, 0, 4000)
m.obj("moneyplane", 6, 92.5, 12.5, 0, 0)
for kind, p, x, y, hp in [("tower_missile", 2, 90.5, 70.5, 600), ("turret", 2, 76.5, 56.0, 200), ("supergun", 2, 106.5, 66.5, 1200),
                          ("tower_electric", 6, 80.5, 22.5, 600), ("turret", 6, 66.5, 32.5, 200), ("tower_missile", 6, 96.5, 26.5, 600),
                          ("turret", 6, 54.5, 36.5, 200)]:
    m.obj(kind, p, x, y, 0, hp)
m.obj("relic", 0, *LETTERS, 0, 100)
m.roads(roads)
for _ in range(900):
    x, y = rng.uniform(2, W - 2), rng.uniform(2, H - 2)
    t = tile(int(x), int(y))
    if roads.dist(x, y) < 2.0 or any(near(b, x, y, 6) for b in (CAMP, EUROPEANS, ORDER)):
        continue
    if t == "k" and rng.random() < 0.25:
        m.obj(rng.choice(["cabin_small", "greecehouses2", "buildwarehouseloadingdock"]), 4, x, y, rng.choice([0, 1.5708]), 100)

m.write(sys.argv[1], LEGEND, tile, height, "Norway_Nephelim_Battle.mp3", [f"reach 1 relic {CAMP[0]} {CAMP[1]} 4 1"],
        [f"reach 6 relic {ORDER[0]} {ORDER[1]} 4 1", f"reach 2 relic {EUROPEANS[0]} {EUROPEANS[1]} 4 1", "no mainbasepad 1"],
        build=["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "cobra_tank", "helicopter", "jet", "bomber"],
        money=[10000, 10000, 0, 10000], fog=True)
