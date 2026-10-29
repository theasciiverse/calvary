"""Cairo, campaign mission 12: the original's own walled town (`egypt2`,
from the Golgotha Forever levels) made a battle. Everything its makers
put down stays: the terraces, the palms and oil wells, our HQ in the
north-east and the European one in the south-east, our two launcher
towers and takeover pad in the west, the neutral pad in the south, the
enemy supertank (moved to its own HQ). Added: factories, banks and turrets, the roads through
the streets, and our airbase with the first helicopters.

usage: python3 tools/gen_cairo.py maps/cairo.cmap
(reads the level through `calvary --export egypt2`, built if need be)"""
import os, subprocess, sys, tempfile, tomllib
from mapgen import Roads

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.dirname(here)
with tempfile.TemporaryDirectory() as tmp:
    out = os.path.join(tmp, "egypt2.cmap")
    subprocess.run(["cargo", "run", "--release", "-q", "--", "--export", "egypt2", out], cwd=root, check=True)
    level = tomllib.load(open(out, "rb"))

objects = level["objects"].strip("\n").split("\n")
# the level parks the European supertank in the west, beside our two
# launchers, which kill it in seconds; it starts at its own HQ instead
objects = [("stank 2 63.5 72.5 4.7124 500" if o.startswith("stank 2 ") else o) for o in objects]
add = []
def obj(kind, player, x, y, theta=0.0, health=0):
    add.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}")

SOUTH = 1.5708
# ours, round the HQ the level already has at (64, 18)
obj("garage", 1, 70.5, 24.5, 0, 1000)
obj("airbase", 1, 71.5, 12.5, 0, 1000)
obj("bank", 1, 72.5, 4.5, 0, 4000)
obj("moneyplane", 1, 72.5, 4.5, 0, 0)
obj("stank", 1, 66.5, 26.5, SOUTH, 1000)
# the Europeans, round their HQ at (67.5, 75.8)
obj("garage", 2, 70.5, 83.5, 0, 1000)
obj("bank", 2, 72.5, 70.5, 0, 4000)
obj("moneyplane", 2, 72.5, 70.5, 0, 0)
for kind, x, y, hp in [("turret", 60.5, 64.5, 200), ("tower_missile", 66.5, 66.5, 600), ("turret", 30.5, 77.5, 200), ("turret", 16.5, 63.5, 200), ("tower_electric", 45.5, 74.5, 600)]:
    obj(kind, 2, x, y, 0, hp)
for k, kind in enumerate(["peon_tank", "electric_car", "rocket_tank"]):
    obj(kind, 2, 56.5, 80.5 + k * 1.4, -SOUTH, 0)

# the roads, through the streets: east down the ring road, west along the
# north street, down the west side and back east along the south streets
roads = Roads()
for n, x, y in [("ours", 64.5, 22.5), ("e1", 63.5, 29.5), ("e2", 63.5, 50.5), ("e3", 56.5, 57.5), ("e4", 56.5, 72.5),
                ("w1", 45.5, 20.5), ("w2", 20.5, 20.5), ("w3", 11.5, 27.5), ("w4", 11.5, 55.5), ("w5", 18.5, 66.5),
                ("w6", 25.5, 80.5), ("w7", 40.5, 80.5), ("w8", 50.5, 76.5), ("theirs", 63.5, 78.5), ("eu", 68.5, 80.5)]:
    roads.node(n, x, y)
roads.ours("ours", "e1", "e2", "e3", "e4", "theirs", "eu")
roads.ours("ours", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "w8", "theirs")
lines = add + roads.lines(len(objects) + len(add))

q = lambda v: "[" + ", ".join(f'"{c}"' for c in v) + "]"
head = [
    'name = "cairo"', f"width = {level['width']}", f"height = {level['height']}", 'music = "Cairo_Egypt.mp3"',
    f"win = {q(['no mainbasepad 2'])}", f"lose = {q(['no mainbasepad 1'])}",
    f"build = {q(['peon_tank', 'electric_car', 'rocket_tank', 'engineer', 'helicopter'])}",
]
nl = "\n"
body = (
    f"\nlegend = '''\n{level['legend'].strip(nl)}\n'''\n"
    f"tiles = '''\n{level['tiles'].strip(nl)}\n'''\n"
    f"heights = '''\n{level['heights'].strip(nl)}\n'''\n"
    f"objects = '''\n{nl.join(objects + lines)}\n'''\n"
)
open(sys.argv[1], "w").write("\n".join(head) + body)
print(f"{sys.argv[1]}: {len(objects)} objects from the level, {len(lines)} added")
