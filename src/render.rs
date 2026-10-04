//! Drawing.
//!
//! Tactical view: one map cell is 2 columns by 1 row (about square at the usual
//! 2:1 cell aspect), so each map cell holds 4x4 Braille dots. The background is the
//! hill-shaded tile colour; dots carry contour lines, water shimmer and everything
//! that moves. Strategy view: the whole map in half-block pixels.

use crate::game::{Game, ShotKind, View};
use crate::tank::Tank;
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

// Supertank shape, in map cells along (l) and across (w) the hull.
const HULL_HALF_LEN: f32 = 0.85;
const HULL_HALF_WID: f32 = 0.6;
const TRACK_WID: f32 = 0.26;
const TURRET_R: f32 = 0.36;
const TURRET_OFFSET: f32 = -0.08;
const BARREL_LEN: f32 = 1.45;
const TREAD_BAND: f32 = 0.16;

const HULL_BODY: Rgb = (176, 138, 58);
const HULL_FRONT: Rgb = (236, 196, 108);
const TRACK: Rgb = (30, 28, 24);
const TREAD: Rgb = (112, 106, 92);
const TURRET: Rgb = (226, 190, 104);
const TURRET_RIM: Rgb = (120, 92, 40);
const HATCH: Rgb = (90, 70, 34);
const BARREL: Rgb = (245, 225, 170);
const MUZZLE: Rgb = (255, 250, 225);

/// What part of the tank covers a map point, by drawing priority.
#[derive(Clone, Copy, PartialEq)]
enum Part {
    None,
    Body,
    Front,
    Track(bool),
    Turret,
    Rim,
    Hatch,
    Barrel,
    Muzzle,
}

impl Part {
    fn color(self) -> Rgb {
        match self {
            Part::None => (0, 0, 0),
            Part::Body => HULL_BODY,
            Part::Front => HULL_FRONT,
            Part::Track(tread) => if tread { TREAD } else { TRACK },
            Part::Turret => TURRET,
            Part::Rim => TURRET_RIM,
            Part::Hatch => HATCH,
            Part::Barrel => BARREL,
            Part::Muzzle => MUZZLE,
        }
    }

    /// Thin parts that must survive down-sampling to a single pixel.
    fn thin(self) -> bool {
        matches!(self, Part::Barrel | Part::Muzzle)
    }
}

fn tank_part(tank: &Tank, x: f32, y: f32, barrel_half: f32) -> Part {
    let (hx, hy) = (tank.heading.cos(), tank.heading.sin());
    let a = tank.aim();
    let (tcx, tcy) = (tank.x + hx * TURRET_OFFSET, tank.y + hy * TURRET_OFFSET);
    let (dx, dy) = (x - tcx, y - tcy);
    let (tl, tw) = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
    if tank.flash > 0.0 && (tl - BARREL_LEN - 0.15).hypot(tw) <= 0.15 + tank.flash * 1.5 {
        return Part::Muzzle;
    }
    if (0.0..=BARREL_LEN).contains(&tl) && tw.abs() <= barrel_half {
        return if tl > BARREL_LEN - 0.16 { Part::Muzzle } else { Part::Barrel };
    }
    let r = tl.hypot(tw);
    if r <= TURRET_R {
        return if r <= 0.1 { Part::Hatch } else if r >= TURRET_R - 0.07 { Part::Rim } else { Part::Turret };
    }
    let (dx, dy) = (x - tank.x, y - tank.y);
    let (l, w) = (dx * hx + dy * hy, -dx * hy + dy * hx);
    if l.abs() > HULL_HALF_LEN || w.abs() > HULL_HALF_WID {
        return Part::None;
    }
    if w.abs() >= HULL_HALF_WID - TRACK_WID {
        // tread bands are fixed to the ground, so they roll back as the tank drives
        return Part::Track(((l + tank.odometer) / TREAD_BAND).floor().rem_euclid(2.0) == 0.0);
    }
    if l > HULL_HALF_LEN - 0.2 {
        Part::Front
    } else {
        Part::Body
    }
}

/// The supertank as a small sprite of half-block pixels: each terminal cell is
/// two pixels (`▀` with separate foreground and background colours), so every
/// part keeps its own colour, which a Braille cell cannot do. Each pixel takes
/// the most common part over a 3x3 sample, except thin parts (barrel), which
/// win if they touch the pixel at all. Pixels off the tank show the terrain.
fn draw_tank(buf: &mut Buffer, area: Rect, origin: (f32, f32), cols: f32, terrain: &Terrain, tank: &Tank) {
    let (cell_w, cell_h) = (1.0 / cols, 2.0 / cols);
    let (px_w, px_h) = (cell_w, cell_h / 2.0);
    let barrel_half = (px_w.min(px_h) * 0.35).max(0.07);
    let reach = HULL_HALF_LEN.hypot(HULL_HALF_WID).max(BARREL_LEN - TURRET_OFFSET + 0.2 + tank.flash * 1.5);
    let col0 = (((tank.x - reach - origin.0) / cell_w).floor() as i32).max(0);
    let row0 = (((tank.y - reach - origin.1) / cell_h).floor() as i32).max(0);
    let col1 = (((tank.x + reach - origin.0) / cell_w).ceil() as i32).min(area.width as i32 - 1);
    let row1 = (((tank.y + reach - origin.1) / cell_h).ceil() as i32).min(area.height as i32 - 1);
    let pixel = |x0: f32, y0: f32| {
        let mut counts: [(Part, u8); 9] = [(Part::None, 0); 9];
        let mut thin = None;
        for sy in 0..3 {
            for sx in 0..3 {
                let p = tank_part(tank, x0 + (sx as f32 + 0.5) * px_w / 3.0, y0 + (sy as f32 + 0.5) * px_h / 3.0, barrel_half);
                if p.thin() {
                    thin = Some(p);
                }
                if let Some(e) = counts.iter_mut().find(|e| e.1 > 0 && e.0 == p) {
                    e.1 += 1;
                } else if let Some(e) = counts.iter_mut().find(|e| e.1 == 0) {
                    *e = (p, 1);
                }
            }
        }
        thin.unwrap_or_else(|| counts.iter().max_by_key(|e| e.1).unwrap().0)
    };
    for row in row0..=row1 {
        for col in col0..=col1 {
            let (x0, y0) = (origin.0 + col as f32 * cell_w, origin.1 + row as f32 * cell_h);
            let (top, bottom) = (pixel(x0, y0), pixel(x0, y0 + px_h));
            if top == Part::None && bottom == Part::None {
                continue;
            }
            let color = |p: Part, y: f32| if p == Part::None { terrain.shaded(x0 + px_w / 2.0, y) } else { p.color() };
            if let Some(cell) = buf.cell_mut((area.x + col as u16, area.y + row as u16)) {
                cell.set_char('▀').set_fg(rgb(color(top, y0 + px_h / 2.0))).set_bg(rgb(color(bottom, y0 + px_h * 1.5)));
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
            ShotKind::Shell => c.plot(s.x, s.y, (255, 255, 255), 7),
            ShotKind::Bullet => c.plot(s.x, s.y, (255, 230, 120), 7),
            ShotKind::Missile => {
                let len = (s.vx * s.vx + s.vy * s.vy).sqrt();
                let tail = (s.x - s.vx / len * 0.8, s.y - s.vy / len * 0.8);
                c.line(tail, (s.x, s.y), (255, 120, 40), 7);
            }
        }
    }
    c.flush(f.buffer_mut());
    draw_tank(f.buffer_mut(), area, origin, cols, &g.terrain, &g.tank);
}

fn draw_strategy(f: &mut Frame, area: Rect, g: &Game) {
    let t = &g.terrain;
    let s = (t.w as f32 / area.width as f32).max(t.h as f32 / (area.height as f32 * 2.0));
    let (ox, oy) = ((area.width as f32 * s - t.w as f32) / 2.0, (area.height as f32 * 2.0 * s - t.h as f32) / 2.0);
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
    let col = ((g.tank.x + ox) / s) as u16;
    let row = ((g.tank.y + oy) / s / 2.0) as u16;
    if col < area.width && row < area.height {
        if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
            cell.set_char('◆').set_fg(AMBER).set_bg(Color::Black);
        }
    }
}

fn draw_hud(f: &mut Frame, area: Rect, g: &Game) {
    let t = &g.tank;
    let compass = ((t.heading.to_degrees() + 90.0).rem_euclid(360.0)) as i32;
    let view = match g.view {
        View::Tactical => ["TACTICAL far", "TACTICAL", "TACTICAL close", "TACTICAL closest"][g.zoom],
        View::Strategy => "STRATEGY",
    };
    let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(DIM));
    let value = |s: String| Span::styled(s, Style::default().fg(GREEN));
    let line1 = Line::from(vec![
        Span::styled(" CALVARY ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)),
        label("  map "),
        value(format!("{} {}x{}", g.terrain.name, g.terrain.w, g.terrain.h)),
        label("  view "),
        value(view.to_string()),
        label("  gear "),
        Span::styled(t.gear_label(), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        label("  spd "),
        value(format!("{:4.1}", t.speed)),
        label("  hdg "),
        value(format!("{compass:03}")),
        label("  pos "),
        value(format!("{:.0},{:.0} alt {:.1}", t.x, t.y, g.terrain.height(t.x, t.y))),
        label("  gun "),
        value(format!("{}", t.main_ammo)),
        label("  mg "),
        value(format!("{}", t.chain_ammo)),
        label("  msl "),
        value(format!("{}", t.missiles)),
        if t.blocked { Span::styled("  BLOCKED", Style::default().fg(AMBER)) } else { Span::raw("") },
    ]);
    let line2 = Line::from(label(
        " W/S gear  A/D turn  Q/E turret  SPACE gun  F machine gun  ENTER missile  TAB map  +/- zoom  F1/H help  ESC quit",
    ));
    f.render_widget(Paragraph::new(vec![line1, line2]).style(Style::default().bg(Color::Black)), area);
}

fn split(full: Rect) -> (Rect, Rect) {
    let hud_h = 2.min(full.height);
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
    let (t, terrain) = (&g.tank, &g.terrain);
    let (mut x, mut y) = g.camera.unwrap_or((t.x - vw / 2.0, t.y - vh / 2.0));
    let follow = |cam: &mut f32, pos: f32, view: f32| {
        let margin = view * EDGE_MARGIN;
        if pos < *cam + margin {
            *cam = pos - margin;
        } else if pos > *cam + view - margin {
            *cam = pos - view + margin;
        }
    };
    follow(&mut x, t.x, vw);
    follow(&mut y, t.y, vh);
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

pub fn draw(f: &mut Frame, g: &Game) {
    let (map, hud) = split(f.area());
    match g.view {
        View::Tactical => draw_tactical(f, map, g),
        View::Strategy => draw_strategy(f, map, g),
    }
    draw_hud(f, hud, g);
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
    ("Space", "main gun (120mm shell, hills stop it)"),
    ("F", "machine gun (hold)"),
    ("Enter", "guided missile; you carry two"),
    ("", ""),
    ("", "VIEW"),
    ("Tab / M", "tactical view / whole map"),
    ("+ / -", "zoom the tactical view in / out"),
    ("[ ]", "previous / next level"),
    ("F1 / H / ?", "this help (the game pauses)"),
    ("Esc", "close help, or quit"),
    ("", ""),
    ("", "Rock edges, deep water and steep slopes stop the tank."),
    ("", "Units, bases and paths come next; for now: drive and shoot."),
];

fn draw_help(f: &mut Frame, full: Rect) {
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
        .title_bottom(Span::styled(" F1 / H / Esc to close ", Style::default().fg(DIM)))
        .style(Style::default().bg(Color::Black));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
}
