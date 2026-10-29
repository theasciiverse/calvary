//! Follow mode: the original's third-person view from behind the supertank
//! (its manual, section 3.3), drawn as a heightmap landscape the way
//! Comanche (1992) did: one ray per screen column, marched outward over the
//! heightmap, each terrain slice drawn only where it rises above what is
//! already there. Half-block pixels carry the real tile colours with hill
//! shading and distance fog into a sky gradient. Everything else (buildings,
//! units, scenery, the supertank itself, shots and blasts) is a depth-tested
//! billboard, so hills hide what is behind them.

use crate::game::Game;
use crate::sprites::{self, palette};
use crate::terrain::Rgb;
use crate::world::{flies, Class, Team};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Horizontal field of view.
const FOV: f32 = 1.2;
/// How far the landscape is drawn, in map cells.
const FAR: f32 = 70.0;
/// Camera distance behind the tank.
const BACK: f32 = 7.0;
/// Where the tank should appear, as a fraction of the view's pixel height;
/// the camera height follows from it (and the screen's shape).
const TANK_AT: f32 = 0.76;
/// Where the horizon sits, as a fraction of the view's pixel height.
const HORIZON: f32 = 0.30;
/// Heights are stretched a little so hills read at terminal resolution.
const RELIEF: f32 = 1.25;

const SKY_TOP: (f32, f32, f32) = (14.0, 18.0, 40.0);
const SKY_LOW: (f32, f32, f32) = (96.0, 112.0, 140.0);

struct Frame {
    w: usize,
    h: usize,
    px: Vec<Rgb>,
    depth: Vec<f32>,
}

impl Frame {
    fn set(&mut self, x: usize, y: usize, c: Rgb, z: f32) {
        if x < self.w && y < self.h {
            let i = y * self.w + x;
            if z < self.depth[i] {
                self.depth[i] = z;
                self.px[i] = c;
            }
        }
    }
}

fn mix(a: Rgb, b: (f32, f32, f32), k: f32) -> Rgb {
    let m = |x: u8, y: f32| (x as f32 + (y - x as f32) * k.clamp(0.0, 1.0)) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn shade(c: Rgb, f: f32) -> Rgb {
    let s = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    (s(c.0), s(c.1), s(c.2))
}

/// Where the follow camera is: behind the tank along `heading`, above the
/// ground (and never below the terrain under it).
pub fn camera(g: &Game, heading: f32, up: f32) -> (f32, f32, f32) {
    let (dx, dy) = (heading.cos(), heading.sin());
    let (x, y) = (g.tank.x - dx * BACK, g.tank.y - dy * BACK);
    // anchored to the tank's ground, so the tank keeps its place on screen;
    // raised only to stay clear of the terrain under the camera itself
    let tank_ground = g.terrain.height(g.tank.x, g.tank.y) * RELIEF;
    let own_ground = g.terrain.height(x, y) * RELIEF;
    (x, y, (tank_ground + up).max(own_ground + 1.0))
}

/// The projection for a view of `area`: horizon row and focal length (in
/// pixels) and the camera position.
struct View {
    w: usize,
    h: usize,
    horizon: f32,
    focal: f32,
    cam: (f32, f32, f32),
}

fn view(g: &Game, area: Rect) -> View {
    let (w, h) = (area.width as usize, area.height as usize * 2);
    let horizon = h as f32 * HORIZON;
    let focal = (w.max(1) as f32 / 2.0) / (FOV / 2.0).tan();
    // high enough that the tank lands at TANK_AT of the view
    let up = ((TANK_AT * h as f32 - horizon) * BACK / focal).clamp(1.2, 6.0);
    View { w, h, horizon, focal, cam: camera(g, g.chase_heading, up) }
}

/// What the mouse points at in follow mode: the ground under screen cell
/// (col, row) of the view `area`, found by marching that column's ray as the
/// renderer does, or None when it points at the sky (then aim along the
/// column at full range). Also returns the column's map angle.
pub fn pick(g: &Game, area: Rect, col: u16, row: u16) -> (f32, Option<(f32, f32)>) {
    let v = view(g, area);
    let (cx, cy, cz) = v.cam;
    let a = ((col.saturating_sub(area.x) as f32 + 0.5) / v.w.max(1) as f32 - 0.5) * FOV;
    let angle = g.chase_heading + a;
    let (rx, ry) = (angle.cos(), angle.sin());
    let py = (row.saturating_sub(area.y) as f32 + 0.5) * 2.0;
    let mut dist = 0.6;
    while dist < FAR {
        let (x, y) = (cx + rx * dist, cy + ry * dist);
        if g.terrain.cell_at(x, y).is_some() {
            let sy = v.horizon + (cz - g.terrain.height(x, y) * RELIEF) * v.focal / (dist * a.cos());
            if sy <= py {
                return (angle, Some((x, y)));
            }
        }
        dist += 0.12 + dist * 0.012;
    }
    (angle, None)
}

pub fn draw(buf: &mut Buffer, area: Rect, g: &Game) {
    let v = view(g, area);
    let (w, h) = (v.w, v.h);
    if w == 0 || h == 0 {
        return;
    }
    let heading = g.chase_heading;
    let (horizon, focal) = (v.horizon, v.focal);
    let (cx, cy, cz) = v.cam;
    let mut f = Frame { w, h, px: vec![(0, 0, 0); w * h], depth: vec![f32::MAX; w * h] };

    // sky
    for y in 0..h {
        let k = (y as f32 / horizon.max(1.0)).min(1.0);
        let c = (
            (SKY_TOP.0 + (SKY_LOW.0 - SKY_TOP.0) * k) as u8,
            (SKY_TOP.1 + (SKY_LOW.1 - SKY_TOP.1) * k) as u8,
            (SKY_TOP.2 + (SKY_LOW.2 - SKY_TOP.2) * k) as u8,
        );
        for x in 0..w {
            f.px[y * w + x] = c;
        }
    }

    // terrain, one ray per column, front to back
    let (hx, hy) = (heading.cos(), heading.sin());
    for col in 0..w {
        let a = ((col as f32 + 0.5) / w as f32 - 0.5) * FOV;
        let (rx, ry) = ((heading + a).cos(), (heading + a).sin());
        // project along the view axis so the horizon stays straight
        let cos_a = a.cos();
        let mut lowest = h as f32;
        let mut dist = 0.6;
        while dist < FAR && lowest > 0.0 {
            let (x, y) = (cx + rx * dist, cy + ry * dist);
            let Some(cell) = g.terrain.cell_at(x, y) else {
                dist += 0.25 + dist * 0.015;
                continue;
            };
            let z = dist * cos_a;
            let ground = g.terrain.height(x, y) * RELIEF;
            let sy = horizon + (cz - ground) * focal / z;
            if sy < lowest {
                let mut c = g.terrain.shaded(x, y);
                if cell.water {
                    // a glint on the water, fixed to the ground
                    let hsh = ((x * 4.0) as i32).wrapping_mul(73856093) ^ ((y * 4.0) as i32).wrapping_mul(19349663) ^ ((g.time * 3.0) as i32);
                    if hsh.rem_euclid(29) == 0 {
                        c = shade(c, 1.5);
                    }
                }
                let fog = (dist / FAR).powf(1.4);
                let c = mix(c, SKY_LOW, fog);
                let top = sy.max(0.0) as usize;
                for py in top..(lowest as usize).min(h) {
                    f.set(col, py, c, z);
                }
                lowest = sy;
            }
            dist += 0.12 + dist * 0.012;
        }
    }

    // billboards: project a ground point and a height into the view
    let right = (-hy, hx);
    let project = |x: f32, y: f32, zw: f32| -> Option<(f32, f32, f32)> {
        let (dx, dy) = (x - cx, y - cy);
        let fwd = dx * hx + dy * hy;
        if fwd < 0.5 || fwd > FAR {
            return None;
        }
        let side = dx * right.0 + dy * right.1;
        Some((w as f32 / 2.0 + side * focal / fwd, horizon + (cz - zw) * focal / fwd, fwd))
    };
    let fog = |c: Rgb, z: f32| mix(c, SKY_LOW, (z / FAR).powf(1.4));
    // a box standing on the ground: half width, height, front and top colours
    let boxed = |f: &mut Frame, x: f32, y: f32, lift: f32, half: f32, tall: f32, front: Rgb, top: Rgb| {
        let ground = g.terrain.height(x, y) * RELIEF + lift;
        let Some((sx, sy, z)) = project(x, y, ground) else { return };
        let s = focal / z;
        let (hw, ht) = ((half * s).max(0.6), (tall * s).max(0.6));
        let (x0, x1) = ((sx - hw).round() as i32, (sx + hw).round() as i32);
        let (y0, y1) = ((sy - ht).round() as i32, sy.round() as i32);
        let cap = ((y1 - y0) as f32 * 0.25).max(1.0) as i32;
        for py in y0.max(0)..=y1.min(h as i32 - 1) {
            for px in x0.max(0)..=x1.min(w as i32 - 1) {
                let c = if py - y0 < cap { top } else { front };
                f.set(px as usize, py as usize, fog(c, z), z - 0.05);
            }
        }
    };

    for e in &g.world.entities {
        if e.class == Class::PlayerTank || !g.world.seen(e) {
            continue;
        }
        // nothing standing between the camera and the tank: it would hide it
        let (dx, dy) = (e.x - cx, e.y - cy);
        if dx * hx + dy * hy < BACK - 0.6 && (dx * right.0 + dy * right.1).abs() < 1.6 + e.radius() {
            continue;
        }
        let alive = e.alive;
        if !alive && matches!(e.class, Class::Vehicle | Class::EnemyTank | Class::Plane) {
            continue;
        }
        let pal = palette(e.team);
        let k = e.kind.as_str();
        let (half, tall, lift, front, top): (f32, f32, f32, Rgb, Rgb) = match e.class {
            Class::PlayerTank => continue,
            _ if !alive => (e.radius() * 0.7, 0.25, 0.0, (70, 64, 58), (96, 88, 80)),
            Class::Hq => (1.5, 0.7, 0.0, pal.body, pal.front),
            Class::Garage | Class::Airbase => (1.1, 1.0, 0.0, pal.body, pal.front),
            Class::Bank => (1.0, 1.2, 0.0, (226, 220, 204), (236, 194, 58)),
            Class::Tower if k == "supergun" => (0.8, 1.1, 0.0, pal.body, pal.turret),
            Class::Tower => (0.35, 1.8, 0.0, pal.body, pal.turret),
            Class::Plane => (0.5, 0.25, 3.0, pal.body, pal.front),
            Class::Bridge => (0.5, 0.1, 0.0, sprites::BRIDGE_SIDE, sprites::BRIDGE_TOP),
            Class::Pad => (0.9, 0.12, 0.0, (120, 118, 112), pal.body),
            Class::Relic => (0.3, 0.35, if e.carrier.is_some() { 0.7 } else { 0.0 }, sprites::RELIC_GOLD, sprites::RELIC_GLOW),
            Class::Vehicle if flies(k) => (0.5, 0.3, 3.0, pal.body, pal.front),
            Class::Vehicle | Class::EnemyTank => (e.radius() * 0.9, 0.55, 0.0, pal.body, pal.turret),
            Class::Prop if k == "giant_skull" => (0.9, 0.8, 0.0, (196, 186, 162), sprites::BONE),
            Class::Prop if k.starts_with("giant_") => (0.12, 1.9, 0.0, (196, 186, 162), sprites::BONE),
            Class::Prop if k == "cathedral" => (3.5, 2.6, 0.0, (190, 182, 162), (104, 146, 126)),
            Class::Prop if k == "cornice" => (1.6, 0.5, 0.8, (196, 208, 222), (246, 249, 252)),
            Class::Prop if k.contains("wall") => (0.6, 0.7, 0.0, (150, 106, 84), (172, 124, 98)),
            Class::Prop if k.contains("palm") || k.contains("tree") => (0.3, 1.6, 0.0, (52, 120, 56), (80, 160, 70)),
            Class::Prop if k.contains("house") || k.contains("build") => (0.5, 0.8, 0.0, (222, 212, 192), (204, 98, 72)),
            Class::Prop if k.contains("tent") => (0.6, 0.5, 0.0, (204, 200, 186), (234, 230, 216)),
            Class::Prop if k.contains("cabin") => (0.4, 0.45, 0.0, (160, 136, 90), (196, 170, 118)),
            Class::Prop if k.contains("ruins") => (0.35, 0.6, 0.0, (196, 186, 162), (210, 200, 176)),
            Class::Prop if k.contains("greeceship") => (2.0, 0.8, 0.0, (92, 98, 110), (120, 126, 138)),
            Class::Prop if k.contains("ship") || k.contains("tug") => (1.0, 0.6, 0.0, (92, 98, 110), (120, 126, 138)),
            Class::Prop => (0.3, 0.35, 0.0, (150, 150, 140), (176, 176, 166)),
        };
        let (front, top) = if e.flash > 0.0 { (shade(front, 1.6), shade(top, 1.6)) } else { (front, top) };
        if k.contains("palm") || k.contains("tree") {
            // trunk and crown
            boxed(&mut f, e.x, e.y, 0.0, 0.07, 1.0, (96, 70, 40), (96, 70, 40));
            boxed(&mut f, e.x, e.y, 0.9, 0.45, 0.6, front, top);
        } else {
            boxed(&mut f, e.x, e.y, lift, half, tall, front, top);
        }
        // a health bar over anything hurt
        if alive && e.class != Class::Prop && e.hp < e.max_hp && e.team != Team::Scenery {
            let ground = g.terrain.height(e.x, e.y) * RELIEF + lift + tall + 0.4;
            if let Some((sx, sy, z)) = project(e.x, e.y, ground) {
                let hw = (half * focal / z).max(2.0);
                let kf = (e.hp / e.max_hp).clamp(0.0, 1.0);
                let c = if kf > 0.5 { (90, 220, 110) } else if kf > 0.25 { (255, 180, 40) } else { (235, 70, 55) };
                let (x0, x1) = ((sx - hw) as i32, (sx + hw) as i32);
                for px in x0.max(0)..=x1.min(w as i32 - 1) {
                    let on = (px - x0) as f32 <= (x1 - x0) as f32 * kf;
                    f.set(px as usize, sy as usize, if on { c } else { (40, 40, 38) }, z - 0.2);
                }
            }
        }
    }

    // the supertank in front of the camera: hull, turret, barrel along the aim
    let t = &g.tank;
    let base = g.terrain.height(t.x, t.y) * RELIEF;
    let hull = if g.world.player_tank.is_some_and(|i| g.world.entities[i].flash > 0.0) { (255, 220, 160) } else { (176, 138, 58) };
    boxed(&mut f, t.x, t.y, 0.0, 0.75, 0.45, hull, (236, 196, 108));
    boxed(&mut f, t.x, t.y, 0.45, 0.32, 0.3, (226, 190, 104), (245, 225, 170));
    // the barrel: a solid tube from the turret to the muzzle, sampled densely
    // and rounded (not truncated), as thick as a 0.08-cell tube at its
    // distance, so it tapers into the scene instead of stepping
    let aim = t.aim();
    let (from, to) = (0.05f32, 1.55f32);
    let steps = 64;
    for k in 0..=steps {
        let kk = k as f32 / steps as f32;
        let d = from + (to - from) * kk;
        let Some((sx, sy, z)) = project(t.x + aim.cos() * d, t.y + aim.sin() * d, base + 0.62) else { continue };
        let r = (0.08 * focal / z).clamp(0.5, 3.0);
        let tip = kk > 0.88;
        let c = if tip && t.flash > 0.0 { (255, 250, 225) } else if tip { (255, 238, 196) } else { (232, 206, 140) };
        let (x0, x1) = ((sx - r).round() as i32, (sx + r).round() as i32 - 1);
        let (y0, y1) = ((sy - r * 0.6).round() as i32, (sy + r * 0.6).round() as i32 - 1);
        for py in y0..=y1.max(y0) {
            for px in x0..=x1.max(x0) {
                if px >= 0 && py >= 0 {
                    f.set(px as usize, py as usize, c, z - 0.3);
                }
            }
        }
    }
    // muzzle flash
    if t.flash > 0.0 {
        if let Some((sx, sy, z)) = project(t.x + aim.cos() * (to + 0.3), t.y + aim.sin() * (to + 0.3), base + 0.62) {
            let r = ((0.3 + t.flash * 2.0) * focal / z).clamp(1.0, 6.0) as i32;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        let (x, y) = (sx.round() as i32 + dx, sy.round() as i32 + dy);
                        if x >= 0 && y >= 0 {
                            f.set(x as usize, y as usize, (255, 245, 200), z - 0.4);
                        }
                    }
                }
            }
        }
    }

    // shots and blasts, as bright points
    for s in &g.shots {
        if let Some((sx, sy, z)) = project(s.x, s.y, s.z * RELIEF) {
            f.set(sx as usize, sy as usize, (255, 250, 230), z - 0.1);
        }
    }
    for b in &g.world.bolts {
        let mut ground = g.terrain.height(b.x, b.y) * RELIEF + 0.6;
        if crate::render::lobbed(b.weapon).is_some() {
            // a supergun's shell or a volcanic bomb climbs and falls on its arc
            let total = (b.to.0 - b.from.0).hypot(b.to.1 - b.from.1).max(1e-3);
            let t = 1.0 - (b.to.0 - b.x).hypot(b.to.1 - b.y) / total;
            ground += 4.0 * total * 0.3 * t * (1.0 - t) * RELIEF;
        }
        if let Some((sx, sy, z)) = project(b.x, b.y, ground) {
            f.set(sx as usize, sy as usize, (255, 200, 120), z - 0.1);
        }
    }
    for p in &g.particles {
        let kk = p.life / p.max_life;
        let ground = g.terrain.height(p.x, p.y) * RELIEF + (1.0 - kk) * 1.2;
        if let Some((sx, sy, z)) = project(p.x, p.y, ground) {
            let c = if p.hot {
                (255, (90.0 + 165.0 * kk) as u8, (40.0 * kk) as u8)
            } else if p.snow {
                shade((244, 248, 252), 0.75 + kk * 0.25)
            } else {
                shade((150, 150, 150), 0.5 + kk * 0.5)
            };
            let r = (0.12 * focal / z).clamp(0.0, 2.0) as i32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (sx as i32 + dx, sy as i32 + dy);
                    if x >= 0 && y >= 0 {
                        f.set(x as usize, y as usize, c, z - 0.1);
                    }
                }
            }
        }
    }

    // out to the terminal, two pixels per cell
    for row in 0..area.height as usize {
        for col in 0..w {
            let (a, b) = (f.px[2 * row * w + col], f.px[(2 * row + 1) * w + col]);
            if let Some(cell) = buf.cell_mut((area.x + col as u16, area.y + row as u16)) {
                cell.set_char('▀').set_fg(Color::Rgb(a.0, a.1, a.2)).set_bg(Color::Rgb(b.0, b.1, b.2));
            }
        }
    }

    // crosshair where the turret points, 12 cells out
    let aim_at = (t.x + aim.cos() * 12.0, t.y + aim.sin() * 12.0);
    let ground = g.terrain.height(aim_at.0, aim_at.1) * RELIEF + 0.5;
    if !g.show_aim {
        return;
    }
    if let Some((sx, sy, _)) = project(aim_at.0, aim_at.1, ground) {
        let (cx_, cy_) = (sx as i32, (sy / 2.0) as i32);
        if cx_ >= 0 && cy_ >= 0 && (cx_ as u16) < area.width && (cy_ as u16) < area.height {
            if let Some(cell) = buf.cell_mut((area.x + cx_ as u16, area.y + cy_ as u16)) {
                cell.set_char('╋').set_fg(Color::Rgb(255, 180, 40));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, View as GameView};

    /// The mouse picks the ground the renderer drew: the screen centre at the
    /// tank's row lands on the ground by the tank, the top row is sky.
    #[test]
    fn picking_matches_the_projection() {
        let mut g = Game::new(0).unwrap();
        g.view = crate::game::View::Chase;
        g.chase_heading = g.tank.heading;
        let area = Rect::new(0, 0, 160, 42);
        let tank_row = (TANK_AT * area.height as f32) as u16;
        let (angle, ground) = pick(&g, area, area.width / 2, tank_row);
        assert!((angle - g.tank.heading).abs() < 0.02);
        let (x, y) = ground.expect("ground under the tank");
        assert!((x - g.tank.x).hypot(y - g.tank.y) < 2.0, "picked ({x:.1},{y:.1}), tank at ({:.1},{:.1})", g.tank.x, g.tank.y);
        assert!(pick(&g, area, area.width / 2, 0).1.is_none(), "top row is sky");
    }

    /// `cargo test --release chase_frame_time -- --ignored --nocapture`:
    /// how long one follow-mode frame takes at several terminal sizes.
    #[test]
    #[ignore]
    fn chase_frame_time() {
        let mut g = Game::new(0).unwrap();
        g.view = GameView::Chase;
        g.chase_heading = g.tank.heading;
        for (w, h) in [(80, 24), (160, 45), (240, 70), (300, 100)] {
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(area);
            let n = 30;
            let t0 = std::time::Instant::now();
            for i in 0..n {
                g.chase_heading += 0.05 * i as f32;
                draw(&mut buf, area, &g);
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
            println!("{w}x{h}: {ms:.2} ms per frame");
        }
    }
}
