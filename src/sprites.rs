//! Sprites for everything that is not terrain: vehicles, structures and
//! scenery, drawn into one layer of half-block pixels (`▀`, two freely coloured
//! pixels per terminal cell). Braille cannot do this: one foreground colour per
//! cell swallows small parts like tracks.
//!
//! Every sprite is a function from a map position to a colour, sampled 3x3 per
//! pixel: the most common colour wins if it covers most of the pixel, and thin
//! parts (barrels) win if they touch it at all. Later sprites paint over earlier
//! ones; pixels nobody painted show the terrain.

use crate::terrain::{Rgb, Terrain};
use crate::world::{Class, Entity, Team};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

type Paint = Option<(Rgb, bool)>;

pub struct PixelLayer {
    area: Rect,
    origin: (f32, f32),
    px_w: f32,
    px_h: f32,
    px: Vec<Option<Rgb>>,
}

impl PixelLayer {
    /// `cols` is columns per map cell; a cell is 1 column wide and 2 pixels tall.
    pub fn new(area: Rect, origin: (f32, f32), cols: f32) -> PixelLayer {
        let n = area.width as usize * area.height as usize * 2;
        PixelLayer { area, origin, px_w: 1.0 / cols, px_h: 1.0 / cols, px: vec![None; n] }
    }

    pub fn pixel_size(&self) -> f32 {
        self.px_w.min(self.px_h)
    }

    /// Paint the shape `f` within `reach` map cells of (cx, cy).
    pub fn paint(&mut self, cx: f32, cy: f32, reach: f32, f: impl Fn(f32, f32) -> Paint) {
        let (w, h) = (self.area.width as i32, self.area.height as i32 * 2);
        let c0 = (((cx - reach - self.origin.0) / self.px_w).floor() as i32).max(0);
        let r0 = (((cy - reach - self.origin.1) / self.px_h).floor() as i32).max(0);
        let c1 = (((cx + reach - self.origin.0) / self.px_w).ceil() as i32).min(w - 1);
        let r1 = (((cy + reach - self.origin.1) / self.px_h).ceil() as i32).min(h - 1);
        for r in r0..=r1 {
            for c in c0..=c1 {
                let (x0, y0) = (self.origin.0 + c as f32 * self.px_w, self.origin.1 + r as f32 * self.px_h);
                let mut samples: [Option<Rgb>; 9] = [None; 9];
                let mut thin = None;
                for (k, s) in samples.iter_mut().enumerate() {
                    let (sx, sy) = ((k % 3) as f32 + 0.5, (k / 3) as f32 + 0.5);
                    if let Some((rgb, is_thin)) = f(x0 + sx * self.px_w / 3.0, y0 + sy * self.px_h / 3.0) {
                        *s = Some(rgb);
                        if is_thin {
                            thin = Some(rgb);
                        }
                    }
                }
                let pick = thin.or_else(|| {
                    let covered: Vec<Rgb> = samples.iter().flatten().copied().collect();
                    if covered.len() < 5 {
                        return None;
                    }
                    covered.iter().max_by_key(|c| covered.iter().filter(|d| d == c).count()).copied()
                });
                if pick.is_some() {
                    self.px[(r * w + c) as usize] = pick;
                }
            }
        }
    }

    pub fn flush(&self, buf: &mut Buffer, terrain: &Terrain) {
        let w = self.area.width as usize;
        for row in 0..self.area.height as usize {
            for col in 0..w {
                let (top, bottom) = (self.px[2 * row * w + col], self.px[(2 * row + 1) * w + col]);
                if top.is_none() && bottom.is_none() {
                    continue;
                }
                let x = self.origin.0 + (col as f32 + 0.5) * self.px_w;
                let at = |p: Option<Rgb>, sub: usize| {
                    p.unwrap_or_else(|| terrain.shaded(x, self.origin.1 + ((2 * row + sub) as f32 + 0.5) * self.px_h))
                };
                let (t, b) = (at(top, 0), at(bottom, 1));
                if let Some(cell) = buf.cell_mut((self.area.x + col as u16, self.area.y + row as u16)) {
                    cell.set_char('▀').set_fg(Color::Rgb(t.0, t.1, t.2)).set_bg(Color::Rgb(b.0, b.1, b.2));
                }
            }
        }
    }
}

fn mix(c: Rgb, to: Rgb, k: f32) -> Rgb {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k.clamp(0.0, 1.0)) as u8;
    (m(c.0, to.0), m(c.1, to.1), m(c.2, to.2))
}

fn shade(c: Rgb, f: f32) -> Rgb {
    let s = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    (s(c.0), s(c.1), s(c.2))
}

fn hash2(x: f32, y: f32) -> u32 {
    let (a, b) = ((x * 8.0).floor() as i32 as u32, (y * 8.0).floor() as i32 as u32);
    let mut h = a.wrapping_mul(0x8da6_b343) ^ b.wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h.wrapping_mul(0x2c1b_3c6d) >> 16
}

/// Team colours: the player amber (the Asciiverse accent), the computer red,
/// neutral grey.
#[derive(Clone, Copy)]
pub struct Palette {
    pub body: Rgb,
    pub front: Rgb,
    pub turret: Rgb,
    pub rim: Rgb,
    pub barrel: Rgb,
}

pub const PLAYER: Palette = Palette { body: (176, 138, 58), front: (236, 196, 108), turret: (226, 190, 104), rim: (120, 92, 40), barrel: (245, 225, 170) };
pub const ENEMY: Palette = Palette { body: (150, 50, 42), front: (214, 96, 76), turret: (196, 72, 58), rim: (90, 28, 22), barrel: (240, 176, 160) };
pub const NEUTRAL: Palette = Palette { body: (150, 150, 144), front: (196, 196, 188), turret: (178, 178, 170), rim: (90, 90, 86), barrel: (220, 220, 212) };

pub fn palette(team: Team) -> Palette {
    match team {
        Team::Player => PLAYER,
        Team::Enemy => ENEMY,
        _ => NEUTRAL,
    }
}

const TRACK: Rgb = (30, 28, 24);
const TREAD: Rgb = (112, 106, 92);
const HATCH: Rgb = (60, 48, 30);
const MUZZLE: Rgb = (255, 250, 225);
const TREAD_BAND: f32 = 0.16;

/// Hull size and turret layout per vehicle, in map cells.
#[derive(Clone, Copy)]
pub struct Look {
    pub half_len: f32,
    pub half_wid: f32,
    pub track_wid: f32,
    pub turret_r: f32,
    pub turret_offset: f32,
    pub barrel_len: f32,
}

pub const SUPERTANK: Look = Look { half_len: 0.85, half_wid: 0.6, track_wid: 0.26, turret_r: 0.36, turret_offset: -0.08, barrel_len: 1.45 };

pub fn look(kind: &str) -> Look {
    let l = |half_len, half_wid, track_wid, turret_r, barrel_len| Look { half_len, half_wid, track_wid, turret_r, turret_offset: -0.05, barrel_len };
    match kind {
        "stank" => SUPERTANK,
        "cobra_tank" => l(0.7, 0.48, 0.18, 0.3, 1.1),
        "tank_buster" => l(0.5, 0.34, 0.12, 0.2, 0.9),
        "rocket_tank" => l(0.5, 0.34, 0.12, 0.22, 0.45),
        "electric_car" => l(0.42, 0.28, 0.08, 0.16, 0.35),
        "trike" => l(0.3, 0.17, 0.06, 0.0, 0.0),
        "engineer" => l(0.38, 0.26, 0.1, 0.0, 0.0),
        "bomb_truck" => l(0.5, 0.3, 0.08, 0.0, 0.0),
        _ => l(0.45, 0.32, 0.12, 0.2, 0.7),
    }
}

#[derive(Clone, Copy)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub aim: f32,
    pub odometer: f32,
    /// Seconds of muzzle flash left.
    pub muzzle: f32,
    /// Seconds of hit flash left.
    pub hit: f32,
}

/// Paint a tracked vehicle: tracks with rolling treads, hull with a lighter
/// front plate, turret with rim and hatch, barrel and muzzle flash.
pub fn paint_vehicle(layer: &mut PixelLayer, look: Look, pal: Palette, pose: Pose) {
    let barrel_half = (layer.pixel_size() * 0.35).max(0.05);
    let reach = look.half_len.hypot(look.half_wid).max(look.barrel_len - look.turret_offset + 0.2 + pose.muzzle * 1.5);
    let (hx, hy) = (pose.heading.cos(), pose.heading.sin());
    let (ax, ay) = (pose.aim.cos(), pose.aim.sin());
    let (tcx, tcy) = (pose.x + hx * look.turret_offset, pose.y + hy * look.turret_offset);
    let hit = |c: Rgb| if pose.hit > 0.0 { mix(c, (255, 255, 255), 0.7) } else { c };
    layer.paint(pose.x, pose.y, reach, |x, y| {
        let (dx, dy) = (x - tcx, y - tcy);
        let (tl, tw) = (dx * ax + dy * ay, -dx * ay + dy * ax);
        if pose.muzzle > 0.0 && (tl - look.barrel_len - 0.15).hypot(tw) <= 0.15 + pose.muzzle * 1.5 {
            return Some((MUZZLE, true));
        }
        if look.barrel_len > 0.0 && (0.0..=look.barrel_len).contains(&tl) && tw.abs() <= barrel_half {
            return Some((hit(if tl > look.barrel_len - 0.16 { MUZZLE } else { pal.barrel }), true));
        }
        let r = tl.hypot(tw);
        if look.turret_r > 0.0 && r <= look.turret_r {
            let c = if r <= look.turret_r * 0.3 { HATCH } else if r >= look.turret_r - 0.07 { pal.rim } else { pal.turret };
            return Some((hit(c), false));
        }
        let (dx, dy) = (x - pose.x, y - pose.y);
        let (l, w) = (dx * hx + dy * hy, -dx * hy + dy * hx);
        if l.abs() > look.half_len || w.abs() > look.half_wid {
            return None;
        }
        if w.abs() >= look.half_wid - look.track_wid {
            // tread bands are fixed to the ground, so they roll back as the vehicle drives
            let tread = ((l + pose.odometer) / TREAD_BAND).floor().rem_euclid(2.0) == 0.0;
            return Some((hit(if tread { TREAD } else { TRACK }), false));
        }
        Some((hit(if l > look.half_len - look.half_len * 0.24 { pal.front } else { pal.body }), false))
    });
}

pub fn paint_entity(layer: &mut PixelLayer, e: &Entity, time: f32) {
    let pal = palette(e.team);
    match e.class {
        Class::Vehicle if crate::world::flies(&e.kind) => paint_aircraft(layer, e, pal, time),
        Class::Vehicle | Class::EnemyTank => {
            let pose = Pose { x: e.x, y: e.y, heading: e.heading, aim: e.heading + e.turret, odometer: e.odometer, muzzle: 0.0, hit: e.flash };
            paint_vehicle(layer, look(&e.kind), pal, pose);
        }
        Class::PlayerTank => {}
        Class::Plane => paint_plane(layer, e, pal),
        Class::Prop => paint_prop(layer, e),
        _ if !e.alive => paint_rubble(layer, e),
        _ => paint_structure(layer, e, pal, time),
    }
}

fn paint_structure(layer: &mut PixelLayer, e: &Entity, pal: Palette, time: f32) {
    let (cx, cy) = (e.x, e.y);
    let hit = e.flash > 0.0;
    let lit = move |c: Rgb| if hit { mix(c, (255, 255, 255), 0.6) } else { c };
    match e.class {
        Class::Hq => layer.paint(cx, cy, 1.6, |x, y| {
            let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
            if dx > 1.5 || dy > 1.5 {
                return None;
            }
            let c = if dx > 1.22 || dy > 1.22 {
                pal.body
            } else if dx < 0.14 || dy < 0.14 {
                (214, 210, 196)
            } else if (dx - 0.75).abs() < 0.08 || (dy - 0.75).abs() < 0.08 {
                pal.front
            } else {
                (64, 64, 62)
            };
            Some((lit(c), false))
        }),
        Class::Garage => layer.paint(cx, cy, 1.2, |x, y| {
            let (dx, dy) = (x - cx, y - cy);
            if dx.abs() > 1.1 || dy.abs() > 1.1 {
                return None;
            }
            let c = if dy > 0.55 && dx.abs() < 0.55 {
                (36, 34, 30)
            } else if ((dx + 1.1) / 0.3).floor() as i32 % 2 == 0 {
                pal.body
            } else {
                shade(pal.body, 0.82)
            };
            Some((lit(c), false))
        }),
        Class::Airbase => layer.paint(cx, cy, 1.2, |x, y| {
            let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
            if dx > 1.1 || dy > 1.1 {
                return None;
            }
            let c = if dx > 0.95 || dy > 0.95 {
                pal.body
            } else if dx < 0.16 || dy < 0.16 {
                (206, 206, 196)
            } else {
                (84, 88, 84)
            };
            Some((lit(c), false))
        }),
        Class::Bank => layer.paint(cx, cy, 1.1, |x, y| {
            let (dx, dy) = (x - cx, y - cy);
            if dx.abs() > 1.0 || dy.abs() > 1.0 {
                return None;
            }
            let r = dx.hypot(dy);
            // the coin glints now and then
            let glint = ((time * 2.0 + cx).sin() > 0.9) as u8 as f32;
            let c = if r < 0.48 {
                mix((236, 194, 58), (255, 250, 210), glint * 0.6)
            } else if dx.abs() > 0.8 || dy.abs() > 0.8 {
                pal.body
            } else {
                (226, 220, 204)
            };
            Some((lit(c), false))
        }),
        _ => {
            // towers and other gun emplacements
            let aim = e.heading + e.turret;
            let (ax, ay) = (aim.cos(), aim.sin());
            let barrel_half = (layer.pixel_size() * 0.35).max(0.06);
            layer.paint(cx, cy, 1.0, |x, y| {
                let (dx, dy) = (x - cx, y - cy);
                let (l, w) = (dx * ax + dy * ay, -dx * ay + dy * ax);
                if (0.0..=0.95).contains(&l) && w.abs() <= barrel_half {
                    return Some((lit(pal.barrel), true));
                }
                let r = dx.hypot(dy);
                if r > 0.62 {
                    return None;
                }
                let c = if r < 0.2 { HATCH } else if r > 0.5 { pal.rim } else { pal.turret };
                Some((lit(c), false))
            });
        }
    }
}

/// Money plane: swept wings, fuselage, tail, and the gold crate slung under
/// it while it carries money. A soft shadow below sells the height.
fn paint_plane(layer: &mut PixelLayer, e: &Entity, pal: Palette) {
    let (cx, cy) = (e.x, e.y);
    let (hx, hy) = (e.heading.cos(), e.heading.sin());
    let shadow = (cx + 0.5, cy + 0.6);
    let hit = e.flash > 0.0;
    let loaded = e.crates > 0;
    let shape = move |l: f32, w: f32| {
        let fuselage = l.abs() <= 0.55 && w.abs() <= 0.08;
        let wing = l.abs() <= 0.14 + (0.5 - w.abs()) * 0.25 && w.abs() <= 0.5 && l > -0.2 && l < 0.25;
        let tail = (-0.55..=-0.38).contains(&l) && w.abs() <= 0.2;
        fuselage || wing || tail
    };
    let local = move |x: f32, y: f32, ox: f32, oy: f32| {
        let (dx, dy) = (x - ox, y - oy);
        (dx * hx + dy * hy, -dx * hy + dy * hx)
    };
    layer.paint(shadow.0, shadow.1, 0.6, |x, y| {
        let (l, w) = local(x, y, shadow.0, shadow.1);
        shape(l, w).then_some(((40, 44, 40), false))
    });
    layer.paint(cx, cy, 0.6, |x, y| {
        let (l, w) = local(x, y, cx, cy);
        if loaded && l.abs() <= 0.12 && w.abs() <= 0.12 {
            return Some(((236, 194, 58), false));
        }
        if !shape(l, w) {
            return None;
        }
        let c = if l > 0.4 { pal.front } else if w.abs() > 0.1 { pal.turret } else { pal.body };
        Some((if hit { mix(c, (255, 255, 255), 0.6) } else { c }, false))
    });
}

/// Helicopter (body, tail boom, spinning rotor), jet (delta) or bomber (long
/// straight wings), each with a shadow on the ground below.
fn paint_aircraft(layer: &mut PixelLayer, e: &Entity, pal: Palette, time: f32) {
    let (cx, cy) = (e.x, e.y);
    let (hx, hy) = (e.heading.cos(), e.heading.sin());
    let kind = e.kind.clone();
    let rotor = time * 14.0;
    let shape = move |l: f32, w: f32| -> Option<u8> {
        match kind.as_str() {
            "helicopter" => {
                let r = l.hypot(w);
                if r <= 0.62 && ((l * rotor.cos() + w * rotor.sin()).abs() < 0.05 || (-l * rotor.sin() + w * rotor.cos()).abs() < 0.05) {
                    return Some(3);
                }
                if (l - 0.05).hypot(w * 1.3) <= 0.22 {
                    return Some(1);
                }
                ((-0.6..=0.0).contains(&l) && w.abs() <= 0.05).then_some(0)
            }
            "jet" => {
                let delta = (-0.45..=0.55).contains(&l) && w.abs() <= (0.55 - l) * 0.45;
                delta.then_some(if l > 0.35 { 2 } else { 0 })
            }
            _ => {
                let body = l.abs() <= 0.5 && w.abs() <= 0.09;
                let wing = (-0.1..=0.12).contains(&l) && w.abs() <= 0.7;
                let tail = (-0.5..=-0.36).contains(&l) && w.abs() <= 0.25;
                (body || wing || tail).then_some(if l > 0.38 { 2 } else { 0 })
            }
        }
    };
    let local = move |x: f32, y: f32, ox: f32, oy: f32| {
        let (dx, dy) = (x - ox, y - oy);
        (dx * hx + dy * hy, -dx * hy + dy * hx)
    };
    let shadow = (cx + 0.6, cy + 0.7);
    layer.paint(shadow.0, shadow.1, 0.75, |x, y| {
        let (l, w) = local(x, y, shadow.0, shadow.1);
        shape(l, w).filter(|&p| p != 3).map(|_| ((40, 44, 40), false))
    });
    let hit = e.flash > 0.0;
    layer.paint(cx, cy, 0.75, |x, y| {
        let (l, w) = local(x, y, cx, cy);
        let c = match shape(l, w)? {
            1 => pal.turret,
            2 => pal.front,
            3 => (210, 210, 200),
            _ => pal.body,
        };
        Some((if hit { mix(c, (255, 255, 255), 0.6) } else { c }, false))
    });
}

fn paint_rubble(layer: &mut PixelLayer, e: &Entity) {
    let (cx, cy, r) = (e.x, e.y, e.radius() * 0.8);
    layer.paint(cx, cy, r, |x, y| {
        let h = hash2(x, y);
        ((x - cx).abs() <= r && (y - cy).abs() <= r && h % 3 != 0).then_some((if h % 2 == 0 { (58, 54, 50) } else { (92, 86, 78) }, false))
    });
}

/// Scenery from the level, by the model name.
fn paint_prop(layer: &mut PixelLayer, e: &Entity) {
    let k = e.kind.as_str();
    let (cx, cy, th) = (e.x, e.y, e.heading);
    let has = |s: &str| k.contains(s);
    if has("palm") || has("tree") {
        layer.paint(cx, cy, 0.45, |x, y| {
            let r = (x - cx).hypot(y - cy);
            if r > 0.42 {
                return None;
            }
            let c = if r < 0.08 {
                (96, 70, 40)
            } else if hash2(x * 2.0, y * 2.0) % 3 == 0 {
                (78, 150, 70)
            } else {
                (40, 104, 48)
            };
            Some((c, false))
        });
        return;
    }
    // (half length, half width, colour, oriented)
    let (hl, hw, color, oriented): (f32, f32, Rgb, bool) = if has("ship") || has("tug") {
        (1.1, 0.38, (92, 98, 110), true)
    } else if has("greecehouses") {
        (0.5, 0.42, (204, 98, 72), false)
    } else if has("house") || has("build") || has("warehouse") || has("silo") || has("lighthouse") || has("pump") || has("toilet") {
        (0.45, 0.45, (222, 212, 192), false)
    } else if has("ruins") {
        (0.35, 0.35, (196, 186, 162), false)
    } else if has("crate") {
        (0.3, 0.3, (146, 104, 60), false)
    } else if has("dock") || has("bridge") || has("barrer") {
        (0.5, 0.5, (120, 112, 98), false)
    } else {
        (0.3, 0.3, (150, 150, 140), false)
    };
    let (hx, hy) = if oriented { (th.cos(), th.sin()) } else { (1.0, 0.0) };
    let ruined = has("ruins");
    layer.paint(cx, cy, hl.hypot(hw), |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        let (l, w) = (dx * hx + dy * hy, -dx * hy + dy * hx);
        if l.abs() > hl || w.abs() > hw {
            return None;
        }
        if ruined && hash2(x, y) % 3 == 0 {
            return None;
        }
        // roof ridge / deck line
        let c = if w.abs() < hw * 0.18 { shade(color, 0.78) } else if w > 0.0 { shade(color, 0.9) } else { color };
        Some((c, false))
    });
}

/// A thin health bar above anything that has taken damage.
pub fn paint_health(layer: &mut PixelLayer, e: &Entity) {
    if !e.alive || e.class == Class::Prop || e.hp >= e.max_hp {
        return;
    }
    let k = (e.hp / e.max_hp).clamp(0.0, 1.0);
    let half = (e.radius() * 0.9).max(0.45);
    let thick = layer.pixel_size() * 0.6;
    let y = e.y - e.radius() - thick * 2.0;
    let color = if k > 0.5 { (90, 220, 110) } else if k > 0.25 { (255, 180, 40) } else { (235, 70, 55) };
    let (x0, x1) = (e.x - half, e.x - half + 2.0 * half * k);
    layer.paint(e.x, y, half + 0.2, |x, py| {
        if (py - y).abs() > thick || x < e.x - half || x > e.x + half {
            return None;
        }
        Some((if x >= x0 && x <= x1 { color } else { (40, 40, 38) }, true))
    });
}
