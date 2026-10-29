//! Reader for Golgotha `.level` files.
//!
//! The format is Crack dot Com's sectioned save file (`loaders/dir_save.cpp`):
//! a signature, a directory of (section id, absolute offset) pairs, then the
//! section bodies. Named sections use `i4_check_sum32(name)` as their id, the
//! numbered ones come from the enum in `saver_id.h`. Everything is little endian.
//!
//! Only what the terrain needs is read so far: map size, vertex heights, cell
//! tile ids, the tile (texture) table and the object type names. Older levels
//! (egypt2) have no texture name table, only `TILE_MATCHUP` checksums of the
//! tile names; those are resolved against the baked tile table in terrain.rs. Objects
//! themselves start with serialized Lisp variables and are left for later.

use std::collections::HashMap;

/// `saver_id.h` enum values.
const SECTION_MAP_DIMENSIONS_V1: u32 = 0;
const SECTION_TILE_MATCHUP_V1: u32 = 1;
const SECTION_OBJECT_TYPES_V1: u32 = 26;
const SECTION_TEXTURE_NAMES_V2: u32 = 35;
const SECTION_OBJECTS_V1: u32 = 3;
const SECTION_OBJECT_LIST: u32 = 0x8000;
/// `g1_object_class::save` ends with x, y, h, theta, pitch, roll (f32),
/// flags (u32), health (i16), player (u8).
const OBJECT_TAIL: usize = 6 * 4 + 4 + 2 + 1;
const NO_REF: u16 = 0xffff;

/// `g1_tile_class` flags (`tile.h`).
pub const TILE_WAVE: u32 = 2;
pub const TILE_BLOCKING: u32 = 4;

/// One cell of vertex height is this many map cells tall (`g1_vert_height_table`).
pub const HEIGHT_STEP: f32 = 0.05;

/// Friction and damage are parsed for the ground-unit pass; nothing uses them yet.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TileInfo {
    /// Empty when the level has no texture name table; then only `checksum` is known.
    pub name: String,
    /// `check_sum32` of the tile's level-facing name (0 when the level does not say).
    pub checksum: u32,
    pub flags: u32,
    pub friction: f32,
    pub damage: u16,
}

#[derive(Clone, Debug)]
pub struct Level {
    /// Map size in cells; vertices are `(width + 1) * (height + 1)`.
    pub width: usize,
    pub height: usize,
    pub vert_heights: Vec<u8>,
    /// Tile id per cell, row major. 0 means no tile.
    pub cell_tiles: Vec<u16>,
    /// Indexed by tile id. With a texture name table entry 0 is "no tile";
    /// without one every entry, 0 included, is a checksum from the matchup table.
    pub tiles: Vec<TileInfo>,
    pub object_types: Vec<String>,
    /// Every saved object in save order; references between objects (path
    /// links) are indexes into this list.
    pub objects: Vec<ObjectRec>,
    /// How the map is won and lost (`.cmap` only; the original kept this in
    /// the level's `.scm`, which Calvary does not read).
    pub rules: crate::rules::Rules,
    /// What the factories may build (`.cmap` only; the original kept a
    /// `can_build` list on each garage). None: everything.
    pub build: Option<Vec<String>>,
}

/// One object from the level: the base fields every Golgotha object saves.
#[derive(Clone, Debug)]
pub struct ObjectRec {
    pub kind: String,
    pub x: f32,
    pub y: f32,
    /// Height above the map base, in map cells (unused: sprites sit on the ground).
    #[allow(dead_code)]
    pub z: f32,
    pub theta: f32,
    pub health: i16,
    /// 1 is the human player (team A), 2 the computer (team E), 0 neutral;
    /// scenery uses 3 or 4.
    pub player: u8,
    /// For `path_object`s: next nodes for team A and for team E.
    pub links_a: Vec<usize>,
    pub links_e: Vec<usize>,
    /// For `path_object`s: a gap a bridger can span (the original's
    /// `bridgeable_spot`, a Lisp variable; `.cmap` only).
    pub bridge: bool,
}

/// The original passes 17 as the length of "GOLG_SECTION_ID=32", so the
/// signature is the checksum of "GOLG_SECTION_ID=3".
pub fn check_sum32(bytes: &[u8]) -> u32 {
    let (mut c1, mut c2, mut c3, mut c4) = (0u8, 0u8, 0u8, 0u8);
    for &b in bytes {
        c1 = c1.wrapping_add(b);
        c2 = c2.wrapping_add(c1);
        c3 = c3.wrapping_add(c2);
        c4 = c4.wrapping_add(c3);
    }
    c1 as u32 | (c2 as u32) << 8 | (c3 as u32) << 16 | (c4 as u32) << 24
}

fn named(name: &str) -> u32 {
    check_sum32(name.as_bytes())
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn at(data: &'a [u8], pos: usize) -> Self {
        Reader { data, pos }
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos + n;
        let s = self.data.get(self.pos..end).ok_or_else(|| format!("truncated at byte {}", self.pos))?;
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    /// `write_counted_str`: u16 length, then the bytes (sometimes NUL terminated).
    fn counted_str(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        let s = self.bytes(n)?;
        Ok(String::from_utf8_lossy(s).trim_end_matches('\0').to_string())
    }

    /// `i4_rle_class<T>`: a control byte with the high bit set means that many raw
    /// values follow, otherwise it is a run length followed by one value.
    fn rle(&mut self, count: usize, read: fn(&mut Self) -> Result<u32, String>) -> Result<Vec<u32>, String> {
        let mut out = Vec::with_capacity(count);
        while out.len() < count {
            let ctl = self.u8()?;
            if ctl & 128 != 0 {
                for _ in 0..ctl & 127 {
                    out.push(read(self)?);
                }
            } else {
                let v = read(self)?;
                out.extend(std::iter::repeat_n(v, ctl as usize));
            }
        }
        out.truncate(count);
        Ok(out)
    }
}

/// An object section: u16 type id, then the base block
/// (`start_version`: u16 version, u16 size including this header) whose last
/// bytes are the position and owner. The Lisp variables in front of them never
/// need decoding. Path objects follow with their own versioned block of links.
fn read_object(data: &[u8], offset: Option<usize>, types: &[String]) -> Result<ObjectRec, String> {
    let Some(offset) = offset else {
        return Ok(ObjectRec { kind: String::new(), x: 0.0, y: 0.0, z: 0.0, theta: 0.0, health: 0, player: 4, links_a: vec![], links_e: vec![], bridge: false });
    };
    let mut r = Reader::at(data, offset);
    let type_id = r.u16()? as usize;
    r.u16()?; // version
    let size = r.u16()? as usize;
    let end = offset + 2 + size;
    if size < 4 + OBJECT_TAIL {
        return Err(format!("object at {offset}: base block too small"));
    }
    let mut t = Reader::at(data, end - OBJECT_TAIL);
    let (x, y, z, theta) = (t.f32()?, t.f32()?, t.f32()?, t.f32()?);
    t.f32()?; // pitch
    t.f32()?; // roll
    t.u32()?; // flags
    let health = t.u16()? as i16;
    let player = t.u8()?;
    let kind = types.get(type_id).cloned().unwrap_or_default();
    let (mut links_a, mut links_e) = (Vec::new(), Vec::new());
    if kind == "path_object" {
        // DATA_VERSION2/3: two u32 ticks, the end index of team A's links and of
        // all links (u8 each), then (path ref, object ref[, u32 desc]) per link
        let mut p = Reader::at(data, end);
        let version = p.u16()?;
        p.u16()?;
        p.u32()?;
        p.u32()?;
        let split = p.u8()? as usize;
        let total = p.u8()? as usize;
        for k in 0..total {
            let path = p.u16()?;
            p.u16()?;
            if version >= 3 {
                p.u32()?;
            }
            if path != NO_REF {
                if k < split { &mut links_a } else { &mut links_e }.push(path as usize);
            }
        }
    }
    Ok(ObjectRec { kind, x, y, z, theta, health, player, links_a, links_e, bridge: false })
}

impl Level {
    pub fn parse(data: &[u8]) -> Result<Level, String> {
        let mut r = Reader::at(data, 0);
        let sig = r.u32()?;
        let count = r.u32()? as usize;
        let mut sections = HashMap::new();
        if sig == check_sum32(b"GOLG_SECTION_ID=3") {
            for _ in 0..count {
                let id = r.u32()?;
                let off = r.u32()? as usize;
                sections.entry(id).or_insert(off);
            }
        } else if sig == check_sum32(b"GOLG") {
            for _ in 0..count {
                let id = r.u16()? as u32;
                let off = r.u32()? as usize;
                sections.entry(id).or_insert(off);
            }
        } else {
            return Err(format!("not a Golgotha level (signature {sig:#010x})"));
        }
        let section = |id: u32, what: &str| {
            sections.get(&id).map(|&o| Reader::at(data, o)).ok_or_else(|| format!("no {what} section"))
        };

        let mut r = section(SECTION_MAP_DIMENSIONS_V1, "map dimensions")?;
        let width = r.u16()? as usize;
        let height = r.u16()? as usize;

        // Levels saved before the RLE cell/vertex arrays (egypt.level) fail here.
        let mut r = section(named("golgotha vert height"), "vertex height (older level format?)")?;
        let vert_heights = r.rle((width + 1) * (height + 1), |r| r.u8().map(u32::from))?.into_iter().map(|v| v as u8).collect();

        let mut r = section(named("golgotha cell type v1"), "cell type")?;
        let cell_tiles = r.rle(width * height, |r| r.u16().map(u32::from))?.into_iter().map(|v| v as u16).collect();

        let mut checksums = Vec::new();
        if let Ok(mut r) = section(SECTION_TILE_MATCHUP_V1, "tile matchup") {
            for _ in 0..r.u32()? {
                checksums.push(r.u32()?);
            }
        }
        let blank = |checksum| TileInfo { name: String::new(), checksum, flags: 0, friction: 0.0, damage: 0 };
        let mut tiles = vec![blank(0)];
        if let Ok(mut r) = section(SECTION_TEXTURE_NAMES_V2, "texture names") {
            for _ in 0..r.u32()? {
                let name = r.counted_str()?;
                let flags = r.u32()?;
                let friction = r.f32()?;
                let damage = r.u16()?;
                r.u32()?; // filename checksum
                r.u32()?; // reserved
                r.counted_str()?; // "_reserved_"
                let checksum = checksums.get(tiles.len()).copied().unwrap_or(0);
                tiles.push(TileInfo { name, checksum, flags, friction, damage });
            }
        } else if !checksums.is_empty() {
            tiles = checksums.iter().map(|&c| blank(c)).collect();
        }

        let mut object_types = Vec::new();
        if let Ok(mut r) = section(SECTION_OBJECT_TYPES_V1, "object types") {
            for _ in 0..r.u32()? {
                object_types.push(r.counted_str()?);
            }
        }

        let objects = match section(SECTION_OBJECTS_V1, "objects") {
            Ok(mut r) => {
                let count = r.u32()? as usize;
                (0..count).map(|i| read_object(data, sections.get(&(SECTION_OBJECT_LIST + i as u32)).copied(), &object_types)).collect::<Result<_, _>>()?
            }
            Err(_) => Vec::new(),
        };

        Ok(Level { width, height, vert_heights, cell_tiles, tiles, object_types, objects, rules: Default::default(), build: None })
    }

    pub fn tile(&self, cx: usize, cy: usize) -> &TileInfo {
        let id = self.cell_tiles[cy * self.width + cx] as usize;
        self.tiles.get(id).unwrap_or(&self.tiles[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values were read independently with a Python probe of the same files.
    #[test]
    fn snow_level() {
        let l = Level::parse(include_bytes!("../data/levels/snow.level")).unwrap();
        assert_eq!((l.width, l.height), (150, 150));
        assert_eq!(l.vert_heights.iter().min(), Some(&51));
        assert_eq!(l.vert_heights.iter().max(), Some(&254));
        assert_eq!(l.tiles.len(), 141);
        assert_eq!(l.tiles[1].name, "snow_rubble");
        assert_eq!(l.tiles[97].name, "snow1");
        assert_eq!(l.object_types.len(), 184);
    }

    #[test]
    fn test_level() {
        let l = Level::parse(include_bytes!("../data/levels/test.level")).unwrap();
        assert_eq!((l.width, l.height), (100, 150));
        assert_eq!(l.tiles.len(), 95);
        assert_eq!(l.tiles[1].name, "greecestreetcorner");
        assert_eq!(l.tiles[54].name, "grassfull");
    }

    // Positions and owners cross-checked with an independent Python reader.
    #[test]
    fn test_level_objects_and_paths() {
        let l = Level::parse(include_bytes!("../data/levels/test.level")).unwrap();
        assert_eq!(l.objects.len(), 854);
        let count = |k: &str| l.objects.iter().filter(|o| o.kind == k).count();
        assert_eq!(count("path_object"), 251);
        assert_eq!(count("bank"), 5);
        let stank = l.objects.iter().find(|o| o.kind == "stank" && o.player == 1).unwrap();
        assert!((stank.x - 4.8).abs() < 0.1 && (stank.y - 110.0).abs() < 0.1);
        // team A's route from the player's garage reaches the enemy garage in 48 nodes
        let mut node = l.objects.iter().position(|o| o.kind == "path_object" && (o.x - 9.95).abs() < 0.05 && (o.y - 104.58).abs() < 0.05).unwrap();
        let mut hops = 1;
        while let Some(&next) = l.objects[node].links_a.first() {
            node = next;
            hops += 1;
        }
        assert_eq!(hops, 48);
        assert!((l.objects[node].x - 81.4).abs() < 0.1);
    }

    #[test]
    fn egypt2_level_has_only_checksums() {
        let l = Level::parse(include_bytes!("../data/levels/egypt2.level")).unwrap();
        assert_eq!((l.width, l.height), (75, 105));
        assert_eq!(l.tiles.len(), 134);
        assert!(l.tiles.iter().all(|t| t.name.is_empty()));
        assert_eq!(l.tiles[0].checksum, check_sum32(b"grassdirtmostlygrasscornergrec"));
    }
}
