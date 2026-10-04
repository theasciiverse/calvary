//! Keyboard state.
//!
//! With the kitty keyboard protocol presses and releases are exact. Elsewhere a key
//! counts as held while press/repeat events keep arriving (the same idea as
//! entity's timing mode, kept simple here because throttle is a sticky gear and
//! only turning, the turret and the guns need holds).

use crate::tank::Controls;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// Before the first OS repeat a tap and a hold look the same; assume held this long.
const PRE_REPEAT: Duration = Duration::from_millis(220);
/// Once repeats flow, a gap longer than this means the key was released.
const REPEAT_GAP: Duration = Duration::from_millis(110);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    Left,
    Right,
    TurretLeft,
    TurretRight,
    Main,
    Chain,
}

const KEYS: usize = 6;

#[derive(Clone, Copy, Default)]
struct Hold {
    down: bool,
    last: Option<Instant>,
    repeats: u32,
}

impl Hold {
    fn held(&self, exact: bool, now: Instant) -> bool {
        if exact {
            return self.down;
        }
        let Some(last) = self.last else { return false };
        let window = if self.repeats == 0 { PRE_REPEAT } else { REPEAT_GAP };
        now.duration_since(last) <= window
    }
}

pub struct Input {
    exact: bool,
    holds: [Hold; KEYS],
    gear_step: i32,
    main_edge: bool,
    missile_edge: bool,
    pub quit: bool,
    /// Esc: closes the help window if it is open, otherwise quits.
    pub escape: bool,
    pub toggle_help: bool,
    pub toggle_view: bool,
    pub level_step: i32,
    pub zoom_step: i32,
}

impl Input {
    pub fn new(exact: bool) -> Input {
        Input {
            exact,
            holds: [Hold::default(); KEYS],
            gear_step: 0,
            main_edge: false,
            missile_edge: false,
            quit: false,
            escape: false,
            toggle_help: false,
            toggle_view: false,
            level_step: 0,
            zoom_step: 0,
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let press = k.kind != KeyEventKind::Release;
        let fresh = k.kind == KeyEventKind::Press;
        let held = match k.code {
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => Some(Key::Left),
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => Some(Key::Right),
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(Key::TurretLeft),
            KeyCode::Char('e') | KeyCode::Char('E') => Some(Key::TurretRight),
            KeyCode::Char(' ') => Some(Key::Main),
            KeyCode::Char('f') | KeyCode::Char('F') => Some(Key::Chain),
            _ => None,
        };
        if let Some(key) = held {
            let h = &mut self.holds[key as usize];
            if self.exact {
                h.down = press;
            } else {
                // without release events every repeat arrives as a press
                if h.held(false, now) {
                    h.repeats += 1;
                } else {
                    h.repeats = 0;
                }
                h.last = Some(now);
            }
            if key == Key::Main && fresh {
                self.main_edge = true;
            }
            return;
        }
        if !fresh {
            return;
        }
        match k.code {
            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => self.gear_step += 1,
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => self.gear_step -= 1,
            KeyCode::Enter => self.missile_edge = true,
            KeyCode::Tab | KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_view = true,
            KeyCode::Char(']') => self.level_step += 1,
            KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_step += 1,
            KeyCode::Char('-') | KeyCode::Char('_') => self.zoom_step -= 1,
            KeyCode::Char('[') => self.level_step -= 1,
            KeyCode::Esc => self.escape = true,
            KeyCode::F(1) | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => self.toggle_help = true,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            _ => {}
        }
    }

    /// Controls for one simulation step; edges are consumed by the first step.
    pub fn controls(&mut self, now: Instant) -> Controls {
        let held = |k: Key| self.holds[k as usize].held(self.exact, now);
        let axis = |neg: Key, pos: Key| held(pos) as i8 - held(neg) as i8;
        let (turn, turret) = (axis(Key::Left, Key::Right), axis(Key::TurretLeft, Key::TurretRight));
        let (main, chain) = (held(Key::Main), held(Key::Chain));
        Controls {
            turn,
            turret,
            gear_step: std::mem::take(&mut self.gear_step),
            fire_main: main || std::mem::take(&mut self.main_edge),
            fire_chain: chain,
            fire_missile: std::mem::take(&mut self.missile_edge),
        }
    }
}
