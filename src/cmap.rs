//! `.cmap`: Calvary's own map format, for the campaign maps the original never
//! had. Plain TOML, readable and diffable, and loaded into the same `Level`
//! the original `.level` reader produces, so everything downstream (terrain,
//! world, renderer) works unchanged.
//!
//! ```toml
//! name = "piraeus"
//! width = 100
//! height = 150
//! music = "Greece.mp3"          # optional, a file from data/music
//! win = ["no_units 2"]          # optional, see `rules.rs` (default: destroy
//! lose = ["no mainbasepad 1"]   # their HQ, lose ours)
//! build = ["peon_tank", "bridger"]   # optional: what the factories offer
//! money = [10000, 5000, 5000]   # optional: starting money, players 1, 2, 5 (and 6)
//! night = true                  # optional: fought at night
//!
//! # one printable character per tile type used on the map
//! legend = '''
//! a 0 grassfull
//! b 4 searockfull               # char, tile flags (2 water, 4 blocking), tile name
//! '''
//! # height rows of width characters
//! tiles = '''
//! aaab...
//! '''
//! # (height + 1) rows of (width + 1) vertex heights, two hex digits each
//! # (one unit is 0.05 map cells, as in the original)
//! heights = '''
//! 0505060a...
//! '''
//! # one object per line, in order; path links name other lines by index:
//! # kind player x y theta health [A:i,j] [E:k] [R:l] [bridge]
//! # (A, E, R and W: the links players 1, 2, 5 and 6 follow; 5 and 6 are a
//! # second and a third hostile army, every army at war with every other)
//! # (`bridge` marks a path node as a gap a bridger can span)
//! objects = '''
//! mainbasepad 1 7.5 107.5 0 4000
//! path_object 1 9.9 104.6 0 100 A:577
//! path_object 1 9.9 101.6 0 100 A:1 E:3 bridge
//! '''
//! ```
//!
//! Tile names are the level-facing names the original uses (the `save_name`
//! where a texture list gives one), so `tools/tile_colors.py`'s table colours
//! them; flags travel with the map so blocking and water need no lookup.

use crate::level::{check_sum32, Level, ObjectRec, TileInfo};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Serialize, Deserialize)]
struct File {
    name: String,
    width: usize,
    height: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    music: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    win: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    build: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    money: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    night: bool,
    legend: String,
    tiles: String,
    heights: String,
    objects: String,
}

/// Legend characters: printable ASCII except space, quote and backslash
/// (TOML literal strings cannot hold a quote run, and these stay unambiguous).
fn legend_chars() -> impl Iterator<Item = char> {
    (b'!'..=b'~').map(char::from).filter(|c| !matches!(c, '\'' | '"' | '\\' | '#'))
}

/// A map in `.cmap` text. `name_of` turns tiles that only carry a checksum
/// (older levels) into names; it should return None when it cannot.
pub fn export(level: &Level, name: &str, music: Option<&str>, name_of: impl Fn(&TileInfo) -> Option<String>) -> Result<String, String> {
    let mut used: Vec<u16> = level.cell_tiles.clone();
    used.sort_unstable();
    used.dedup();
    let chars: Vec<char> = legend_chars().collect();
    if used.len() > chars.len() {
        return Err(format!("{} tile types used; the format holds {}", used.len(), chars.len()));
    }
    let mut legend = String::new();
    let mut char_of = std::collections::HashMap::new();
    for (k, &id) in used.iter().enumerate() {
        let t = &level.tiles[id as usize];
        let tile_name = if t.name.is_empty() { name_of(t).ok_or_else(|| format!("no name for tile checksum {:#010x}", t.checksum))? } else { t.name.clone() };
        let flags = if t.name.is_empty() { name_of_flags(t) } else { t.flags };
        writeln!(legend, "{} {} {}", chars[k], flags, tile_name).unwrap();
        char_of.insert(id, chars[k]);
    }
    let mut tiles = String::new();
    for row in level.cell_tiles.chunks(level.width) {
        tiles.extend(row.iter().map(|id| char_of[id]));
        tiles.push('\n');
    }
    let mut heights = String::new();
    for row in level.vert_heights.chunks(level.width + 1) {
        for h in row {
            write!(heights, "{h:02x}").unwrap();
        }
        heights.push('\n');
    }
    let mut objects = String::new();
    for o in &level.objects {
        let kind = if o.kind.is_empty() { "-" } else { &o.kind };
        write!(objects, "{kind} {} {} {} {} {}", o.player, o.x, o.y, o.theta, o.health).unwrap();
        let list = |v: &[usize]| v.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        if !o.links_a.is_empty() {
            write!(objects, " A:{}", list(&o.links_a)).unwrap();
        }
        if !o.links_e.is_empty() {
            write!(objects, " E:{}", list(&o.links_e)).unwrap();
        }
        if !o.links_r.is_empty() {
            write!(objects, " R:{}", list(&o.links_r)).unwrap();
        }
        if !o.links_w.is_empty() {
            write!(objects, " W:{}", list(&o.links_w)).unwrap();
        }
        if o.bridge {
            objects.push_str(" bridge");
        }
        objects.push('\n');
    }
    let (win, lose) = if level.rules.is_default() {
        (Vec::new(), Vec::new())
    } else {
        (level.rules.win.iter().map(|c| c.text()).collect(), level.rules.lose.iter().map(|c| c.text()).collect())
    };
    let file = File { name: name.to_string(), width: level.width, height: level.height, music: music.map(str::to_string), win, lose, build: level.build.clone(), money: level.money.map(|m| if m[3] == crate::world::START_MONEY { m[..3].to_vec() } else { m.to_vec() }), night: level.night, legend, tiles, heights, objects };
    toml::to_string(&file).map_err(|e| e.to_string())
}

/// Checksum-only tiles keep the flags the baked table gives them.
fn name_of_flags(t: &TileInfo) -> u32 {
    crate::terrain::tile_flags_for_checksum(t.checksum).unwrap_or(t.flags)
}

/// A `.cmap` into a `Level`, plus its music track if it names one.
pub fn parse(text: &str) -> Result<(Level, String, Option<String>), String> {
    let f: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let (w, h) = (f.width, f.height);
    if w == 0 || h == 0 {
        return Err("empty map".into());
    }

    let mut tiles = vec![TileInfo { name: String::new(), checksum: 0, flags: 0, friction: 0.0, damage: 0 }];
    let mut id_of = std::collections::HashMap::new();
    for line in f.legend.lines().filter(|l| !l.trim().is_empty()) {
        let mut p = line.split_whitespace();
        let (Some(c), Some(flags), Some(name)) = (p.next(), p.next(), p.next()) else {
            return Err(format!("legend line {line:?}: want `char flags name`"));
        };
        let c = c.chars().next().unwrap();
        let flags: u32 = flags.parse().map_err(|_| format!("legend {c}: bad flags {flags:?}"))?;
        id_of.insert(c, tiles.len() as u16);
        tiles.push(TileInfo { name: name.to_string(), checksum: check_sum32(name.as_bytes()), flags, friction: 0.0, damage: 0 });
    }

    let rows: Vec<&str> = f.tiles.lines().filter(|l| !l.is_empty()).collect();
    if rows.len() != h || rows.iter().any(|r| r.chars().count() != w) {
        return Err(format!("tiles: want {h} rows of {w} characters"));
    }
    let mut cell_tiles = Vec::with_capacity(w * h);
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            cell_tiles.push(*id_of.get(&c).ok_or_else(|| format!("tiles ({x},{y}): {c:?} is not in the legend"))?);
        }
    }

    let hrows: Vec<&str> = f.heights.lines().filter(|l| !l.is_empty()).collect();
    if hrows.len() != h + 1 || hrows.iter().any(|r| r.len() != 2 * (w + 1)) {
        return Err(format!("heights: want {} rows of {} hex digits", h + 1, 2 * (w + 1)));
    }
    let mut vert_heights = Vec::with_capacity((w + 1) * (h + 1));
    for row in hrows {
        for k in (0..row.len()).step_by(2) {
            vert_heights.push(u8::from_str_radix(&row[k..k + 2], 16).map_err(|_| format!("heights: bad hex {:?}", &row[k..k + 2]))?);
        }
    }

    let mut objects = Vec::new();
    for (n, line) in f.objects.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let mut p = line.split_whitespace();
        let mut field = |what: &str| p.next().ok_or_else(|| format!("objects line {n}: missing {what}"));
        let kind = field("kind")?;
        let player: u8 = field("player")?.parse().map_err(|_| format!("objects line {n}: bad player"))?;
        let mut num = |what: &str| -> Result<f32, String> { field(what)?.parse().map_err(|_| format!("objects line {n}: bad {what}")) };
        let (x, y, theta, health) = (num("x")?, num("y")?, num("theta")?, num("health")?);
        let mut o = ObjectRec { kind: if kind == "-" { String::new() } else { kind.to_string() }, x, y, z: 0.0, theta, health: health as i16, player, links_a: vec![], links_e: vec![], links_r: vec![], links_w: vec![], bridge: false };
        for extra in p {
            if extra == "bridge" {
                o.bridge = true;
                continue;
            }
            let (team, list) = extra.split_once(':').ok_or_else(|| format!("objects line {n}: bad link {extra:?}"))?;
            let ids: Vec<usize> = list.split(',').map(|i| i.parse().map_err(|_| format!("objects line {n}: bad link {extra:?}"))).collect::<Result<_, _>>()?;
            match team {
                "A" => o.links_a = ids,
                "E" => o.links_e = ids,
                "R" => o.links_r = ids,
                "W" => o.links_w = ids,
                _ => return Err(format!("objects line {n}: link team must be A, E, R or W")),
            }
        }
        objects.push(o);
    }
    let count = objects.len();
    if let Some(bad) = objects.iter().flat_map(|o| o.links_a.iter().chain(&o.links_e).chain(&o.links_r).chain(&o.links_w)).find(|&&i| i >= count) {
        return Err(format!("objects: link to line {bad}, but there are {count} lines"));
    }

    if let Some(m) = f.music.as_deref().filter(|m| !crate::audio::MUSIC.iter().any(|(n, _)| n == m)) {
        return Err(format!("music: {m:?} is not one of the bundled tracks"));
    }
    let rules = crate::rules::Rules::parse(&f.win, &f.lose)?;
    if let Some(bad) = f.build.iter().flatten().find(|k| crate::balance::unit(k).is_none()) {
        return Err(format!("build: {bad:?} is not a unit"));
    }
    let money = match f.money.as_deref() {
        None => None,
        Some([a, b, c]) => Some([*a, *b, *c, crate::world::START_MONEY]),
        Some([a, b, c, d]) => Some([*a, *b, *c, *d]),
        Some(_) => return Err("money: three amounts (players 1, 2, 5) or four (and 6)".into()),
    };
    let level = Level { width: w, height: h, vert_heights, cell_tiles, tiles, object_types: Vec::new(), objects, rules, build: f.build, money, night: f.night };
    Ok((level, f.name, f.music))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::LEVELS;

    /// Every bundled level survives a trip through .cmap unchanged: size,
    /// heights, each cell's tile name and flags, every object and path link.
    #[test]
    fn bundled_levels_round_trip() {
        for src in LEVELS {
            let a = Level::parse(src.data).unwrap();
            let text = export(&a, src.name, None, crate::terrain::tile_name_for).unwrap();
            let (b, name, _) = parse(&text).unwrap_or_else(|e| panic!("{}: {e}", src.name));
            assert_eq!(name, src.name);
            assert_eq!((a.width, a.height), (b.width, b.height));
            assert_eq!(a.vert_heights, b.vert_heights, "{} heights", src.name);
            for i in 0..a.cell_tiles.len() {
                let (ta, tb) = (a.tile(i % a.width, i / a.width), b.tile(i % b.width, i / b.width));
                let name_a = if ta.name.is_empty() { crate::terrain::tile_name_for(ta).unwrap() } else { ta.name.clone() };
                assert_eq!(name_a, tb.name, "{} cell {i}", src.name);
            }
            assert_eq!(a.objects.len(), b.objects.len());
            for (oa, ob) in a.objects.iter().zip(&b.objects) {
                assert_eq!((&oa.kind, oa.player, oa.x, oa.y, oa.theta, oa.health), (&ob.kind, ob.player, ob.x, ob.y, ob.theta, ob.health));
                assert_eq!((&oa.links_a, &oa.links_e), (&ob.links_a, &ob.links_e));
            }
        }
    }

    #[test]
    fn rejects_broken_maps() {
        let ok = "name = 'm'\nwidth = 2\nheight = 1\nlegend = '''\na 0 grassfull\n'''\ntiles = '''\naa\n'''\nheights = '''\n050505\n050505\n'''\nobjects = '''\nbank 0 1 0.5 0 4000\n'''\n";
        assert!(parse(ok).is_ok());
        assert!(parse(&ok.replace("\naa\n", "\nab\n")).unwrap_err().contains("not in the legend"));
        assert!(parse(&ok.replace("050505\n050505", "0505\n0505")).unwrap_err().contains("heights"));
        assert!(parse(&ok.replace("4000\n", "4000 A:7\n")).unwrap_err().contains("link to line 7"));
        assert!(parse(&ok.replace("height = 1\n", "height = 1\nwin = ['hold 60']\n")).unwrap_err().contains("hold 60"));
        // rules survive a save
        let (mut level, _, _) = parse(&ok.replace("height = 1\n", "height = 1\nwin = ['no_units 2']\n")).unwrap();
        assert_eq!(level.rules.win, vec![crate::rules::Cond::NoUnits(2)]);
        level.rules.lose.clear();
        let (back, _, _) = parse(&export(&level, "m", None, |_| None).unwrap()).unwrap();
        assert_eq!(back.rules.win, level.rules.win);
        // so do bridge spots and the build list
        let (level, _, _) = parse(&ok.replace("4000\n", "4000 bridge\n").replace("height = 1\n", "height = 1\nbuild = ['bridger']\n")).unwrap();
        assert!(level.objects[0].bridge);
        let (back, _, _) = parse(&export(&level, "m", None, |_| None).unwrap()).unwrap();
        assert!(back.objects[0].bridge);
        assert_eq!(back.build, Some(vec!["bridger".to_string()]));
        assert!(parse(&ok.replace("height = 1\n", "height = 1\nbuild = ['tractor']\n")).unwrap_err().contains("tractor"));
        assert!(parse(&ok.replace("height = 1\n", "height = 1\nmusic = 'Nakhayb.mp3'\n")).unwrap_err().contains("Nakhayb.mp3"));
    }
}
