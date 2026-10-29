//! The supertank: the unit the player drives directly.
//!
//! Numbers come from the original `scheme/balance.scm` row
//! `(stank 0.3 0.2 0.1 ...)`: speed, turn speed and acceleration per tick at the
//! original 10 Hz (`G1_HZ`), converted here to per second.

use crate::terrain::Terrain;

use crate::balance::ORIGINAL_HZ;
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

/// The 120mm's reload in `def_stank_weapons` is 5 ticks; also used for the
/// computer's supertank.
pub const MAIN_RELOAD: f32 = 0.5;
pub const CHAIN_RELOAD: f32 = 0.08;

/// `upgrade_levels` from the original `scheme/start.scm`: main gun, rocket,
/// machine gun, armour per supertank upgrade level.
pub const UPGRADE_LEVELS: [(&str, &str, &str, &str); 3] = [
    ("b120mm", "heavy_rocket", "chain_gun", "kevlar"),
    ("acid", "vortex_missile", "chain_gun", "reactive"),
    ("napalm", "nuke_missile", "chain_gun", "titanium"),
];

/// Ammunition (or armour hit points) for a supertank weapon, from the table.
pub fn ammo_max(weapon: &str) -> u32 {
    crate::balance::stank_weapon(weapon).map_or(0, |w| w.max.max(0) as u32)
}

/// Seconds between shots of a supertank weapon (at least one tick).
pub fn reload(weapon: &str) -> f32 {
    crate::balance::stank_weapon(weapon).map_or(1.0, |w| w.fire_delay.max(1.0) / ORIGINAL_HZ)
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    /// -1 left .. 1 right (keys give -1, 0, 1; a stick anything between).
    pub turn: f32,
    /// Turret rotation rate, -1 .. 1.
    pub turret: f32,
    pub gear_step: i32,
    /// Analog throttle from a gamepad, -1 .. 1; overrides the gear while set.
    pub throttle: Option<f32>,
    /// Absolute turret aim (map angle) from a gamepad's right stick.
    pub aim: Option<f32>,
    /// Swing the turret back to straight ahead (C, or the left stick click).
    pub center: bool,
    /// Distance to the mouse pointer when the mouse aims: shells land there.
    pub aim_range: Option<f32>,
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
    /// Upgrade level this supertank was built at, and its weapons.
    pub level: usize,
    pub main_weapon: &'static str,
    pub rocket_weapon: &'static str,
    pub armor: &'static str,
    pub main_ammo: u32,
    pub chain_ammo: u32,
    pub missiles: u32,
    pub main_cooldown: f32,
    pub chain_cooldown: f32,
    pub rocket_cooldown: f32,
    /// Turret swinging back to straight ahead until it gets there.
    pub centering: bool,
    /// Set when the tank stopped against a wall, slope or the map edge this tick.
    pub blocked: bool,
    /// Distance rolled, signed; drives the tread animation.
    pub odometer: f32,
    /// Seconds of muzzle flash left after a main gun shot.
    pub flash: f32,
}

impl Tank {
    pub fn new(x: f32, y: f32, level: usize) -> Tank {
        let level = level.min(UPGRADE_LEVELS.len() - 1);
        let (main_weapon, rocket_weapon, chain, armor) = UPGRADE_LEVELS[level];
        Tank {
            level,
            main_weapon,
            rocket_weapon,
            armor,
            x,
            y,
            heading: 0.0,
            turret: 0.0,
            speed: 0.0,
            gear: 1,
            main_ammo: ammo_max(main_weapon),
            chain_ammo: ammo_max(chain),
            missiles: ammo_max(rocket_weapon),
            main_cooldown: 0.0,
            chain_cooldown: 0.0,
            rocket_cooldown: 0.0,
            centering: false,
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

    /// `bridged` says whether a point is on a bridge deck, which carries the
    /// tank over blocking ground.
    pub fn update(&mut self, c: &Controls, terrain: &Terrain, bridged: impl Fn(f32, f32) -> bool, dt: f32) {
        if c.gear_step != 0 {
            self.gear = (self.gear as i32 + c.gear_step).clamp(0, GEARS.len() as i32 - 1) as usize;
        }
        // reversing turns the hull the way a car does
        let dir = if self.speed < -0.05 { -1.0 } else { 1.0 };
        self.heading = (self.heading + c.turn * TURN_SPEED * dt * dir).rem_euclid(std::f32::consts::TAU);
        if c.center {
            self.centering = true;
        }
        if c.turret != 0.0 || c.aim.is_some() {
            self.centering = false;
        }
        if self.centering {
            // turret angle as -pi..pi, so it turns the short way back to zero
            let diff = (self.turret + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let step = TURRET_SPEED * dt;
            if diff.abs() <= step {
                self.turret = 0.0;
                self.centering = false;
            } else {
                // shortest way round to zero
                self.turret = (self.turret - diff.signum() * step).rem_euclid(std::f32::consts::TAU);
            }
        } else if let Some(aim) = c.aim {
            // swing the turret toward a world direction at its own speed
            let want = aim - self.heading;
            let diff = (want - self.turret + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let step = TURRET_SPEED * dt;
            self.turret = (self.turret + diff.clamp(-step, step)).rem_euclid(std::f32::consts::TAU);
        } else {
            self.turret = (self.turret + c.turret * TURRET_SPEED * dt).rem_euclid(std::f32::consts::TAU);
        }

        let target = match c.throttle {
            // reverse is as slow as the R gear
            Some(t) => t.clamp(GEARS[0], 1.0) * TOP_SPEED,
            None => GEARS[self.gear] * TOP_SPEED,
        };
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
            Some(cell) => !cell.blocking || bridged(probe.0, probe.1),
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
        self.rocket_cooldown = (self.rocket_cooldown - dt).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::LEVELS;
    use crate::level::Level;

    #[test]
    fn centering_brings_the_turret_straight_ahead_the_short_way() {
        let level = Level::parse(LEVELS[0].data).unwrap();
        let terrain = Terrain::from_level("test", &level);
        for start in [0.5f32, 5.8, 3.0] {
            let mut t = Tank::new(50.0, 75.0, 0);
            t.turret = start;
            let c = Controls { center: true, ..Default::default() };
            t.update(&c, &terrain, |_, _| false, 1.0 / 30.0);
            let after_one = t.turret;
            let idle = Controls::default();
            for _ in 0..90 {
                t.update(&idle, &terrain, |_, _| false, 1.0 / 30.0);
            }
            assert_eq!(t.turret, 0.0, "from {start}");
            // the first step went toward zero, not away
            let dist = |a: f32| a.min(std::f32::consts::TAU - a);
            assert!(dist(after_one) < dist(start), "from {start}: {after_one}");
        }
    }
}
