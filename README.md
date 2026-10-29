# Calvary

An action-RTS for the terminal, built from Golgotha, the game Crack dot Com
was making when the company closed in 1998. You drive a supertank yourself
while your garage sends tanks, rocket tanks, electric cars and engineers
down the roads to the enemy, and you choose which roads they take. Kills
earn upgrades for your next supertank; banks fill crates of money that
planes fly home.

2048. An American archaeologist is murdered, Iraq is blamed, and America
goes in. Commander John Fisher leads the incursion, and at her dig sites
his column finds what she was killed for: bones, human in shape and far
too large. The first act runs from the beachhead at Al Basrah to Nineveh,
where Fisher refuses an order and both armies turn on him; the second
takes the deserters through Palmyra, the King's Highway, Sinai and the
Suez Canal to Cairo, Giza and the ships at Alexandria, and at Giza a
third army comes up out of the ground; the third crosses Europe from
Piraeus to Vienna, racing that army, the Watchers' Order, for the relics
it is collecting; the fourth follows the Order's map north, over the
frozen Baltic and through Finland with the Finns beside him, to a lake
in Norway where the giants are waking; the fifth goes home, by Cyprus and
Jaffa, to Jerusalem and down into Calloway's dig under Golgotha.

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install calvary
av play calvary
```

Or from the source: `cargo run --release`. Needs a terminal of at least
100x30 with truecolour and a font with the Braille block.

- **Campaign:** five acts, 40 missions, each with a briefing and
  its own objective: bridge the channels, hold a dig site, take a camp
  with an engineer, bring down a ring of towers, get convoys through,
  hold out as a rearguard, cross a desert on captured money, board ships
  under a jet strike, take superguns off a mountain, bomb an island,
  hold a dig under a waking volcano, race for relics at night, bring an
  avalanche down on a convoy, storm a chateau, fight two armies and then
  three, take trucks over sea ice that breaks under shellfire, fight
  beside the Finns, hunt hidden factories in the polar night, duel the
  Order's own supertank on a fjord, hold a frozen lake while the giants
  under it wake, land at Jaffa against all three armies, hold Jerusalem
  without firing on its holy places, dig down to the keyhole under
  Golgotha and close it. As in the original, the strategy map shows only
  where your side has been. Progress is saved.
- **Battles:** from the title menu, a full battle on the Greek coast (the
  original's own level), a skirmish in the Swiss valley, and the walled
  town in Egypt to drive around in (in the campaign they are Piraeus,
  Zurich and Cairo).
- **Three views:** top-down tactical (the default), the strategy map with
  the road network (Tab) and a follow view behind the supertank (M).

## Controls

- W/S change gear (reverse, neutral, three forward), A/D turn, Q/E turret
- Space main gun, F machine gun, Enter rockets, T turret lock
- Mouse: the turret follows the pointer, left button fires, click a
  junction to switch it, click or drag to select units, right-click to
  send them or to attack
- 1 to 8 build at the garage, V a bridger, 9 0 B helicopter, jet, bomber
  at the airbase (each map offers its own)
- J switches the route at the nearest junction; U selects nearby units, G
  rallies them, X holds, R sends them back onto the road, Z deselects
- Tab map, M follow view and first person, + and - zoom, C centre the turret
- F1 help, F8 sound, F9 next music track, F12 records a film, Esc back
- Gamepad: left stick drives, right stick aims (click to lock), RT gun,
  LT machine gun, X rockets, d-pad and A build, Y map, Start help

## Options

- `calvary LEVEL` or `calvary path/to/map.cmap`: straight into a battle
- `--no-intro` skips the opening, `--mute` starts silent
- `--edit maps/x.cmap`, `--new NAME WxH TILE`: the map editor (F5 plays
  the map from inside it, I sets its rules)
- `--film FILE`: play a recorded film

## Maps

Campaign maps are plain-text `.cmap` files (TOML; the format is described
at the top of `src/cmap.rs`), with their win and lose rules, what the
factories may build and each army's starting money. The map editor shows
them through the game's own renderer. The `tools/gen_*.py` scripts make
the campaign's maps.

## Credits

Golgotha was made by Crack dot Com: Dave Taylor, Jonathan Clark, Carly
Taylor, who created the name and the plot, and the whole team. When the
company closed in 1998 they released its source, graphics, music
and levels to the public domain. Calvary uses its levels, balance table,
textures (for the map colours), music and sound effects, and follows its
source; Patrick Grawehr's Golgotha Forever (github.com/pgrawehr/golgotha)
kept that source alive and is the reference it was built from.

The globe is NASA's Blue Marble. The credits font is X11's misc-fixed.

Calvary by Asciiverse, MIT licensed (see LICENSE).
