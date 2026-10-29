"""Shared pieces for the campaign map generators (tools/gen_*.py from
Act II on): value noise, a road network with each army's links, and the
.cmap writer. See src/cmap.rs for the format."""
import math, random


class Noise:
    """Smooth value noise, reproducible from the generator's seed."""

    def __init__(self, rng):
        self.rng, self.grid = rng, {}

    def _g(self, i, j):
        if (i, j) not in self.grid:
            self.grid[(i, j)] = self.rng.random()
        return self.grid[(i, j)]

    def __call__(self, x, y, s):
        fx, fy = x / s, y / s
        i, j = int(math.floor(fx)), int(math.floor(fy))
        u, v = fx - i, fy - j
        u, v = u * u * (3 - 2 * u), v * v * (3 - 2 * v)
        a, b, c, d = self._g(i, j), self._g(i + 1, j), self._g(i, j + 1), self._g(i + 1, j + 1)
        return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v


def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    if dx == 0 and dy == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)


def near(p, x, y, r):
    return math.hypot(x - p[0], y - p[1]) < r


class Roads:
    """Named path nodes and each army's links: A (ours, player 1), E (the
    enemy, player 2), R (the rival, player 5), W (the Watchers, player 6)."""

    def __init__(self):
        self.pos, self.bridge, self.links = {}, set(), {t: [] for t in "AERW"}

    def node(self, name, x, y, bridge=False):
        self.pos[name] = (x, y)
        if bridge:
            self.bridge.add(name)

    def link(self, team, *names):
        for p, q in zip(names, names[1:]):
            if (p, q) not in self.links[team]:
                self.links[team].append((p, q))

    def ours(self, *names):
        """Our road: we drive it forward, the enemy back along it."""
        self.link("A", *names)
        self.link("E", *reversed(names))

    def dist(self, x, y):
        segs = [(*self.pos[p], *self.pos[q]) for t in self.links for p, q in self.links[t]]
        return min((seg_dist(x, y, *s) for s in segs), default=1e9)

    def lines(self, first):
        """The path_object lines, numbered from object index `first`."""
        names = list(self.pos)
        idx = {n: first + k for k, n in enumerate(names)}
        out = []
        for n in names:
            x, y = self.pos[n]
            links = ""
            for t in "AERW":
                nxt = [idx[q] for p, q in self.links[t] if p == n]
                if nxt:
                    links += f" {t}:{','.join(map(str, nxt))}"
            out.append(f"path_object 1 {x:.1f} {y:.1f} 0.0000 100{links}{' bridge' if n in self.bridge else ''}")
        return out


class Map:
    def __init__(self, name, w, h):
        self.name, self.w, self.h = name, w, h
        self.objects = []

    def obj(self, kind, player, x, y, theta=0.0, health=0):
        self.objects.append(f"{kind} {player} {x:.1f} {y:.1f} {theta:.4f} {health}")
        return len(self.objects) - 1

    def roads(self, roads):
        self.objects += roads.lines(len(self.objects))

    def write(self, path, legend, tiles, heights, music, win, lose, build=None, money=None):
        """`tiles(x, y)` gives a legend character per cell, `heights(vx, vy)`
        a height byte per vertex."""
        grid = ["".join(tiles(x, y) for x in range(self.w)) for y in range(self.h)]
        hrows = ["".join("%02x" % int(round(max(0, min(255, heights(vx, vy))))) for vx in range(self.w + 1)) for vy in range(self.h + 1)]
        q = lambda v: "[" + ", ".join(f'"{c}"' for c in v) + "]"
        head = [f'name = "{self.name}"', f"width = {self.w}", f"height = {self.h}", f'music = "{music}"', f"win = {q(win)}", f"lose = {q(lose)}"]
        if build:
            head.append(f"build = {q(build)}")
        if money:
            head.append(f"money = [{', '.join(map(str, money))}]")
        legend_text = "\n".join(f"{c} {f} {n}" for c, f, n in legend)
        nl = "\n"
        text = "\n".join(head) + f"\nlegend = '''\n{legend_text}\n'''\ntiles = '''\n{nl.join(grid)}\n'''\nheights = '''\n{nl.join(hrows)}\n'''\nobjects = '''\n{nl.join(self.objects)}\n'''\n"
        open(path, "w").write(text)
        print(f"{path}: {len(self.objects)} objects")
        return grid
