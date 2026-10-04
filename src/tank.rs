//! The supertank: the unit the player drives directly.
//!
//! Numbers come from the original `scheme/balance.scm` row
//! `(stank 0.3 0.2 0.1 ...)`: speed, turn speed and acceleration per tick at the
//! original 10 Hz (`G1_HZ`), converted here to per second.

use crate::terrain::Terrain;

const ORIGINAL_HZ: f32 = 10.0;
pub const TOP_SPEED: f32 = 0.3 * ORIGINAL_HZ; // cells per second
const TURN_SPEED: f32 = 0.2 * ORIGINAL_HZ; // radians per second
const ACCEL: f32 = 0.1 * ORIGINAL_HZ * ORIGINAL_HZ; // cells per second squared
const TURRET_SPEED: f32 = 2.5;
/// Throttle is a sticky gear rather than a held key, so driving works the same on
/// terminals that never report key release. Index 1 is stop.
pub const GEARS: [f32; 5] = [-0.4, 0.0, 0.35, 0.7, 1.0];
/// Grade (height per cell) at which the tank can no longer climb.
const MAX_GRADE: f32 = 1.6;
const RADIUS: f32 = 0.6;

pub const MAIN_RELOAD: f32 = 0.9;
pub const CHAIN_RELOAD: f32 = 0.08;

#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    /// -1 left, 1 right.
    pub turn: i8,
    pub turret: i8,
    pub gear_step: i32,
    pub fire_main: bool,
    pub fire_chain: bool,
    pub fire_missile: bool,
}

#[derive(Clone, Debug)]
pub struct Tank {
    pub x: f32,
    pub y: f32,
    /// Hull heading in radians, 0 = east (+x), positive turns towards +y.
    pub heading: f32,
    /// Turret angle relative to the hull.
    pub turret: f32,
    pub speed: f32,
    pub gear: usize,
    pub main_ammo: u32,
    pub chain_ammo: u32,
    pub missiles: u32,
    pub main_cooldown: f32,
    pub chain_cooldown: f32,
    /// Set when the tank stopped against a wall, slope or the map edge this tick.
    pub blocked: bool,
    /// Distance rolled, signed; drives the tread animation.
    pub odometer: f32,
    /// Seconds of muzzle flash left after a main gun shot.
    pub flash: f32,
}

impl Tank {
    pub fn new(x: f32, y: f32) -> Tank {
        Tank {
            x,
            y,
            heading: 0.0,
            turret: 0.0,
            speed: 0.0,
            gear: 1,
            main_ammo: 40,
            chain_ammo: 600,
            missiles: 2,
            main_cooldown: 0.0,
            chain_cooldown: 0.0,
            blocked: false,
            odometer: 0.0,
            flash: 0.0,
        }
    }

    pub fn aim(&self) -> f32 {
        self.heading + self.turret
    }

    pub fn gear_label(&self) -> &'static str {
        ["R", "N", "1", "2", "3"][self.gear]
    }

    pub fn update(&mut self, c: &Controls, terrain: &Terrain, dt: f32) {
        if c.gear_step != 0 {
            self.gear = (self.gear as i32 + c.gear_step).clamp(0, GEARS.len() as i32 - 1) as usize;
        }
        // reversing turns the hull the way a car does
        let dir = if self.speed < -0.05 { -1.0 } else { 1.0 };
        self.heading = (self.heading + c.turn as f32 * TURN_SPEED * dt * dir).rem_euclid(std::f32::consts::TAU);
        self.turret = (self.turret + c.turret as f32 * TURRET_SPEED * dt).rem_euclid(std::f32::consts::TAU);

        let target = GEARS[self.gear] * TOP_SPEED;
        let dv = (target - self.speed).clamp(-ACCEL * dt, ACCEL * dt);
        self.speed += dv;

        let (dx, dy) = (self.heading.cos(), self.heading.sin());
        let step = self.speed * dt;
        let (nx, ny) = (self.x + dx * step, self.y + dy * step);
        // grade along the direction of travel
        let (sx, sy) = terrain.slope(nx, ny);
        let grade = (sx * dx + sy * dy) * self.speed.signum();
        let climb = (1.0 - grade.max(0.0) / MAX_GRADE).clamp(0.0, 1.0);
        let (nx, ny) = (self.x + dx * step * climb, self.y + dy * step * climb);
        let probe = (nx + dx * RADIUS * self.speed.signum(), ny + dy * RADIUS * self.speed.signum());
        let free = match terrain.cell_at(probe.0, probe.1) {
            Some(cell) => !cell.blocking,
            None => false,
        };
        self.blocked = !free || climb == 0.0;
        if free {
            self.odometer += ((nx - self.x).powi(2) + (ny - self.y).powi(2)).sqrt() * self.speed.signum();
            self.x = nx;
            self.y = ny;
        } else {
            self.speed = 0.0;
        }
        self.main_cooldown = (self.main_cooldown - dt).max(0.0);
        self.flash = (self.flash - dt).max(0.0);
        self.chain_cooldown = (self.chain_cooldown - dt).max(0.0);
    }
}
