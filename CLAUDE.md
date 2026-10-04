# Calvary

Asciiverse action-RTS. Rust + Ratatui, Braille dots for sub-cell detail.
Built from Golgotha (1998) by Crack dot Com, released to the public domain.
Crack dot Com was co-founded by Dave Taylor (co-author of Doom and Quake) and
Jonathan Clark (author of Abuse).

**Naming note:** Renamed from Golgotha to Calvary. Both names refer to the
same place, the site of the crucifixion ("place of the skull"): Golgotha is
the Aramaic/Hebrew name, Calvary the Latin (via the Vulgate "Calvariae
Locus"). Collision-checked clean, no existing game uses this title. Bonus: it
reads as a near-homophone of "cavalry", which suits a tank-arcade/action-RTS
hybrid. The rename is also required, not just tasteful: crack.com's release
page keeps "Golgotha" as a trademark (see Sources).

The mission: finally bring this game to a fully playable state. Twenty-eight
years after Crack dot Com released it, it deserves to be finished.

## The story

Crack dot Com's first game, Abuse, sold 80,000 copies. Golgotha was the
follow-up: an arcade tank game fused with a real-time strategy game, "C&C
but with 3D graphics". Despite deals with AMD, Red Hat Software and a
European publisher, the company ran out of money and closed in September
1998. On Oct. 22 1998 they released the source code and game data to the
public domain rather than let the work disappear.

## The original story (recovered 2026-10-04)

From crack.com's own "Golgotha Script" pages (1997-98, via the Internet
Archive; copies and URLs in `references/Golgotha/crack-com-story/`):

- 2048: global nuclear disarmament and the murder of a beloved American
  archaeologist lead to an American military incursion into Iraq, a
  European military power play, and World War III.
- Commander John Fisher II, leading the American force sent into Iraq,
  questions his mission, abandons his country and takes his troops on an
  "epic quest for truth", discovering "an awesome supernatural conflict
  behind a veil of political discord".
- With him: Lieutenant Elaine Kruese and Sergeant Reid Charles, "among
  the first to learn what has really happened at Golgotha".
- The epic was to run through "over 40 cities all over the world".
- Act 1, Scene 1 ("The Golgotha Transmission"): a press conference; a
  reporter asks Lieutenant Kruese what really happened at the Golgotha
  site "back in '48". The site title page quotes Shakespeare's Richard II
  ("...this land be call'd / The field of Golgotha and dead men's skulls").

The music titles fit this exactly: Al Basrah and Nakhayb (the Iraq
incursion), the European capitals (the power play), Jerusalem (Golgotha)
and "Norway Nephelim Battle" (the Nephilim of Genesis 6: the supernatural
conflict). Licence: the public-domain statement names "source, graphics,
music, and level files"; the web pages are not named, so use them as the
basis for Calvary's own writing rather than copying their text verbatim.

## Sources (checked 2026-10-04)

All under `asciiverse/references/Golgotha/` (note: `references/`,
not the `reference/` the top-level Asciiverse CLAUDE.md mentions):

- `golgotha-src.tar.gz`, `golgotha-textures.tar.gz`, `golgotha-music.tar.gz`,
  `golgotha-sfx.tar.gz`: the original 1998 release (2015 mirror copies).
  The source tarball has 524 `.gmod` models but only two levels (`demo`,
  `test`).
- `golgotha-forever/`: git clone of **github.com/pgrawehr/golgotha**, the
  "Golgotha Forever" revival (Patrick Grawehr, history from 2003-12-23, last
  master commit 2022-03-19). **This is the reference to work from.** It has
  19 levels, its own `textures/`, `sfx/`, `objects/`, and `Documentation/`
  (Manual.doc, GMOD format, screenshots). Branches: `X64Build` (64-bit port,
  started, unfinished), `RemoveDx5`, `AircraftModel`, `VS2019`.
  Remote `poikilos` = **github.com/Poikilos/golgotha**: 20 extra commits of
  modern g++ fixes on `poikilos/master`. Its README says it must be built
  32-bit (the Lisp engine), and its open issue #18 is a crash on Fedora 35.
  Nobody has shown either fork running on a modern 64-bit Linux; neither has
  been built here.
- Manual.doc reads fine with `catdoc`.

**License**, from the Wayback copy of crack.com/golgotha/free.html: "You can
do anything you like with the source, graphics, music, and level files,
including: sell it, include parts or all of it in your project, post it, copy
it, and change it to your heart's content. The only restrictions lay in the
code which we did not write ... (i.e. the MP3 decoder code). ... This code
and data is "public domain" ... The trademarks "Crack dot Com" and "Golgotha"
remain the property of crack.com." Consequences: never ship the bundled
mpg123 decoder (non-profit only); never call the game Golgotha.

## What the original actually is (read from the source)

- **Lane-pushing plus a hero, not C&C micro.** Factories build units, which
  drive along the level's path networks toward the enemy (`path_object`,
  ground paths white, air paths pink on the radar). The player picks which
  path new units take by clicking path nodes in the strategy view; units can
  be unlinked and sent elsewhere, but mostly they flow. The player's own
  weapon is the **supertank**, driven directly. Kills upgrade it (three
  levels, `upgrade_kill_ratio`). Ammo and health refill at your HQ.
- **Three views** (`camera.hh`, Manual 3.3): action mode (first person in the
  supertank, the default for driving), follow mode (third person, `M`) and
  strategy mode (top-down with radar and build buttons, `TAB`, "you will use
  this mode very often"). So top-down is an original view, but the
  original's driving happened mostly in first person. A top-down v1 changes
  the feel of driving; that is a deliberate choice, see Plan.
- **The map is a tile grid** (`map_cell`): one ground tile per cell, heights
  on the `(w+1)*(h+1)` vertices (byte times 0.05 cells). Tile flags mark
  water (`WAVE`) and `BLOCKING` ground.
- **The whole balance sheet is one readable Lisp table**,
  `scheme/balance.scm`: speed, turn speed, accel, cost, health, fire delay,
  detection range and weapon for every unit; damage, radius, speed and range
  per weapon; a tech tree (`dependancies`). Sim rate is `G1_HZ = 10`.
- **Units:** supertank (`stank`), peon tank, rocket tank, electric car,
  trike, bomb truck, tank buster, engineer, bridger, helicopter, jet,
  bomber, money plane. Structures: turrets, popup turrets, missile/electric
  towers, supergun, banks (money), takeover pads, factories.
- **AI** is small: `objs/ai_joe.cc` (554 lines, with ASCII formation
  tables) and `ai_jim.cc` (333).
- **The radar was already an icon map**: team-coloured unit icons, white and
  pink path lines, `$` for banks (`Documentation/shot_strategy1.jpg`). Close
  to a terminal UI as it stands.
- **The music sketches the campaign.** 15 tracks: a splash theme and 14
  named for places: Zurich, Munich, Vienna, Turin, Rome, Naples, Greece,
  Cairo, Al Basrah, Nakhayb (Iraq), Jerusalem, Helsinki (filed as
  "Helsinki_Sweden"), Finland, Norway. That reads as a route through Europe and the Middle East ending in
  Jerusalem, the city of Golgotha itself. Use it as the campaign spine.

### Corrections to the earlier draft of this file

- "Two playable demo levels (Switzerland and Cairo)": not in the 1998
  tarballs. The fork has `snow` (150x150, a river valley with a lake;
  plausibly the Swiss level) and `egypt2` (75x105, a walled town with
  roads and terraces; plausibly Cairo). Neither is confirmed by name.
- "Complete voice actor script": not in either source tree (only 10
  narrative clips and one computer voice clip in the sfx), but crack.com
  published the opening and a story teaser in 1997; both pages are
  recovered from the Internet Archive into
  `references/Golgotha/crack-com-story/` (see "The original story").
- "15 pieces of music, 100+ sfx, 2000+ textures": confirmed (15 mp3
  including the splash theme; 120 sfx entries; 2079 texture entries).
- "Mostly complete engine": about 245,000 lines of C++ (i4 core 86k, game
  31k, objects 23k, renderers 22k), 32-bit only, DirectX-era. Irrelevant for
  Calvary, which replaces it.

### Level files

All 19 fork levels probed. Bundled (verbatim copies in `data/levels/`):

| level | size | heights | tiles used |
|---|---|---|---|
| test | 100x150 | 0..255 | 45 (Greek-textured coast: river, lake, harbour wall) |
| snow | 150x150 | 51..254 | 35 (snowy valley, lake, roads) |
| egypt2 | 75x105 | 0..255 | 41 (walled town, roads, terraces, a pool) |

The rest: `some` and `tmp_level` are variants of `test`; `matter` (150x150)
and `512mb` (1500x1500) have heightmaps but one or two tile types (terrain
experiments); `demo`, `ch03`, `axhausen01`, `te01`-`te03`, `castles`,
`new`, `empty` and `large` read as flat (all heights 5); `egypt` and
`movie` use an older cell/vertex format the reader does not handle.

Objects and paths: each object section (`0x8000 + i`) is a u16 type id
(an index into the object type names) followed by the base save block,
`start_version` u16 version + u16 size (the size includes that 4-byte
header). The block ends with x, y, h, theta, pitch, roll (f32), flags
(u32), health (i16), player (u8), so position and owner are read from its
tail and the serialized Lisp variables in front never need decoding.
`path_object` adds a second versioned block: two u32 ticks, two u8 link
counts (team A's end, then the total), then per link a u16 path ref, a u16
object ref and (version 3) a u32 description. Object refs are u16 indexes
into the level's object list. On `test`: 854 objects, 251 path nodes;
player 1 is the human (team A), player 2 the computer (team E), player 0
neutral (capturable banks and towers), 3 and 4 scenery. Team A's links
from the player garage reach the enemy garage in 48 nodes.
`snow` is a skirmish sketch (bases, a few placed units, 6 path nodes, no
garages); `egypt2` holds scenery only (no bases, no paths).

Format, as implemented in `src/level.rs`: signature
`check_sum32("GOLG_SECTION_ID=3")` (the original passes length 17, so the
"2" is never summed), then a directory of (u32 id, u32 absolute offset),
little endian. Named sections use `check_sum32(name)`; numbered ones follow
the enum in `saver_id.h`. Cell and vertex arrays are `i4_rle_class` runs (a
control byte: high bit set means that many raw values, otherwise a run
count and one value). Tile ids index the `TEXTURE_NAMES_V2` table (id 0 is
"none"). Levels without that table (egypt2) only have `TILE_MATCHUP`:
`check_sum32` of each tile's level-facing name, which is the `save_name`
alias from the level `.scm` texture lists when there is one. The bake tool
emits every alias with its texture colour and `(wave T)`/`(block T)` flags,
so all 134 egypt2 tiles resolve. Objects start with serialized Lisp variables
(`li_save_object`) before their position, so they need the level's
`li_type_info` section decoded first; not done yet.

### Calvary's own maps (`.cmap`)

New maps (the campaign's 37 new missions) use a plain-text TOML format,
`src/cmap.rs` (format description at the top of that file): name, size,
optional music track, a legend of one printable character per tile type
(`char flags name`, with the original's level-facing tile names and its
water/blocking flags), the tile grid as rows of those characters, vertex
heights as rows of two-digit hex, and one object per line
(`kind player x y theta health [A:links] [E:links]`, path links naming
other lines by index). It loads into the same `Level` the original reader
produces, so the game runs on it unchanged. `calvary --export LEVEL
OUT.cmap` writes a bundled level in it, `calvary path/to/map.cmap` plays
one, and a test round-trips all three bundled levels with no difference
in heights, tiles, objects or links. `maps/piraeus.cmap` is the `test`
level exported (campaign mission 15).

The map editor (`src/editor.rs`): `calvary --edit maps/x.cmap`, or
`calvary --new NAME WxH TILE [PATH]` for a blank map (saved to
`maps/NAME.cmap` by default). It shows the map through the game's own
renderer. Tools 1-8: raise, lower, smooth, flatten (to the height under
the cursor at the start of a stroke), paint tiles, place objects
(structures, supertank start, money planes, scenery), draw paths (each
click adds or joins a node; team A links run forward along the chain,
team E back; Enter ends a chain), erase objects (path links renumbered).
Arrows/WASD or the mouse move the cursor, Space or left click applies
(hold the button to keep painting), `[` `]` brush, `,` `.` tile or object,
T tile set (used, grass, greece, sea, snow, watr, egypt, france, rome,
crom, lava, marsh, castle, temple, rock, alien), F tile flags (ground,
water, blocking), P owner, U undo, Tab map view, +/- zoom, F2 save, Esc
quit (twice when unsaved). Open ideas: test-play from inside the editor,
object rotation, a tile-transition helper for coastlines.

## Rendering

- **Tactical view (built):** top-down, centred on the supertank. One map
  cell is 2 columns x 1 row, so 4x4 Braille dots per cell. Background is the
  tile's colour (average of its original texture, baked by
  `tools/tile_colors.py`) with hill shading from the heightmap. Dots carry
  contour lines (every 0.5 cells of height, so elevation reads like a topo
  map), water shimmer, the tank, shots and explosion particles.
- **Supertank sprite (built):** drawn over the tactical view in half-block
  pixels (`▀`, separate foreground and background colours, so two freely
  coloured pixels per cell; at the closest zoom 6x6 pixels per map cell).
  Braille was tried first and dropped for units: one foreground colour per
  cell swallowed the tracks. Parts: tracks with tread bands that roll with
  the odometer, amber hull with a lighter front plate, turret with rim and
  hatch, barrel, muzzle tip and flash. Each pixel takes the most common part
  over a 3x3 sample; the barrel wins if it touches a pixel at all. Use the
  same approach for every unit type.
- **Strategy view (built):** the whole map in half-block pixels, the
  original's TAB view. Next: paths, bases and unit icons on it, like the
  original radar.
- **Follow mode (built, 2026-10-04):** the original's third-person view
  (its manual, 3.3), on M (pad: Back). `src/chase.rs` renders the
  heightmap Comanche-style: one ray per screen column marched front to back
  over the terrain, each slice drawn only above what is already there, in
  half-block pixels with the tile colours, hill shading, water glints and
  distance fog into a sky gradient; heights stretched 1.25x to read at
  terminal resolution. Structures, units, scenery (palms as trunk and
  crown), planes, the supertank (hull, turret, barrel along the aim),
  shots, bolts and blasts are depth-tested billboards, so hills hide them;
  health bars over the damaged; a crosshair where the turret points. The
  camera sits behind the tank on a heading that eases after the hull,
  anchored to the tank's ground so the tank stays three quarters down the
  screen (raised only to clear the terrain under the camera), so a hill
  ahead blocks the view as it would. Tab still opens the map and returns
  to the view you came from. The mouse does not aim in this view yet.
- **First person (next):** the same renderer with the camera at the
  turret looking along the aim, as the third entry in M's cycle.
- **Earlier note, first person / chase view (v2):** the original's main driving view. The
  terrain is a plain heightmap, which suits a voxel-space style renderer
  (column ray marching, as in Comanche, 1992) in half-blocks or Braille.
  That is a judgement from the data format, not tested yet.

## Plan

1. **Skeleton (done, 2026-10-04).** Level reader (3 levels), terrain, supertank driving
   and firing, two views, session logging. See "The skeleton" below.
2. **Objects out of levels (done, 2026-10-04).** No Lisp decoding needed:
   see "Objects and paths" under Level files.
3. **Ground war on paths (mostly done, 2026-10-04).** Garage builds (keys
   1-8), units drive the team's path links, fight, die; towers, the enemy
   supertank, banks and money, engineers capture, trikes and bomb trucks
   ram, `balance.scm` embedded verbatim and parsed at startup, HQ rearm,
   supertank lives and respawn, win/lose on HQ destruction. Supertank
   upgrades as in the original: kills score the victim's cost
   (`add_points`), the kill ratio is points per supertank lost, 1000 and
   20000 unlock levels 1 and 2 (`upgrade_kill_ratio`), and each new
   supertank is built at the earned level (`upgrade_levels` in
   `scheme/start.scm`: b120mm/heavy rocket/kevlar, acid/vortex/reactive,
   napalm/nuke/titanium; ammo, reloads and armour hit points from
   `def_stank_weapons`). Single-target weapons with a tick count (acid,
   napalm, electric beams) burn once per original tick. Still open:
   bridger, units placed in `snow` that have no garage to come from.
   Money as in the original: banks fill a crate every 500 ticks (up to
   10), and the money planes placed at each bank in the level fly the
   crates to the HQ, where they pay out; planes can be shot down, and
   with no HQ nothing arrives.
4. **Strategy layer (started).** Build bar in the HUD, J switches the
   route at the junction nearest the supertank, the strategy map draws the
   path network (chosen branches bright) and icons for every structure and
   unit. A mouse click on a junction (◇ on the strategy map, or the spot
   in the tactical view) switches it, as clicking path nodes did in the
   original. Unit commands (done): click a unit or drag a box to select,
   right-click ground to send the selection there (a group spreads into a
   grid) or an enemy to attack it; keys U (units within 12 cells of the
   supertank), `,` `.` (cycle one by one), G (rally on the supertank), X
   (hold), R (back onto the route), Z (clear); pad X selects nearby units
   and rallies them. Orders: Route, Hold, Move, Attack; under orders units
   still fire at anything in weapon range, as in the original, but do not
   leave their order to chase. Selected units show corner arcs (a pip when
   zoomed out) and their move destination; inverted icons on the map.
5. **AI (ported, 2026-10-04).** The computer follows the original
   `ai_joe`: it picks one of the 13 `formation_joe` layouts at random
   ("stock", "hole puncher", "heli hell", "rockets R us", ...), builds its
   units one by one (15 ticks apart, retrying after 20 when money is short,
   skipping a unit whose factory is gone), holds them at the garage until
   the formation is complete, releases them together as a convoy, then
   waits 100 to 600 ticks; it rebuilds its supertank at the HQ when it can
   pay the 10000. Junction branches are chosen by rotation. Balance probe:
   `cargo test --release battle_report -- --ignored --nocapture` plays 12
   minutes of a scripted player (a unit every 6 s, no supertank) against
   it; with the ported AI that ends in a stalemate in the middle of the
   map, which the human's supertank is there to break.
6. **Air (done, 2026-10-04).** The airbase builds helicopter (9), jet (0)
   and bomber (B); they fly the airbase's own route (on `test`, 43 nodes
   from airbase to enemy airbase: the original's pink paths) straight over
   terrain and fight like ground units. Sprites with ground shadows and a
   spinning rotor. Money planes see Plan 3. Open: which ground weapons
   may hit aircraft (everything can, for now).
7. **Sound (done, 2026-10-04).** The 15 original tracks (`data/music`)
   and the original sfx set (`data/sfx`, 102 files) are bundled and the
   ones in use are compiled into the binary (`src/audio.rs`), decoded by
   rodio's pure-Rust symphonia, so the release's mpg123 decoder never
   ships. Each level plays its place's track (test: Greece, snow: Zurich,
   egypt2: Cairo), F9 steps through the tracks as in the original, F8
   toggles sound, `--mute` starts silent; no audio device means silence,
   not an error. Effects: supertank main gun per upgrade level, rockets,
   machine gun, unit and tower fire, explosions, and the original radio
   voice lines (money received, bank/turret/building captured, bank and
   turret lost, supertank lost, enemy base destroyed, mission failed, and
   the per-unit "vehicle ready" calls), fading with distance from the
   supertank. A test decodes every embedded file.
8. **Campaign (drafted, 2026-10-04; map format and editor built).** Forty missions in five acts,
   following the original premise and music from Iraq through Egypt,
   Greece, Italy, the Alps, France, Germany and Austria to Scandinavia and
   back to Jerusalem: see `CAMPAIGN.md`. Three missions reuse the existing
   maps; the rest need a terrain format and map editor of our own.
9. **Follow mode (done) and first person (next)** (see Rendering).

Open questions:
- Map orientation: whether Golgotha's +y is south (as drawn here) or north.
  If maps come out mirrored compared to the original, flip y in one place
  (`render.rs`).
- Which fork level is the Switzerland demo and which the Cairo one.
- The flat levels (`ch03`, `axhausen01`, ...): genuinely flat, or terrain
  stored somewhere the reader does not look yet.
- Pace: units drive at the original 0.08 cells per tick (0.8 cells/s), so
  on `test` a push takes about five minutes. Faithful, maybe slow for a
  terminal session; a game-speed option is easy if playtesting says so.
- Credits tribute (waiting on David): the fork's movie level carries a
  Crack dot Com logo billboard (`1crackbillboard*`, plasma ball and
  lightning) and the team's group photo on a supertank (`1group_ontank*`).
  Showing them on the Credits screen in half-block pixels would be a
  fitting nod; still to check that both are in the 1998 public-domain
  textures (not only the fork), and David to decide given that "Crack dot
  Com" stays a trademark and the photo shows real people.
- Globe shape (settled 2026-10-04): in David's terminal the title globe
  comes out a little taller than wide (no pixel size reported, so the 2:1
  cell fallback is used). He likes it that way: leave it, no aspect fix.
- Mouse aim is new (2026-10-04) and untested by hand; the pad layout too
  (only detection was verified here).

## The skeleton

```bash
cargo run --release              # test level
cargo run --release -- snow      # snow level (also: egypt2)
cargo run --release -- --info    # what the level reader sees
cargo run --release -- --at 70,104   # dev aid: start the supertank at a map position
cargo run --release -- --mute    # no sound (F8 toggles in game, F9 next track)
cargo run --release -- --export snow maps/zurich.cmap   # a bundled level as .cmap
cargo run --release -- maps/piraeus.cmap                # play a .cmap
cargo run --release -- --new delphi 120x120 grassfull   # new map in the editor
cargo run --release -- --edit maps/piraeus.cmap         # edit a map
cargo test                       # level reader + every bundled cell has a tile colour
RUST_LOG=debug cargo run --release 2>calvary.log   # session log
python3 tools/tile_colors.py     # rebake data/tile_colors.tsv (needs Pillow)
```

Controls: W/S throttle gear (sticky: R, N, 1, 2, 3, so driving works on
terminals that never report key release), A/D turn, Q/E turret, SPACE main
gun, F machine gun, ENTER rocket (two), 1-8 build at the garage, 9/0/B
helicopter/jet/bomber at the airbase,
J switch route at the nearest junction (or click a junction), unit
commands (click/drag select, right-click to move or attack; U , . G X R Z), T turret lock (tracks the nearest
enemy within 14 cells and aims shells at its height), TAB
strategy view, M follow mode, `+`/`=` and `-` zoom the tactical view (1, 2, 4 or 6
columns per map cell; contour spacing scales with it), `[` `]` previous/next level, F1/H/? help window (opens
at start, pauses the game), ESC closes help or quits. With the kitty
keyboard protocol keys are exact; otherwise a key counts as held while OS
repeats keep arriving.

Mouse, after the original's action mode (its manual, section 3.3): in the
tactical view the turret follows the pointer (a `╋` crosshair marks it)
and shells arc to land where you point; left button fires the main gun,
unless it is on one of your units (selects it) or on a junction (switches
it); left drag box-selects; right button commands the selection (go
there, or attack what is under it) or, with nothing selected, fires the
machine gun. Q/E or the pad's right stick take the turret back from the
mouse. The scroll wheel zooms (battle and editor) and moves
through the title menu. On the strategy map the mouse only selects and commands.

Gamepad (gilrs, feature `gamepad`, on by default; the same approach as
`games/marble-recovery`): left stick drives with analog throttle and
steering (a W/S press hands driving back to the gears), right stick points
the turret in a map direction and clicks for the turret lock, RT main gun,
LT machine gun, X rocket, LB/RB zoom out/in, d-pad left/right moves a
build cursor shown in the HUD and A builds, d-pad up rallies nearby units
and down holds them, B switches the nearest junction, Y map view, Start
help, Back next music track; in the title menu the d-pad moves and A or
Start picks. Only pads in SDL's controller database are used, because
gilrs also reports 3D mice and tablet key pads as gamepads (David's
3Dconnexion receiver is one; it is skipped, the 8BitDo SF30 Pro in XInput
mode shows as "Xbox 360 Controller" and is used);
`CALVARY_ANY_PAD=1` accepts everything. `--no-default-features` builds
without it.

Layout: `src/level.rs` (format reader, objects, paths + tests),
`src/balance.rs` (reads `data/balance.scm`), `src/terrain.rs` (heights,
tile colours, shading), `src/tank.rs` (supertank physics), `src/world.rs`
(entities, path graph, combat, economy, computer player, battle probe),
`src/game.rs` (ties the supertank, its shots, the world and particles
together; bundled levels), `src/sprites.rs` (half-block pixel layer and
every sprite), `src/render.rs` (views, HUD, help), `src/input.rs`,
`data/levels/` and `data/balance.scm` (verbatim copies from the fork),
`data/tile_colors.tsv` (generated: name, colour, flags, aliases).

The tactical camera follows a look-ahead point in front of the tank
(along the hull, scaled by speed up to 35% of the view, behind when
reversing, eased in and out), so a moving tank settles near the trailing
third with two thirds of the view ahead. It holds still while that point
is in the middle 40% of the view, scrolls only near the edges, stops at
the map border, and always moves in whole terminal cells (fractional camera positions re-sample the
terrain every frame and make the ground shimmer).

Colour-key textures (`pink`, `alpha`) that some levels use as stray marker
cells are painted like their neighbours.

Shells and rockets fly a ballistic arc (apex 18% of the distance) to the
locked target or to full range, and stop where the terrain rises above
them, so tall hills still give cover; machine gun bullets fly flat. The
player's weapons use the table's b120mm, chain_gun and heavy_rocket.
Unit weapons fly to their target point (guided missiles and rockets home)
and instant weapons (speed 0: bolt, electric beam) hit at once and draw a
beam. Health bars appear over anything damaged. Blocking tiles (the original `BLOCKING` flag,
e.g. rock edges along rivers) and slopes steeper than 1.6 stop the tank.

Session logging follows the Asciiverse convention: every raw key event, view,
gear and level changes, each shot, and a state snapshot every 500 ms.

## Title and opening (`src/title.rs`)

Starting without a level or map argument plays the opening over the
splash theme: the Shakespeare epigraph from crack.com's own Golgotha pages
(Richard II, IV.i, "...the field of Golgotha and dead men's skulls"), then
the original Act 1 Scene 1's spinning globe against the stars, in Braille,
with the Golgotha site (Jerusalem, 31.8 N 35.2 E) pulsing red, captioned
"2048" and "The Golgotha Transmission". Any key, click or pad button
skips it. The globe is NASA's Blue Marble (public domain, `data/EARTH.txt`)
on a sphere in half-block pixels. The title: CALVARY in block letters
over the dimmed globe, which turns and tilts to the selected entry's place
(an amber ring and a caption with its coordinates; Golgotha for Controls,
Credits and Quit; campaign city names in `.cmap` file names are
recognised), a menu low on the screen (the three bundled levels, every `maps/*.cmap`, Controls, Credits,
Quit; keys, mouse or pad), and a dedication to Dave Taylor, Jonathan
Clark, Carly Taylor and the team. Credits list the Golgotha team exactly
as crack.com's credits page did (copy and URL in
`references/Golgotha/crack-com-story/`), including Carly Taylor as the
creator of the name and the plot. Esc in a battle returns to the title
(opening skipped); Ctrl+C quits outright; `--no-intro` starts at the
title; a level, map or `--at` argument skips the title entirely.

## The catalog position

Calvary fills the C&C-speed action-RTS slot. The Asciiverse RTS titles
cover different registers:

- **Antibiosis**: background hum, biological simulation, set and watch
- **Curse of War**: high-level strategic planning, fast and abstract
- **Civitas**: civilisation builder, turn-based, the long game
- **Calvary**: action-RTS, direct tank control, battlefield tempo

Calvary is also Panzer's sibling: pure tank arcade (Panzer) versus tank
arcade plus RTS command layer (Calvary).

## Dave Taylor and Jonathan Clark

Their public domain release was a generous act. Worth finding them and
telling them what Asciiverse is doing with their game. Not for permission
(none is needed), because it is the right thing and they would probably be
delighted. The source headers name Jonathan Clark at
golgotha_source@usa.net (1998, almost certainly dead). Dave Taylor has been
findable over the years in Linux gaming and id Software alumni circles.
Patrick Grawehr (the Golgotha Forever revival) deserves the same note.
Current contact details: not researched yet.

(Cross-reference: this contact is also tracked in the Asciiverse outreach
CLAUDE.md as a courtesy notification. Update that entry to the Calvary name
if not already done.)

## Status

As of 2026-10-04: opening and title (textured Earth, regions per battle,
credits), a full battle on `test` with units, economy, air, upgrades,
the ported `ai_joe`, sound, keyboard + mouse + gamepad controls, the
`.cmap` format and map editor, and a 40-mission campaign draft
(`CAMPAIGN.md`). Next candidates: decode the original's Lisp variables
(bridger, 3 campaign missions), test-play from the editor, the first new
campaign map (Al Basrah).

Playable battle on `test`: both bases, the path network, units built from
the garage, towers, banks, capturable neutrals, the computer player, and a
win/lose condition. `snow` and `egypt2` load their objects but are not
full battles (see Level files).
Conceived 2026-07-02 at scout camp. Renamed from Golgotha 2026-07-08.
Sources checked, skeleton built, objects/paths/units/economy added 2026-10-04.
