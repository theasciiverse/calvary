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
//! ```
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
            _ => Err(format!("condition {text:?}: want `no_units PLAYER` or `no KIND PLAYER`")),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Cond::NoUnits(p) => format!("no_units {p}"),
            Cond::No(kind, p) => format!("no {kind} {p}"),
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
        }
    }

    /// What the condition means, for the player.
    pub fn describe(&self) -> String {
        let side = |p: u8| if p == 1 { "our" } else { "the enemy's" };
        let name = |kind: &str| match kind {
            "mainbasepad" => "base".to_string(),
            k => k.replace('_', " "),
        };
        match self {
            Cond::NoUnits(1) => "Nothing of ours is left on the field.".into(),
            Cond::NoUnits(_) => "Every enemy unit and tower is destroyed.".into(),
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
        assert!(Rules::parse(&[], &[]).unwrap().is_default());
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
