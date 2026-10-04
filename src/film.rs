//! Films: a recording of what the follow view shows, tick by tick, for the
//! credits finale (and `--film FILE` to watch one).
//!
//! A film is not an input recording. Replaying inputs through the
//! simulation (a Doom-style demo) would desync with every balance, AI or
//! physics change; a film stores the visible state instead, so it keeps
//! playing whatever happens to the rules later.
//!
//! Format: gzip-compressed text, one record per line.
//!
//! ```text
//! calvary-film 1
//! level test                     a bundled level, or:
//! cmap 1234                      a .cmap map, followed by its 1234 bytes
//! t TIME CHASE X Y HEADING TURRET FLASH PLAYER_ENTITY   starts each tick
//! n I KIND CLASS TEAM MAX_HP     entity I (re)defined (first tick: all of them)
//! e I X Y HEADING TURRET HP ALIVE FLASH   entity I's state, when it changed
//! s X Y Z ...                    shots in flight
//! b X Y ...                      bolts in flight
//! x X Y POWER                    an explosion
//! ```
//!
//! Positions and angles are rounded to 0.01, which also makes the
//! unchanged-entity test cheap and the gzip small (about 100-300 KB a
//! minute of battle).

use crate::game::{Game, LEVELS};
use crate::world::{Class, Entity, Team};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::fmt::Write as _;
use std::io::{self, Read, Write};

const CLASSES: [(Class, &str); 10] = [
    (Class::Hq, "hq"),
    (Class::Garage, "garage"),
    (Class::Airbase, "airbase"),
    (Class::Bank, "bank"),
    (Class::Tower, "tower"),
    (Class::Vehicle, "vehicle"),
    (Class::PlayerTank, "stank"),
    (Class::EnemyTank, "estank"),
    (Class::Plane, "plane"),
    (Class::Prop, "prop"),
];
const TEAMS: [(Team, &str); 4] = [(Team::Player, "p"), (Team::Enemy, "e"), (Team::Neutral, "n"), (Team::Scenery, "s")];

fn q(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    // no "-0"
    if r == 0.0 { "0".into() } else { format!("{r}") }
}

/// What a recorded entity looked like last tick, as written.
#[derive(Clone, PartialEq)]
struct Seen {
    kind: String,
    class: Class,
    team: Team,
    state: String,
}

/// Records a running battle; `capture` after every simulation tick.
pub struct Recorder {
    text: String,
    seen: Vec<Option<Seen>>,
    pub ticks: usize,
}

impl Recorder {
    /// `cmap`: the map's text when the battle is on a `.cmap` map.
    pub fn new(game: &Game, cmap: Option<&str>) -> Recorder {
        let mut text = String::from("calvary-film 1\n");
        match cmap {
            Some(src) => {
                let _ = writeln!(text, "cmap {}", src.len());
                text.push_str(src);
                text.push('\n');
            }
            None => {
                let _ = writeln!(text, "level {}", LEVELS[game.level_index].name);
            }
        }
        Recorder { text, seen: Vec::new(), ticks: 0 }
    }

    pub fn capture(&mut self, g: &Game) {
        let t = &g.tank;
        let player = g.world.player_tank.map_or(-1, |i| i as i64);
        let _ = writeln!(self.text, "t {} {} {} {} {} {} {} {player}", q(g.time), q(g.chase_heading), q(t.x), q(t.y), q(t.heading), q(t.turret), q(t.flash));
        for (i, e) in g.world.entities.iter().enumerate() {
            let state = format!("{} {} {} {} {} {} {}", q(e.x), q(e.y), q(e.heading), q(e.turret), q(e.hp), e.alive as u8, q(e.flash));
            if i >= self.seen.len() {
                self.seen.push(None);
            }
            let prev = &self.seen[i];
            let same_kind = prev.as_ref().is_some_and(|s| s.kind == e.kind && s.class == e.class && s.team == e.team);
            if !same_kind {
                let class = CLASSES.iter().find(|c| c.0 == e.class).map_or("prop", |c| c.1);
                let team = TEAMS.iter().find(|c| c.0 == e.team).map_or("s", |c| c.1);
                let _ = writeln!(self.text, "n {i} {} {class} {team} {}", e.kind, q(e.max_hp));
            }
            if !same_kind || prev.as_ref().is_some_and(|s| s.state != state) {
                let _ = writeln!(self.text, "e {i} {state}");
            }
            self.seen[i] = Some(Seen { kind: e.kind.clone(), class: e.class, team: e.team, state });
        }
        if !g.shots.is_empty() {
            self.text.push('s');
            for s in &g.shots {
                let _ = write!(self.text, " {} {} {}", q(s.x), q(s.y), q(s.z));
            }
            self.text.push('\n');
        }
        if !g.world.bolts.is_empty() {
            self.text.push('b');
            for b in &g.world.bolts {
                let _ = write!(self.text, " {} {}", q(b.x), q(b.y));
            }
            self.text.push('\n');
        }
        for b in &g.last_blasts {
            let _ = writeln!(self.text, "x {} {} {}", q(b.x), q(b.y), q(b.power));
        }
        self.ticks += 1;
    }

    /// Gzip the film and write it.
    pub fn save(&self, path: &std::path::Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut gz = GzEncoder::new(std::fs::File::create(path)?, flate2::Compression::best());
        gz.write_all(self.text.as_bytes())?;
        gz.finish()?;
        Ok(())
    }
}

/// One tick of a film.
#[derive(Default)]
struct Tick {
    time: f32,
    chase: f32,
    tank: [f32; 5],
    player: Option<usize>,
    defs: Vec<(usize, String, Class, Team, f32)>,
    states: Vec<(usize, [f32; 7])>,
    shots: Vec<(f32, f32, f32)>,
    bolts: Vec<(f32, f32)>,
    blasts: Vec<(f32, f32, f32)>,
}

/// A loaded film, ready to play into a `Game`.
pub struct Film {
    ticks: Vec<Tick>,
    level: FilmLevel,
}

enum FilmLevel {
    Bundled(usize),
    Cmap(String),
}

impl Film {
    pub fn parse(gz: &[u8]) -> Result<Film, String> {
        let mut text = String::new();
        GzDecoder::new(gz).read_to_string(&mut text).map_err(|e| format!("not a film: {e}"))?;
        let mut rest = text.as_str();
        let mut line = || -> Option<&str> {
            let (l, r) = rest.split_once('\n').unwrap_or((rest, ""));
            rest = r;
            (!l.is_empty() || !r.is_empty()).then_some(l)
        };
        if line() != Some("calvary-film 1") {
            return Err("not a calvary-film 1 file".into());
        }
        let head = line().ok_or("no level line")?;
        let level = if let Some(name) = head.strip_prefix("level ") {
            FilmLevel::Bundled(LEVELS.iter().position(|l| l.name == name).ok_or_else(|| format!("no bundled level {name}"))?)
        } else if let Some(n) = head.strip_prefix("cmap ") {
            let n: usize = n.parse().map_err(|_| "bad cmap length")?;
            drop(line);
            let src = rest.get(..n).ok_or("cmap cut short")?.to_string();
            rest = rest.get(n + 1..).unwrap_or("");
            FilmLevel::Cmap(src)
        } else {
            return Err(format!("bad level line: {head}"));
        };
        let mut ticks: Vec<Tick> = Vec::new();
        for l in rest.lines() {
            let mut it = l.split(' ');
            let tag = it.next().unwrap_or("");
            let nums = |it: std::str::Split<'_, char>| it.map(|v| v.parse::<f32>().unwrap_or(0.0)).collect::<Vec<f32>>();
            match tag {
                "t" => {
                    let v = nums(it);
                    if v.len() < 8 {
                        return Err(format!("bad tick: {l}"));
                    }
                    ticks.push(Tick { time: v[0], chase: v[1], tank: [v[2], v[3], v[4], v[5], v[6]], player: (v[7] >= 0.0).then_some(v[7] as usize), ..Tick::default() });
                }
                _ if ticks.is_empty() => return Err(format!("record before the first tick: {l}")),
                "n" => {
                    let f: Vec<&str> = it.collect();
                    if f.len() < 5 {
                        return Err(format!("bad entity: {l}"));
                    }
                    let class = CLASSES.iter().find(|c| c.1 == f[2]).map_or(Class::Prop, |c| c.0);
                    let team = TEAMS.iter().find(|c| c.1 == f[3]).map_or(Team::Scenery, |c| c.0);
                    let i = f[0].parse().map_err(|_| format!("bad entity: {l}"))?;
                    ticks.last_mut().unwrap().defs.push((i, f[1].to_string(), class, team, f[4].parse().unwrap_or(1.0)));
                }
                "e" => {
                    let v = nums(it);
                    if v.len() < 8 {
                        return Err(format!("bad state: {l}"));
                    }
                    ticks.last_mut().unwrap().states.push((v[0] as usize, [v[1], v[2], v[3], v[4], v[5], v[6], v[7]]));
                }
                "s" => ticks.last_mut().unwrap().shots = nums(it).chunks_exact(3).map(|c| (c[0], c[1], c[2])).collect(),
                "b" => ticks.last_mut().unwrap().bolts = nums(it).chunks_exact(2).map(|c| (c[0], c[1])).collect(),
                "x" => {
                    let v = nums(it);
                    if v.len() >= 3 {
                        ticks.last_mut().unwrap().blasts.push((v[0], v[1], v[2]));
                    }
                }
                _ => {}
            }
        }
        if ticks.is_empty() {
            return Err("empty film".into());
        }
        Ok(Film { ticks, level })
    }

    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    /// A game on the film's level, set up to play it from the start.
    pub fn stage(&self) -> Result<Game, String> {
        let mut g = match &self.level {
            FilmLevel::Bundled(i) => Game::new(*i)?,
            FilmLevel::Cmap(src) => {
                let (level, name, _) = crate::cmap::parse(src)?;
                Game::from_level(&name, &level)
            }
        };
        g.world.entities.clear();
        g.show_aim = false;
        g.view = crate::game::View::Chase;
        Ok(g)
    }

    /// Apply tick `i` to a game staged by `stage` (ticks must come in order
    /// from 0; changes accumulate).
    pub fn apply(&self, i: usize, g: &mut Game) {
        let Some(t) = self.ticks.get(i) else { return };
        g.time = t.time;
        g.chase_heading = t.chase;
        (g.tank.x, g.tank.y, g.tank.heading, g.tank.turret, g.tank.flash) = (t.tank[0], t.tank[1], t.tank[2], t.tank[3], t.tank[4]);
        g.world.player_tank = t.player;
        for (idx, kind, class, team, max_hp) in &t.defs {
            let e = Entity::shown(kind, *class, *team, *max_hp);
            if *idx < g.world.entities.len() {
                g.world.entities[*idx] = e;
            } else {
                while g.world.entities.len() < *idx {
                    g.world.entities.push(Entity::shown("none", Class::Prop, Team::Scenery, 1.0));
                }
                g.world.entities.push(e);
            }
        }
        for (idx, v) in &t.states {
            if let Some(e) = g.world.entities.get_mut(*idx) {
                (e.x, e.y, e.heading, e.turret, e.hp) = (v[0], v[1], v[2], v[3], v[4]);
                e.alive = v[5] != 0.0;
                e.flash = v[6];
            }
        }
        g.shots = t.shots.iter().map(|&(x, y, z)| crate::game::Shot::shown(x, y, z)).collect();
        g.world.bolts = t.bolts.iter().map(|&(x, y)| crate::world::Bolt::shown(x, y)).collect();
        for &(x, y, power) in &t.blasts {
            g.blast(x, y, power);
        }
        g.step_particles();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tank::Controls;

    /// A recorded battle plays back into the same picture: tank, every
    /// entity, shots and bolts match the original at every tick.
    #[test]
    fn film_round_trip() {
        let mut g = Game::new(0).unwrap();
        let mut rec = Recorder::new(&g, None);
        let mut frames = Vec::new();
        for k in 0..30 * 40 {
            let c = Controls { throttle: Some(0.7), turn: ((k / 90) % 3) as f32 - 1.0, fire_main: k % 20 == 0, ..Controls::default() };
            if k % 150 == 0 {
                g.world.order(Team::Player, crate::world::BUILDABLE[0]);
            }
            g.update(&c);
            g.world.sounds.clear();
            rec.capture(&g);
            frames.push((g.tank.x, g.tank.y, g.world.entities.iter().map(|e| (e.x, e.y, e.alive)).collect::<Vec<_>>(), g.shots.len()));
        }
        let path = std::env::temp_dir().join(format!("calvary-film-test-{}.cfilm", std::process::id()));
        rec.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let film = Film::parse(&bytes).unwrap();
        assert_eq!(film.len(), frames.len());
        let mut p = film.stage().unwrap();
        for (i, (x, y, ents, shots)) in frames.iter().enumerate() {
            film.apply(i, &mut p);
            assert!((p.tank.x - x).abs() <= 0.006 && (p.tank.y - y).abs() <= 0.006, "tick {i}");
            assert_eq!(p.world.entities.len(), ents.len(), "tick {i}");
            for (j, (e, (ex, ey, alive))) in p.world.entities.iter().zip(ents).enumerate() {
                assert!((e.x - ex).abs() <= 0.006 && (e.y - ey).abs() <= 0.006 && e.alive == *alive, "tick {i} entity {j}");
            }
            assert_eq!(p.shots.len(), *shots, "tick {i}");
        }
        eprintln!("{} ticks, {} bytes gzipped", film.len(), bytes.len());
    }
}
