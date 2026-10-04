//! Everything on the battlefield except the player's own supertank physics:
//! structures and scenery from the level, the path network, units driving it,
//! combat, money and the computer player.
//!
//! Rules follow the original data: `balance.scm` for unit and weapon numbers,
//! `scheme/start.scm` (10000 starting money, 3 supertank lives) and
//! `scheme/classes.scm` (banks pay a 2000 crate every 500 ticks, garages
//! deploy one unit per 10 ticks). Player 1 in the level is the human (team A,
//! follows the A links), player 2 the computer (team E), player 0 neutral.

use crate::balance::{self, ORIGINAL_HZ};
use crate::level::Level;
use std::collections::{HashMap, VecDeque};

const START_MONEY: i32 = 10000;
pub const START_LIVES: u32 = 3;
const BANK_CRATE: i32 = 2000;
const BANK_INTERVAL: f32 = 500.0 / ORIGINAL_HZ;
const DEPLOY_INTERVAL: f32 = 10.0 / ORIGINAL_HZ;
const NODE_REACHED: f32 = 0.35;
const CAPTURE_RANGE: f32 = 2.0;
/// Instant weapons (speed 0 in the table) draw their beam this long.
const BEAM_TIME: f32 = 0.15;

/// What the garage offers, in build-key order (keys 1-8).
pub const BUILDABLE: [&str; 8] = ["peon_tank", "electric_car", "rocket_tank", "trike", "tank_buster", "engineer", "bomb_truck", "cobra_tank"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
    Scenery,
}

impl Team {
    fn from_player(p: u8) -> Team {
        match p {
            1 => Team::Player,
            2 => Team::Enemy,
            0 => Team::Neutral,
            _ => Team::Scenery,
        }
    }

    pub fn hostile(self, other: Team) -> bool {
        matches!((self, other), (Team::Player, Team::Enemy) | (Team::Enemy, Team::Player))
    }

    fn index(self) -> usize {
        match self {
            Team::Player => 0,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Hq,
    Garage,
    Airbase,
    Bank,
    Tower,
    /// A moving unit from the garage (or placed in the level).
    Vehicle,
    /// The player's supertank: position mirrored from `Tank` every tick.
    PlayerTank,
    /// The computer's supertank, guarding its base.
    EnemyTank,
    Prop,
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub kind: String,
    pub class: Class,
    pub team: Team,
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub turret: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub alive: bool,
    pub cooldown: f32,
    pub target: Option<usize>,
    /// Path node this unit is driving to.
    pub next_node: Option<usize>,
    pub odometer: f32,
    /// Seconds of hit flash left.
    pub flash: f32,
    home: (f32, f32),
}

impl Entity {
    pub fn radius(&self) -> f32 {
        match self.class {
            Class::Hq => 1.6,
            Class::Garage | Class::Airbase | Class::Bank => 1.2,
            Class::Tower => 0.7,
            Class::PlayerTank | Class::EnemyTank => 0.9,
            Class::Vehicle => 0.5,
            Class::Prop => 0.5,
        }
    }

    pub fn is_target(&self) -> bool {
        self.alive && self.class != Class::Prop
    }

    fn stats(&self) -> Option<&'static balance::UnitStats> {
        balance::unit(&self.kind)
    }

    fn weapon(&self) -> Option<&'static str> {
        match self.class {
            // the supertanks' weapon slot is nil in the table; they carry the 120mm
            Class::EnemyTank => Some("b120mm"),
            Class::PlayerTank => None,
            _ => self.stats().and_then(|s| s.weapon.as_deref()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PathNode {
    pub x: f32,
    pub y: f32,
    pub links_a: Vec<usize>,
    pub links_e: Vec<usize>,
}

impl PathNode {
    pub fn links(&self, team: Team) -> &[usize] {
        if team == Team::Enemy { &self.links_e } else { &self.links_a }
    }
}

/// A shot fired by a unit or structure. It flies to its target point and does
/// its damage on arrival; guided missiles keep steering at their target.
#[derive(Clone, Debug)]
pub struct Bolt {
    pub weapon: &'static str,
    pub team: Team,
    pub x: f32,
    pub y: f32,
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub target: Option<usize>,
    pub speed: f32,
    /// Remaining display time for instant beams.
    pub beam: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Blast {
    pub x: f32,
    pub y: f32,
    pub power: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

pub struct World {
    pub entities: Vec<Entity>,
    pub nodes: Vec<PathNode>,
    pub bolts: Vec<Bolt>,
    /// Explosions for the renderer's particle system, drained by `Game`.
    pub blasts: Vec<Blast>,
    pub money: [i32; 2],
    pub lives: u32,
    pub player_tank: Option<usize>,
    pub messages: VecDeque<(f32, String)>,
    pub outcome: Outcome,
    /// Junction choices: path node -> which of its links new units take.
    pub route_choice: HashMap<usize, usize>,
    queues: [VecDeque<&'static str>; 2],
    deploy_timer: [f32; 2],
    bank_timer: f32,
    ai_timer: f32,
    time: f32,
    rng: u64,
}

impl World {
    pub fn from_level(level: &Level) -> World {
        let mut node_of = HashMap::new();
        let mut nodes = Vec::new();
        for (i, o) in level.objects.iter().enumerate() {
            if o.kind == "path_object" {
                node_of.insert(i, nodes.len());
                nodes.push(PathNode { x: o.x, y: o.y, links_a: Vec::new(), links_e: Vec::new() });
            }
        }
        for (i, o) in level.objects.iter().enumerate() {
            if let Some(&n) = node_of.get(&i) {
                nodes[n].links_a = o.links_a.iter().filter_map(|l| node_of.get(l).copied()).collect();
                nodes[n].links_e = o.links_e.iter().filter_map(|l| node_of.get(l).copied()).collect();
            }
        }

        let mut entities = Vec::new();
        let mut player_tank = None;
        for o in &level.objects {
            let team = Team::from_player(o.player);
            let class = match o.kind.as_str() {
                "" | "path_object" | "field_camera" | "sfx_obj" | "lightbulb" | "moneyplane" | "moneycrate" | "tunnel" | "tunnel_entrance" => continue,
                "mainbasepad" => Class::Hq,
                "garage" => Class::Garage,
                "airbase" => Class::Airbase,
                "bank" => Class::Bank,
                "tower_electric" | "tower_missile" | "turret" | "popup_turret" | "verybiggun" | "supergun" | "gunport" | "base_launcher" => Class::Tower,
                "stank" if team == Team::Player => {
                    if player_tank.is_some() {
                        continue;
                    }
                    player_tank = Some(entities.len());
                    Class::PlayerTank
                }
                "stank" => Class::EnemyTank,
                k if balance::unit(k).is_some() && matches!(team, Team::Player | Team::Enemy) => Class::Vehicle,
                _ => Class::Prop,
            };
            let max_hp = if o.health > 0 { o.health as f32 } else { balance::unit(&o.kind).map_or(100.0, |s| s.health as f32) };
            entities.push(Entity {
                kind: o.kind.clone(),
                class,
                team: if class == Class::Prop { Team::Scenery } else { team },
                x: o.x,
                y: o.y,
                heading: o.theta,
                turret: 0.0,
                hp: max_hp,
                max_hp,
                alive: true,
                cooldown: 0.0,
                target: None,
                next_node: None,
                odometer: 0.0,
                flash: 0.0,
                home: (o.x, o.y),
            });
        }
        let mut w = World {
            entities,
            nodes,
            bolts: Vec::new(),
            blasts: Vec::new(),
            money: [START_MONEY; 2],
            lives: START_LIVES,
            player_tank,
            messages: VecDeque::new(),
            outcome: Outcome::Playing,
            route_choice: HashMap::new(),
            queues: [VecDeque::new(), VecDeque::new()],
            deploy_timer: [0.0; 2],
            bank_timer: BANK_INTERVAL,
            ai_timer: 8.0,
            time: 0.0,
            rng: 0x2545_F491_4F6C_DD1D,
        };
        // units placed in the level join the nearest route of their team
        for i in 0..w.entities.len() {
            if w.entities[i].class == Class::Vehicle {
                let e = &w.entities[i];
                w.entities[i].next_node = w.nearest_node(e.x, e.y, e.team, true);
            }
        }
        log::info!(
            "world: {} entities, {} path nodes, player tank {:?}",
            w.entities.len(),
            w.nodes.len(),
            w.player_tank.map(|i| (w.entities[i].x, w.entities[i].y))
        );
        w
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        log::info!("message: {text}");
        self.messages.push_back((self.time, text));
        while self.messages.len() > 4 {
            self.messages.pop_front();
        }
    }

    pub fn recent_messages(&self) -> impl Iterator<Item = &str> {
        self.messages.iter().filter(move |(t, _)| self.time - t < 8.0).map(|(_, m)| m.as_str())
    }

    /// Nearest path node that has somewhere to go for `team`.
    pub fn nearest_node(&self, x: f32, y: f32, team: Team, with_links: bool) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !with_links || !n.links(team).is_empty())
            .min_by(|a, b| {
                let da = (a.1.x - x).powi(2) + (a.1.y - y).powi(2);
                let db = (b.1.x - x).powi(2) + (b.1.y - y).powi(2);
                da.total_cmp(&db)
            })
            .map(|(i, _)| i)
    }

    pub fn find(&self, class: Class, team: Team) -> Option<usize> {
        self.entities.iter().position(|e| e.alive && e.class == class && e.team == team)
    }

    pub fn queue_len(&self, team: Team) -> usize {
        self.queues[team.index()].len()
    }

    /// Order a unit from `team`'s garage. Returns false when it cannot be paid for
    /// or there is no garage.
    pub fn order(&mut self, team: Team, kind: &'static str) -> bool {
        let Some(stats) = balance::unit(kind) else { return false };
        if self.find(Class::Garage, team).is_none() || self.money[team.index()] < stats.cost {
            return false;
        }
        self.money[team.index()] -= stats.cost;
        self.queues[team.index()].push_back(kind);
        log::info!("{team:?} ordered {kind} for {}", stats.cost);
        true
    }

    /// Switch the player's route at the junction nearest (x, y), within `reach`.
    /// Returns the junction's position when one was switched.
    pub fn switch_junction(&mut self, x: f32, y: f32, reach: f32) -> Option<(f32, f32)> {
        let (node, _) = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.links_a.len() > 1)
            .map(|(i, n)| (i, (n.x - x).hypot(n.y - y)))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let n = self.nodes[node].links_a.len();
        let c = self.route_choice.entry(node).or_insert(0);
        *c = (*c + 1) % n;
        let (c, nx, ny) = (*c, self.nodes[node].x, self.nodes[node].y);
        self.say(format!("Route switched at junction ({nx:.0},{ny:.0}): branch {} of {n}", c + 1));
        Some((nx, ny))
    }

    fn next_after(&self, node: usize, team: Team) -> Option<usize> {
        let links = self.nodes[node].links(team);
        if links.is_empty() {
            return None;
        }
        let choice = if team == Team::Player { self.route_choice.get(&node).copied().unwrap_or(0) } else { (node * 7 + self.time as usize / 30) % links.len() };
        Some(links[choice.min(links.len() - 1)])
    }

    /// Sync the player's supertank into the world before the step.
    pub fn place_player_tank(&mut self, x: f32, y: f32, heading: f32) {
        if let Some(i) = self.player_tank {
            let e = &mut self.entities[i];
            e.x = x;
            e.y = y;
            e.heading = heading;
        }
    }

    pub fn step(&mut self, dt: f32) {
        if self.outcome != Outcome::Playing {
            return;
        }
        self.time += dt;
        self.economy(dt);
        self.computer_player(dt);
        self.deploy(dt);
        for i in 0..self.entities.len() {
            if self.entities[i].alive {
                self.think(i, dt);
            }
        }
        self.step_bolts(dt);
        for e in &mut self.entities {
            e.flash = (e.flash - dt).max(0.0);
        }
    }

    fn economy(&mut self, dt: f32) {
        self.bank_timer -= dt;
        if self.bank_timer <= 0.0 {
            self.bank_timer += BANK_INTERVAL;
            for team in [Team::Player, Team::Enemy] {
                let banks = self.entities.iter().filter(|e| e.alive && e.class == Class::Bank && e.team == team).count() as i32;
                if banks > 0 {
                    self.money[team.index()] += banks * BANK_CRATE;
                    if team == Team::Player {
                        self.say(format!("Money received: {} from {banks} bank{}", banks * BANK_CRATE, if banks == 1 { "" } else { "s" }));
                    }
                }
            }
        }
    }

    /// The computer buys a unit every few seconds when it can afford one.
    fn computer_player(&mut self, dt: f32) {
        self.ai_timer -= dt;
        if self.ai_timer > 0.0 {
            return;
        }
        self.ai_timer = 5.0 + self.rand() * 6.0;
        let weights = [("peon_tank", 5.0), ("electric_car", 2.0), ("rocket_tank", 2.0), ("trike", 1.5), ("engineer", 1.0), ("tank_buster", 1.0), ("cobra_tank", 0.5)];
        let total: f32 = weights.iter().map(|w| w.1).sum();
        let mut pick = self.rand() * total;
        for (kind, w) in weights {
            pick -= w;
            if pick <= 0.0 {
                self.order(Team::Enemy, kind);
                break;
            }
        }
    }

    fn deploy(&mut self, dt: f32) {
        for team in [Team::Player, Team::Enemy] {
            let t = team.index();
            self.deploy_timer[t] = (self.deploy_timer[t] - dt).max(0.0);
            if self.deploy_timer[t] > 0.0 || self.queues[t].is_empty() {
                continue;
            }
            let Some(g) = self.find(Class::Garage, team) else {
                self.queues[t].clear();
                continue;
            };
            let kind = self.queues[t].pop_front().unwrap();
            let (gx, gy) = (self.entities[g].x, self.entities[g].y);
            let start = self.nearest_node(gx, gy, team, true);
            let stats = balance::unit(kind).unwrap();
            self.entities.push(Entity {
                kind: kind.to_string(),
                class: Class::Vehicle,
                team,
                x: gx,
                y: gy,
                heading: 0.0,
                turret: 0.0,
                hp: stats.health as f32,
                max_hp: stats.health as f32,
                alive: true,
                cooldown: 0.0,
                target: None,
                next_node: start,
                odometer: 0.0,
                flash: 0.0,
                home: (gx, gy),
            });
            self.deploy_timer[t] = DEPLOY_INTERVAL;
            if team == Team::Player {
                self.say(format!("Vehicle advancing: {kind}"));
            }
        }
    }

    fn dist(&self, a: usize, b: usize) -> f32 {
        let (ea, eb) = (&self.entities[a], &self.entities[b]);
        (ea.x - eb.x).hypot(ea.y - eb.y)
    }

    fn nearest_hostile(&self, i: usize, range: f32) -> Option<usize> {
        let me = &self.entities[i];
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_target() && me.team.hostile(e.team))
            .map(|(j, e)| (j, (e.x - me.x).hypot(e.y - me.y) - e.radius()))
            .filter(|&(_, d)| d <= range)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(j, _)| j)
    }

    fn think(&mut self, i: usize, dt: f32) {
        let class = self.entities[i].class;
        let team = self.entities[i].team;
        if matches!(team, Team::Neutral | Team::Scenery) || class == Class::PlayerTank {
            return;
        }
        self.entities[i].cooldown = (self.entities[i].cooldown - dt).max(0.0);
        match class {
            Class::Tower | Class::Vehicle | Class::EnemyTank => {}
            _ => return,
        }
        let stats = self.entities[i].stats();
        let detect = match class {
            Class::EnemyTank => 15.0,
            _ => stats.map_or(5.0, |s| s.detection_range.max(2.0)),
        };
        let weapon = self.entities[i].weapon();
        let wrange = weapon.and_then(balance::weapon).map_or(1.0, |w| w.range.max(1.5)).min(detect.max(1.5));

        // keep or pick a target
        let keep = self.entities[i].target.filter(|&t| self.entities[t].is_target() && self.dist(i, t) - self.entities[t].radius() <= detect);
        let target = keep.or_else(|| self.nearest_hostile(i, detect));
        self.entities[i].target = target;

        let kind = self.entities[i].kind.clone();
        // specials: engineers capture, trikes and bomb trucks ram
        if class == Class::Vehicle && kind == "engineer" && self.try_capture(i) {
            return;
        }
        if let Some(t) = target {
            let d = self.dist(i, t) - self.entities[t].radius();
            let (tx, ty) = (self.entities[t].x, self.entities[t].y);
            let e = &mut self.entities[i];
            let aim = (ty - e.y).atan2(tx - e.x);
            e.turret = aim - e.heading;
            if matches!(kind.as_str(), "trike" | "bomb_truck") {
                if d <= 0.6 {
                    let w = if kind == "trike" { "trike" } else { "bomb_truck" };
                    let (x, y) = (e.x, e.y);
                    e.alive = false;
                    self.damage(w, x, y, Some(t), team);
                    self.blasts.push(Blast { x, y, power: 2.0 });
                    return;
                }
                self.drive_towards(i, tx, ty, dt);
                return;
            }
            if let Some(w) = weapon {
                if d <= wrange {
                    if self.entities[i].cooldown == 0.0 {
                        let delay = stats.map_or(1.0, |s| s.fire_delay).max(1.0) / ORIGINAL_HZ;
                        self.entities[i].cooldown = delay;
                        self.fire(i, w, t);
                    }
                    return;
                }
            }
            if class == Class::EnemyTank {
                self.drive_towards(i, tx, ty, dt);
                return;
            }
        }
        match class {
            Class::Vehicle => self.follow_path(i, dt),
            Class::EnemyTank => {
                let (hx, hy) = self.entities[i].home;
                if (self.entities[i].x - hx).hypot(self.entities[i].y - hy) > 1.0 {
                    self.drive_towards(i, hx, hy, dt);
                }
            }
            _ => {}
        }
    }

    fn speed_of(&self, i: usize) -> f32 {
        let e = &self.entities[i];
        let s = if e.class == Class::EnemyTank { balance::unit("stank").map_or(0.3, |s| s.speed) * 0.6 } else { e.stats().map_or(0.08, |s| s.speed) };
        s * ORIGINAL_HZ
    }

    fn drive_towards(&mut self, i: usize, tx: f32, ty: f32, dt: f32) -> bool {
        let speed = self.speed_of(i);
        let e = &self.entities[i];
        let (dx, dy) = (tx - e.x, ty - e.y);
        let d = dx.hypot(dy);
        if d < 1e-3 {
            return true;
        }
        let turn_rate = self.entities[i].stats().map_or(2.0, |s| s.turn_speed * ORIGINAL_HZ).clamp(1.0, 6.0);
        let e = &mut self.entities[i];
        let want = dy.atan2(dx);
        let mut diff = (want - e.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        let turn = turn_rate * dt;
        let off = diff.abs();
        diff = diff.clamp(-turn, turn);
        e.heading += diff;
        // tracks pivot on the spot: far off course, turn first, then drive;
        // otherwise a unit can orbit a waypoint inside its turning circle
        let step = if off > 0.9 { 0.0 } else { (speed * dt * (1.0 - off / 1.2)).min(d) };
        e.x += e.heading.cos() * step;
        e.y += e.heading.sin() * step;
        e.odometer += step;
        d - step < NODE_REACHED
    }

    fn follow_path(&mut self, i: usize, dt: f32) {
        let team = self.entities[i].team;
        let Some(node) = self.entities[i].next_node else {
            // end of the line: go for the nearest enemy structure
            let me = &self.entities[i];
            let goal = self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.is_target() && team.hostile(e.team) && e.class != Class::Vehicle)
                .min_by(|a, b| ((a.1.x - me.x).hypot(a.1.y - me.y)).total_cmp(&(b.1.x - me.x).hypot(b.1.y - me.y)))
                .map(|(_, e)| (e.x, e.y));
            if let Some((gx, gy)) = goal {
                self.drive_towards(i, gx, gy, dt);
            }
            return;
        };
        let (nx, ny) = (self.nodes[node].x, self.nodes[node].y);
        if self.drive_towards(i, nx, ny, dt) {
            self.entities[i].next_node = self.next_after(node, team);
        }
    }

    fn try_capture(&mut self, i: usize) -> bool {
        let me = self.entities[i].clone();
        let spot = self.entities.iter().position(|e| {
            e.alive && matches!(e.class, Class::Bank | Class::Tower) && e.team != me.team && e.team != Team::Scenery && (e.x - me.x).hypot(e.y - me.y) <= CAPTURE_RANGE + e.radius()
        });
        let Some(s) = spot else { return false };
        let kind = self.entities[s].kind.clone();
        self.entities[s].team = me.team;
        self.entities[s].target = None;
        self.entities[i].alive = false;
        let who = if me.team == Team::Player { "We captured" } else { "Enemy captured" };
        self.say(format!("{who} a {}", kind.replace('_', " ")));
        true
    }

    fn fire(&mut self, i: usize, weapon: &'static str, target: usize) {
        let Some(w) = balance::weapon(weapon) else { return };
        let (e, t) = (&self.entities[i], &self.entities[target]);
        let speed = if w.speed <= 0.0 { 0.0 } else { (w.speed * ORIGINAL_HZ).max(4.0) };
        let aim = e.heading + e.turret;
        let muzzle = (e.x + aim.cos() * e.radius() * 0.8, e.y + aim.sin() * e.radius() * 0.8);
        self.bolts.push(Bolt {
            weapon,
            team: e.team,
            x: muzzle.0,
            y: muzzle.1,
            from: muzzle,
            to: (t.x, t.y),
            target: Some(target),
            speed,
            beam: BEAM_TIME,
        });
        if speed == 0.0 {
            self.damage(weapon, t.x, t.y, Some(target), e.team);
        }
    }

    fn step_bolts(&mut self, dt: f32) {
        let mut hits = Vec::new();
        let entities = &self.entities;
        self.bolts.retain_mut(|b| {
            if b.speed == 0.0 {
                b.beam -= dt;
                return b.beam > 0.0;
            }
            if b.weapon.contains("missile") || b.weapon.contains("rocket") {
                if let Some(t) = b.target.filter(|&t| entities[t].alive) {
                    b.to = (entities[t].x, entities[t].y);
                }
            }
            let (dx, dy) = (b.to.0 - b.x, b.to.1 - b.y);
            let d = dx.hypot(dy);
            let step = b.speed * dt;
            if d <= step {
                hits.push((b.weapon, b.to.0, b.to.1, b.target, b.team));
                return false;
            }
            b.x += dx / d * step;
            b.y += dy / d * step;
            true
        });
        for (w, x, y, target, team) in hits {
            // a moving target may have left the spot
            let direct = target.filter(|&t| (self.entities[t].x - x).hypot(self.entities[t].y - y) <= self.entities[t].radius() + 0.4);
            self.damage(w, x, y, direct, team);
            let power = balance::weapon(w).map_or(0.4, |s| if s.area { 0.6 + s.radius * 0.3 } else { 0.4 });
            self.blasts.push(Blast { x, y, power });
        }
    }

    /// Apply `weapon`'s damage at (x, y) from `team`. Area weapons hurt every
    /// hostile within their radius; single-target weapons only `direct`.
    pub fn damage(&mut self, weapon: &str, x: f32, y: f32, direct: Option<usize>, team: Team) {
        let Some(w) = balance::weapon(weapon) else { return };
        let victims: Vec<usize> = if w.area {
            (0..self.entities.len())
                .filter(|&j| {
                    let e = &self.entities[j];
                    e.is_target() && team.hostile(e.team) && (e.x - x).hypot(e.y - y) <= w.radius + e.radius()
                })
                .collect()
        } else {
            direct.into_iter().filter(|&j| self.entities[j].is_target() && team.hostile(self.entities[j].team)).collect()
        };
        for j in victims {
            let amount = w.damage_to(&self.entities[j].kind);
            self.hurt(j, amount);
        }
    }

    pub fn hurt(&mut self, j: usize, amount: f32) {
        let e = &mut self.entities[j];
        if !e.alive {
            return;
        }
        e.hp -= amount;
        e.flash = 0.15;
        if e.hp > 0.0 {
            return;
        }
        let (class, team, kind, x, y, r) = (e.class, e.team, e.kind.clone(), e.x, e.y, e.radius());
        if class == Class::PlayerTank {
            // the supertank respawns at the HQ while lives remain
            e.hp = e.max_hp;
            self.blasts.push(Blast { x, y, power: 2.5 });
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.entities[j].alive = false;
                self.outcome = Outcome::Lost;
                self.say("Supertank lost. No supertanks left.");
            } else {
                self.say(format!("Supertank lost. {} left.", self.lives));
            }
            return;
        }
        e.alive = false;
        self.blasts.push(Blast { x, y, power: 1.0 + r });
        log::info!("destroyed {team:?} {kind}");
        match (class, team) {
            (Class::Hq, Team::Enemy) => {
                self.outcome = Outcome::Won;
                self.say("Enemy base destroyed!");
            }
            (Class::Hq, Team::Player) => {
                self.outcome = Outcome::Lost;
                self.say("Our base has been destroyed.");
            }
            (Class::EnemyTank, _) => self.say("Enemy supertank destroyed!"),
            (_, Team::Enemy) if class != Class::Vehicle => self.say(format!("Destroyed enemy {}", kind.replace('_', " "))),
            (_, Team::Player) if class != Class::Vehicle => self.say(format!("Lost our {}", kind.replace('_', " "))),
            _ => {}
        }
    }

    /// The player's supertank shot something: first hostile or neutral-free
    /// target within reach of (x, y), if any.
    pub fn hit_test(&self, x: f32, y: f32, team: Team) -> Option<usize> {
        self.entities.iter().position(|e| e.is_target() && team.hostile(e.team) && (e.x - x).hypot(e.y - y) <= e.radius())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::LEVELS;

    #[test]
    fn test_level_has_two_sides() {
        let w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        for team in [Team::Player, Team::Enemy] {
            assert!(w.find(Class::Hq, team).is_some(), "{team:?} hq");
            assert!(w.find(Class::Garage, team).is_some(), "{team:?} garage");
        }
        assert!(w.player_tank.is_some());
        assert!(w.find(Class::EnemyTank, Team::Enemy).is_some());
    }

    #[test]
    fn ordered_units_drive_to_the_enemy() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        assert!(w.order(Team::Player, "peon_tank"));
        assert_eq!(w.money[0], START_MONEY - 300);
        let start = {
            w.step(1.0 / 30.0);
            let u = w.entities.iter().rposition(|e| e.class == Class::Vehicle && e.team == Team::Player).unwrap();
            (u, w.entities[u].x)
        };
        for _ in 0..30 * 20 {
            w.step(1.0 / 30.0);
        }
        let u = &w.entities[start.0];
        // peon tanks drive 0.08 cells per original tick; most of 20 s should be driving
        assert!(!u.alive || u.odometer > 10.0, "unit should drive its route: odometer {} at ({:.1},{:.1}) next {:?}", u.odometer, u.x, u.y, u.next_node);
    }
}

#[cfg(test)]
mod sim {
    use super::*;
    use crate::game::LEVELS;

    /// `cargo test --release battle_report -- --ignored --nocapture`: twelve minutes
    /// of both garages building, printed as a report. A balance probe, not a check.
    #[test]
    #[ignore]
    fn battle_report() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        let dt = 1.0 / 30.0;
        for tick in 0..30 * 720 {
            if tick % (30 * 6) == 0 {
                let kind = BUILDABLE[(tick / (30 * 6)) % 4];
                w.order(Team::Player, kind);
            }
            w.step(dt);
            if tick % (30 * 60) == 0 {
                let count = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Vehicle && e.team == t).count();
                let front = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Vehicle && e.team == t).map(|e| e.x).fold(f32::NAN, f32::max);
                println!(
                    "t={:3}s money {:?} units P{} E{} furthest player x {:.0} enemy x {:.0} outcome {:?}",
                    tick / 30,
                    w.money,
                    count(Team::Player),
                    count(Team::Enemy),
                    front(Team::Player),
                    w.entities.iter().filter(|e| e.alive && e.class == Class::Vehicle && e.team == Team::Enemy).map(|e| e.x).fold(f32::NAN, f32::min),
                    w.outcome
                );
            }
        }
        for team in [Team::Player, Team::Enemy] {
            let hq = w.entities.iter().find(|e| e.class == Class::Hq && e.team == team).unwrap();
            println!("{team:?} hq hp {} alive {}", hq.hp, hq.alive);
        }
        for (t, m) in &w.messages {
            println!("{t:6.1} {m}");
        }
        let dead: Vec<String> = w.entities.iter().filter(|e| !e.alive && e.class != Class::Vehicle).map(|e| format!("{:?} {}", e.team, e.kind)).collect();
        println!("structures destroyed: {dead:?}");
    }
}
