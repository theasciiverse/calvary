//! Playable terrain built from a parsed level: heights in map cells, a colour per
//! cell from the baked texture table, and the tile flags that matter for driving.

use crate::level::{check_sum32, Level, TileInfo, HEIGHT_STEP, TILE_BLOCKING, TILE_WAVE};
use std::collections::HashMap;
use std::sync::OnceLock;

pub type Rgb = (u8, u8, u8);

const UNKNOWN: Rgb = (110, 110, 110);
/// Colour-key textures, not ground.
const MARKER_TILES: &[&str] = &["pink", "alpha"];

/// The baked tile table (`tools/tile_colors.py`): colour and flags per tile
/// name, looked up by lowercase name or by `check_sum32` of the exact name.
struct TileTable {
    by_name: HashMap<String, (Rgb, u32)>,
    by_checksum: HashMap<u32, (Rgb, u32)>,
    name_of: HashMap<u32, String>,
}

fn tile_table() -> &'static TileTable {
    static TABLE: OnceLock<TileTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = TileTable { by_name: HashMap::new(), by_checksum: HashMap::new(), name_of: HashMap::new() };
        for line in include_str!("../data/tile_colors.tsv").lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            let [name, r, g, b, flags] = f[..] else { continue };
            let (Ok(r), Ok(g), Ok(b), Ok(flags)) = (r.parse(), g.parse(), b.parse(), flags.parse()) else { continue };
            t.by_name.insert(name.to_lowercase(), ((r, g, b), flags));
            t.by_checksum.insert(check_sum32(name.as_bytes()), ((r, g, b), flags));
            t.name_of.insert(check_sum32(name.as_bytes()), name.to_string());
        }
        t
    })
}

/// A tile's level-facing name: its own, or the baked table's for a tile that
/// only carries a checksum (levels without a texture name table).
pub fn tile_name_for(t: &TileInfo) -> Option<String> {
    if !t.name.is_empty() {
        return Some(t.name.clone());
    }
    tile_table().name_of.get(&t.checksum).cloned()
}

/// Every tile name in the baked table, sorted, for the editor's palette.
pub fn tile_names() -> Vec<String> {
    let mut v: Vec<String> = tile_table().name_of.values().cloned().collect();
    v.sort_unstable_by_key(|n| n.to_lowercase());
    v.dedup();
    v
}

/// Flags the baked table gives a tile name.
pub fn tile_flags_for_name(name: &str) -> u32 {
    tile_flags_for_checksum(check_sum32(name.as_bytes())).unwrap_or(0)
}

/// Flags the baked table gives a tile checksum.
pub fn tile_flags_for_checksum(checksum: u32) -> Option<u32> {
    tile_table().by_checksum.get(&checksum).map(|&(_, f)| f)
}

/// Colour and flags for a level tile. A named tile keeps the level's own flags.
fn resolve(t: &TileInfo) -> Option<(Rgb, u32)> {
    let table = tile_table();
    if t.name.is_empty() {
        table.by_checksum.get(&t.checksum).copied()
    } else {
        table.by_name.get(&t.name.to_lowercase()).map(|&(rgb, _)| (rgb, t.flags))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub color: Rgb,
    pub blocking: bool,
    pub water: bool,
}

pub struct Terrain {
    pub name: String,
    pub w: usize,
    pub h: usize,
    /// Vertex heights in map cells, `(w + 1) * (h + 1)`.
    heights: Vec<f32>,
    cells: Vec<Cell>,
}

impl Terrain {
    pub fn from_level(name: &str, level: &Level) -> Terrain {
        let mut missing = 0;
        let cells = (0..level.width * level.height)
            .map(|i| {
                let t = level.tile(i % level.width, i / level.width);
                let (color, flags) = resolve(t).unwrap_or_else(|| {
                    missing += 1;
                    (UNKNOWN, t.flags)
                });
                Cell { color, blocking: flags & TILE_BLOCKING != 0, water: flags & TILE_WAVE != 0 }
            })
            .collect();
        if missing > 0 {
            log::warn!("{name}: {missing} cells have no baked tile colour");
        }
        let mut cells: Vec<Cell> = cells;
        // Colour-key textures (`pink`, `alpha`) show up as stray marker cells in
        // some levels; paint them like the ground next to them.
        let marker = |i: usize| MARKER_TILES.contains(&level.tile(i % level.width, i / level.width).name.to_lowercase().as_str());
        for i in (0..cells.len()).filter(|&i| marker(i)) {
            let (x, y) = ((i % level.width) as i64, (i / level.width) as i64);
            let near = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().map(|(dx, dy)| (x + dx, y + dy)).find(|&(nx, ny)| {
                nx >= 0 && ny >= 0 && (nx as usize) < level.width && (ny as usize) < level.height && !marker(ny as usize * level.width + nx as usize)
            });
            if let Some((nx, ny)) = near {
                cells[i] = cells[ny as usize * level.width + nx as usize];
            }
        }
        Terrain {
            name: name.to_string(),
            w: level.width,
            h: level.height,
            heights: level.vert_heights.iter().map(|&v| v as f32 * HEIGHT_STEP).collect(),
            cells,
        }
    }

    pub fn cell(&self, cx: usize, cy: usize) -> &Cell {
        &self.cells[cy * self.w + cx]
    }

    /// Cell under a map position, or None off the map.
    pub fn cell_at(&self, x: f32, y: f32) -> Option<&Cell> {
        if x < 0.0 || y < 0.0 || x >= self.w as f32 || y >= self.h as f32 {
            return None;
        }
        Some(self.cell(x as usize, y as usize))
    }

    fn vert(&self, vx: usize, vy: usize) -> f32 {
        self.heights[vy.min(self.h) * (self.w + 1) + vx.min(self.w)]
    }

    /// Ground height at a map position, bilinear between the four corner vertices.
    pub fn height(&self, x: f32, y: f32) -> f32 {
        let x = x.clamp(0.0, self.w as f32);
        let y = y.clamp(0.0, self.h as f32);
        let (vx, vy) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - vx as f32, y - vy as f32);
        let top = self.vert(vx, vy) * (1.0 - fx) + self.vert(vx + 1, vy) * fx;
        let bottom = self.vert(vx, vy + 1) * (1.0 - fx) + self.vert(vx + 1, vy + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Height gradient (dh/dx, dh/dy) at a map position.
    pub fn slope(&self, x: f32, y: f32) -> (f32, f32) {
        let e = 0.25;
        ((self.height(x + e, y) - self.height(x - e, y)) / (2.0 * e), (self.height(x, y + e) - self.height(x, y - e)) / (2.0 * e))
    }

    /// Terrain colour at a map position, hill shaded with light from the north west.
    pub fn shaded(&self, x: f32, y: f32) -> Rgb {
        let Some(cell) = self.cell_at(x, y) else { return (0, 0, 0) };
        let (dx, dy) = self.slope(x, y);
        let light = (0.85 - (dx + dy) * 0.35).clamp(0.45, 1.25);
        let s = |c: u8| (c as f32 * light).clamp(0.0, 255.0) as u8;
        (s(cell.color.0), s(cell.color.1), s(cell.color.2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::LEVELS;

    #[test]
    fn every_cell_of_every_bundled_level_has_a_tile_colour() {
        for src in LEVELS {
            let l = Level::parse(src.data).unwrap();
            let missing: Vec<&str> =
                l.cell_tiles.iter().map(|&id| &l.tiles[id as usize]).filter(|t| resolve(t).is_none()).map(|t| t.name.as_str()).collect();
            assert!(missing.is_empty(), "{}: {} cells unresolved, e.g. {:?}", src.name, missing.len(), missing.first());
        }
    }
}
