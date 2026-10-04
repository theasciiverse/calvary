//! World state and the fixed-step simulation: the supertank, its shots and the
//! explosion particles. Units, bases and paths join later (see CLAUDE.md plan).

use crate::level::Level;
use crate::tank::{Controls, Tank, CHAIN_RELOAD, MAIN_RELOAD};
use crate::terrain::Terrain;

pub const DT: f32 = 1.0 / 30.0;

pub struct LevelSource {
    pub name: &'static str,
    pub data: &'static [u8],
}

/// Bundled levels, copied verbatim from the golgotha-forever repository.
pub const LEVELS: &[LevelSource] = &[
    LevelSource { name: "test", data: include_bytes!("../data/levels/test.level") },
    LevelSource { name: "snow", data: include_bytes!("../data/levels/snow.level") },
    LevelSource { name: "egypt2", data: include_bytes!("../data/levels/egypt2.level") },
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum View {
    /// Close top-down view following the supertank.
    Tactical,
    /// Whole map, the original's TAB view.
    Strategy,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShotKind {
    Shell,
    Bullet,
    Missile,
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub kind: ShotKind,
    pub x: f32,
    pub y: f32,
    /// Flight height in map cells; terrain above it stops the shot.
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub range: f32,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub hot: bool,
}

pub struct Game {
    pub level_index: usize,
    pub terrain: Terrain,
    pub tank: Tank,
    pub shots: Vec<Shot>,
    pub particles: Vec<Particle>,
    pub view: View,
    /// Index into `render::ZOOMS`.
    pub zoom: usize,
    /// Help window open; the simulation is paused while it is.
    pub help: bool,
    /// Map position of the tactical view's top-left corner; None until the first
    /// frame knows the screen size (see `render::follow_camera`).
    pub camera: Option<(f32, f32)>,
    pub time: f32,
    rng: u64,
}

impl Game {
    pub fn new(level_index: usize) -> Result<Game, String> {
        let src = &LEVELS[level_index];
        let level = Level::parse(src.data).map_err(|e| format!("{}: {e}", src.name))?;
        let terrain = Terrain::from_level(src.name, &level);
        let (x, y) = spawn_point(&terrain);
        log::info!("level {} {}x{} spawn ({x:.1},{y:.1})", src.name, terrain.w, terrain.h);
        Ok(Game {
            level_index,
            terrain,
            tank: Tank::new(x, y),
            shots: Vec::new(),
            particles: Vec::new(),
            view: View::Tactical,
            zoom: crate::render::DEFAULT_ZOOM,
            help: true,
            camera: None,
            time: 0.0,
            rng: 0x9E37_79B9_7F4A_7C15,
        })
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Step the tactical zoom; the camera re-centres on the tank.
    pub fn zoom_by(&mut self, step: i32) {
        let z = (self.zoom as i32 + step).clamp(0, crate::render::ZOOMS.len() as i32 - 1) as usize;
        if z != self.zoom {
            self.zoom = z;
            self.camera = None;
            log::info!("zoom {} cols per cell", crate::render::ZOOMS[z]);
        }
    }

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Tactical => View::Strategy,
            View::Strategy => View::Tactical,
        };
        log::info!("view {:?}", self.view);
    }

    pub fn update(&mut self, c: &Controls) {
        self.time += DT;
        let gear = self.tank.gear;
        self.tank.update(c, &self.terrain, DT);
        if self.tank.gear != gear {
            log::info!("gear {} -> {}", gear, self.tank.gear);
        }
        if c.fire_main && self.tank.main_cooldown == 0.0 && self.tank.main_ammo > 0 {
            self.tank.main_ammo -= 1;
            self.tank.main_cooldown = MAIN_RELOAD;
            self.tank.flash = 0.12;
            self.fire(ShotKind::Shell, 16.0, 14.0, 0.0);
        }
        if c.fire_chain && self.tank.chain_cooldown == 0.0 && self.tank.chain_ammo > 0 {
            self.tank.chain_ammo -= 1;
            self.tank.chain_cooldown = CHAIN_RELOAD;
            let spread = (self.rand() - 0.5) * 0.08;
            self.fire(ShotKind::Bullet, 24.0, 8.0, spread);
        }
        if c.fire_missile && self.tank.missiles > 0 {
            self.tank.missiles -= 1;
            self.fire(ShotKind::Missile, 9.0, 30.0, 0.0);
        }
        self.step_shots();
        self.step_particles();
    }

    fn fire(&mut self, kind: ShotKind, speed: f32, range: f32, spread: f32) {
        let t = &self.tank;
        let a = t.aim() + spread;
        let (x, y) = (t.x + a.cos() * 1.1, t.y + a.sin() * 1.1);
        let z = self.terrain.height(t.x, t.y) + 0.6;
        log::debug!("fire {kind:?} from ({x:.1},{y:.1}) aim {a:.2}");
        self.shots.push(Shot { kind, x, y, z, vx: a.cos() * speed, vy: a.sin() * speed, range });
    }

    fn step_shots(&mut self) {
        let mut hits = Vec::new();
        self.shots.retain_mut(|s| {
            s.x += s.vx * DT;
            s.y += s.vy * DT;
            s.range -= (s.vx * s.vx + s.vy * s.vy).sqrt() * DT;
            let ground = self.terrain.height(s.x, s.y);
            let off_map = self.terrain.cell_at(s.x, s.y).is_none();
            if off_map || ground > s.z || s.range <= 0.0 {
                if !off_map {
                    hits.push((s.kind, s.x, s.y));
                }
                return false;
            }
            true
        });
        for (kind, x, y) in hits {
            let (n, power) = match kind {
                ShotKind::Shell => (26, 1.0),
                ShotKind::Missile => (60, 1.8),
                ShotKind::Bullet => (2, 0.25),
            };
            self.explode(x, y, n, power);
        }
    }

    fn explode(&mut self, x: f32, y: f32, n: usize, power: f32) {
        for _ in 0..n {
            let a = self.rand() * std::f32::consts::TAU;
            let v = (0.3 + self.rand()) * 3.0 * power;
            let life = (0.3 + self.rand() * 0.7) * power.max(0.4);
            let hot = self.rand() < 0.6;
            self.particles.push(Particle { x, y, vx: a.cos() * v, vy: a.sin() * v, life, max_life: life, hot });
        }
    }

    fn step_particles(&mut self) {
        self.particles.retain_mut(|p| {
            p.x += p.vx * DT;
            p.y += p.vy * DT;
            p.vx *= 0.9;
            p.vy *= 0.9;
            p.life -= DT;
            p.life > 0.0
        });
    }
}

/// Open ground nearest the map centre. Real levels place the supertank at the
/// player's base; that waits on reading the object section.
fn spawn_point(t: &Terrain) -> (f32, f32) {
    let (cx, cy) = (t.w as i32 / 2, t.h as i32 / 2);
    for r in 0..t.w.max(t.h) as i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if open_ground(t, x, y, 3) {
                    return (x as f32 + 0.5, y as f32 + 0.5);
                }
            }
        }
    }
    (t.w as f32 / 2.0, t.h as f32 / 2.0)
}

/// No blocking or water cell within `r` cells of (x, y), all on the map.
fn open_ground(t: &Terrain, x: i32, y: i32, r: i32) -> bool {
    (-r..=r).all(|dy| {
        (-r..=r).all(|dx| {
            let (cx, cy) = (x + dx, y + dy);
            cx >= 0 && cy >= 0 && (cx as usize) < t.w && (cy as usize) < t.h && {
                let c = t.cell(cx as usize, cy as usize);
                !c.blocking && !c.water
            }
        })
    })
}
