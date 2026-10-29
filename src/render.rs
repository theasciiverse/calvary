//! Drawing.
//!
//! Tactical view: one map cell is 2 columns by 1 row (about square at the usual
//! 2:1 cell aspect), so each map cell holds 4x4 Braille dots. The background is the
//! hill-shaded tile colour; dots carry contour lines, water shimmer and everything
//! that moves. Strategy view: the whole map in half-block pixels.

use crate::game::{Game, ShotKind, View};
use crate::sprites::{self, PixelLayer, Pose};
use crate::world::{flies, Class, Outcome, Team, BUILDABLE, BUILD_KEYS};
use crate::terrain::{Rgb, Terrain};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use ratatui::Frame;

/// Tactical zoom levels as columns per map cell; rows per map cell is half that,
/// which keeps cells about square at the usual 2:1 character aspect.
pub const ZOOMS: [f32; 4] = [1.0, 2.0, 4.0, 6.0];
pub const DEFAULT_ZOOM: usize = 1;
/// The camera holds still while the tank stays this far (fraction of the view)
/// from every edge, and scrolls only when it gets closer.
const EDGE_MARGIN: f32 = 0.3;
/// At full speed the camera follows a point this far ahead of the tank
/// (fraction of the view), so there is more to see in the direction of travel.
/// The dead zone holds that point at its leading edge (0.7), so the tank
/// itself settles near 0.7 - 0.35 = 0.35 of the view: two thirds ahead.
const LOOK_AHEAD: f32 = 0.35;
/// How quickly the look-ahead point catches up, per frame.
const LOOK_EASE: f32 = 0.06;
/// Height difference between contour lines at the default zoom, in map cells.
/// Zooming out spreads them so the ground does not turn into solid hatching.
const CONTOUR: f32 = 0.5;

const GREEN: Color = Color::Rgb(80, 220, 120);
const AMBER: Color = Color::Rgb(255, 180, 40);
const DIM: Color = Color::Rgb(70, 110, 80);

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

fn scale(c: Rgb, f: f32) -> Rgb {
    let s = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    (s(c.0), s(c.1), s(c.2))
}

/// Braille bit for dot (dx, dy) within a cell, dx 0..2, dy 0..4.
const fn braille_bit(dx: usize, dy: usize) -> u8 {
    [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]][dx][dy]
}

/// One frame of the tactical view as Braille dots over coloured cells.
struct DotCanvas {
    area: Rect,
    bits: Vec<u8>,
    fg: Vec<Rgb>,
    prio: Vec<u8>,
    bg: Vec<Rgb>,
    /// Map position of the top-left corner of the area, and map cells per dot.
    origin: (f32, f32),
    dot_w: f32,
    dot_h: f32,
}

impl DotCanvas {
    fn new(area: Rect, origin: (f32, f32), cols_per_cell: f32) -> DotCanvas {
        let n = area.width as usize * area.height as usize;
        DotCanvas {
            area,
            bits: vec![0; n],
            fg: vec![(0, 0, 0); n],
            prio: vec![0; n],
            bg: vec![(0, 0, 0); n],
            origin,
            dot_w: 1.0 / (cols_per_cell * 2.0),
            dot_h: 1.0 / (cols_per_cell / 2.0 * 4.0),
        }
    }

    fn dot_to_map(&self, dx: usize, dy: usize) -> (f32, f32) {
        (self.origin.0 + (dx as f32 + 0.5) * self.dot_w, self.origin.1 + (dy as f32 + 0.5) * self.dot_h)
    }

    /// Light the dot under map position (x, y). A higher priority takes over the
    /// cell's foreground colour, since a terminal cell has only one.
    fn plot(&mut self, x: f32, y: f32, color: Rgb, prio: u8) {
        let dx = ((x - self.origin.0) / self.dot_w).floor();
        let dy = ((y - self.origin.1) / self.dot_h).floor();
        if dx < 0.0 || dy < 0.0 {
            return;
        }
        let (dx, dy) = (dx as usize, dy as usize);
        let (col, row) = (dx / 2, dy / 4);
        if col >= self.area.width as usize || row >= self.area.height as usize {
            return;
        }
        let i = row * self.area.width as usize + col;
        if prio >= self.prio[i] {
            if prio > self.prio[i] {
                // a new layer starts with its own dots only
                self.bits[i] = 0;
            }
            self.prio[i] = prio;
            self.fg[i] = color;
            self.bits[i] |= braille_bit(dx % 2, dy % 4);
        }
    }

    fn line(&mut self, from: (f32, f32), to: (f32, f32), color: Rgb, prio: u8) {
        let len = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let steps = (len / (self.dot_w * 0.7)).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            self.plot(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, color, prio);
        }
    }

    fn flush(&self, buf: &mut Buffer) {
        let w = self.area.width as usize;
        for row in 0..self.area.height as usize {
            for col in 0..w {
                let i = row * w + col;
                let ch = char::from_u32(0x2800 + self.bits[i] as u32).unwrap_or(' ');
                if let Some(cell) = buf.cell_mut((self.area.x + col as u16, self.area.y + row as u16)) {
                    cell.set_char(ch).set_fg(rgb(self.fg[i])).set_bg(rgb(self.bg[i]));
                }
            }
        }
    }
}

fn hash(x: i32, y: i32, t: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (t as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h.wrapping_mul(0x2c1b_3c6d)
}

fn draw_terrain(c: &mut DotCanvas, t: &Terrain, time: f32, contour: f32) {
    let w = c.area.width as usize;
    for row in 0..c.area.height as usize {
        for col in 0..w {
            let (mx, my) = c.dot_to_map(col * 2 + 1, row * 4 + 2);
            c.bg[row * w + col] = t.shaded(mx - c.dot_w * 0.5, my - c.dot_h * 0.5);
        }
    }
    let shimmer = (time * 3.0) as i32;
    for dy in 0..c.area.height as usize * 4 {
        for dx in 0..w * 2 {
            let (x, y) = c.dot_to_map(dx, dy);
            let Some(cell) = t.cell_at(x, y) else { continue };
            let band = |x: f32, y: f32| (t.height(x, y) / contour).floor() as i32;
            let b = band(x, y);
            if b != band(x + c.dot_w, y) || b != band(x, y + c.dot_h) {
                c.plot(x, y, scale(cell.color, 0.55), 1);
            } else if cell.water && hash((x / c.dot_w).floor() as i32, (y / c.dot_h).floor() as i32, shimmer) % 23 == 0 {
                c.plot(x, y, scale(cell.color, 1.5), 1);
            }
        }
    }
}

fn draw_tactical(f: &mut Frame, area: Rect, g: &Game) {
    let origin = g.camera.unwrap_or((g.tank.x, g.tank.y));
    let cols = ZOOMS[g.zoom];
    let mut c = DotCanvas::new(area, origin, cols);
    draw_terrain(&mut c, &g.terrain, g.time, CONTOUR * (ZOOMS[DEFAULT_ZOOM] / cols).max(0.5));
    for p in &g.particles {
        let k = p.life / p.max_life;
        let color = if p.hot {
            (255, (90.0 + 165.0 * k) as u8, (40.0 * k) as u8)
        } else {
            scale((150, 150, 150), 0.5 + k * 0.5)
        };
        c.plot(p.x, p.y, color, 3);
    }
    for s in &g.shots {
        match s.kind {
            ShotKind::Shell => c.plot(s.x, s.y, bolt_color(s.weapon), 7),
            ShotKind::Bullet => c.plot(s.x, s.y, (255, 230, 120), 7),
            ShotKind::Missile => {
                let len = (s.vx * s.vx + s.vy * s.vy).sqrt();
                let tail = (s.x - s.vx / len * 0.8, s.y - s.vy / len * 0.8);
                c.line(tail, (s.x, s.y), bolt_color(s.weapon), 7);
            }
        }
    }
    for b in &g.world.bolts {
        let color = bolt_color(b.weapon);
        if b.speed == 0.0 {
            c.line(b.from, b.to, color, 7);
        } else {
            let (dx, dy) = (b.to.0 - b.x, b.to.1 - b.y);
            let d = dx.hypot(dy).max(1e-3);
            c.line((b.x - dx / d * 0.35, b.y - dy / d * 0.35), (b.x, b.y), color, 7);
        }
    }
    c.flush(f.buffer_mut());

    let mut layer = PixelLayer::new(area, origin, cols);
    let w = &g.world;
    for (_, n) in w.nodes.iter().enumerate().filter(|&(k, _)| w.gap(k)) {
        sprites::paint_gap(&mut layer, n.x, n.y, n.dir, g.time);
    }
    for (team, x, y, r, secs, held) in w.sites() {
        sprites::paint_site(&mut layer, x, y, r, (held / secs).min(1.0), team);
    }
    // scenery first, then structures, then vehicles, aircraft on top
    for pass in 0..4 {
        for e in &w.entities {
            let p = match e.class {
                Class::Prop | Class::Bridge | Class::Pad => 0,
                Class::Vehicle if flies(&e.kind) => 3,
                Class::Vehicle | Class::EnemyTank | Class::PlayerTank => 2,
                Class::Plane => 3,
                _ => 1,
            };
            if p == pass && (e.alive || p == 1) {
                sprites::paint_entity(&mut layer, e, g.time);
            }
        }
    }
    for e in &w.entities {
        if e.class != Class::PlayerTank {
            sprites::paint_health(&mut layer, e);
        }
    }
    for &i in &g.selected {
        let e = &w.entities[i];
        sprites::paint_selection(&mut layer, e);
        if let crate::world::Order::Move(x, y) = e.order {
            sprites::paint_waypoint(&mut layer, x, y);
        }
    }
    let t = &g.tank;
    let hit = w.player_tank.map_or(0.0, |i| w.entities[i].flash);
    let pose = Pose { x: t.x, y: t.y, heading: t.heading, aim: t.aim(), odometer: t.odometer, muzzle: t.flash, hit };
    if !g.hide_tank {
        sprites::paint_vehicle(&mut layer, sprites::SUPERTANK, sprites::PLAYER, pose);
    }
    layer.flush(f.buffer_mut(), &g.terrain);
    if let Some(pos) = g.crosshair {
        if let Some(cell) = f.buffer_mut().cell_mut(pos) {
            cell.set_char('╋').set_fg(AMBER);
        }
    }
    draw_messages(f, area, g);
}

fn bolt_color(weapon: &str) -> Rgb {
    match weapon {
        "bolt" | "electric_beam" | "beam" | "plasma" => (130, 220, 255),
        "acid" => (150, 255, 90),
        "vortex_missile" => (200, 120, 255),
        "nuke_missile" => (255, 255, 200),
        "super_mortar" | "napalm" => (255, 90, 40),
        "chain_gun" => (255, 230, 120),
        w if w.contains("missile") || w.contains("rocket") => (255, 130, 50),
        _ => (255, 250, 230),
    }
}

fn team_color(team: Team) -> Color {
    match team {
        Team::Player => AMBER,
        Team::Enemy => Color::Rgb(235, 70, 55),
        _ => Color::Rgb(205, 205, 200),
    }
}

fn draw_messages(f: &mut Frame, area: Rect, g: &Game) {
    let lines: Vec<Line> = g
        .world
        .recent_messages()
        .map(|m| Line::from(Span::styled(format!(" {m} "), Style::default().fg(GREEN).bg(Color::Black))))
        .collect();
    let h = (lines.len() as u16).min(area.height);
    if h > 0 {
        f.render_widget(Paragraph::new(lines), Rect { height: h, ..area });
    }
}

/// Strategy view scale (map cells per half-block pixel) and the map offset
/// that centres the map in `area`.
fn strategy_fit(area: Rect, t: &Terrain) -> (f32, f32, f32) {
    let s = (t.w as f32 / area.width as f32).max(t.h as f32 / (area.height as f32 * 2.0));
    (s, (area.width as f32 * s - t.w as f32) / 2.0, (area.height as f32 * 2.0 * s - t.h as f32) / 2.0)
}

/// Map position under screen cell (col, row) for a screen of `size`, in
/// whichever view is showing, and how far a click there may reach (map cells).
pub fn screen_to_map(g: &Game, size: Rect, col: u16, row: u16) -> Option<(f32, f32, f32)> {
    let (map, _) = split(size);
    if row >= map.y + map.height || col >= map.x + map.width {
        return None;
    }
    let (c, r) = ((col - map.x) as f32 + 0.5, (row - map.y) as f32 + 0.5);
    match g.view {
        // the follow view is a perspective; clicks do not map onto the ground
        View::Chase => None,
        View::Tactical => {
            let cols = ZOOMS[g.zoom];
            let origin = g.camera?;
            Some((origin.0 + c / cols, origin.1 + r * 2.0 / cols, (2.0 / cols).max(1.5)))
        }
        View::Strategy => {
            let (s, ox, oy) = strategy_fit(map, &g.terrain);
            Some((c * s - ox, r * 2.0 * s - oy, (s * 3.0).max(1.5)))
        }
    }
}

fn draw_strategy(f: &mut Frame, area: Rect, g: &Game) {
    let t = &g.terrain;
    let (s, ox, oy) = strategy_fit(area, t);
    let px = |col: u16, sub: u16| ((col as f32 + 0.5) * s - ox, ((sub as f32) + 0.5) * s - oy);
    let buf = f.buffer_mut();
    for row in 0..area.height {
        for col in 0..area.width {
            let (x, y_top) = px(col, row * 2);
            let (_, y_bot) = px(col, row * 2 + 1);
            let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) else { continue };
            if t.cell_at(x, y_top).is_none() && t.cell_at(x, y_bot).is_none() {
                cell.set_char(' ').set_bg(Color::Black);
            } else {
                cell.set_char('▀').set_fg(rgb(t.shaded(x, y_top))).set_bg(rgb(t.shaded(x, y_bot)));
            }
        }
    }
    let to_cell = |x: f32, y: f32| {
        let (c, r) = ((x + ox) / s, (y + oy) / s / 2.0);
        (c >= 0.0 && r >= 0.0 && (c as u16) < area.width && (r as u16) < area.height).then(|| (area.x + c as u16, area.y + r as u16))
    };
    let w = &g.world;
    // ground paths, as the original radar drew them; the player's chosen
    // branches bright, the others dim
    for (i, n) in w.nodes.iter().enumerate() {
        for (k, &l) in n.links_a.iter().enumerate() {
            let chosen = n.links_a.len() == 1 || w.route_choice.get(&i).copied().unwrap_or(0) == k;
            let m = &w.nodes[l];
            let steps = ((m.x - n.x).hypot(m.y - n.y) / (s * 0.8)).ceil().max(1.0) as usize;
            for st in 0..=steps {
                let t = st as f32 / steps as f32;
                if let Some(pos) = to_cell(n.x + (m.x - n.x) * t, n.y + (m.y - n.y) * t) {
                    if let Some(cell) = buf.cell_mut(pos) {
                        cell.set_char('·').set_fg(if chosen { Color::Rgb(235, 235, 225) } else { Color::Rgb(120, 120, 115) });
                    }
                }
            }
        }
    }
    for n in w.nodes.iter().filter(|n| n.links_a.len() > 1) {
        if let Some(pos) = to_cell(n.x, n.y) {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_char('◇').set_fg(Color::Rgb(250, 250, 240));
            }
        }
    }
    // sites to hold: a ring of dots, the part held so far bright
    for (team, x, y, r, secs, held) in w.sites() {
        let steps = (r * 8.0) as usize + 8;
        for k in 0..steps {
            let a = k as f32 / steps as f32 * std::f32::consts::TAU;
            if let Some(pos) = to_cell(x + a.cos() * r, y + a.sin() * r) {
                if let Some(cell) = buf.cell_mut(pos) {
                    let lit = (k as f32) < steps as f32 * held / secs;
                    cell.set_char(if lit { '●' } else { '·' }).set_fg(team_color(team));
                }
            }
        }
    }
    // gaps waiting for a bridger
    for (_, n) in w.nodes.iter().enumerate().filter(|&(k, _)| w.gap(k)) {
        if let Some(pos) = to_cell(n.x, n.y) {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_char('╳').set_fg(AMBER).set_bg(Color::Black);
            }
        }
    }
    for e in w.entities.iter().filter(|e| e.alive && e.class != Class::Prop) {
        let glyph = match e.class {
            Class::Hq => 'H',
            Class::Garage => 'G',
            Class::Airbase => 'A',
            Class::Bank => '$',
            Class::Tower => 'T',
            Class::EnemyTank | Class::PlayerTank => '◆',
            Class::Plane => '+',
            Class::Bridge => '=',
            Class::Pad => 'P',
            Class::Vehicle if flies(&e.kind) => '+',
            _ => '•',
        };
        if let Some(pos) = to_cell(e.x, e.y) {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_char(glyph).set_fg(team_color(e.team)).set_bg(Color::Black);
            }
        }
    }
    for &i in &g.selected {
        let e = &w.entities[i];
        if let Some(pos) = to_cell(e.x, e.y) {
            if let Some(cell) = buf.cell_mut(pos) {
                cell.set_fg(Color::Black).set_bg(Color::Rgb(240, 240, 230));
            }
        }
    }
    if let Some(pos) = to_cell(g.tank.x, g.tank.y) {
        if let Some(cell) = buf.cell_mut(pos) {
            cell.set_char('◆').set_fg(AMBER).set_bg(Color::Black);
        }
    }
    draw_messages(f, area, g);
}

fn draw_hud(f: &mut Frame, area: Rect, g: &Game) {
    let t = &g.tank;
    let w = &g.world;
    let view = match g.view {
        View::Tactical => ["far", "normal", "close", "closest"][g.zoom],
        View::Strategy => "map",
        View::Chase => "follow",
    };
    let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(DIM));
    let value = |s: String| Span::styled(s, Style::default().fg(GREEN));
    let hp = g.tank_health();
    let hp_color = if hp > 0.5 { GREEN } else if hp > 0.25 { AMBER } else { Color::Rgb(235, 70, 55) };
    let mut line1 = Line::from(vec![
        Span::styled(" CALVARY ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)),
        // a recording film blinks its dot, like a camera
        if g.recording {
            let dot = if (g.time * 2.0) as i32 % 2 == 0 { "●" } else { " " };
            Span::styled(format!(" {dot} REC "), Style::default().fg(Color::Rgb(235, 70, 55)).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("")
        },
        label("  $"),
        Span::styled(format!("{}", w.money[0]), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        label("  armor "),
        Span::styled(format!("{:3.0}%", hp * 100.0), Style::default().fg(hp_color)),
        label("  tanks "),
        value(format!("{}", w.lives)),
        label("  lvl "),
        value(format!("{}", t.level)),
        label(" pts "),
        value(match crate::balance::balance().upgrade_kill_ratio.get(w.upgrade_level()) {
            Some(need) => format!("{:.0}/{need:.0}", w.kill_ratio()),
            None => format!("{:.0}", w.kill_ratio()),
        }),
        label("  gear "),
        Span::styled(t.gear_label(), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        label("  spd "),
        value(format!("{:3.1}", t.speed)),
        label("  gun "),
        value(format!("{}", t.main_ammo)),
        label("  mg "),
        value(format!("{}", t.chain_ammo)),
        label("  msl "),
        value(format!("{}", t.missiles)),
        label("  map "),
        value(format!("{} ({view})", g.terrain.name)),
        if g.selected.is_empty() {
            Span::raw("")
        } else {
            Span::styled(format!("  SEL {}", g.selected.len()), Style::default().fg(Color::Black).bg(Color::Rgb(240, 240, 230)))
        },
        if t.blocked { Span::styled("  BLOCKED", Style::default().fg(AMBER)) } else { Span::raw("") },
        match g.lock_target() {
            Some(name) => Span::styled(format!("  LOCK {}", name.replace('_', " ")), Style::default().fg(Color::Rgb(235, 70, 55)).add_modifier(Modifier::BOLD)),
            None if g.lock => Span::styled("  LOCK -", Style::default().fg(DIM)),
            None => Span::raw(""),
        },
    ]);
    for (team, _, _, _, secs, held) in w.sites() {
        let (who, color) = if team == Team::Player { ("HOLD", AMBER) } else { ("ENEMY HOLD", team_color(Team::Enemy)) };
        let text = if held >= secs { format!("  {who} done") } else { format!("  {who} {:.0}/{secs:.0}s", held.floor()) };
        line1.spans.push(Span::styled(text, Style::default().fg(color).add_modifier(Modifier::BOLD)));
    }
    let slots = g.build_slots();
    let mut build = if slots.is_empty() { vec![label(" no factory: supertank only")] } else { vec![label(" BUILD ")] };
    for (i, kind) in BUILDABLE.iter().enumerate().filter(|(i, _)| slots.contains(i)) {
        let cost = crate::balance::unit(kind).map_or(0, |s| s.cost);
        let afford = w.money[0] >= cost;
        let cursor = g.pad.as_ref().is_some_and(|p| p.1 == i);
        let short = match *kind {
            "electric_car" => "elec",
            "rocket_tank" => "rocket",
            "tank_buster" => "buster",
            "engineer" => "eng",
            "bomb_truck" => "bomb",
            "helicopter" => "heli",
            "peon_tank" => "peon",
            "cobra_tank" => "cobra",
            k => k,
        };
        let mark = if cursor { Style::default().fg(Color::Black).bg(if afford { AMBER } else { DIM }) } else { Style::default() };
        build.push(Span::styled(format!(" {}", BUILD_KEYS[i]), Style::default().fg(if afford { AMBER } else { DIM })));
        build.push(Span::styled(format!(" {short} {cost} "), if cursor { mark } else { Style::default().fg(if afford { GREEN } else { DIM }) }));
    }
    let queued = w.queue_len(Team::Player);
    if queued > 0 {
        build.push(label(&format!("  queued {queued}")));
    }
    let line3 = match &g.pad {
        Some((name, _)) => Line::from(vec![
            Span::styled(format!(" {name} "), Style::default().fg(Color::Black).bg(GREEN)),
            label("  L stick drive (click centre)  R stick aim (click lock)  RT gun  LT mg  X rocket  LB/RB zoom  D-pad+A build  B route  Y map"),
        ]),
        None => Line::from(label(" W/S gear  A/D turn  Q/E turret  C centre  mouse aim  LMB gun  RMB mg  SPACE gun  F mg  ENTER rocket  M follow  TAB map  F1 help")),
    };
    f.render_widget(Paragraph::new(vec![line1, Line::from(build), line3]).style(Style::default().bg(Color::Black)), area);

    if w.outcome != Outcome::Playing {
        let (title, color) = match w.outcome {
            Outcome::Won => (" VICTORY ", AMBER),
            _ => (" DEFEAT ", Color::Rgb(235, 70, 55)),
        };
        let text = if w.verdict.is_empty() { "The battle is decided." } else { w.verdict.as_str() };
        let full = f.area();
        let (bw, bh) = (44.min(full.width), 5.min(full.height));
        let r = Rect { x: full.x + (full.width - bw) / 2, y: full.y + (full.height - bh) / 2, width: bw, height: bh };
        let block = Block::bordered()
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(color))
            .title(Span::styled(title, Style::default().fg(Color::Black).bg(color).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(Color::Black));
        let body = vec![Line::from(Span::styled(format!(" {text}"), Style::default().fg(GREEN))), Line::from(label(if g.campaign { " Esc: on" } else { " [ ] next level, Esc quit" }))];
        f.render_widget(Clear, r);
        f.render_widget(Paragraph::new(body).block(block), r);
    }
}

/// The map view alone (tactical or strategy), for the editor.
pub fn draw_view(f: &mut Frame, area: Rect, g: &Game) {
    match g.view {
        View::Tactical => draw_tactical(f, area, g),
        View::Strategy => draw_strategy(f, area, g),
        View::Chase => {
            crate::chase::draw(f.buffer_mut(), area, g);
            draw_messages(f, area, g);
        }
    }
}

/// Screen cell showing map position (x, y) in a map `area`, if visible.
pub fn map_to_screen(g: &Game, area: Rect, x: f32, y: f32) -> Option<(u16, u16)> {
    let (c, r) = match g.view {
        View::Chase => return None,
        View::Tactical => {
            let cols = ZOOMS[g.zoom];
            let o = g.camera?;
            ((x - o.0) * cols, (y - o.1) * cols / 2.0)
        }
        View::Strategy => {
            let (s, ox, oy) = strategy_fit(area, &g.terrain);
            ((x + ox) / s, (y + oy) / s / 2.0)
        }
    };
    (c >= 0.0 && r >= 0.0 && c < area.width as f32 && r < area.height as f32).then(|| (area.x + c as u16, area.y + r as u16))
}

pub fn split(full: Rect) -> (Rect, Rect) {
    let hud_h = 3.min(full.height);
    let map = Rect { height: full.height - hud_h, ..full };
    let hud = Rect { y: full.y + full.height - hud_h, height: hud_h, ..full };
    (map, hud)
}

/// Move the tactical camera for a screen of `size`. It stays put while the tank is
/// inside the middle of the view, pushes when the tank nears an edge, stops at the
/// map border, and always lands on whole terminal cells: sliding by fractions
/// re-samples the terrain colours every frame and makes the ground shimmer.
pub fn follow_camera(g: &mut Game, size: Rect) {
    let (map, _) = split(size);
    let cols = ZOOMS[g.zoom];
    let rows = cols / 2.0;
    let (vw, vh) = (map.width as f32 / cols, map.height as f32 / rows);
    // look ahead along the hull, scaled by speed (behind when reversing)
    let k = g.tank.speed / crate::tank::TOP_SPEED;
    let want = (g.tank.heading.cos() * k * vw * LOOK_AHEAD, g.tank.heading.sin() * k * vh * LOOK_AHEAD);
    if g.camera.is_none() {
        g.look = want;
    }
    g.look.0 += (want.0 - g.look.0) * LOOK_EASE;
    g.look.1 += (want.1 - g.look.1) * LOOK_EASE;
    let (t, terrain) = (&g.tank, &g.terrain);
    let (fx, fy) = (t.x + g.look.0, t.y + g.look.1);
    let (mut x, mut y) = g.camera.unwrap_or((fx - vw / 2.0, fy - vh / 2.0));
    let follow = |cam: &mut f32, pos: f32, view: f32| {
        let margin = view * EDGE_MARGIN;
        if pos < *cam + margin {
            *cam = pos - margin;
        } else if pos > *cam + view - margin {
            *cam = pos - view + margin;
        }
    };
    follow(&mut x, fx, vw);
    follow(&mut y, fy, vh);
    let clamp = |cam: f32, view: f32, size: usize| {
        if view >= size as f32 {
            (size as f32 - view) / 2.0
        } else {
            cam.clamp(0.0, size as f32 - view)
        }
    };
    x = clamp(x, vw, terrain.w);
    y = clamp(y, vh, terrain.h);
    let snapped = ((x * cols).round() / cols, (y * rows).round() / rows);
    if g.camera != Some(snapped) {
        log::trace!("camera {:?}", snapped);
    }
    g.camera = Some(snapped);
}

fn draw_drag(f: &mut Frame, g: &Game) {
    let Some((a, b)) = g.drag else { return };
    if a.0.abs_diff(b.0) < 2 && a.1.abs_diff(b.1) < 1 {
        return;
    }
    let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
    let r = Rect { x: x0, y: y0, width: x1 - x0 + 1, height: y1 - y0 + 1 }.intersection(f.area());
    f.render_widget(Block::bordered().border_style(Style::default().fg(Color::Rgb(240, 240, 230))), r);
}

pub fn draw(f: &mut Frame, g: &Game) {
    let (map, hud) = split(f.area());
    match g.view {
        View::Tactical => draw_tactical(f, map, g),
        View::Strategy => draw_strategy(f, map, g),
        View::Chase => {
            crate::chase::draw(f.buffer_mut(), map, g);
            if let Some(pos) = g.crosshair {
                if let Some(cell) = f.buffer_mut().cell_mut(pos) {
                    cell.set_char('╋').set_fg(AMBER);
                }
            }
            draw_messages(f, map, g);
        }
    }
    draw_hud(f, hud, g);
    draw_drag(f, g);
    if g.help {
        draw_help(f, f.area());
    }
}

const HELP: &[(&str, &str)] = &[
    ("", "SUPERTANK"),
    ("W / Up", "shift up a gear (R N 1 2 3); the tank keeps that speed"),
    ("S / Down", "shift down a gear; below N is reverse"),
    ("A D / Left Right", "turn the hull (hold)"),
    ("Q E", "turn the turret (hold)"),
    ("C", "turret back to straight ahead"),
    ("Space", "main gun (120mm, later acid, then napalm)"),
    ("F", "machine gun (hold)"),
    ("Enter", "heavy rocket (later vortex, then nuke); you carry two"),
    ("", ""),
    ("", "VIEW"),
    ("M", "top-down view / follow mode (behind the tank)"),
    ("Tab", "whole map, and back"),
    ("+ / - / wheel", "zoom the tactical view in / out"),
    ("", ""),
    ("", "UNITS"),
    ("Mouse", "point to aim the turret; shells land where you point"),
    ("Left click", "fire; on your unit selects it, on a junction switches it"),
    ("Drag", "box-select units"),
    ("Right click", "selection: go there or attack; none: machine gun"),
    ("U , .", "select units near the supertank / cycle one by one"),
    ("G X R Z", "rally on me, hold, back to route, clear selection"),
    ("", ""),
    ("", "GAMEPAD"),
    ("Sticks", "left drives, right aims; click left to centre, right to lock"),
    ("RT LT X", "main gun, machine gun, rocket"),
    ("LB RB", "zoom out / in"),
    ("D-pad", "left/right pick a unit, A builds; up rally, down hold"),
    ("B Y", "switch nearest junction, map view"),
    ("Start Back", "help, top-down / follow mode"),
    ("", ""),
    ("", "COMMAND"),
    ("1 - 8, V", "build a unit at your garage, V a bridger (costs money)"),
    ("9 0 B", "build a helicopter, jet or bomber at your airbase"),
    ("J", "switch the route at the junction nearest your supertank"),
    ("Mouse click", "switch the route at a junction (◇ on the map)"),
    ("T", "turret lock: track the nearest enemy in gun range"),
    ("", "Units drive the paths to the enemy base; money planes fly bank crates home."),
    ("", "Engineers capture banks and towers; bridgers span gaps (╳ on the map)."),
    ("", "Kills earn upgrades for your next supertank. Park at your HQ to rearm."),
    ("", "Destroy the enemy HQ to win."),
    ("[ ]", "previous / next level"),
    ("F1 / H / ?", "this help (the game pauses)"),
    ("F9 / F8", "next music track / sound on or off (start with --mute)"),
    ("F12", "record a film of the battle (again to stop; saved in films/)"),
    ("Esc", "close help, or quit"),
    ("", ""),
    ("", "Rock edges, deep water and steep slopes stop the tank."),
];

pub fn draw_help(f: &mut Frame, full: Rect) {
    let lines: Vec<Line> = HELP
        .iter()
        .map(|&(key, text)| {
            if key.is_empty() {
                let heading = text.chars().all(|c| c.is_uppercase() || c == ' ') && !text.is_empty();
                let style = if heading { Style::default().fg(AMBER).add_modifier(Modifier::BOLD) } else { Style::default().fg(DIM) };
                Line::from(Span::styled(format!(" {text}"), style))
            } else {
                Line::from(vec![
                    Span::styled(format!(" {key:<18}"), Style::default().fg(AMBER)),
                    Span::styled(text, Style::default().fg(GREEN)),
                ])
            }
        })
        .collect();
    let w = 78.min(full.width);
    let h = (lines.len() as u16 + 2).min(full.height);
    let area = Rect { x: full.x + (full.width - w) / 2, y: full.y + (full.height - h) / 2, width: w, height: h };
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(GREEN))
        .title(Span::styled(" CALVARY: CONTROLS ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(" F1 / H / Esc to close, pad: Start, A or B ", Style::default().fg(DIM)))
        .style(Style::default().bg(Color::Black));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No panics at any terminal size, in either view, with or without help.
    #[test]
    fn renders_at_awkward_sizes() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut g = Game::new(0).unwrap();
        for (w, h) in [(1, 1), (2, 3), (10, 3), (10, 4), (30, 8), (79, 24), (300, 100)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            for view in [View::Tactical, View::Strategy, View::Chase] {
                for help in [false, true] {
                    for zoom in 0..ZOOMS.len() {
                        g.view = view;
                        g.help = help;
                        g.zoom = zoom;
                        g.camera = None;
                        follow_camera(&mut g, Rect::new(0, 0, w, h));
                        term.draw(|f| draw(f, &g)).unwrap();
                        g.world.outcome = Outcome::Won;
                        term.draw(|f| draw(f, &g)).unwrap();
                        g.world.outcome = Outcome::Playing;
                        let _ = screen_to_map(&g, Rect::new(0, 0, w, h), w / 2, h / 2);
                    }
                }
            }
        }
    }

    /// Driving at full speed, the camera leads: the tank sits toward the
    /// trailing side, with more of the map visible ahead than behind. (South
    /// on the test map: it is 150 cells tall, the view about 43, so there is
    /// room to scroll.)
    #[test]
    fn camera_looks_ahead_of_a_moving_tank() {
        let mut g = Game::new(0).unwrap();
        let size = Rect::new(0, 0, 160, 45);
        let (map, _) = split(size);
        g.tank.x = 50.0;
        g.tank.y = 20.0;
        g.tank.heading = std::f32::consts::FRAC_PI_2;
        g.tank.speed = crate::tank::TOP_SPEED;
        g.camera = None;
        follow_camera(&mut g, size);
        for _ in 0..450 {
            g.tank.y += crate::tank::TOP_SPEED / 30.0;
            follow_camera(&mut g, size);
        }
        let (_, row) = map_to_screen(&g, map, g.tank.x, g.tank.y).unwrap();
        assert!((row as f32) < map.height as f32 * 0.42, "tank at row {row} of {}", map.height);
        // standing still, the look-ahead fades back out
        g.tank.speed = 0.0;
        for _ in 0..200 {
            follow_camera(&mut g, size);
        }
        assert!(g.look.1.abs() < 0.1, "look {:?}", g.look);
    }

    /// Clicking the screen cell a junction is drawn in switches that junction.
    #[test]
    fn strategy_click_hits_the_drawn_junction() {
        let mut g = Game::new(0).unwrap();
        g.view = View::Strategy;
        let size = Rect::new(0, 0, 160, 45);
        let (map, _) = split(size);
        let (s, ox, oy) = strategy_fit(map, &g.terrain);
        let (j, n) = g.world.nodes.iter().enumerate().find(|(_, n)| n.links_a.len() > 1).map(|(i, n)| (i, n.clone())).unwrap();
        // the same cell draw_strategy puts the ◇ in
        let (col, row) = (((n.x + ox) / s) as u16, ((n.y + oy) / s / 2.0) as u16);
        let (x, y, reach) = screen_to_map(&g, size, map.x + col, map.y + row).unwrap();
        assert!(g.world.switch_junction(x, y, reach).is_some());
        assert_eq!(g.world.route_choice.get(&j).copied(), Some(1), "junction {j} at ({:.1},{:.1})", n.x, n.y);
    }
}
