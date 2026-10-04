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
- "Complete voice actor script": not found anywhere in either source tree.
  What exists: 10 short narrative clips ("bank captured", "supertank lost"
  ...) and one computer voice clip in the sfx. Treat the script as lost
  unless it turns up.
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
- **First person / chase view (v2):** the original's main driving view. The
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
   supertank lives and respawn, win/lose on HQ destruction. Still open:
   bridger, the supertank upgrade levels, money planes flying the crates
   (income is paid directly for now), units placed in `snow` that have no
   garage to come from.
4. **Strategy layer (started).** Build bar in the HUD, J switches the
   route at the junction nearest the supertank, the strategy map draws the
   path network (chosen branches bright) and icons for every structure and
   unit. Still open: mouse clicks on path nodes, unit select/unlink/stop.
5. **AI.** The computer now buys a weighted random unit every 5-11 s and
   picks junction branches by rotation; port `ai_joe` (build logic and
   formations) next, then tune. Balance probe:
   `cargo test --release battle_report -- --ignored --nocapture` plays 12
   minutes of a scripted player against the computer (the computer wins
   around 7 minutes when the player has no supertank in the fight).
6. **Air.** Helicopter, jet, bomber, money plane on the pink air paths.
7. **Sound.** The 15 tracks and the sfx through a Rust audio crate (decoding
   in Rust, so mpg123 never ships). Narrative clips for the radio barks.
8. **Campaign.** The music route as the mission spine, `test`, `snow` and
   `egypt2` as the first maps, new maps for the rest (an in-game or offline level
   format for new terrain; the original editor is not portable). Story
   text, briefings, the missing script written fresh.
9. **v2: first-person driving** (see Rendering).

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

## The skeleton

```bash
cargo run --release              # test level
cargo run --release -- snow      # snow level (also: egypt2)
cargo run --release -- --info    # what the level reader sees
cargo run --release -- --at 70,104   # dev aid: start the supertank at a map position
cargo test                       # level reader + every bundled cell has a tile colour
RUST_LOG=debug cargo run --release 2>calvary.log   # session log
python3 tools/tile_colors.py     # rebake data/tile_colors.tsv (needs Pillow)
```

Controls: W/S throttle gear (sticky: R, N, 1, 2, 3, so driving works on
terminals that never report key release), A/D turn, Q/E turret, SPACE main
gun, F machine gun, ENTER heavy rocket (two), 1-8 build at the garage,
J switch route at the nearest junction, T turret lock (tracks the nearest
enemy within 14 cells and aims shells at its height), TAB
or M strategy view, `+`/`=` and `-` zoom the tactical view (1, 2, 4 or 6
columns per map cell; contour spacing scales with it), `[` `]` previous/next level, F1/H/? help window (opens
at start, pauses the game), ESC closes help or quits. With the kitty
keyboard protocol keys are exact; otherwise a key counts as held while OS
repeats keep arriving.

Layout: `src/level.rs` (format reader, objects, paths + tests),
`src/balance.rs` (reads `data/balance.scm`), `src/terrain.rs` (heights,
tile colours, shading), `src/tank.rs` (supertank physics), `src/world.rs`
(entities, path graph, combat, economy, computer player, battle probe),
`src/game.rs` (ties the supertank, its shots, the world and particles
together; bundled levels), `src/sprites.rs` (half-block pixel layer and
every sprite), `src/render.rs` (views, HUD, help), `src/input.rs`,
`data/levels/` and `data/balance.scm` (verbatim copies from the fork),
`data/tile_colors.tsv` (generated: name, colour, flags, aliases).

The tactical camera holds still while the tank is in the middle 40% of
the view, scrolls only near the edges, stops at the map border, and always
moves in whole terminal cells (fractional camera positions re-sample the
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

Playable battle on `test`: both bases, the path network, units built from
the garage, towers, banks, capturable neutrals, the computer player, and a
win/lose condition. `snow` and `egypt2` load their objects but are not
full battles (see Level files).
Conceived 2026-07-02 at scout camp. Renamed from Golgotha 2026-07-08.
Sources checked, skeleton built, objects/paths/units/economy added 2026-10-04.
