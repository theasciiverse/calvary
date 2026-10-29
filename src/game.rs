//! World state and the fixed-step simulation: the supertank, its shots and the
//! explosion particles. Units, bases and paths join later (see CLAUDE.md plan).

use crate::level::Level;
use crate::tank::{self, Controls, Tank, CHAIN_RELOAD};
use crate::terrain::Terrain;

/// Open water where sea ice broke: the original's `seawaterfull2`.
const ICE_WATER: crate::terrain::Rgb = (25, 98, 140);
use crate::world::{Class, Command, Outcome, Team, World, BUILDABLE};

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
    /// Third person from behind the supertank, the original's follow mode.
    Chase,
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
    /// Weapon name in the balance table.
    pub weapon: &'static str,
    pub x: f32,
    pub y: f32,
    /// Flight height in map cells; terrain above it stops the shot.
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub range: f32,
    /// Ballistic flight: launch height, straight-line climb to the landing
    /// height over `span` cells, plus an arc peaking `apex` cells above it.
    pub z0: f32,
    pub climb: f32,
    pub span: f32,
    pub apex: f32,
    pub flown: f32,
}

impl Shot {
    /// A shot that is only drawn (film playback).
    pub fn shown(x: f32, y: f32, z: f32) -> Shot {
        Shot { kind: ShotKind::Shell, weapon: "", x, y, z, vx: 0.0, vy: 0.0, range: 0.0, z0: z, climb: 0.0, span: 1.0, apex: 0.0, flown: 0.0 }
    }
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
    /// Powder snow from an avalanche.
    pub snow: bool,
}

pub struct Game {
    pub level_index: usize,
    pub terrain: Terrain,
    pub tank: Tank,
    pub world: World,
    /// Turret lock: the turret tracks the nearest enemy in gun range.
    pub lock: bool,
    lock_on: Option<usize>,
    /// Connected gamepad's name and its build cursor, for the HUD.
    pub pad: Option<(String, usize)>,
    /// The player's selected units (world entity indexes).
    pub selected: Vec<usize>,
    /// Box selection being dragged: screen cells where it started and is now.
    pub drag: Option<((u16, u16), (u16, u16))>,
    /// The map editor uses the game's view without a supertank on it.
    pub hide_tank: bool,
    /// Camera look-ahead in map cells: eased toward a point ahead of the
    /// supertank, further the faster it drives (see `render::follow_camera`).
    pub look: (f32, f32),
    /// The follow camera's heading, easing after the tank's.
    pub chase_heading: f32,
    /// The view to return to when the strategy map closes.
    prev_view: View,
    /// Mouse aim point on screen, for the crosshair.
    pub crosshair: Option<(u16, u16)>,
    /// Follow mode marks where the turret points (off for the credits' battle).
    pub show_aim: bool,
    /// The explosions of the last tick (for films).
    pub last_blasts: Vec<crate::world::Blast>,
    /// A film is being recorded (F12): the HUD says so.
    pub recording: bool,
    /// A campaign mission: no level keys, and the battle ends by itself.
    pub campaign: bool,
    /// Distance to the mouse aim point: shells land there.
    aim_range: Option<f32>,
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
        let mut game = Game::from_level(src.name, &level);
        game.level_index = level_index;
        Ok(game)
    }

    /// A game on any parsed level: a bundled original or a `.cmap`.
    pub fn from_level(name: &str, level: &Level) -> Game {
        let level_index = 0;
        let terrain = Terrain::from_level(name, level);
        let world = World::from_level(level);
        // the level's own supertank start, else open ground near the centre
        let start = world.player_tank.map(|i| (world.entities[i].x, world.entities[i].y, world.entities[i].heading));
        let (cx, cy) = start.map_or((terrain.w as f32 / 2.0, terrain.h as f32 / 2.0), |s| (s.0, s.1));
        let (x, y) = spawn_point(&terrain, cx, cy);
        log::info!("level {} {}x{} spawn ({x:.1},{y:.1})", name, terrain.w, terrain.h);
        let mut tank = Tank::new(x, y, 0);
        tank.heading = start.map_or(0.0, |s| s.2);
        let mut game = Game {
            level_index,
            terrain,
            tank,
            world,
            lock: false,
            lock_on: None,
            pad: None,
            selected: Vec::new(),
            drag: None,
            hide_tank: false,
            look: (0.0, 0.0),
            chase_heading: 0.0,
            prev_view: View::Tactical,
            crosshair: None,
            show_aim: true,
            last_blasts: Vec::new(),
            recording: false,
            campaign: false,
            aim_range: None,
            shots: Vec::new(),
            particles: Vec::new(),
            view: View::Tactical,
            zoom: crate::render::DEFAULT_ZOOM,
            help: true,
            camera: None,
            time: 0.0,
            rng: 0x9E37_79B9_7F4A_7C15,
        };
        game.arm_player_entity();
        game
    }

    /// The supertank's world entity takes its hit points from the armour.
    fn arm_player_entity(&mut self) {
        let armor = tank::ammo_max(self.tank.armor) as f32;
        if let Some(i) = self.world.player_tank {
            let e = &mut self.world.entities[i];
            e.max_hp = armor;
            e.hp = armor;
        }
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

    /// Tab: the strategy map, and back to whichever view was before it.
    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Strategy => self.prev_view,
            v => {
                self.prev_view = v;
                View::Strategy
            }
        };
        log::info!("view {:?}", self.view);
    }

    /// M: top-down and follow mode, in turn.
    pub fn cycle_view(&mut self) {
        self.view = match self.view {
            View::Tactical => {
                self.chase_heading = self.tank.heading;
                View::Chase
            }
            _ => View::Tactical,
        };
        self.camera = None;
        log::info!("view {:?}", self.view);
    }

    /// The build slots (indexes into `BUILDABLE`) the player's factories
    /// offer on this map.
    pub fn build_slots(&self) -> Vec<usize> {
        (0..BUILDABLE.len()).filter(|&i| self.world.offers(Team::Player, BUILDABLE[i])).collect()
    }

    /// Order build slot `slot` (`BUILD_KEYS`) from the player's factories.
    pub fn order(&mut self, slot: usize) {
        let Some(&kind) = BUILDABLE.get(slot) else { return };
        if !self.world.order(Team::Player, kind) {
            let (factory, name) = if crate::world::flies(kind) { (Class::Airbase, "no airbase") } else { (Class::Garage, "no garage") };
            let why = if !self.world.can_build(kind) {
                "not on this map"
            } else if self.world.find(factory, Team::Player).is_none() {
                name
            } else {
                "not enough money"
            };
            self.world.say(format!("Cannot build {}: {why}", kind.replace('_', " ")));
        }
    }

    pub fn toggle_lock(&mut self) {
        self.lock = !self.lock;
        self.lock_on = None;
        log::info!("turret lock {}", self.lock);
    }

    /// Name of the locked target, if the lock has one.
    pub fn lock_target(&self) -> Option<&str> {
        self.lock_on.filter(|_| self.lock).map(|i| self.world.entities[i].kind.as_str())
    }

    /// Lock on: the nearest live enemy within gun range (14 cells) that the
    /// player can see (not in the fog, nor lost in the dark); the
    /// swing the turret at it at the turret's own speed.
    fn track_target(&mut self) {
        if !self.lock {
            return;
        }
        let (tx, ty) = (self.tank.x, self.tank.y);
        let w = &self.world;
        let valid = |i: usize| w.entities[i].is_target() && w.entities[i].team.hostile(Team::Player) && (w.entities[i].x - tx).hypot(w.entities[i].y - ty) <= 14.0 && w.seen(&w.entities[i]);
        // always the nearest: a tank closing in matters more than a building
        self.lock_on = (0..w.entities.len()).filter(|&i| valid(i)).min_by(|&a, &b| {
            let d = |i: usize| (w.entities[i].x - tx).hypot(w.entities[i].y - ty);
            d(a).total_cmp(&d(b))
        });
        if let Some(i) = self.lock_on {
            let e = &self.world.entities[i];
            let want = (e.y - ty).atan2(e.x - tx) - self.tank.heading;
            let diff = (want - self.tank.turret + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let step = 3.0 * DT;
            self.tank.turret = (self.tank.turret + diff.clamp(-step, step)).rem_euclid(std::f32::consts::TAU);
        }
    }

    /// Drop destroyed units from the selection.
    pub fn prune_selection(&mut self) {
        let w = &self.world;
        self.selected.retain(|&i| w.entities[i].alive);
    }

    pub fn select(&mut self, units: Vec<usize>) {
        self.selected = units;
        if !self.selected.is_empty() {
            let n = self.selected.len();
            let first = self.world.entities[self.selected[0]].kind.replace('_', " ");
            self.world.say(if n == 1 { format!("Selected: {first}") } else { format!("Selected {n} units") });
        }
    }

    /// Select the player's units within `r` cells of the supertank.
    pub fn select_near_tank(&mut self, r: f32) {
        let (x, y) = (self.tank.x, self.tank.y);
        let near = self.world.player_units_in((x - r, y - r), (x + r, y + r));
        if near.is_empty() {
            self.world.say(format!("No units within {r:.0} cells"));
        }
        self.select(near);
    }

    /// Select the next (or previous) of the player's units, one at a time.
    pub fn cycle_selection(&mut self, step: i32) {
        let units = self.world.player_units_in((f32::MIN, f32::MIN), (f32::MAX, f32::MAX));
        if units.is_empty() {
            self.world.say("No units on the field");
            return;
        }
        let at = self.selected.first().and_then(|s| units.iter().position(|u| u == s));
        let next = match at {
            Some(k) => (k as i32 + step).rem_euclid(units.len() as i32) as usize,
            None => 0,
        };
        self.select(vec![units[next]]);
    }

    pub fn command(&mut self, cmd: Command) {
        if self.selected.is_empty() {
            self.world.say("No units selected (U selects units near the supertank)");
            return;
        }
        let sel = self.selected.clone();
        self.world.command(&sel, cmd);
        let what = match cmd {
            Command::Hold => "hold position".to_string(),
            Command::Move(..) => "on the move".to_string(),
            Command::Attack(t) => format!("attacking {}", self.world.entities[t].kind.replace('_', " ")),
            Command::Resume => "back on the route".to_string(),
        };
        self.world.say(format!("{} unit{}: {what}", sel.len(), if sel.len() == 1 { "" } else { "s" }));
    }

    /// "Rally on me": selected units drive to the supertank.
    pub fn rally(&mut self) {
        self.command(Command::Move(self.tank.x, self.tank.y));
    }

    /// Supertank health as a fraction, from its world entity.
    pub fn tank_health(&self) -> f32 {
        self.world.player_tank.map_or(1.0, |i| {
            let e = &self.world.entities[i];
            (e.hp / e.max_hp).clamp(0.0, 1.0)
        })
    }

    pub fn update(&mut self, c: &Controls) {
        self.time += DT;
        if self.world.outcome != Outcome::Playing {
            self.step_shots();
            self.step_particles();
            return;
        }
        self.aim_range = c.aim_range;
        // the follow camera swings round after the hull, not with it
        let d = (self.tank.heading - self.chase_heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.chase_heading += d * 0.08;
        let gear = self.tank.gear;
        let (px, py) = (self.tank.x, self.tank.y);
        let was_inside = self.world.blocks(px, py, 0.6);
        let world = &self.world;
        self.tank.update(c, &self.terrain, |x, y| world.on_bridge(x, y), DT);
        // buildings stop the supertank like rock does (a tank already inside
        // one, e.g. spawned there, may drive out)
        if !was_inside && self.world.blocks(self.tank.x, self.tank.y, 0.6) {
            self.tank.x = px;
            self.tank.y = py;
            self.tank.speed = 0.0;
            self.tank.blocked = true;
        }
        if self.tank.gear != gear {
            log::info!("gear {} -> {}", gear, self.tank.gear);
        }
        if c.fire_main && self.tank.main_cooldown == 0.0 && self.tank.main_ammo > 0 {
            self.tank.main_ammo -= 1;
            self.tank.main_cooldown = tank::reload(self.tank.main_weapon);
            self.tank.flash = 0.12;
            let sound = match self.tank.main_weapon {
                "acid" => "fire/supertank_acid_main_barrel_interrior_22khz.mp3",
                "napalm" => "fire/supertank_plazma_fireball_22khz.mp3",
                _ => "fire/supertank/main1.mp3",
            };
            self.world.sound(sound, None, 0.8);
            self.fire(ShotKind::Shell, self.tank.main_weapon, 16.0, 14.0, 0.0);
        }
        if c.fire_chain && self.tank.chain_cooldown == 0.0 && self.tank.chain_ammo > 0 {
            self.tank.chain_ammo -= 1;
            self.tank.chain_cooldown = CHAIN_RELOAD;
            let spread = (self.rand() - 0.5) * 0.08;
            self.world.sound("fire/machine_gun_fire_22khz_lp.wav", None, 0.5);
            self.fire(ShotKind::Bullet, "chain_gun", 24.0, 8.0, spread);
        }
        if c.fire_missile && self.tank.missiles > 0 && self.tank.rocket_cooldown == 0.0 {
            self.tank.missiles -= 1;
            self.tank.rocket_cooldown = tank::reload(self.tank.rocket_weapon);
            self.world.sound("fire/supertank_fires_rocket_22khz.mp3", None, 0.8);
            self.fire(ShotKind::Missile, self.tank.rocket_weapon, 9.0, 30.0, 0.0);
        }
        if c.center && self.lock {
            // straight ahead means not tracking anything
            self.toggle_lock();
        }
        if c.turret == 0.0 && c.aim.is_none() && !self.tank.centering {
            self.track_target();
        }
        self.world.place_player_tank(self.tank.x, self.tank.y, self.tank.heading);
        self.step_shots();
        let lives = self.world.lives;
        self.world.step(DT);
        if self.world.lives < lives && self.world.outcome == Outcome::Playing {
            self.respawn();
        }
        self.sync_terrain();
        self.refill_at_base();
        self.last_blasts = std::mem::take(&mut self.world.blasts);
        for b in self.last_blasts.clone() {
            self.blast(b.x, b.y, b.power, b.snow);
        }
        self.step_particles();
    }

    /// Carry what changed on the ground into the terrain the views draw:
    /// the fog lifting, and sea ice cracking and breaking.
    pub fn sync_terrain(&mut self) {
        if self.world.fog_changed {
            self.world.fog_changed = false;
            self.terrain.set_fog(self.world.fog.as_deref().unwrap_or(&[]));
        }
        for (i, left) in std::mem::take(&mut self.world.ice_hits) {
            if left <= 0.0 {
                self.terrain.break_ice(i, ICE_WATER);
            } else {
                self.terrain.crack(i, left);
            }
        }
    }

    /// An explosion's particles, sized by its power (or a burst of powder
    /// snow, for an avalanche).
    pub fn blast(&mut self, x: f32, y: f32, power: f32, snow: bool) {
        self.explode(x, y, (14.0 * power) as usize + 4, power, snow);
    }

    /// A new supertank rolls out next to the HQ.
    fn respawn(&mut self) {
        let (hx, hy) = self.world.find(Class::Hq, Team::Player).map_or((self.tank.x, self.tank.y), |i| (self.world.entities[i].x, self.world.entities[i].y));
        let (x, y) = spawn_point(&self.terrain, hx + 2.5, hy);
        let heading = self.tank.heading;
        let level = self.world.upgrade_level();
        self.tank = Tank::new(x, y, level);
        self.tank.heading = heading;
        self.arm_player_entity();
        if level > 0 {
            self.world.say(format!("Level {level} supertank rolls out: {}, {}", self.tank.main_weapon, self.tank.rocket_weapon.replace('_', " ")));
        }
        self.world.place_player_tank(x, y, heading);
        log::info!("supertank respawn at ({x:.1},{y:.1}), lives {}", self.world.lives);
    }

    /// Parked at the HQ, the supertank is repaired and rearmed (the original
    /// refills ammo and health inside your headquarters).
    fn refill_at_base(&mut self) {
        let Some(hq) = self.world.find(Class::Hq, Team::Player) else { return };
        let e = &self.world.entities[hq];
        if (e.x - self.tank.x).hypot(e.y - self.tank.y) > 3.5 {
            return;
        }
        let t = &mut self.tank;
        let tick = ((self.time * 4.0) as i32) != (((self.time - DT) * 4.0) as i32);
        if tick {
            t.main_ammo = (t.main_ammo + 1).min(tank::ammo_max(t.main_weapon));
            t.chain_ammo = (t.chain_ammo + 5).min(tank::ammo_max("chain_gun"));
            if t.missiles < tank::ammo_max(t.rocket_weapon) && (self.time as i32) % 5 == 0 {
                t.missiles += 1;
            }
            if let Some(i) = self.world.player_tank {
                let me = &mut self.world.entities[i];
                me.hp = (me.hp + me.max_hp * 0.02).min(me.max_hp);
            }
        }
    }

    fn fire(&mut self, kind: ShotKind, weapon: &'static str, speed: f32, range: f32, spread: f32) {
        let t = &self.tank;
        let a = t.aim() + spread;
        let (x, y) = (t.x + a.cos() * 1.1, t.y + a.sin() * 1.1);
        let z = self.terrain.height(t.x, t.y) + 0.6;
        log::debug!("fire {kind:?} from ({x:.1},{y:.1}) aim {a:.2}");
        // shells and missiles fly an arc to the locked target, or to full range;
        // bullets fly flat
        let (span, land) = self
            .lock_on
            .filter(|_| self.lock && kind != ShotKind::Bullet)
            .map(|i| {
                let e = &self.world.entities[i];
                ((e.x - x).hypot(e.y - y).clamp(1.0, range), self.terrain.height(e.x, e.y) + 0.4)
            })
            .unwrap_or_else(|| {
                // to the mouse pointer when it aims, else full range
                let reach = if kind == ShotKind::Bullet { range } else { self.aim_range.map_or(range, |r| r.clamp(1.0, range)) };
                let (ex, ey) = (x + a.cos() * reach, y + a.sin() * reach);
                (reach, self.terrain.height(ex, ey).max(z - 1.0))
            });
        let climb = (land - z) / span;
        let apex = if kind == ShotKind::Bullet { 0.0 } else { span * 0.18 };
        let range = if kind == ShotKind::Bullet { range } else { span + 0.5 };
        self.shots.push(Shot { kind, weapon, x, y, z, vx: a.cos() * speed, vy: a.sin() * speed, range, z0: z, climb, span, apex, flown: 0.0 });
    }

    fn step_shots(&mut self) {
        let mut hits = Vec::new();
        let (terrain, world) = (&self.terrain, &self.world);
        self.shots.retain_mut(|s| {
            s.x += s.vx * DT;
            s.y += s.vy * DT;
            let flown = (s.vx * s.vx + s.vy * s.vy).sqrt() * DT;
            s.range -= flown;
            s.flown += flown;
            let f = (s.flown / s.span).min(1.0);
            s.z = s.z0 + s.climb * s.flown + 4.0 * s.apex * f * (1.0 - f);
            let ground = terrain.height(s.x, s.y);
            let off_map = terrain.cell_at(s.x, s.y).is_none();
            let struck = world.hit_test(s.x, s.y, Team::Player);
            if struck.is_some() || off_map || ground > s.z || s.range <= 0.0 {
                if !off_map {
                    hits.push((s.kind, s.weapon, s.x, s.y, struck));
                }
                return false;
            }
            true
        });
        for (kind, weapon, x, y, struck) in hits {
            let boom = match weapon {
                "acid" => "explosion/acid.mp3",
                "nuke_missile" | "vortex_missile" => "explosion/super_mortar.mp3",
                _ if kind == ShotKind::Bullet => "",
                _ => "explosion/old_generic.mp3",
            };
            if !boom.is_empty() {
                self.world.sound(boom, Some((x, y)), 0.7);
            }
            log::debug!(
                "impact {kind:?} at ({x:.1},{y:.1}) struck {:?} ground {:.2}",
                struck.map(|i| &self.world.entities[i].kind),
                self.terrain.height(x, y)
            );
            if self.world.outcome == Outcome::Playing {
                self.world.damage_by(weapon, x, y, struck, Team::Player, true);
            }
            let (n, power) = match kind {
                ShotKind::Shell => (26, 1.0),
                ShotKind::Missile => (60, 1.8),
                ShotKind::Bullet => (2, 0.25),
            };
            self.explode(x, y, n, power, false);
        }
    }

    fn explode(&mut self, x: f32, y: f32, n: usize, power: f32, snow: bool) {
        for _ in 0..n {
            let a = self.rand() * std::f32::consts::TAU;
            let v = (0.3 + self.rand()) * 3.0 * power;
            let life = (0.3 + self.rand() * 0.7) * power.max(0.4);
            let hot = !snow && self.rand() < 0.6;
            self.particles.push(Particle { x, y, vx: a.cos() * v, vy: a.sin() * v, life, max_life: life, hot, snow });
        }
    }

    pub fn step_particles(&mut self) {
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

/// Open ground nearest (x, y).
fn spawn_point(t: &Terrain, x: f32, y: f32) -> (f32, f32) {
    let (cx, cy) = (x as i32, y as i32);
    for r in 0..t.w.max(t.h) as i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if open_ground(t, x, y, 1) {
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
