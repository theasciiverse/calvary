"""Check a campaign map's ground: which places can reach which over
non-blocking cells (8-connected, as a tank's probe could slip a corner),
and that no structure, unit or path node stands on blocking ground.

usage: python3 tools/check_map.py MAP.cmap X,Y [X,Y ...]
The first point is the start; each other is reported reachable or not."""
import re, sys

text = open(sys.argv[1]).read()
legend = {l.split()[0]: int(l.split()[1]) for l in re.search(r"legend = '''\n(.*?)'''", text, re.S).group(1).strip().split("\n")}
tiles = re.search(r"tiles = '''\n(.*?)'''", text, re.S).group(1).strip("\n").split("\n")
objs = re.search(r"objects = '''\n(.*?)'''", text, re.S).group(1).strip("\n").split("\n")
H, W = len(tiles), len(tiles[0])
blocked = lambda x, y: legend[tiles[y][x]] & 4
pts = [tuple(int(float(v)) for v in p.split(",")) for p in sys.argv[2:]]
seen, stack = {pts[0]}, [pts[0]]
while stack:
    x, y = stack.pop()
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            a, b = x + dx, y + dy
            if 0 <= a < W and 0 <= b < H and (a, b) not in seen and not blocked(a, b):
                seen.add((a, b))
                stack.append((a, b))
for p in pts[1:]:
    print(f"{p}: {'reachable' if p in seen else 'NOT reachable'}")
props = ("palmtree", "ruins", "tent", "crate", "cabin", "giant_", "wall", "house", "ship", "silo", "moneyplane")
bad = [o for o in objs if not o.split()[0].startswith(props) and blocked(int(float(o.split()[2])), int(float(o.split()[3])))]
print(f"{len(bad)} on blocking ground", *bad[:5], sep="\n  ")
