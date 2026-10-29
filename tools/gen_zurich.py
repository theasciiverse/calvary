"""Zurich, campaign mission 23: the original's own snowy valley (`snow`,
from the Golgotha Forever levels) made a battle. Everything its makers
put down stays: both bases with their garages, the European airbase and
launchers, the placed tanks, the six path nodes round the garages. Added:
path nodes along the level's own road tiles (round the top past our base,
down the east side and over the river, then west along the south shore
or round the south-east), joining the two bases' own nodes, and five
neutral banks on the roads to take: the banks of Zurich.

usage: python3 tools/gen_zurich.py maps/zurich.cmap
(reads the level through `calvary --export snow`, built if need be)"""
import os, subprocess, sys, tempfile, tomllib
from mapgen import Roads

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.dirname(here)
with tempfile.TemporaryDirectory() as tmp:
    out = os.path.join(tmp, "snow.cmap")
    subprocess.run(["cargo", "run", "--release", "-q", "--", "--export", "snow", out], cwd=root, check=True)
    level = tomllib.load(open(out, "rb"))

objects = level["objects"].strip("\n").split("\n")
# the level's own nodes where its roads end: ours past the garage, theirs
OUR_END, THEIR_END = 11, 36
assert objects[OUR_END].startswith("path_object 1 64.96") and objects[THEIR_END].startswith("path_object 1 20.33")

add = []
def obj(kind, player, x, y, theta=0.0, health=0):
    add.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}")

# the banks of Zurich, beside the roads (an engineer driving by takes one), nobody's yet
for x, y in [(95.5, 26.0), (113.5, 84.0), (60.5, 94.0), (119.0, 110.5), (48.5, 122.0)]:
    obj("bank", 0, x, y, 0, 4000)
# the Europeans watch the river crossing
for kind, x, y, hp in [("tower_missile", 116.5, 61.5, 600), ("turret", 125.5, 72.5, 200), ("turret", 26.5, 122.5, 200)]:
    obj(kind, 2, x, y, 0, hp)

roads = Roads()
for n, x, y in [("r0", 70.5, 16.5), ("r1", 79.5, 25.5), ("r2", 95.5, 28.5), ("r3", 106.5, 35.5), ("r4", 115.5, 45.5), ("r5", 119.5, 55.5),
                ("r6", 121.5, 66.5), ("r7", 121.5, 76.5), ("j", 113.5, 81.5),
                ("w1", 95.5, 85.5), ("w2", 88.5, 88.5), ("w3", 60.5, 91.5), ("w4", 45.5, 88.5), ("w5", 16.5, 90.5), ("w6", 9.5, 97.5),
                ("w7", 12.5, 108.5), ("w8", 14.5, 118.5), ("w9", 18.5, 124.5),
                ("s1", 116.5, 95.5), ("s2", 116.5, 110.5), ("s3", 108.5, 128.5), ("s4", 98.5, 135.5), ("s5", 80.5, 138.5), ("s6", 64.5, 129.5),
                ("s7", 48.5, 124.5), ("s8", 32.5, 126.5)]:
    roads.node(n, x, y)
roads.ours("r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "j", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "w8", "w9")
roads.ours("j", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8")
first = len(objects) + len(add)
lines = roads.lines(first)
index = {n: first + k for k, n in enumerate(roads.pos)}

def link(line, team, to):
    """Add a link for `team` to path line `line`."""
    parts = line.split()
    for k, p in enumerate(parts):
        if p.startswith(team + ":"):
            parts[k] = f"{p},{to}"
            return " ".join(parts)
    return " ".join(parts[:6] + [f"{team}:{to}"] + parts[6:])

# join the level's own nodes to the new roads, both ways
objects[OUR_END] = link(objects[OUR_END], "A", index["r0"])
lines[list(roads.pos).index("r0")] = link(lines[list(roads.pos).index("r0")], "E", OUR_END)
for end in ("w9", "s8"):
    k = list(roads.pos).index(end)
    lines[k] = link(lines[k], "A", THEIR_END)
objects[THEIR_END] = link(objects[THEIR_END], "E", index["w9"])
objects[THEIR_END] = link(objects[THEIR_END], "E", index["s8"])

q = lambda v: "[" + ", ".join(f'"{c}"' for c in v) + "]"
head = [
    'name = "zurich"', f"width = {level['width']}", f"height = {level['height']}", 'music = "Zurich_Switzerland.mp3"',
    f"win = {q(['own bank 1 4'])}", f"lose = {q(['no mainbasepad 1'])}",
    f"build = {q(['peon_tank', 'electric_car', 'rocket_tank', 'trike', 'tank_buster', 'engineer', 'cobra_tank', 'helicopter'])}",
]
nl = "\n"
body = (
    f"\nlegend = '''\n{level['legend'].strip(nl)}\n'''\n"
    f"tiles = '''\n{level['tiles'].strip(nl)}\n'''\n"
    f"heights = '''\n{level['heights'].strip(nl)}\n'''\n"
    f"objects = '''\n{nl.join(objects + add + lines)}\n'''\n"
)
open(sys.argv[1], "w").write("\n".join(head) + body)
print(f"{sys.argv[1]}: {len(objects)} objects from the level, {len(add) + len(lines)} added")
