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
const CRATE_CAPACITY: i32 = 10;

/// `formation_joe` from the original `objs/ai_joe.cpp`: 6x6 layouts whose
/// digits are unit types (0 supertank, 1 peon tank, 2 electric car, 3 rocket
/// tank, 4 helicopter, 5 jet, 6 engineer, 7 trike). Units follow paths here,
/// so only the build order of each layout matters.
const FORMATIONS: [(&str, &str); 13] = [
    ("stock", "            121212      1 63 1  33  "),
    ("electric brigade", "222222                              "),
    ("hole puncher", "  11   1221 123321                  "),
    ("armored convoy", "12221 23632                         "),
    ("gene's checker", "            1 1 1  1 1 17 7 7  7 7 7"),
    ("peon V", "  1    161  1   1                   "),
    ("heli hell", "  4    4 4                          "),
    ("rockets R us", "111111 3663  3333                   "),
    ("small band", "22    66                            "),
    ("yet another small band", "22    36                            "),
    ("yet another small band", "33    66                            "),
    ("long shot", "6                                   "),
    ("ouch!", "2222  3333                          "),
];

fn formation_unit(c: char) -> Option<&'static str> {
    Some(match c {
        '1' => "peon_tank",
        '2' => "electric_car",
        '3' => "rocket_tank",
        '4' => "helicopter",
        '5' => "jet",
        '6' => "engineer",
        '7' => "trike",
        _ => return None,
    })
}
const BANK_INTERVAL: f32 = 500.0 / ORIGINAL_HZ;
const DEPLOY_INTERVAL: f32 = 10.0 / ORIGINAL_HZ;
const NODE_REACHED: f32 = 0.35;
/// How far short of an unbridged gap route units stop.
const GAP_WAIT: f32 = 1.4;
const CAPTURE_RANGE: f32 = 2.0;
/// Instant weapons (speed 0 in the table) draw their beam this long.
const BEAM_TIME: f32 = 0.15;

/// What the garage (keys 1-8, V) and the airbase (9, 0, B) offer, in the
/// order the HUD lists them.
pub const BUILDABLE: [&str; 12] = [
    "peon_tank",
    "electric_car",
    "rocket_tank",
    "trike",
    "tank_buster",
    "engineer",
    "bomb_truck",
    "cobra_tank",
    "bridger",
    "helicopter",
    "jet",
    "bomber",
];
pub const BUILD_KEYS: [&str; 12] = ["1", "2", "3", "4", "5", "6", "7", "8", "V", "9", "0", "B"];
/// Built at the airbase and flown along its (the original's pink) routes.
pub const AIR_UNITS: [&str; 3] = ["helicopter", "jet", "bomber"];

pub fn flies(kind: &str) -> bool {
    AIR_UNITS.contains(&kind)
}

fn factory_for(kind: &str) -> Class {
    if flies(kind) { Class::Airbase } else { Class::Garage }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
    Scenery,
}

impl Team {
    pub fn from_player(p: u8) -> Team {
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
    /// A money plane: flies its bank's crates to the HQ.
    Plane,
    /// A bridger that has parked over a gap: a bridge deck anything can
    /// drive over, until it is shot away.
    Bridge,
    Prop,
}

/// What a unit is doing besides fighting what it meets. As in the original,
/// units always fire at enemies in weapon range on their own.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    /// Follow the path network (the default for everything built).
    Route,
    /// Stand still.
    Hold,
    /// Drive (or fly) to a map position, then hold.
    Move(f32, f32),
    /// Go after one enemy until it is destroyed, then hold.
    Attack(usize),
}

/// A command from the player to the selected units.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    Hold,
    Move(f32, f32),
    Attack(usize),
    /// Back onto the nearest route node.
    Resume,
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
    /// Banks: crates waiting. Planes: money carried.
    pub crates: i32,
    /// Planes: the bank they serve.
    pub bank: Option<usize>,
    /// Computer units waiting at the garage for the rest of their convoy.
    pub held: bool,
    pub order: Order,
    home: (f32, f32),
}

impl Entity {
    /// An entity that is only drawn (film playback): no orders, no home.
    pub fn shown(kind: &str, class: Class, team: Team, max_hp: f32) -> Entity {
        Entity {
            kind: kind.to_string(),
            class,
            team,
            x: 0.0,
            y: 0.0,
            heading: 0.0,
            turret: 0.0,
            hp: max_hp,
            max_hp,
            alive: true,
            cooldown: 0.0,
            target: None,
            next_node: None,
            odometer: 0.0,
            flash: 0.0,
            crates: 0,
            bank: None,
            held: false,
            order: Order::Route,
            home: (0.0, 0.0),
        }
    }

    pub fn radius(&self) -> f32 {
        match self.class {
            Class::Hq => 1.6,
            Class::Garage | Class::Airbase | Class::Bank => 1.2,
            Class::Tower => 0.7,
            Class::PlayerTank | Class::EnemyTank => 0.9,
            Class::Vehicle => 0.5,
            Class::Plane => 0.45,
            Class::Bridge => 0.5,
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
    /// A gap a bridger can span (the original's `bridgeable_spot`).
    pub bridge_spot: bool,
    /// The bridge laid here (an entity), while it stands.
    pub bridge: Option<usize>,
    /// Which way the route runs through the node, for the bridge deck.
    pub dir: f32,
}

impl PathNode {
    pub fn links(&self, team: Team) -> &[usize] {
        if team == Team::Enemy { &self.links_e } else { &self.links_a }
    }
}

impl Bolt {
    /// A bolt that is only drawn (film playback).
    pub fn shown(x: f32, y: f32) -> Bolt {
        Bolt { weapon: "", team: Team::Neutral, x, y, from: (x, y), to: (x, y), target: None, speed: 0.0, beam: 0.0 }
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

/// A sound for the audio layer: a file from `audio::SFX`, where it happened
/// (None: the radio, heard everywhere) and how loud.
#[derive(Clone, Copy, Debug)]
pub struct Sfx {
    pub name: &'static str,
    pub at: Option<(f32, f32)>,
    pub volume: f32,
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
    /// Sounds for the audio layer, drained by the main loop.
    pub sounds: Vec<Sfx>,
    pub money: [i32; 2],
    pub lives: u32,
    pub player_tank: Option<usize>,
    pub messages: VecDeque<(f32, String)>,
    pub outcome: Outcome,
    /// Why the battle was won or lost, once it is.
    pub verdict: String,
    /// The map's win and lose conditions.
    rules: crate::rules::Rules,
    /// What the factories may build (None: everything).
    build: Option<Vec<String>>,
    /// Seconds each `hold` condition has been held, by its text, and who
    /// was last on the site (for the radio messages).
    hold_time: HashMap<String, f32>,
    hold_side: HashMap<String, Option<Team>>,
    /// Blocking cells (the tile flag), row major, `width` wide: units under
    /// orders stop at them; routes are trusted as the map maker drew them.
    blocked: Vec<bool>,
    width: usize,
    /// Junction choices: path node -> which of its links new units take.
    pub route_choice: HashMap<usize, usize>,
    /// The player's supertank kills, scored at the victim's cost as the
    /// original's `add_points` does.
    pub points: i32,
    /// Damage over time: (victim, damage per original tick, ticks left,
    /// dealt by the player's supertank).
    burns: Vec<(usize, f32, u32, bool)>,
    burn_clock: f32,
    queues: [VecDeque<&'static str>; 2],
    deploy_timer: [f32; 2],
    bank_timer: f32,
    ai_timer: f32,
    /// Formation being built (index into FORMATIONS) and the next unit in it.
    ai_form: Option<usize>,
    ai_place: usize,
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
                nodes.push(PathNode { x: o.x, y: o.y, links_a: Vec::new(), links_e: Vec::new(), bridge_spot: o.bridge, bridge: None, dir: 0.0 });
            }
        }
        for (i, o) in level.objects.iter().enumerate() {
            if let Some(&n) = node_of.get(&i) {
                nodes[n].links_a = o.links_a.iter().filter_map(|l| node_of.get(l).copied()).collect();
                nodes[n].links_e = o.links_e.iter().filter_map(|l| node_of.get(l).copied()).collect();
            }
        }
        // the route's direction through each node: toward its next node, or
        // away from the node that leads to it
        for n in 0..nodes.len() {
            let from = (0..nodes.len()).find(|&m| nodes[m].links_a.contains(&n) || nodes[m].links_e.contains(&n));
            let to = nodes[n].links_a.first().or(nodes[n].links_e.first()).copied();
            let (ax, ay, bx, by) = match (from, to) {
                (_, Some(t)) => (nodes[n].x, nodes[n].y, nodes[t].x, nodes[t].y),
                (Some(f), None) => (nodes[f].x, nodes[f].y, nodes[n].x, nodes[n].y),
                _ => continue,
            };
            nodes[n].dir = (by - ay).atan2(bx - ax);
        }
        let blocked = (0..level.width * level.height).map(|i| crate::terrain::tile_flags(level.tile(i % level.width, i / level.width)) & crate::level::TILE_BLOCKING != 0).collect();

        let mut entities = Vec::new();
        let mut player_tank = None;
        for o in &level.objects {
            let team = Team::from_player(o.player);
            let class = match o.kind.as_str() {
                "" | "path_object" | "field_camera" | "sfx_obj" | "lightbulb" | "moneycrate" | "tunnel" | "tunnel_entrance" => continue,
                "moneyplane" if matches!(team, Team::Player | Team::Enemy) => Class::Plane,
                "moneyplane" => continue,
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
                crates: 0,
                bank: None,
                held: false,
                order: Order::Route,
                home: (o.x, o.y),
            });
        }
        let mut w = World {
            entities,
            nodes,
            bolts: Vec::new(),
            blasts: Vec::new(),
            sounds: Vec::new(),
            money: [START_MONEY; 2],
            lives: START_LIVES,
            player_tank,
            messages: VecDeque::new(),
            outcome: Outcome::Playing,
            verdict: String::new(),
            rules: level.rules.clone(),
            build: level.build.clone(),
            hold_time: HashMap::new(),
            hold_side: HashMap::new(),
            blocked,
            width: level.width,
            route_choice: HashMap::new(),
            points: 0,
            burns: Vec::new(),
            burn_clock: 0.0,
            queues: [VecDeque::new(), VecDeque::new()],
            deploy_timer: [0.0; 2],
            bank_timer: BANK_INTERVAL,
            ai_timer: 5.0,
            ai_form: None,
            ai_place: 0,
            time: 0.0,
            rng: 0x2545_F491_4F6C_DD1D,
        };
        // each money plane serves the nearest bank of its team
        for i in 0..w.entities.len() {
            if w.entities[i].class == Class::Plane {
                let (x, y, team) = (w.entities[i].x, w.entities[i].y, w.entities[i].team);
                w.entities[i].bank = w.nearest_bank(x, y, team);
            }
        }
        // units placed in the level join the nearest route of their team
        for i in 0..w.entities.len() {
            if w.entities[i].class == Class::Vehicle {
                let e = &w.entities[i];
                w.entities[i].next_node = w.nearest_node(e.x, e.y, e.team, true);
            }
        }
        // a condition already true before the battle starts cannot be what the
        // map means (no HQ to lose on a sandbox map): drop it
        let rules = std::mem::take(&mut w.rules);
        let (win, lose) = (rules.win.into_iter().filter(|c| !c.holds(&w)).collect(), rules.lose.into_iter().filter(|c| !c.holds(&w)).collect());
        w.rules = crate::rules::Rules { win, lose };
        log::info!("rules: win {:?} lose {:?}", w.rules.win, w.rules.lose);
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

    pub fn sound(&mut self, name: &'static str, at: Option<(f32, f32)>, volume: f32) {
        self.sounds.push(Sfx { name, at, volume });
    }

    /// The original's radio voice lines.
    pub fn radio(&mut self, name: &'static str) {
        self.sound(name, None, 0.9);
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
        if !self.can_build(kind) || self.find(factory_for(kind), team).is_none() || self.money[team.index()] < stats.cost {
            return false;
        }
        self.money[team.index()] -= stats.cost;
        self.queues[team.index()].push_back(kind);
        log::info!("{team:?} ordered {kind} for {}", stats.cost);
        true
    }

    /// Whether this map's factories offer `kind` at all.
    pub fn can_build(&self, kind: &str) -> bool {
        self.build.as_ref().is_none_or(|b| b.iter().any(|k| k == kind))
    }

    /// Whether `team` has a factory that builds `kind` on this map.
    pub fn offers(&self, team: Team, kind: &str) -> bool {
        self.can_build(kind) && self.find(factory_for(kind), team).is_some()
    }

    /// A bridge spot with no bridge standing on it.
    pub fn gap(&self, node: usize) -> bool {
        let n = &self.nodes[node];
        n.bridge_spot && !n.bridge.is_some_and(|b| self.entities[b].alive)
    }

    /// On a standing bridge deck: one cell long across the gap (the
    /// original's `bridger_road` model is 10 by 5.8 units at its 0.1 model
    /// scale), with a little room either side for the tracks.
    pub fn on_bridge(&self, x: f32, y: f32) -> bool {
        self.nodes.iter().filter_map(|n| n.bridge.filter(|&b| self.entities[b].alive).map(|_| n)).any(|n| {
            let (dx, dy) = (x - n.x, y - n.y);
            let (l, w) = (dx * n.dir.cos() + dy * n.dir.sin(), -dx * n.dir.sin() + dy * n.dir.cos());
            l.abs() <= 0.6 && w.abs() <= 0.6
        })
    }

    /// Ground a unit can drive onto: on the map, not blocking, or bridged.
    fn ground_free(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return false;
        }
        let (cx, cy) = (x as usize, y as usize);
        if cx >= self.width || cy * self.width + cx >= self.blocked.len() {
            return false;
        }
        !self.blocked[cy * self.width + cx] || self.on_bridge(x, y)
    }

    /// The bridger `i` parks over gap `node` and becomes its bridge, as the
    /// original's does when it finds a `bridgeable_spot` under it.
    fn lay_bridge(&mut self, i: usize, node: usize) {
        let (x, y, dir) = (self.nodes[node].x, self.nodes[node].y, self.nodes[node].dir);
        let e = &mut self.entities[i];
        e.class = Class::Bridge;
        e.x = x;
        e.y = y;
        e.heading = dir;
        e.next_node = None;
        e.target = None;
        e.order = Order::Hold;
        let team = e.team;
        self.nodes[node].bridge = Some(i);
        log::info!("{team:?} bridge laid at ({x:.1},{y:.1})");
        if team == Team::Player {
            self.say("Bridge laid");
        } else {
            self.say("The enemy has bridged a crossing");
        }
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
        self.step_burns(dt);
        for e in &mut self.entities {
            e.flash = (e.flash - dt).max(0.0);
        }
        self.step_holds(dt);
        self.check_rules();
    }

    /// Seconds a `hold` condition has been held so far.
    pub fn held_for(&self, c: &crate::rules::Cond) -> f32 {
        self.hold_time.get(&c.text()).copied().unwrap_or(0.0)
    }

    /// The map's sites to hold: (holder's team, x, y, radius, seconds
    /// needed, seconds held).
    pub fn sites(&self) -> Vec<(Team, f32, f32, f32, f32, f32)> {
        self.rules
            .win
            .iter()
            .chain(&self.rules.lose)
            .filter_map(|c| match *c {
                crate::rules::Cond::Hold { player, x, y, r, secs } => Some((Team::from_player(player), x, y, r, secs, self.held_for(c))),
                _ => None,
            })
            .collect()
    }

    /// Run the hold clocks: on while only the holder's ground units or
    /// supertank are on the site, paused while both sides or neither are,
    /// back to zero while only the other side is.
    fn step_holds(&mut self, dt: f32) {
        let conds: Vec<crate::rules::Cond> = self.rules.win.iter().chain(&self.rules.lose).filter(|c| matches!(c, crate::rules::Cond::Hold { .. })).cloned().collect();
        for c in conds {
            let crate::rules::Cond::Hold { player, x, y, r, secs } = c else { continue };
            let key = c.text();
            let t = self.held_for(&c);
            if t >= secs {
                continue;
            }
            let holder = Team::from_player(player);
            let on_site = |team: Team| {
                self.entities.iter().any(|e| {
                    e.alive
                        && e.team == team
                        && matches!(e.class, Class::Vehicle | Class::PlayerTank | Class::EnemyTank)
                        && !flies(&e.kind)
                        && (e.x - x).hypot(e.y - y) <= r
                })
            };
            let other = if holder == Team::Player { Team::Enemy } else { Team::Player };
            let (ours, theirs) = (on_site(holder), on_site(other));
            let side = match (ours, theirs) {
                (true, false) => Some(holder),
                (false, true) => Some(other),
                _ => self.hold_side.get(&key).copied().flatten(),
            };
            let t = match (ours, theirs) {
                (true, false) => t + dt,
                (false, true) => 0.0,
                _ => t,
            };
            self.hold_time.insert(key.clone(), t);
            if self.hold_side.get(&key).copied().flatten() != side {
                self.hold_side.insert(key, side);
                match (side == Some(Team::Player), holder == Team::Player) {
                    (true, true) => self.say(format!("We hold the site: {secs:.0} seconds to go")),
                    (false, true) => self.say("The enemy is on the site: the clock is back to zero"),
                    (true, false) => self.say("We have retaken the site"),
                    (false, false) => self.say(format!("The enemy holds the site: {secs:.0} seconds and it is theirs")),
                }
            }
        }
    }

    /// The map's conditions, losing first as in the original.
    fn check_rules(&mut self) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if let Some(c) = self.rules.lose.iter().find(|c| c.holds(self)) {
            self.verdict = c.describe();
            self.outcome = Outcome::Lost;
        } else if !self.rules.win.is_empty() && self.rules.win.iter().all(|c| c.holds(self)) {
            self.verdict = self.rules.win.first().map_or(String::new(), |c| c.describe());
            self.outcome = Outcome::Won;
        } else {
            return;
        }
        log::info!("outcome {:?}: {}", self.outcome, self.verdict);
    }

    fn nearest_bank(&self, x: f32, y: f32, team: Team) -> Option<usize> {
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive && e.class == Class::Bank && e.team == team)
            .min_by(|a, b| (a.1.x - x).hypot(a.1.y - y).total_cmp(&(b.1.x - x).hypot(b.1.y - y)))
            .map(|(i, _)| i)
    }

    fn random_bank(&mut self, team: Team) -> Option<usize> {
        let banks: Vec<usize> = (0..self.entities.len()).filter(|&b| self.entities[b].alive && self.entities[b].class == Class::Bank && self.entities[b].team == team).collect();
        if banks.is_empty() {
            return None;
        }
        let k = (self.rand() * banks.len() as f32) as usize;
        Some(banks[k.min(banks.len() - 1)])
    }

    /// Banks fill a crate every 500 ticks (up to 10, `crate_capacity`); the
    /// money only arrives when a money plane flies it to the HQ.
    fn economy(&mut self, dt: f32) {
        self.bank_timer -= dt;
        if self.bank_timer <= 0.0 {
            self.bank_timer += BANK_INTERVAL;
            for e in &mut self.entities {
                if e.alive && e.class == Class::Bank && matches!(e.team, Team::Player | Team::Enemy) {
                    e.crates = (e.crates + 1).min(CRATE_CAPACITY);
                }
            }
        }
        for i in 0..self.entities.len() {
            if self.entities[i].alive && self.entities[i].class == Class::Plane {
                self.fly_plane(i, dt);
            }
        }
    }

    /// The original's money plane: to its bank, take the crates, to the HQ,
    /// drop them for money, repeat. Planes fly straight over everything.
    fn fly_plane(&mut self, i: usize, dt: f32) {
        let team = self.entities[i].team;
        let speed = balance::unit("moneyplane").map_or(0.2, |s| s.speed) * ORIGINAL_HZ;
        let bank = self.entities[i].bank.filter(|&b| self.entities[b].alive && self.entities[b].team == team);
        if bank.is_none() {
            self.entities[i].bank = self.random_bank(team);
        }
        let carrying = self.entities[i].crates > 0;
        let goal = if carrying { self.find(Class::Hq, team) } else { self.entities[i].bank };
        let Some(goal) = goal else { return };
        let (gx, gy) = (self.entities[goal].x, self.entities[goal].y);
        let e = &mut self.entities[i];
        let (dx, dy) = (gx - e.x, gy - e.y);
        let d = dx.hypot(dy);
        if d > 0.3 {
            e.heading = dy.atan2(dx);
            let step = (speed * dt).min(d);
            e.x += dx / d * step;
            e.y += dy / d * step;
            return;
        }
        if carrying {
            let amount = e.crates;
            e.crates = 0;
            self.money[team.index()] += amount;
            // the next trip goes to any bank of ours, picked at random as the
            // original's plane does (`objs/moneyplane.cpp`), so captured
            // banks pay too
            self.entities[i].bank = self.random_bank(team);
            if team == Team::Player {
                self.say(format!("Money received: ${amount}"));
                self.radio("narrative/money_received_22khz.mp3");
            }
        } else if self.entities[goal].crates > 0 {
            let n = self.entities[goal].crates;
            self.entities[goal].crates = 0;
            self.entities[i].crates = n * BANK_CRATE;
        }
    }

    /// The computer player, after the original `ai_joe`: pick one of its
    /// formations at random, build the formation's units one by one (15 ticks
    /// apart, retrying after 20 when the money is short), hold them at the
    /// garage until the formation is complete, release them together as a
    /// convoy, then wait 100 to 600 ticks before the next. A lost supertank is
    /// rebuilt at the HQ when the money allows.
    fn computer_player(&mut self, dt: f32) {
        self.ai_timer -= dt;
        if self.ai_timer > 0.0 {
            return;
        }
        let tick = 1.0 / ORIGINAL_HZ;
        if self.find(Class::EnemyTank, Team::Enemy).is_none() && self.resurrect_enemy_tank() {
            self.ai_timer = 20.0 * tick;
            return;
        }
        let Some(form) = self.ai_form else {
            let f = (self.rand() * FORMATIONS.len() as f32) as usize % FORMATIONS.len();
            log::info!("computer: formation {}", FORMATIONS[f].0);
            self.ai_form = Some(f);
            self.ai_place = 0;
            self.ai_timer = tick;
            return;
        };
        let units: Vec<&'static str> = FORMATIONS[form].1.chars().filter_map(formation_unit).collect();
        if let Some(&kind) = units.get(self.ai_place) {
            if !self.can_build(kind) || self.find(factory_for(kind), Team::Enemy).is_none() {
                // not built on this map, or its factory is gone: skip
                // rather than wait forever
                self.ai_place += 1;
            } else if self.order(Team::Enemy, kind) {
                self.ai_place += 1;
                self.ai_timer = 15.0 * tick;
            } else {
                self.ai_timer = 20.0 * tick;
            }
            return;
        }
        // formation complete once its units have left the factory queue
        if self.queues[Team::Enemy.index()].is_empty() {
            let mut released = 0;
            for e in &mut self.entities {
                if e.held && e.team == Team::Enemy {
                    e.held = false;
                    released += 1;
                }
            }
            log::info!("computer: convoy of {released} released ({})", FORMATIONS[form].0);
            self.ai_form = None;
            self.ai_timer = (100.0 + self.rand() * 500.0) * tick;
        } else {
            self.ai_timer = 5.0 * tick;
        }
    }

    fn resurrect_enemy_tank(&mut self) -> bool {
        let cost = balance::unit("stank").map_or(10000, |s| s.cost);
        let Some(hq) = self.find(Class::Hq, Team::Enemy) else { return false };
        let Some(tank) = self.entities.iter().position(|e| e.class == Class::EnemyTank) else { return false };
        if self.money[Team::Enemy.index()] < cost {
            return false;
        }
        self.money[Team::Enemy.index()] -= cost;
        let (x, y) = (self.entities[hq].x + 2.0, self.entities[hq].y);
        let e = &mut self.entities[tank];
        e.alive = true;
        e.hp = e.max_hp;
        e.x = x;
        e.y = y;
        e.home = (x, y);
        e.target = None;
        self.say("Enemy supertank rebuilt");
        true
    }

    fn deploy(&mut self, dt: f32) {
        for team in [Team::Player, Team::Enemy] {
            let t = team.index();
            self.deploy_timer[t] = (self.deploy_timer[t] - dt).max(0.0);
            if self.deploy_timer[t] > 0.0 || self.queues[t].is_empty() {
                continue;
            }
            let kind = self.queues[t].pop_front().unwrap();
            let Some(g) = self.find(factory_for(kind), team) else {
                // the factory was destroyed after the order: give the money back
                self.money[t] += balance::unit(kind).map_or(0, |s| s.cost);
                if team == Team::Player {
                    self.say(format!("Order refunded: {} (factory lost)", kind.replace('_', " ")));
                }
                continue;
            };
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
                crates: 0,
                bank: None,
                held: false,
                order: Order::Route,
                home: (gx, gy),
            });
            self.deploy_timer[t] = DEPLOY_INTERVAL;
            if team == Team::Enemy {
                let last = self.entities.len() - 1;
                self.entities[last].held = true;
            }
            if team == Team::Player {
                self.say(format!("Vehicle advancing: {kind}"));
                self.radio(crate::audio::ready_voice(kind));
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

        // an attack order overrides target choice until that enemy is gone
        let mut order = self.entities[i].order;
        if let Order::Attack(t) = order {
            if !(self.entities[t].is_target() && team.hostile(self.entities[t].team)) {
                order = Order::Hold;
                self.entities[i].order = order;
            }
        }
        // keep or pick a target; units under orders only take on what is
        // already within weapon range instead of what they can see
        let reach = if matches!(order, Order::Route) { detect } else { wrange };
        let keep = self.entities[i].target.filter(|&t| self.entities[t].is_target() && self.dist(i, t) - self.entities[t].radius() <= reach);
        let target = match order {
            Order::Attack(t) => Some(t),
            _ => keep.or_else(|| self.nearest_hostile(i, reach)),
        };
        self.entities[i].target = target;

        let kind = self.entities[i].kind.clone();
        // specials: engineers capture, trikes and bomb trucks ram, bridgers
        // become a bridge over the first gap they reach
        if class == Class::Vehicle && kind == "engineer" && self.try_capture(i) {
            return;
        }
        if class == Class::Vehicle && kind == "bridger" {
            let (x, y) = (self.entities[i].x, self.entities[i].y);
            if let Some(n) = (0..self.nodes.len()).find(|&n| self.gap(n) && (self.nodes[n].x - x).hypot(self.nodes[n].y - y) <= 0.5) {
                self.lay_bridge(i, n);
                return;
            }
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
                self.drive_off_route(i, tx, ty, dt);
                return;
            }
            if let Some(w) = weapon {
                if d <= wrange {
                    if self.entities[i].cooldown == 0.0 {
                        // the supertank row has fire delay 0; give the computer's
                        // supertank the player's own 120mm reload instead
                        let delay = if class == Class::EnemyTank { crate::tank::MAIN_RELOAD } else { stats.map_or(1.0, |s| s.fire_delay).max(1.0) / ORIGINAL_HZ };
                        self.entities[i].cooldown = delay;
                        self.fire(i, w, t);
                    }
                    return;
                }
            }
            if class == Class::EnemyTank || matches!(order, Order::Attack(_)) {
                self.drive_off_route(i, tx, ty, dt);
                return;
            }
        }
        match class {
            Class::Vehicle if self.entities[i].held => {}
            Class::Vehicle => match order {
                Order::Route => self.follow_path(i, dt),
                Order::Hold => {}
                Order::Move(x, y) => {
                    let (arrived, stuck) = self.drive_off_route(i, x, y, dt);
                    if arrived || stuck {
                        self.entities[i].order = Order::Hold;
                    }
                }
                Order::Attack(_) => self.entities[i].order = Order::Hold,
            },
            Class::EnemyTank => {
                let (hx, hy) = self.entities[i].home;
                if (self.entities[i].x - hx).hypot(self.entities[i].y - hy) > 1.0 {
                    self.drive_off_route(i, hx, hy, dt);
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
        let step = if flies(&e.kind) {
            // aircraft fly straight at the waypoint and bank round to face it
            let (dx, dy) = (dx / d, dy / d);
            let step = (speed * dt).min(d);
            e.x += dx * step;
            e.y += dy * step;
            e.odometer += step;
            return d - step < NODE_REACHED * 2.0;
        } else if off > 0.9 {
            // tracks pivot on the spot: far off course, turn first, then drive;
            // otherwise a unit can orbit a waypoint inside its turning circle
            0.0
        } else {
            (speed * dt * (1.0 - off / 1.2)).min(d)
        };
        e.x += e.heading.cos() * step;
        e.y += e.heading.sin() * step;
        e.odometer += step;
        d - step < NODE_REACHED
    }

    /// Driving anywhere but along a route: ground units stop at blocking
    /// ground (water, rock) unless a bridge spans it. Returns (arrived, stuck).
    fn drive_off_route(&mut self, i: usize, tx: f32, ty: f32, dt: f32) -> (bool, bool) {
        let (px, py) = (self.entities[i].x, self.entities[i].y);
        let arrived = self.drive_towards(i, tx, ty, dt);
        let e = &self.entities[i];
        if flies(&e.kind) || self.ground_free(e.x, e.y) || !self.ground_free(px, py) {
            return (arrived, false);
        }
        let e = &mut self.entities[i];
        e.x = px;
        e.y = py;
        (false, true)
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
        // a gap stops everything on the ground but the bridger that spans it
        let e = &self.entities[i];
        if self.gap(node) && e.kind != "bridger" && !flies(&e.kind) && (nx - e.x).hypot(ny - e.y) <= GAP_WAIT {
            return;
        }
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
        let (class, was) = (self.entities[s].class, self.entities[s].team);
        if me.team == Team::Player {
            self.radio(match class {
                Class::Bank => "narrative/bank_captured_22khz.mp3",
                Class::Tower => "narrative/turret_captured_22khz.mp3",
                _ => "narrative/building_captured_22khz.mp3",
            });
        } else if was == Team::Player {
            self.radio(if class == Class::Bank { "narrative/bank_lost_22khz.mp3" } else { "narrative/turret_lost_22khz.mp3" });
        }
        self.entities[s].team = me.team;
        self.entities[s].target = None;
        self.entities[i].alive = false;
        let who = if me.team == Team::Player { "We captured" } else { "Enemy captured" };
        self.say(format!("{who} a {}", kind.replace('_', " ")));
        true
    }

    fn fire(&mut self, i: usize, weapon: &'static str, target: usize) {
        let Some(w) = balance::weapon(weapon) else { return };
        let at = (self.entities[i].x, self.entities[i].y);
        self.sound(crate::audio::fire_sound(weapon), Some(at), 0.5);
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

    /// Supertanks lost so far.
    pub fn deaths(&self) -> u32 {
        START_LIVES - self.lives
    }

    /// The original's kill ratio: points per supertank lost (at least one).
    pub fn kill_ratio(&self) -> f32 {
        self.points as f32 / self.deaths().max(1) as f32
    }

    /// Upgrade level the next supertank is built at.
    pub fn upgrade_level(&self) -> usize {
        let ratio = self.kill_ratio();
        balance::balance().upgrade_kill_ratio.iter().filter(|&&need| ratio >= need).count()
    }

    fn step_burns(&mut self, dt: f32) {
        self.burn_clock += dt;
        while self.burn_clock >= 1.0 / ORIGINAL_HZ {
            self.burn_clock -= 1.0 / ORIGINAL_HZ;
            let burns = std::mem::take(&mut self.burns);
            for (j, amount, ticks, by_stank) in burns {
                if !self.entities[j].alive {
                    continue;
                }
                self.hurt_by(j, amount, by_stank);
                if ticks > 1 {
                    self.burns.push((j, amount, ticks - 1, by_stank));
                }
            }
        }
    }

    /// Apply `weapon`'s damage at (x, y) from `team`. Area weapons hurt every
    /// hostile within their radius; single-target weapons only `direct`.
    pub fn damage(&mut self, weapon: &str, x: f32, y: f32, direct: Option<usize>, team: Team) {
        self.damage_by(weapon, x, y, direct, team, false);
    }

    /// As `damage`, noting whether the player's supertank dealt it (for points).
    /// A single-target weapon with a tick count (acid, napalm, electric beams)
    /// does its damage once per original tick for that many ticks.
    pub fn damage_by(&mut self, weapon: &str, x: f32, y: f32, direct: Option<usize>, team: Team, by_stank: bool) {
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
            if !w.area && w.radius >= 1.0 {
                self.burns.push((j, amount, w.radius as u32, by_stank));
                self.entities[j].flash = 0.15;
            } else {
                self.hurt_by(j, amount, by_stank);
            }
        }
    }

    fn hurt_by(&mut self, j: usize, amount: f32, by_stank: bool) {
        let was_alive = self.entities[j].alive && self.entities[j].hp > 0.0;
        self.hurt_inner(j, amount);
        let e = &self.entities[j];
        if by_stank && was_alive && !e.alive && e.team == Team::Enemy {
            let before = self.upgrade_level();
            let score = match e.class {
                Class::EnemyTank => balance::unit("stank").map_or(0, |s| s.cost),
                _ => e.stats().map_or(0, |s| s.cost),
            };
            self.points += score;
            let after = self.upgrade_level();
            if after > before {
                let (main, rocket, _, armor) = crate::tank::UPGRADE_LEVELS[after.min(2)];
                self.say(format!("Upgrade earned! Next supertank: {main}, {}, {armor} armor", rocket.replace('_', " ")));
            }
        }
    }

    fn hurt_inner(&mut self, j: usize, amount: f32) {
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
            self.sound("explosion/generic.mp3", Some((x, y)), 1.0);
            self.radio("narrative/supertank_lost_22khz.mp3");
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.entities[j].alive = false;
                self.outcome = Outcome::Lost;
                self.verdict = "No supertanks left.".into();
                self.say("Supertank lost. No supertanks left.");
            } else {
                self.say(format!("Supertank lost. {} left.", self.lives));
            }
            return;
        }
        e.alive = false;
        self.blasts.push(Blast { x, y, power: 1.0 + r });
        let boom = if matches!(class, Class::Vehicle | Class::EnemyTank | Class::Plane) { "explosion/ground_vehicle.mp3" } else { "explosion/generic.mp3" };
        self.sound(boom, Some((x, y)), 1.0);
        match (class, team) {
            (Class::Hq, Team::Enemy) => self.radio("narrative/enemy_base_destroyed_22khz.mp3"),
            (Class::Hq, Team::Player) => self.radio("narrative/mission_failed_22khz.mp3"),
            (Class::Bank, Team::Player) => self.radio("narrative/bank_lost_22khz.mp3"),
            (Class::Tower, Team::Player) => self.radio("narrative/turret_lost_22khz.mp3"),
            _ => {}
        }
        log::info!("destroyed {team:?} {kind}");
        if class == Class::Bridge {
            self.say(if team == Team::Player { "Our bridge is down" } else { "Enemy bridge destroyed" });
            return;
        }
        // whether that decides the battle is the map's rules' call (`check_rules`)
        match (class, team) {
            (Class::Hq, Team::Enemy) => self.say("Enemy base destroyed!"),
            (Class::Hq, Team::Player) => self.say("Our base has been destroyed."),
            (Class::EnemyTank, _) => self.say("Enemy supertank destroyed!"),
            (_, Team::Enemy) if class != Class::Vehicle => self.say(format!("Destroyed enemy {}", kind.replace('_', " "))),
            (_, Team::Player) if class != Class::Vehicle => self.say(format!("Lost our {}", kind.replace('_', " "))),
            _ => {}
        }
    }

    /// The player's unit nearest (x, y) within `reach`, for selection.
    pub fn player_unit_at(&self, x: f32, y: f32, reach: f32) -> Option<usize> {
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive && e.class == Class::Vehicle && e.team == Team::Player)
            .map(|(i, e)| (i, (e.x - x).hypot(e.y - y)))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The player's units inside a map rectangle (corners in any order).
    pub fn player_units_in(&self, a: (f32, f32), b: (f32, f32)) -> Vec<usize> {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        (0..self.entities.len())
            .filter(|&i| {
                let e = &self.entities[i];
                e.alive && e.class == Class::Vehicle && e.team == Team::Player && (x0..=x1).contains(&e.x) && (y0..=y1).contains(&e.y)
            })
            .collect()
    }

    /// The enemy nearest (x, y) within `reach`: a right-click attack target.
    pub fn enemy_at(&self, x: f32, y: f32, reach: f32) -> Option<usize> {
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_target() && e.team == Team::Enemy)
            .map(|(i, e)| (i, (e.x - x).hypot(e.y - y) - e.radius()))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Give the selected player units a command. Several units sent to one
    /// spot spread into a small grid around it.
    pub fn command(&mut self, selected: &[usize], cmd: Command) {
        let units: Vec<usize> = selected.iter().copied().filter(|&i| self.entities[i].alive && self.entities[i].team == Team::Player).collect();
        let side = (units.len() as f32).sqrt().ceil().max(1.0) as usize;
        for (k, &i) in units.iter().enumerate() {
            let order = match cmd {
                Command::Hold => Order::Hold,
                Command::Attack(t) => Order::Attack(t),
                Command::Move(x, y) => {
                    let (gx, gy) = ((k % side) as f32 - (side as f32 - 1.0) / 2.0, (k / side) as f32 - (side as f32 - 1.0) / 2.0);
                    Order::Move(x + gx * 1.2, y + gy * 1.2)
                }
                Command::Resume => {
                    let (x, y, team) = (self.entities[i].x, self.entities[i].y, self.entities[i].team);
                    self.entities[i].next_node = self.nearest_node(x, y, team, true);
                    Order::Route
                }
            };
            self.entities[i].order = order;
            self.entities[i].target = None;
        }
        if !units.is_empty() {
            log::info!("command {cmd:?} to {} units", units.len());
        }
    }

    /// A standing structure overlapping a vehicle of radius `r` at (x, y).
    pub fn blocks(&self, x: f32, y: f32, r: f32) -> bool {
        self.entities.iter().any(|e| {
            e.alive && matches!(e.class, Class::Hq | Class::Garage | Class::Airbase | Class::Bank | Class::Tower) && (e.x - x).hypot(e.y - y) < e.radius() + r
        })
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
mod rules {
    use super::*;
    use crate::game::LEVELS;

    #[test]
    fn supertank_kills_earn_points_and_upgrades() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        let foe = w.find(Class::EnemyTank, Team::Enemy).unwrap();
        let (x, y) = (w.entities[foe].x, w.entities[foe].y);
        assert_eq!(w.upgrade_level(), 0);
        while w.entities[foe].alive {
            w.damage_by("b120mm", x, y, Some(foe), Team::Player, true);
        }
        // the supertank's cost (10000) is above the first threshold (1000)
        assert_eq!(w.points, 10000);
        assert_eq!(w.upgrade_level(), 1);
    }

    #[test]
    fn move_then_hold_and_resume() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        w.order(Team::Player, "peon_tank");
        w.step(1.0 / 30.0);
        let u = w.entities.iter().rposition(|e| e.kind == "peon_tank" && e.team == Team::Player).unwrap();
        let (x, y) = (w.entities[u].x - 3.0, w.entities[u].y);
        w.command(&[u], Command::Move(x, y));
        for _ in 0..30 * 10 {
            w.step(1.0 / 30.0);
        }
        let e = &w.entities[u];
        assert_eq!(e.order, Order::Hold, "arrived and holding");
        assert!((e.x - x).hypot(e.y - y) < 0.6, "at ({:.1},{:.1})", e.x, e.y);
        let parked = (e.x, e.y);
        for _ in 0..30 * 3 {
            w.step(1.0 / 30.0);
        }
        assert_eq!((w.entities[u].x, w.entities[u].y), parked, "holding means not moving");
        w.command(&[u], Command::Resume);
        assert_eq!(w.entities[u].order, Order::Route);
        assert!(w.entities[u].next_node.is_some());
    }

    #[test]
    fn attack_order_chases_and_hits_its_target() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        let foe = w.entities.iter().position(|e| e.class == Class::Tower && e.team == Team::Enemy).unwrap();
        // a peon tank placed 8 cells from the tower, out of its 3-cell gun range
        w.order(Team::Player, "peon_tank");
        w.step(1.0 / 30.0);
        let u = w.entities.iter().rposition(|e| e.kind == "peon_tank" && e.team == Team::Player).unwrap();
        w.entities[u].x = w.entities[foe].x - 8.0;
        w.entities[u].y = w.entities[foe].y;
        let hp = w.entities[foe].hp;
        w.command(&[u], Command::Attack(foe));
        for _ in 0..30 * 20 {
            w.step(1.0 / 30.0);
        }
        assert!(!w.entities[u].alive || w.entities[foe].hp < hp, "the tower should have taken fire");
    }

    #[test]
    fn orders_for_a_lost_factory_are_refunded() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        assert!(w.order(Team::Player, "peon_tank"));
        assert!(w.order(Team::Player, "peon_tank"));
        w.step(1.0 / 30.0); // first one deploys
        let g = w.find(Class::Garage, Team::Player).unwrap();
        w.entities[g].alive = false;
        for _ in 0..30 * 2 {
            w.step(1.0 / 30.0);
        }
        assert_eq!(w.money[0], START_MONEY - 300);
    }

    /// A river across a small map, one bridge spot on the road over it.
    fn river_map() -> World {
        river_map_with("")
    }

    fn river_map_with(header: &str) -> World {
        let mut tiles = String::new();
        for y in 0..30 {
            tiles.push_str(&(if y == 15 { "w" } else { "g" }).repeat(12));
            tiles.push('\n');
        }
        let heights = format!("{}\n", "05".repeat(13)).repeat(31);
        let mut objects = String::from("mainbasepad 1 6 28 0 4000\ngarage 1 3 27 0 1000\nmainbasepad 2 6 2 0 4000\n");
        // nodes 3..=8 run north from the garage; node 6 sits on the river
        for (k, y) in [26.5f32, 21.5, 18.5, 15.5, 12.5, 6.5].iter().enumerate() {
            let n = 3 + k;
            let a = if k < 5 { format!(" A:{}", n + 1) } else { String::new() };
            let bridge = if n == 6 { " bridge" } else { "" };
            objects.push_str(&format!("path_object 1 6.5 {y} 0 100{a}{bridge}\n"));
        }
        let q = "'''";
        let text = format!(
            "name = 'river'\nwidth = 12\nheight = 30\n{header}build = ['peon_tank', 'bridger']\nlegend = {q}\ng 0 grassfull\nw 6 egyptwater1\n{q}\ntiles = {q}\n{tiles}{q}\nheights = {q}\n{heights}{q}\nobjects = {q}\n{objects}{q}\n"
        );
        World::from_level(&crate::cmap::parse(&text).unwrap().0)
    }

    #[test]
    fn holding_a_site_runs_its_clock() {
        let mut w = river_map_with("win = ['hold 1 6 22 3 10']\n");
        let tick = |w: &mut World, secs: f32| {
            for _ in 0..(secs * 30.0) as usize {
                w.step(1.0 / 30.0);
            }
        };
        let unit = |w: &mut World, team: Team, x: f32, y: f32| {
            let mut e = Entity::shown("peon_tank", Class::Vehicle, team, 50.0);
            (e.x, e.y, e.order) = (x, y, Order::Hold);
            // unarmed for the test: nothing shoots
            e.kind = "bridger".into();
            w.entities.push(e);
            w.entities.len() - 1
        };
        tick(&mut w, 2.0);
        assert_eq!(w.sites()[0].5, 0.0, "nobody on the site");
        let ours = unit(&mut w, Team::Player, 6.0, 21.0);
        tick(&mut w, 4.0);
        assert!((w.sites()[0].5 - 4.0).abs() < 0.2);
        // both there: paused
        let theirs = unit(&mut w, Team::Enemy, 7.0, 23.0);
        tick(&mut w, 3.0);
        assert!((w.sites()[0].5 - 4.0).abs() < 0.2);
        // only theirs: back to zero
        w.entities[ours].alive = false;
        tick(&mut w, 1.0);
        assert_eq!(w.sites()[0].5, 0.0);
        w.entities[theirs].alive = false;
        unit(&mut w, Team::Player, 6.0, 22.0);
        tick(&mut w, 9.0);
        assert_eq!(w.outcome, Outcome::Playing);
        tick(&mut w, 1.5);
        assert_eq!(w.outcome, Outcome::Won);
        assert_eq!(w.verdict, "We held the site.");
    }

    #[test]
    fn bridgers_span_gaps_and_units_wait_for_them() {
        let mut w = river_map();
        let spot = w.nodes.iter().position(|n| n.bridge_spot).unwrap();
        assert!(w.gap(spot) && !w.on_bridge(6.5, 15.5));
        assert!(!w.can_build("rocket_tank") && !w.order(Team::Player, "rocket_tank"));
        assert!(w.order(Team::Player, "peon_tank"));
        for _ in 0..30 * 60 {
            w.step(1.0 / 30.0);
        }
        let peon = w.entities.iter().position(|e| e.kind == "peon_tank").unwrap();
        assert!(w.entities[peon].y > 15.5 + GAP_WAIT - 0.1, "the peon waits south of the river, at {}", w.entities[peon].y);
        // ordered straight across, it stops at the water
        w.command(&[peon], Command::Move(6.5, 10.0));
        for _ in 0..30 * 20 {
            w.step(1.0 / 30.0);
        }
        assert!(w.entities[peon].y > 15.0, "no driving through the river");
        w.command(&[peon], Command::Resume);
        assert!(w.order(Team::Player, "bridger"));
        for _ in 0..30 * 60 {
            w.step(1.0 / 30.0);
        }
        let bridge = w.entities.iter().position(|e| e.class == Class::Bridge).expect("the bridger lays itself");
        assert!(!w.gap(spot) && w.on_bridge(6.5, 15.5) && w.on_bridge(6.5, 16.0));
        assert!(w.entities[peon].y < 15.0, "the peon crosses once the bridge is down");
        // shot away, the gap opens again
        w.hurt_inner(bridge, 1000.0);
        assert!(w.gap(spot) && !w.on_bridge(6.5, 15.5));
    }

    #[test]
    fn helicopters_fly_the_air_route() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        assert!(w.order(Team::Player, "helicopter"));
        w.step(1.0 / 30.0);
        let h = w.entities.iter().rposition(|e| e.kind == "helicopter" && e.team == Team::Player).unwrap();
        let airbase = w.find(Class::Airbase, Team::Player).unwrap();
        assert!((w.entities[h].x - w.entities[airbase].x).abs() < 0.1, "starts at the airbase");
        for _ in 0..30 * 15 {
            w.step(1.0 / 30.0);
        }
        // 0.16 cells per tick: 15 s is about 24 cells of route
        assert!(!w.entities[h].alive || w.entities[h].odometer > 15.0, "flew {}", w.entities[h].odometer);
    }

    #[test]
    fn money_planes_fly_crates_home() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        let planes = w.entities.iter().filter(|e| e.class == Class::Plane && e.team == Team::Player).count();
        assert!(planes > 0, "the level places money planes at the banks");
        for _ in 0..30 * 49 {
            w.step(1.0 / 30.0);
        }
        assert_eq!(w.money[0], START_MONEY, "no money before the first crate");
        for _ in 0..30 * 60 {
            w.step(1.0 / 30.0);
        }
        assert!(w.money[0] >= START_MONEY + BANK_CRATE, "a crate should have been flown home: {}", w.money[0]);
    }

    #[test]
    fn acid_burns_over_forty_ticks() {
        let mut w = World::from_level(&Level::parse(LEVELS[0].data).unwrap());
        w.ai_timer = f32::MAX;
        let tower = w.entities.iter().position(|e| e.class == Class::Tower && e.team == Team::Enemy).unwrap();
        let (x, y, hp) = (w.entities[tower].x, w.entities[tower].y, w.entities[tower].hp);
        w.damage_by("acid", x, y, Some(tower), Team::Player, true);
        assert_eq!(w.entities[tower].hp, hp, "acid does nothing up front");
        for _ in 0..30 * 5 {
            w.step(1.0 / 30.0);
        }
        // 2 per tick for 40 ticks; other units may chip in, so at least 80
        assert!(hp - w.entities[tower].hp >= 80.0 - 1e-3, "burned {}", hp - w.entities[tower].hp);
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
