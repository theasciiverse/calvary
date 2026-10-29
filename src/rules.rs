//! How a map is won or lost, after the original's per-level hooks: every
//! level's `.scm` could define `loose_function` and `winning_function`
//! (defaults in `scheme/map_init.scm`), which the game called every 16 ticks,
//! the losing one first ("if the player destroys the designated target but
//! dies from the explosion himself, he has lost"). The fork's `test.scm` uses
//! `(get_object_of_player "mainbasepad" local_player)` to lose and
//! `(player_has_no_units 2)` to win.
//!
//! Calvary keeps those conditions as data in the `.cmap` instead of Lisp:
//!
//! ```toml
//! win = ["no_units 2"]          # player_has_no_units: nothing of player 2 left
//! lose = ["no mainbasepad 1"]   # get_object_of_player is nil
//! win = ["hold 1 55 42 4 90"]   # player 1 holds 4 cells round (55, 42) for 90 s
//! ```
//!
//! Holding follows the original's trigger objects (`trigger_vars` in
//! `scheme/classes.scm`: a range round a point, which team and which units
//! set it off) and the `tick` every win and lose function was given: the
//! clock runs while the player has ground units or a supertank inside and
//! the other side has none, pauses while both are there or neither, and
//! starts over when only the other side is. Once run out it stays won.
//!
//! The map is lost as soon as any `lose` condition holds, and won when every
//! `win` condition holds. Losing the last supertank always loses (the
//! original's `start.scm` lives). Without these keys a map plays as the
//! bundled battles do: lose your HQ, or destroy theirs.

use crate::world::{Class, Team, World};

#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    /// `no_units P`: player P has nothing left on the field (the original's
    /// `player_has_no_units`: every object the player owns, buildings too).
    NoUnits(u8),
    /// `no KIND P`: player P has no KIND left (`get_object_of_player` is nil).
    No(String, u8),
    /// `own KIND P [N]`: player P has N of KIND standing (default one; taken
    /// by engineers, say: `own takeover_pad 1 2`).
    Own(String, u8, usize),
    /// `time SECS`: the battle has lasted SECS seconds (a rearguard holds
    /// out; the original's win and lose functions were given the tick).
    Time(f32),
    /// `reach P KIND X Y R N`: N of player P's KIND stand within R of
    /// (X, Y) (a convoy brought home: `reach 1 car 104 48 5 4`).
    Reach { player: u8, kind: String, x: f32, y: f32, r: f32, n: usize },
    /// `lost P KIND N`: N of player P's KIND have been destroyed.
    Lost { player: u8, kind: String, n: usize },
    /// `hold P X Y R SECS`: player P has held the ground within R of
    /// (X, Y) for SECS seconds.
    Hold { player: u8, x: f32, y: f32, r: f32, secs: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    pub win: Vec<Cond>,
    pub lose: Vec<Cond>,
}

impl Default for Rules {
    fn default() -> Rules {
        Rules { win: vec![Cond::No("mainbasepad".into(), 2)], lose: vec![Cond::No("mainbasepad".into(), 1)] }
    }
}

impl Cond {
    pub fn parse(text: &str) -> Result<Cond, String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let player = |w: &str| w.parse::<u8>().map_err(|_| format!("condition {text:?}: bad player {w:?}"));
        match words.as_slice() {
            ["no_units", p] => Ok(Cond::NoUnits(player(p)?)),
            ["no", kind, p] => Ok(Cond::No(kind.to_string(), player(p)?)),
            ["own", kind, p] => Ok(Cond::Own(kind.to_string(), player(p)?, 1)),
            ["own", kind, p, n] => Ok(Cond::Own(kind.to_string(), player(p)?, n.parse().map_err(|_| format!("condition {text:?}: bad count {n:?}"))?)),
            ["time", secs] => Ok(Cond::Time(secs.parse().map_err(|_| format!("condition {text:?}: bad seconds {secs:?}"))?)),
            ["reach", p, kind, x, y, r, n] => {
                let num = |w: &str| w.parse::<f32>().map_err(|_| format!("condition {text:?}: bad number {w:?}"));
                let n = n.parse().map_err(|_| format!("condition {text:?}: bad count {n:?}"))?;
                Ok(Cond::Reach { player: player(p)?, kind: kind.to_string(), x: num(x)?, y: num(y)?, r: num(r)?, n })
            }
            ["lost", p, kind, n] => Ok(Cond::Lost { player: player(p)?, kind: kind.to_string(), n: n.parse().map_err(|_| format!("condition {text:?}: bad count {n:?}"))? }),
            ["hold", p, x, y, r, secs] => {
                let num = |w: &str| w.parse::<f32>().map_err(|_| format!("condition {text:?}: bad number {w:?}"));
                Ok(Cond::Hold { player: player(p)?, x: num(x)?, y: num(y)?, r: num(r)?, secs: num(secs)? })
            }
            _ => Err(format!("condition {text:?}: want `no_units PLAYER`, `no KIND PLAYER`, `own KIND PLAYER [COUNT]`, `time SECONDS`, `hold PLAYER X Y RADIUS SECONDS`, `reach PLAYER KIND X Y RADIUS COUNT` or `lost PLAYER KIND COUNT`")),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Cond::NoUnits(p) => format!("no_units {p}"),
            Cond::No(kind, p) => format!("no {kind} {p}"),
            Cond::Own(kind, p, 1) => format!("own {kind} {p}"),
            Cond::Own(kind, p, n) => format!("own {kind} {p} {n}"),
            Cond::Time(secs) => format!("time {secs}"),
            Cond::Hold { player, x, y, r, secs } => format!("hold {player} {x} {y} {r} {secs}"),
            Cond::Reach { player, kind, x, y, r, n } => format!("reach {player} {kind} {x} {y} {r} {n}"),
            Cond::Lost { player, kind, n } => format!("lost {player} {kind} {n}"),
        }
    }

    pub fn holds(&self, w: &World) -> bool {
        let owned = |p: u8| {
            let team = Team::from_player(p);
            move |e: &crate::world::Entity| e.alive && e.team == team && e.class != Class::Prop
        };
        match self {
            Cond::NoUnits(p) => !w.entities.iter().any(owned(*p)),
            Cond::No(kind, p) => {
                let owned = owned(*p);
                !w.entities.iter().any(|e| owned(e) && e.kind == *kind)
            }
            Cond::Own(kind, p, n) => {
                let team = Team::from_player(*p);
                w.entities.iter().filter(|e| e.alive && e.team == team && e.kind == *kind).count() >= *n
            }
            Cond::Time(secs) => w.elapsed() >= *secs,
            Cond::Hold { secs, .. } => w.held_for(self) >= *secs,
            Cond::Reach { player, kind, x, y, r, n } => {
                let team = Team::from_player(*player);
                w.entities.iter().filter(|e| e.alive && e.team == team && e.kind == *kind && (e.x - x).hypot(e.y - y) <= *r).count() >= *n
            }
            Cond::Lost { player, kind, n } => {
                let team = Team::from_player(*player);
                w.entities.iter().filter(|e| !e.alive && e.team == team && e.kind == *kind).count() >= *n
            }
        }
    }

    /// What the condition means, for the player.
    pub fn describe(&self) -> String {
        let side = |p: u8| if p == 1 { "our" } else { "the enemy's" };
        let name = |kind: &str| match kind {
            "mainbasepad" => "base".to_string(),
            "takeover_pad" => "takeover pad".to_string(),
            "tower_electric" => "electric tower".to_string(),
            "tower_missile" => "missile tower".to_string(),
            "car" => "supply truck".to_string(),
            k => k.replace('_', " "),
        };
        match self {
            Cond::NoUnits(1) => "Nothing of ours is left on the field.".into(),
            Cond::NoUnits(_) => "Every enemy unit and tower is destroyed.".into(),
            Cond::Own(kind, 1, 1) => format!("We have taken the {}.", name(kind)),
            Cond::Own(kind, 1, n) => format!("We have taken {n} {}s.", name(kind)),
            Cond::Own(kind, _, _) => format!("The enemy has taken the {}.", name(kind)),
            Cond::Time(_) => "We held out long enough.".into(),
            Cond::Hold { player: 1, .. } => "We held the site.".into(),
            Cond::Hold { .. } => "The enemy held the site.".into(),
            Cond::Reach { player: 1, kind, n, .. } => format!("{n} {}s got through.", name(kind)),
            Cond::Reach { kind, n, .. } => format!("{n} enemy {}s got through.", name(kind)),
            Cond::Lost { player: 1, kind, n } => format!("We lost {n} {}s: too few can get through.", name(kind)),
            Cond::Lost { kind, n, .. } => format!("The enemy lost {n} {}s.", name(kind)),
            Cond::No(kind, 2) if kind != "mainbasepad" => format!("Every enemy {} is destroyed.", name(kind)),
            Cond::No(kind, 1) if kind != "mainbasepad" => format!("We have lost every {}.", name(kind)),
            Cond::No(kind, p) => {
                let s = format!("{} {} is destroyed.", side(*p), name(kind));
                let mut c = s.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or(s)
            }
        }
    }
}

impl Rules {
    pub fn parse(win: &[String], lose: &[String]) -> Result<Rules, String> {
        let d = Rules::default();
        let list = |v: &[String], default: Vec<Cond>| -> Result<Vec<Cond>, String> {
            if v.is_empty() { Ok(default) } else { v.iter().map(|t| Cond::parse(t)).collect() }
        };
        Ok(Rules { win: list(win, d.win)?, lose: list(lose, d.lose)? })
    }

    pub fn is_default(&self) -> bool {
        *self == Rules::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_read_and_write() {
        for t in ["no_units 2", "no mainbasepad 1", "no tower_missile 2"] {
            assert_eq!(Cond::parse(t).unwrap().text(), t);
        }
        assert!(Cond::parse("no_units").is_err());
        assert!(Cond::parse("no mainbasepad x").is_err());
        assert!(Cond::parse("hold 60").is_err());
        assert_eq!(Cond::parse("own takeover_pad 1").unwrap().text(), "own takeover_pad 1");
        assert_eq!(Cond::parse("own takeover_pad 1 2").unwrap().text(), "own takeover_pad 1 2");
        assert_eq!(Cond::parse("time 300").unwrap(), Cond::Time(300.0));
        assert_eq!(Cond::parse("reach 1 car 104 48 5 4").unwrap().text(), "reach 1 car 104 48 5 4");
        assert_eq!(Cond::parse("lost 1 car 3").unwrap().text(), "lost 1 car 3");
        assert_eq!(Cond::parse("hold 1 55.5 42 4 90").unwrap(), Cond::Hold { player: 1, x: 55.5, y: 42.0, r: 4.0, secs: 90.0 });
        assert_eq!(Cond::parse("hold 1 55.5 42 4 90").unwrap().text(), "hold 1 55.5 42 4 90");
        assert!(Rules::parse(&[], &[]).unwrap().is_default());
    }

    #[test]
    fn holding_out_wins_when_the_time_is_up() {
        let mut level = crate::level::Level::parse(crate::game::LEVELS[0].data).unwrap();
        level.rules = Rules::parse(&["time 3".into()], &[]).unwrap();
        let mut w = World::from_level(&level);
        assert_eq!(w.time_left(), Some(3.0));
        for _ in 0..80 {
            w.step(1.0 / 30.0);
        }
        assert_eq!(w.outcome, crate::world::Outcome::Playing);
        for _ in 0..20 {
            w.step(1.0 / 30.0);
        }
        assert_eq!(w.outcome, crate::world::Outcome::Won);
        assert_eq!(w.verdict, "We held out long enough.");
    }

    #[test]
    fn conditions_hold_on_the_test_level() {
        let level = crate::level::Level::parse(crate::game::LEVELS[0].data).unwrap();
        let mut w = World::from_level(&level);
        let hq2 = Cond::parse("no mainbasepad 2").unwrap();
        let none2 = Cond::parse("no_units 2").unwrap();
        assert!(!hq2.holds(&w) && !none2.holds(&w));
        for e in w.entities.iter_mut().filter(|e| e.team == Team::Enemy && e.kind == "mainbasepad") {
            e.alive = false;
        }
        assert!(hq2.holds(&w) && !none2.holds(&w));
        for e in w.entities.iter_mut().filter(|e| e.team == Team::Enemy) {
            e.alive = false;
        }
        assert!(none2.holds(&w));
    }
}
