//! The original balance sheet, `scheme/balance.scm` from golgotha-forever,
//! embedded verbatim and read at startup so every number stays the original's.
//!
//! Units: speeds and accelerations are per tick and fire delays are in ticks at
//! `ORIGINAL_HZ`; ranges and radii are in map cells.

use std::collections::HashMap;
use std::sync::OnceLock;

pub const ORIGINAL_HZ: f32 = 10.0;

/// `accel` is read for completeness; units reach speed at once for now.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct UnitStats {
    pub speed: f32,
    pub turn_speed: f32,
    pub accel: f32,
    pub cost: i32,
    pub health: i32,
    pub fire_delay: f32,
    pub detection_range: f32,
    pub weapon: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WeaponStats {
    /// Area of effect (`mult`) rather than one target (`sing`).
    pub area: bool,
    pub damage: f32,
    pub radius: f32,
    pub speed: f32,
    pub range: f32,
    /// Per-target overrides, e.g. b120mm does 1000 to a peon_tank.
    pub special: Vec<(String, f32)>,
}

pub struct Balance {
    pub units: HashMap<String, UnitStats>,
    pub weapons: HashMap<String, WeaponStats>,
}

#[derive(Clone, Debug)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

impl Sexp {
    fn atom(&self) -> Option<&str> {
        match self {
            Sexp::Atom(a) => Some(a),
            Sexp::List(_) => None,
        }
    }

    fn num(&self) -> Option<f32> {
        self.atom()?.parse().ok()
    }
}

/// Enough Lisp reading for balance.scm: `;` comments, `/* */` blocks, quotes.
fn parse(src: &str) -> Vec<Sexp> {
    let mut clean = String::new();
    let mut rest = src;
    while let Some(i) = rest.find("/*") {
        clean.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    clean.push_str(rest);
    let mut tokens = Vec::new();
    for line in clean.lines() {
        let line = line.split(';').next().unwrap_or("");
        let spaced = line.replace('(', " ( ").replace(')', " ) ").replace('\'', " ");
        tokens.extend(spaced.split_whitespace().map(str::to_string));
    }
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    for t in tokens {
        match t.as_str() {
            "(" => stack.push(Vec::new()),
            ")" => {
                if stack.len() > 1 {
                    let list = stack.pop().unwrap();
                    stack.last_mut().unwrap().push(Sexp::List(list));
                }
            }
            _ => stack.last_mut().unwrap().push(Sexp::Atom(t)),
        }
    }
    stack.swap_remove(0)
}

pub fn balance() -> &'static Balance {
    static B: OnceLock<Balance> = OnceLock::new();
    B.get_or_init(|| {
        let mut b = Balance { units: HashMap::new(), weapons: HashMap::new() };
        for form in parse(include_str!("../data/balance.scm")) {
            let Sexp::List(items) = form else { continue };
            let Some(head) = items.first().and_then(Sexp::atom) else { continue };
            for row in &items[1..] {
                let Sexp::List(f) = row else { continue };
                let Some(name) = f.first().and_then(Sexp::atom) else { continue };
                match head {
                    "defaults" if f.len() >= 9 => {
                        let n = |i: usize| f[i].num().unwrap_or(0.0);
                        let weapon = f[8].atom().filter(|w| *w != "nil").map(str::to_string);
                        b.units.insert(
                            name.to_string(),
                            UnitStats {
                                speed: n(1),
                                turn_speed: n(2),
                                accel: n(3),
                                cost: n(4) as i32,
                                health: n(5) as i32,
                                fire_delay: n(6),
                                detection_range: n(7),
                                weapon,
                            },
                        );
                    }
                    "def_weapon_damage" if f.len() >= 6 => {
                        let n = |i: usize| f[i].num().unwrap_or(0.0);
                        let special = f
                            .get(6)
                            .map(|s| match s {
                                Sexp::List(p) => p.chunks(2).filter_map(|c| Some((c.first()?.atom()?.to_string(), c.get(1)?.num()?))).collect(),
                                Sexp::Atom(_) => Vec::new(),
                            })
                            .unwrap_or_default();
                        b.weapons.insert(
                            name.to_string(),
                            WeaponStats { area: f[1].atom() == Some("mult"), damage: n(2), radius: n(3), speed: n(4), range: n(5), special },
                        );
                    }
                    _ => {}
                }
            }
        }
        b
    })
}

pub fn unit(name: &str) -> Option<&'static UnitStats> {
    balance().units.get(name)
}

pub fn weapon(name: &str) -> Option<&'static WeaponStats> {
    balance().weapons.get(name)
}

impl WeaponStats {
    pub fn damage_to(&self, target: &str) -> f32 {
        self.special.iter().find(|(n, _)| n == target).map_or(self.damage, |&(_, d)| d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_original_numbers() {
        let peon = unit("peon_tank").unwrap();
        assert_eq!((peon.cost, peon.health, peon.weapon.as_deref()), (300, 50, Some("b90mm")));
        assert_eq!(unit("stank").unwrap().speed, 0.3);
        // second defaults block: structures
        assert_eq!(unit("tower_electric").unwrap().weapon.as_deref(), Some("bolt"));
        let b120 = weapon("b120mm").unwrap();
        assert_eq!((b120.area, b120.damage, b120.damage_to("peon_tank")), (false, 40.0, 1000.0));
        assert!(weapon("super_mortar").unwrap().area);
        assert_eq!(weapon("guided_missile").unwrap().radius, 1.5);
    }
}
