//! The map editor: `calvary --edit map.cmap` (or `--new NAME WxH TILE` for a
//! blank one). It edits a `Level` in place and shows it through the game's
//! own renderer, so what you see is what plays.
//!
//! Tools (keys 1-8): raise, lower, smooth, flatten terrain; paint tiles;
//! place objects; draw paths; erase objects. Arrows/WASD or the mouse move
//! the cursor, Space or the left button applies (hold to keep applying),
//! `[` `]` brush size, `,` `.` previous/next tile or object kind, T tile set,
//! F tile flags (water/blocking), P owner, Enter ends a path chain, B marks
//! the path node under the cursor as a bridge spot, L whose roads the path
//! tool draws (ours and the enemy's back along them, the rival's, the
//! enemy's own), I the map's rules (win, lose, build, money, music, night; Tab
//! steps through them, Enter sets one), U undo, Tab map view, +/- zoom,
//! F5 test-play (F6 with the supertank at the cursor; Esc comes back),
//! F2 save, Esc quit (twice if unsaved).

use crate::cmap;
use crate::game::{Game, View};
use crate::level::{check_sum32, Level, ObjectRec, TileInfo};
use crate::render;
use crate::terrain;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::DefaultTerminal;
use std::io;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Raise,
    Lower,
    Smooth,
    Flatten,
    Paint,
    Place,
    Path,
    Erase,
}

const TOOLS: [(Tool, &str); 8] = [
    (Tool::Raise, "raise"),
    (Tool::Lower, "lower"),
    (Tool::Smooth, "smooth"),
    (Tool::Flatten, "flatten"),
    (Tool::Paint, "paint"),
    (Tool::Place, "place"),
    (Tool::Path, "path"),
    (Tool::Erase, "erase"),
];

/// Tile sets for the palette, by name prefix in the original's texture names
/// ("used" is whatever the map already has).
const TILE_SETS: [&str; 16] =
    ["used", "grass", "greece", "sea", "snow", "watr", "egypt", "france", "rome", "crom", "lava", "marsh", "castle", "temple", "rock", "alien"];

/// What the place tool can put down: structures, the supertank start, money
/// planes, and the commoner pieces of scenery from the original levels.
const OBJECT_KINDS: [&str; 35] = [
    "mainbasepad",
    "garage",
    "airbase",
    "bank",
    "takeover_pad",
    "turret",
    "moneyplane",
    "tower_electric",
    "tower_missile",
    "supergun",
    "volcano",
    "cornice",
    "relic",
    "cathedral",
    "stank",
    "peon_tank",
    "palmtree1",
    "greecehouses1",
    "greecehouses2",
    "greecehouses3",
    "ruinsonepillar",
    "ruinstwopillar",
    "ruinsruin",
    "brickwall",
    "giant_skull",
    "giant_spine",
    "giant_ribs",
    "giant_rib",
    "onelargecrate",
    "armycratebig",
    "bigwhitetent",
    "cabin_small",
    "silobigfull2x2",
    "greeceship1",
    "greclighthouse",
];

const OWNERS: [(u8, &str); 6] = [(1, "player"), (2, "enemy"), (5, "rival"), (6, "watchers"), (0, "neutral"), (4, "scenery")];

/// Whose roads the path tool draws (L cycles): ours, which the Europeans
/// (player 2) also drive the other way, as on every map so far; the rival
/// army's (player 5, `R:` links); or the Europeans' own, forward.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Road {
    Ours,
    Rival,
    Enemy,
    Watchers,
}
const ROADS: [(Road, &str); 4] = [(Road::Ours, "ours, enemy back"), (Road::Rival, "rival"), (Road::Enemy, "enemy"), (Road::Watchers, "watchers")];

/// The map's settings the I prompt edits, in order.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Field {
    Win,
    Lose,
    Build,
    Money,
    Music,
    Night,
}
const FIELDS: [(Field, &str); 6] =
    [(Field::Win, "win"), (Field::Lose, "lose"), (Field::Build, "build"), (Field::Money, "money"), (Field::Music, "music"), (Field::Night, "night")];

/// What the editor asks of its caller when it returns.
pub enum Exit {
    Quit,
    /// Play the map as it stands (unsaved edits included), the supertank at
    /// this position if one is given; then come back to the editor.
    Play(Box<Editor>, Option<(f32, f32)>),
}
const FLAG_SETS: [(u32, &str); 4] = [(0, "ground"), (2, "water"), (4, "blocking"), (6, "deep water")];

pub struct Editor {
    path: String,
    name: String,
    music: Option<String>,
    level: Level,
    game: Game,
    cursor: (f32, f32),
    tool: Tool,
    brush: f32,
    tile_set: usize,
    tile_pick: usize,
    flags: usize,
    object_pick: usize,
    owner: usize,
    /// Last path node of the chain being drawn.
    chain: Option<usize>,
    flatten_to: Option<u8>,
    undo: Vec<Level>,
    dirty: bool,
    rebuild: bool,
    applying: bool,
    stroke_started: bool,
    message: String,
    quit_armed: bool,
    road: usize,
    /// The I prompt: which setting, and its text being typed.
    prompt: Option<(usize, String)>,
}

fn default_health(kind: &str) -> i16 {
    match kind {
        "mainbasepad" | "garage" | "airbase" | "bank" => 4000,
        "stank" => 1000,
        _ => crate::balance::unit(kind).map_or(100, |s| s.health.max(1) as i16),
    }
}

impl Editor {
    pub fn new(path: &str, level: Level, name: String, music: Option<String>) -> Editor {
        let mut game = Game::from_level(&name, &level);
        game.hide_tank = true;
        game.help = false;
        // a night map is edited by day (F5 plays it at night)
        game.world.night = false;
        let cursor = (level.width as f32 / 2.0, level.height as f32 / 2.0);
        Editor {
            path: path.to_string(),
            name,
            music,
            level,
            game,
            cursor,
            tool: Tool::Raise,
            brush: 2.0,
            tile_set: 0,
            tile_pick: 0,
            flags: 0,
            object_pick: 0,
            owner: 0,
            chain: None,
            flatten_to: None,
            undo: Vec::new(),
            dirty: false,
            rebuild: false,
            applying: false,
            stroke_started: false,
            message: "1-8 tools, Space or click to apply, I map rules, F5 play, F2 save, Esc quit".into(),
            quit_armed: false,
            road: 0,
            prompt: None,
        }
    }

    /// A blank map: `w` x `h` cells of one tile at a gentle height.
    pub fn blank(w: usize, h: usize, tile: &str) -> Level {
        Level {
            width: w,
            height: h,
            vert_heights: vec![40; (w + 1) * (h + 1)],
            cell_tiles: vec![1; w * h],
            tiles: vec![
                TileInfo { name: String::new(), checksum: 0, flags: 0, friction: 0.0, damage: 0 },
                TileInfo { name: tile.to_string(), checksum: check_sum32(tile.as_bytes()), flags: terrain::tile_flags_for_name(tile), friction: 0.0, damage: 0 },
            ],
            object_types: Vec::new(),
            objects: Vec::new(),
            rules: Default::default(),
            build: None,
            money: None,
            night: false,
        }
    }

    fn palette(&self) -> Vec<String> {
        let set = TILE_SETS[self.tile_set];
        if set == "used" {
            let mut used: Vec<String> = self.level.cell_tiles.iter().map(|&id| self.level.tiles[id as usize].clone()).filter_map(|t| terrain::tile_name_for(&t)).collect();
            used.sort();
            used.dedup();
            return used;
        }
        terrain::tile_names().into_iter().filter(|n| n.to_lowercase().starts_with(set)).collect()
    }

    fn current_tile(&self) -> Option<String> {
        let p = self.palette();
        (!p.is_empty()).then(|| p[self.tile_pick % p.len()].clone())
    }

    fn snapshot(&mut self) {
        self.undo.push(self.level.clone());
        if self.undo.len() > 40 {
            self.undo.remove(0);
        }
    }

    fn vert(&mut self, vx: usize, vy: usize) -> &mut u8 {
        &mut self.level.vert_heights[vy * (self.level.width + 1) + vx]
    }

    /// Vertices within the brush, with a 0..1 falloff weight.
    fn brush_verts(&self) -> Vec<(usize, usize, f32)> {
        let (cx, cy, r) = (self.cursor.0, self.cursor.1, self.brush);
        let (w, h) = (self.level.width, self.level.height);
        let mut v = Vec::new();
        for vy in ((cy - r).floor().max(0.0) as usize)..=((cy + r).ceil() as usize).min(h) {
            for vx in ((cx - r).floor().max(0.0) as usize)..=((cx + r).ceil() as usize).min(w) {
                let d = (vx as f32 - cx).hypot(vy as f32 - cy);
                if d <= r {
                    v.push((vx, vy, 1.0 - d / (r + 0.5)));
                }
            }
        }
        v
    }

    fn apply(&mut self) {
        if !self.stroke_started {
            self.stroke_started = true;
            if !matches!(self.tool, Tool::Path) {
                self.snapshot();
            }
            if self.tool == Tool::Flatten {
                let (x, y) = (self.cursor.0.round() as usize, self.cursor.1.round() as usize);
                self.flatten_to = Some(*self.vert(x.min(self.level.width), y.min(self.level.height)));
            }
        }
        match self.tool {
            Tool::Raise | Tool::Lower => {
                let sign = if self.tool == Tool::Raise { 1.0 } else { -1.0 };
                for (vx, vy, k) in self.brush_verts() {
                    let h = self.vert(vx, vy);
                    *h = (*h as f32 + sign * 3.0 * k).round().clamp(0.0, 255.0) as u8;
                }
            }
            Tool::Smooth => {
                let w = self.level.width + 1;
                let old = self.level.vert_heights.clone();
                for (vx, vy, k) in self.brush_verts() {
                    let mut sum = 0.0;
                    let mut n = 0.0;
                    for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (0, 0)] {
                        let (x, y) = (vx as i32 + dx, vy as i32 + dy);
                        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) <= self.level.height {
                            sum += old[y as usize * w + x as usize] as f32;
                            n += 1.0;
                        }
                    }
                    let h = self.vert(vx, vy);
                    *h = (*h as f32 * (1.0 - k * 0.5) + sum / n * k * 0.5).round() as u8;
                }
            }
            Tool::Flatten => {
                let target = self.flatten_to.unwrap_or(40) as f32;
                for (vx, vy, k) in self.brush_verts() {
                    let h = self.vert(vx, vy);
                    *h = (*h as f32 + (target - *h as f32) * k.min(1.0) * 0.5).round() as u8;
                }
            }
            Tool::Paint => {
                let Some(name) = self.current_tile() else { return };
                let flags = FLAG_SETS[self.flags].0;
                let id = match self.level.tiles.iter().position(|t| t.name == name && t.flags == flags) {
                    Some(i) => i,
                    None => {
                        self.level.tiles.push(TileInfo { name: name.clone(), checksum: check_sum32(name.as_bytes()), flags, friction: 0.0, damage: 0 });
                        self.level.tiles.len() - 1
                    }
                } as u16;
                let (cx, cy, r) = (self.cursor.0, self.cursor.1, self.brush);
                for cy_ in ((cy - r).floor().max(0.0) as usize)..((cy + r).ceil() as usize).min(self.level.height) {
                    for cx_ in ((cx - r).floor().max(0.0) as usize)..((cx + r).ceil() as usize).min(self.level.width) {
                        if (cx_ as f32 + 0.5 - cx).hypot(cy_ as f32 + 0.5 - cy) <= r.max(0.5) {
                            self.level.cell_tiles[cy_ * self.level.width + cx_] = id;
                        }
                    }
                }
            }
            Tool::Place => {
                if self.stroke_started && self.applying_repeat() {
                    return;
                }
                let kind = OBJECT_KINDS[self.object_pick];
                let player = if matches!(kind, "palmtree1" | "onelargecrate" | "armycratebig" | "silobigfull2x2" | "greeceship1" | "greclighthouse" | "bigwhitetent" | "cabin_small")
                    || kind.starts_with("greecehouses")
                    || kind.starts_with("ruins")
                    || kind.starts_with("giant_")
                    || kind == "brickwall"
                {
                    4
                } else {
                    OWNERS[self.owner].0
                };
                self.level.objects.push(ObjectRec {
                    kind: kind.to_string(),
                    x: self.cursor.0,
                    y: self.cursor.1,
                    z: 0.0,
                    theta: 0.0,
                    health: default_health(kind),
                    player,
                    links_a: vec![],
                    links_e: vec![],
                    links_r: vec![],
                    links_w: vec![],
                    bridge: false,
                });
                self.message = format!("placed {kind} ({})", OWNERS.iter().find(|o| o.0 == player).map_or("scenery", |o| o.1));
            }
            Tool::Path => {
                if self.stroke_started && self.applying_repeat() {
                    return;
                }
                self.snapshot();
                // click an existing node to join the chain to it, else add one
                let near = self.level.objects.iter().position(|o| o.kind == "path_object" && (o.x - self.cursor.0).hypot(o.y - self.cursor.1) < 0.8);
                let node = match near {
                    Some(n) => n,
                    None => {
                        self.level.objects.push(ObjectRec {
                            kind: "path_object".into(),
                            x: self.cursor.0,
                            y: self.cursor.1,
                            z: 0.0,
                            theta: 0.0,
                            health: 100,
                            player: 1,
                            links_a: vec![],
                            links_e: vec![],
                            links_r: vec![],
                            links_w: vec![],
                            bridge: false,
                        });
                        self.level.objects.len() - 1
                    }
                };
                // ours: team A drives the chain forward, team E back along it;
                // the rival's and the enemy's own roads run forward only
                if let Some(prev) = self.chain.filter(|&p| p != node) {
                    let push = |v: &mut Vec<usize>, n: usize| {
                        if !v.contains(&n) {
                            v.push(n);
                        }
                    };
                    match ROADS[self.road].0 {
                        Road::Ours => {
                            push(&mut self.level.objects[prev].links_a, node);
                            push(&mut self.level.objects[node].links_e, prev);
                        }
                        Road::Rival => push(&mut self.level.objects[prev].links_r, node),
                        Road::Watchers => push(&mut self.level.objects[prev].links_w, node),
                        Road::Enemy => push(&mut self.level.objects[prev].links_e, node),
                    }
                }
                self.chain = Some(node);
                self.message = "path: click the next node, Enter ends the chain, B on a node makes it a bridge spot".into();
            }
            Tool::Erase => {
                if self.stroke_started && self.applying_repeat() {
                    return;
                }
                let hit = self
                    .level
                    .objects
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (i, (o.x - self.cursor.0).hypot(o.y - self.cursor.1)))
                    .filter(|&(_, d)| d < self.brush.max(1.0))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(i, _)| i);
                if let Some(i) = hit {
                    let kind = self.level.objects[i].kind.clone();
                    remove_object(&mut self.level, i);
                    if self.chain == Some(i) {
                        self.chain = None;
                    }
                    self.message = format!("erased {kind}");
                }
            }
        }
        self.dirty = true;
        self.rebuild = true;
    }

    /// One-shot tools act once per press, not every frame while held.
    fn applying_repeat(&self) -> bool {
        self.applying
    }

    fn save(&mut self) {
        match cmap::export(&self.level, &self.name, self.music.as_deref(), terrain::tile_name_for).and_then(|t| std::fs::write(&self.path, t).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.dirty = false;
                self.message = format!("saved {}", self.path);
            }
            Err(e) => self.message = format!("save failed: {e}"),
        }
    }

    fn rebuild_view(&mut self) {
        let (view, zoom, camera) = (self.game.view, self.game.zoom, self.game.camera);
        self.game = Game::from_level(&self.name, &self.level);
        self.game.hide_tank = true;
        self.game.help = false;
        self.game.world.night = false;
        self.game.view = view;
        self.game.zoom = zoom;
        self.game.camera = camera;
        self.rebuild = false;
    }

    fn status(&self) -> Vec<Line<'static>> {
        let amber = Style::default().fg(Color::Rgb(255, 180, 40));
        let green = Style::default().fg(Color::Rgb(80, 220, 120));
        let dim = Style::default().fg(Color::Rgb(70, 110, 80));
        let mut tools = vec![Span::styled(" EDIT ", Style::default().fg(Color::Black).bg(Color::Rgb(255, 180, 40)).add_modifier(Modifier::BOLD))];
        for (k, (t, name)) in TOOLS.iter().enumerate() {
            let style = if *t == self.tool { Style::default().fg(Color::Black).bg(Color::Rgb(80, 220, 120)) } else { dim };
            tools.push(Span::styled(format!(" {} {name} ", k + 1), style));
        }
        let h = {
            let (x, y) = (self.cursor.0.round() as usize, self.cursor.1.round() as usize);
            self.level.vert_heights[y.min(self.level.height) * (self.level.width + 1) + x.min(self.level.width)]
        };
        let detail = match self.tool {
            Tool::Paint => format!(
                "tile {} [{}] set {} ({}/{})  flags {}",
                self.current_tile().unwrap_or_else(|| "-".into()),
                self.tile_pick % self.palette().len().max(1) + 1,
                TILE_SETS[self.tile_set],
                self.tile_pick % self.palette().len().max(1) + 1,
                self.palette().len(),
                FLAG_SETS[self.flags].1
            ),
            Tool::Place => format!("object {}  owner {}", OBJECT_KINDS[self.object_pick], OWNERS[self.owner].1),
            Tool::Path => format!("path chain {}  roads {} (L)", if self.chain.is_some() { "open" } else { "-" }, ROADS[self.road].1),
            _ => String::new(),
        };
        let info = Line::from(vec![
            Span::styled(format!(" {} ", self.name), amber),
            Span::styled(format!("{}x{}", self.level.width, self.level.height), green),
            Span::styled(format!("  cursor {:.0},{:.0} h {h}  brush {:.0}  ", self.cursor.0, self.cursor.1, self.brush), dim),
            Span::styled(detail, green),
            Span::styled(if self.dirty { "  *unsaved" } else { "" }.to_string(), amber),
        ]);
        let msg = match &self.prompt {
            Some((f, text)) => Line::from(vec![
                Span::styled(format!(" {}: ", FIELDS[*f].1), amber),
                Span::styled(format!("{text}_"), green),
                if self.message.is_empty() {
                    Span::styled(format!("   {}", field_help(FIELDS[*f].0)), dim)
                } else {
                    Span::styled(format!("   {}", self.message), amber)
                },
            ]),
            None => Line::from(Span::styled(format!(" {}", self.message), dim)),
        };
        vec![Line::from(tools), info, msg]
    }
}

/// What a setting's text looks like, under the prompt.
fn field_help(f: Field) -> &'static str {
    match f {
        Field::Win | Field::Lose => "conditions split by ;  e.g. no mainbasepad 2; hold 1 50 40 4 90   (empty: the default)",
        Field::Build => "units split by ,  e.g. peon_tank, bridger   (empty: everything)",
        Field::Money => "players 1 2 5 [6]  e.g. 10000 5000 5000   (empty: 10000 each)",
        Field::Music => "a track, e.g. Al_Basrah.mp3   (empty: none)",
        Field::Night => "yes: fought at night   (empty: by day)",
    }
}

impl Editor {
    /// The map as it stands, for a test-play.
    pub fn map_for_play(&self) -> (Level, String, Option<String>) {
        (self.level.clone(), self.name.clone(), self.music.clone())
    }

    /// A setting as the prompt shows it.
    fn field_text(&self, f: Field) -> String {
        let conds = |v: &[crate::rules::Cond]| v.iter().map(|c| c.text()).collect::<Vec<_>>().join("; ");
        let default = self.level.rules.is_default();
        match f {
            Field::Win if default => String::new(),
            Field::Lose if default => String::new(),
            Field::Win => conds(&self.level.rules.win),
            Field::Lose => conds(&self.level.rules.lose),
            Field::Build => self.level.build.as_ref().map_or(String::new(), |b| b.join(", ")),
            Field::Money => self.level.money.map_or(String::new(), |m| {
                let s = format!("{} {} {}", m[0], m[1], m[2]);
                if m[3] == crate::world::START_MONEY { s } else { format!("{s} {}", m[3]) }
            }),
            Field::Music => self.music.clone().unwrap_or_default(),
            Field::Night => if self.level.night { "yes".into() } else { String::new() },
        }
    }

    /// Take the prompt's text into the map, or say what is wrong with it.
    fn set_field(&mut self, f: Field, text: &str) -> Result<(), String> {
        let list = |t: &str, sep: char| t.split(sep).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect::<Vec<_>>();
        match f {
            Field::Win | Field::Lose => {
                let (mut win, mut lose) = if self.level.rules.is_default() {
                    (Vec::new(), Vec::new())
                } else {
                    (self.level.rules.win.iter().map(|c| c.text()).collect(), self.level.rules.lose.iter().map(|c| c.text()).collect())
                };
                if f == Field::Win {
                    win = list(text, ';');
                } else {
                    lose = list(text, ';');
                }
                self.level.rules = crate::rules::Rules::parse(&win, &lose)?;
            }
            Field::Build => {
                let units = list(text, ',');
                if let Some(bad) = units.iter().find(|k| crate::balance::unit(k).is_none()) {
                    return Err(format!("{bad:?} is not a unit"));
                }
                self.level.build = (!units.is_empty()).then_some(units);
            }
            Field::Money => {
                let n: Vec<i32> = text.split_whitespace().map(|w| w.parse().map_err(|_| format!("{w:?} is not a number"))).collect::<Result<_, _>>()?;
                self.level.money = match n.as_slice() {
                    [] => None,
                    [a, b, c] => Some([*a, *b, *c, crate::world::START_MONEY]),
                    [a, b, c, d] => Some([*a, *b, *c, *d]),
                    _ => return Err("three amounts, for players 1, 2 and 5, or four, and 6".into()),
                };
            }
            Field::Music => {
                let t = text.trim();
                if !t.is_empty() && !crate::audio::MUSIC.iter().any(|(n, _)| *n == t) {
                    let all: Vec<&str> = crate::audio::MUSIC.iter().map(|(n, _)| *n).collect();
                    return Err(format!("not a track; the tracks: {}", all.join(" ")));
                }
                self.music = (!t.is_empty()).then(|| t.to_string());
            }
            Field::Night => {
                self.level.night = match text.trim() {
                    "yes" | "y" | "true" => true,
                    "" | "no" | "n" | "false" => false,
                    t => return Err(format!("{t:?}: yes, or empty for day")),
                };
            }
        }
        Ok(())
    }

    /// A key while the I prompt is open.
    fn prompt_key(&mut self, code: KeyCode) {
        let Some((f, mut text)) = self.prompt.take() else { return };
        match code {
            KeyCode::Esc => {
                self.message = format!("{} unchanged", FIELDS[f].1);
                return;
            }
            KeyCode::Enter | KeyCode::Tab | KeyCode::BackTab => {
                self.snapshot();
                match self.set_field(FIELDS[f].0, &text) {
                    Ok(()) => {
                        self.dirty = true;
                        self.rebuild = true;
                        self.message.clear();
                        if code == KeyCode::Enter {
                            self.message = format!("{} set", FIELDS[f].1);
                            return;
                        }
                        let n = if code == KeyCode::Tab { (f + 1) % FIELDS.len() } else { (f + FIELDS.len() - 1) % FIELDS.len() };
                        self.prompt = Some((n, self.field_text(FIELDS[n].0)));
                        return;
                    }
                    Err(e) => {
                        self.undo.pop();
                        self.message = format!("{}: {e}", FIELDS[f].1);
                    }
                }
            }
            KeyCode::Backspace => {
                text.pop();
                self.message.clear();
            }
            KeyCode::Char(c) => {
                text.push(c);
                self.message.clear();
            }
            _ => {}
        }
        self.prompt = Some((f, text));
    }
}

/// Remove object `i`, dropping links to it and renumbering the rest.
fn remove_object(level: &mut Level, i: usize) {
    level.objects.remove(i);
    for o in &mut level.objects {
        for links in [&mut o.links_a, &mut o.links_e, &mut o.links_r, &mut o.links_w] {
            links.retain(|&l| l != i);
            for l in links.iter_mut() {
                if *l > i {
                    *l -= 1;
                }
            }
        }
    }
}

pub fn run(terminal: &mut DefaultTerminal, mut ed: Editor) -> io::Result<Exit> {
    loop {
        if ed.rebuild {
            ed.rebuild_view();
        }
        let size: Rect = terminal.size()?.into();
        ed.game.tank.x = ed.cursor.0;
        ed.game.tank.y = ed.cursor.1;
        render::follow_camera(&mut ed.game, size);
        terminal.draw(|f| {
            let (map, hud) = render::split(f.area());
            render::draw_view(f, map, &ed.game);
            draw_overlay(f, map, &ed);
            f.render_widget(Paragraph::new(ed.status()).style(Style::default().bg(Color::Black)), hud);
        })?;
        if ed.applying {
            ed.apply();
        }
        if !event::poll(Duration::from_millis(33))? {
            continue;
        }
        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Release && ed.prompt.is_some() => ed.prompt_key(k.code),
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                let step = if ed.game.view == View::Strategy { 3.0 } else { 1.0 };
                match k.code {
                    KeyCode::Esc => {
                        if !ed.dirty || ed.quit_armed {
                            return Ok(Exit::Quit);
                        }
                        ed.quit_armed = true;
                        ed.message = "unsaved changes: Esc again to quit without saving, F2 to save".into();
                        continue;
                    }
                    KeyCode::F(2) => ed.save(),
                    KeyCode::F(5) | KeyCode::F(6) => {
                        let at = (k.code == KeyCode::F(6)).then_some(ed.cursor);
                        ed.chain = None;
                        ed.applying = false;
                        return Ok(Exit::Play(Box::new(ed), at));
                    }
                    KeyCode::Char('i') => {
                        ed.prompt = Some((0, ed.field_text(FIELDS[0].0)));
                        ed.message = String::new();
                    }
                    KeyCode::Char('l') => {
                        ed.road = (ed.road + 1) % ROADS.len();
                        ed.chain = None;
                        ed.message = format!("path tool draws roads: {}", ROADS[ed.road].1);
                    }
                    KeyCode::Char(c @ '1'..='8') => {
                        ed.tool = TOOLS[c as usize - '1' as usize].0;
                        ed.chain = None;
                    }
                    KeyCode::Left | KeyCode::Char('a') => ed.cursor.0 = (ed.cursor.0 - step).max(0.0),
                    KeyCode::Right | KeyCode::Char('d') => ed.cursor.0 = (ed.cursor.0 + step).min(ed.level.width as f32),
                    KeyCode::Up | KeyCode::Char('w') => ed.cursor.1 = (ed.cursor.1 - step).max(0.0),
                    KeyCode::Down | KeyCode::Char('s') => ed.cursor.1 = (ed.cursor.1 + step).min(ed.level.height as f32),
                    KeyCode::Char(' ') => {
                        ed.stroke_started = false;
                        ed.apply();
                    }
                    KeyCode::Char('[') => ed.brush = (ed.brush - 1.0).max(0.0),
                    KeyCode::Char(']') => ed.brush = (ed.brush + 1.0).min(12.0),
                    KeyCode::Char(',') | KeyCode::Char('.') => {
                        let d: i32 = if k.code == KeyCode::Char('.') { 1 } else { -1 };
                        if ed.tool == Tool::Place {
                            ed.object_pick = (ed.object_pick as i32 + d).rem_euclid(OBJECT_KINDS.len() as i32) as usize;
                        } else {
                            let n = ed.palette().len().max(1) as i32;
                            ed.tile_pick = (ed.tile_pick as i32 + d).rem_euclid(n) as usize;
                            ed.tool = Tool::Paint;
                        }
                    }
                    KeyCode::Char('t') => {
                        ed.tile_set = (ed.tile_set + 1) % TILE_SETS.len();
                        ed.tile_pick = 0;
                        ed.tool = Tool::Paint;
                    }
                    KeyCode::Char('f') => ed.flags = (ed.flags + 1) % FLAG_SETS.len(),
                    KeyCode::Char('p') => ed.owner = (ed.owner + 1) % OWNERS.len(),
                    KeyCode::Enter => {
                        ed.chain = None;
                        ed.message = "path chain ended".into();
                    }
                    KeyCode::Char('b') => {
                        // the path node under the cursor: a gap a bridger spans, or not
                        let c = ed.cursor;
                        match ed.level.objects.iter().position(|o| o.kind == "path_object" && (o.x - c.0).hypot(o.y - c.1) < 0.8) {
                            Some(n) => {
                                ed.snapshot();
                                let o = &mut ed.level.objects[n];
                                o.bridge = !o.bridge;
                                ed.message = if o.bridge { "bridge spot: a bridger spans this node" } else { "bridge spot removed" }.into();
                                ed.dirty = true;
                                ed.rebuild = true;
                            }
                            None => ed.message = "B marks a path node as a bridge spot: put the cursor on one".into(),
                        }
                    }
                    KeyCode::Char('u') => match ed.undo.pop() {
                        Some(l) => {
                            ed.level = l;
                            ed.chain = None;
                            ed.dirty = true;
                            ed.rebuild = true;
                            ed.message = "undone".into();
                        }
                        None => ed.message = "nothing to undo".into(),
                    },
                    KeyCode::Tab => ed.game.toggle_view(),
                    KeyCode::Char('+') | KeyCode::Char('=') => ed.game.zoom_by(1),
                    KeyCode::Char('-') => ed.game.zoom_by(-1),
                    _ => {}
                }
                ed.quit_armed = false;
            }
            Event::Mouse(m) => {
                let size: Rect = terminal.size()?.into();
                if let Some((x, y, _)) = render::screen_to_map(&ed.game, size, m.column, m.row) {
                    ed.cursor = (x.clamp(0.0, ed.level.width as f32), y.clamp(0.0, ed.level.height as f32));
                }
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        ed.stroke_started = false;
                        ed.apply();
                        ed.applying = true;
                    }
                    MouseEventKind::Up(MouseButton::Left) => ed.applying = false,
                    MouseEventKind::ScrollUp => ed.game.zoom_by(1),
                    MouseEventKind::ScrollDown => ed.game.zoom_by(-1),
                    _ => {}
                }
                ed.quit_armed = false;
            }
            _ => {}
        }
    }
}

/// Cursor, brush outline and the path network, over the map view.
fn draw_overlay(f: &mut ratatui::Frame, area: Rect, ed: &Editor) {
    let g = &ed.game;
    let buf = f.buffer_mut();
    let mut mark = |x: f32, y: f32, ch: char, color: Color| {
        if let Some(pos) = render::map_to_screen(g, area, x, y) {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_char(ch).set_fg(color);
            }
        }
    };
    // path links and nodes: ours white, the enemy's own red, the rival's blue
    // (enemy links that only run ours backwards are not drawn again)
    for (i, o) in ed.level.objects.iter().enumerate().filter(|(_, o)| o.kind == "path_object") {
        let enemy_own = o.links_e.iter().filter(|&&l| !ed.level.objects[l].links_a.contains(&i));
        let roads = o.links_a.iter().map(|&l| (l, Color::Rgb(235, 235, 225))).chain(enemy_own.map(|&l| (l, Color::Rgb(235, 70, 55)))).chain(o.links_r.iter().map(|&l| (l, Color::Rgb(110, 160, 245)))).chain(o.links_w.iter().map(|&l| (l, Color::Rgb(200, 190, 160))));
        for (l, color) in roads.collect::<Vec<_>>() {
            let m = &ed.level.objects[l];
            let steps = ((m.x - o.x).hypot(m.y - o.y) * 2.0).ceil().max(1.0) as usize;
            for s in 1..steps {
                let t = s as f32 / steps as f32;
                mark(o.x + (m.x - o.x) * t, o.y + (m.y - o.y) * t, '·', color);
            }
        }
    }
    for (i, o) in ed.level.objects.iter().enumerate().filter(|(_, o)| o.kind == "path_object") {
        let open = ed.chain == Some(i);
        mark(o.x, o.y, if open { '◆' } else { '◇' }, if open { Color::Rgb(255, 180, 40) } else { Color::Rgb(250, 250, 240) });
    }
    // brush outline
    if matches!(ed.tool, Tool::Raise | Tool::Lower | Tool::Smooth | Tool::Flatten | Tool::Paint | Tool::Erase) && ed.brush >= 1.0 {
        let n = (ed.brush * 12.0) as usize;
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            mark(ed.cursor.0 + a.cos() * ed.brush, ed.cursor.1 + a.sin() * ed.brush, '·', Color::Rgb(255, 180, 40));
        }
    }
    mark(ed.cursor.0, ed.cursor.1, '+', Color::Rgb(255, 180, 40));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erase_renumbers_path_links() {
        let mut level = Editor::blank(10, 10, "grassfull");
        for x in 0..3 {
            level.objects.push(ObjectRec { kind: "path_object".into(), x: x as f32, y: 1.0, z: 0.0, theta: 0.0, health: 100, player: 1, links_a: vec![], links_e: vec![], links_r: vec![], links_w: vec![], bridge: false });
        }
        level.objects[0].links_a = vec![1, 2];
        level.objects[2].links_e = vec![1, 0];
        remove_object(&mut level, 1);
        assert_eq!(level.objects[0].links_a, vec![1]);
        assert_eq!(level.objects[1].links_e, vec![0]);
    }

    #[test]
    fn a_painted_and_saved_map_loads_back() {
        let mut ed = Editor::new("unused.cmap", Editor::blank(20, 12, "grassfull"), "t".into(), None);
        ed.tool = Tool::Raise;
        ed.cursor = (10.0, 6.0);
        ed.apply();
        ed.tool = Tool::Paint;
        ed.tile_set = TILE_SETS.iter().position(|s| *s == "snow").unwrap();
        ed.stroke_started = false;
        ed.apply();
        ed.tool = Tool::Path;
        for x in [2.0, 6.0, 10.0] {
            ed.cursor = (x, 3.0);
            ed.stroke_started = false;
            ed.applying = false;
            ed.apply();
        }
        let text = cmap::export(&ed.level, "t", None, terrain::tile_name_for).unwrap();
        let (back, _, _) = cmap::parse(&text).unwrap();
        assert!(back.vert_heights[6 * 21 + 10] > 40, "raised");
        assert!(back.tile(10, 6).name.starts_with("snow"), "painted {}", back.tile(10, 6).name);
        let nodes: Vec<&ObjectRec> = back.objects.iter().filter(|o| o.kind == "path_object").collect();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].links_a, vec![1]);
        assert_eq!(nodes[2].links_e, vec![1]);
    }

    /// L switches the path tool to the rival's roads; I sets the map's rules,
    /// refusing what would not load.
    #[test]
    fn rival_roads_and_map_rules() {
        let mut ed = Editor::new("unused.cmap", Editor::blank(20, 12, "grassfull"), "t".into(), None);
        ed.tool = Tool::Path;
        ed.road = ROADS.iter().position(|r| r.0 == Road::Rival).unwrap();
        for x in [2.0, 8.0] {
            ed.cursor = (x, 3.0);
            ed.applying = false;
            ed.apply();
        }
        assert_eq!(ed.level.objects[0].links_r, vec![1]);
        assert!(ed.level.objects[0].links_a.is_empty() && ed.level.objects[1].links_e.is_empty());
        ed.set_field(Field::Win, "hold 1 5 5 3 60; no mainbasepad 2").unwrap();
        ed.set_field(Field::Build, "peon_tank, bridger").unwrap();
        ed.set_field(Field::Money, "9000 4000 3000").unwrap();
        ed.set_field(Field::Music, "Greece.mp3").unwrap();
        assert!(ed.set_field(Field::Win, "hold 1 5").is_err());
        assert!(ed.set_field(Field::Build, "tractor").is_err());
        assert!(ed.set_field(Field::Money, "9000").is_err());
        assert!(ed.set_field(Field::Music, "Nope.mp3").is_err());
        let text = cmap::export(&ed.level, "t", ed.music.as_deref(), terrain::tile_name_for).unwrap();
        let (back, _, music) = cmap::parse(&text).unwrap();
        assert_eq!(ed.field_text(Field::Win), "hold 1 5 5 3 60; no mainbasepad 2");
        assert_eq!(back.rules.win.len(), 2);
        assert_eq!(back.build, Some(vec!["peon_tank".to_string(), "bridger".to_string()]));
        assert_eq!(back.money, Some([9000, 4000, 3000, crate::world::START_MONEY]));
        assert_eq!(music.as_deref(), Some("Greece.mp3"));
        assert_eq!(back.objects[0].links_r, vec![1]);
    }
}
