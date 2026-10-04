//! Keyboard and gamepad state.
//!
//! Gamepad (feature `gamepad`, on by default, via gilrs): left stick drives
//! (analog throttle and steering), right stick points the turret and clicks
//! for the turret lock, RT main gun, LT machine gun, X rocket, LB/RB zoom
//! out/in, d-pad left/right picks a unit and A builds it, d-pad up rallies
//! nearby units and down holds them, B switches the nearest junction, Y the
//! map, Start help, Back next music track. As in marble-recovery, only pads
//! in SDL's controller database count (gilrs also reports 3D mice and tablet
//! key pads as gamepads); `CALVARY_ANY_PAD=1` accepts everything.
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
#[cfg(feature = "gamepad")]
const STICK_DEADZONE: f32 = 0.18;
/// The right stick only aims when pushed this far.
const AIM_THRESHOLD: f32 = 0.5;

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings || std::env::var("CALVARY_ANY_PAD").is_ok_and(|v| v == "1")
}

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
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    /// Connected pad in use, for the HUD.
    pub pad_name: Option<String>,
    /// Left stick (x right, y up) and right stick, after the dead zone.
    pad_drive: (f32, f32),
    pad_aim: (f32, f32),
    pad_main: bool,
    pad_chain: bool,
    pad_rocket: bool,
    /// The stick drove last (else the keyboard gears do).
    drive_by_pad: bool,
    /// Build menu cursor for the pad (d-pad left/right, A builds).
    pub build_cursor: usize,
    /// D-pad up/down presses for menus (+1 = up); the game ignores them.
    pub menu_moves: i32,
    center_edge: bool,
    /// Mouse: pointer cell, whether it is aiming the turret (any movement in
    /// the tactical view; Q/E or the pad's right stick take over again), the
    /// aim it gives (map angle and distance, set by the main loop), and the
    /// buttons held for firing.
    pub mouse_at: Option<(u16, u16)>,
    pub mouse_aiming: bool,
    pub mouse_aim: Option<(f32, f32)>,
    pub mouse_main: bool,
    pub mouse_chain: bool,
    /// Ctrl+C: leave the program, not just the battle.
    pub exit_app: bool,
    /// Esc: closes the help window if it is open, otherwise quits.
    pub escape: bool,
    pub toggle_help: bool,
    pub toggle_view: bool,
    pub level_step: i32,
    pub zoom_step: i32,
    /// Garage build slot (keys 1-8).
    pub build: Option<usize>,
    pub switch_route: bool,
    pub toggle_lock: bool,
    pub next_track: bool,
    pub toggle_sound: bool,
    /// Unit command keys: U nearby, , . cycle, G rally, X hold, R resume, Z clear.
    pub unit_key: Option<char>,
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
            #[cfg(feature = "gamepad")]
            gilrs: None,
            pad_name: None,
            pad_drive: (0.0, 0.0),
            pad_aim: (0.0, 0.0),
            pad_main: false,
            pad_chain: false,
            pad_rocket: false,
            drive_by_pad: false,
            build_cursor: 0,
            menu_moves: 0,
            exit_app: false,
            center_edge: false,
            mouse_at: None,
            mouse_aiming: false,
            mouse_aim: None,
            mouse_main: false,
            mouse_chain: false,
            escape: false,
            toggle_help: false,
            toggle_view: false,
            level_step: 0,
            zoom_step: 0,
            build: None,
            switch_route: false,
            toggle_lock: false,
            next_track: false,
            toggle_sound: false,
            unit_key: None,
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
            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                self.gear_step += 1;
                self.drive_by_pad = false;
            }
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                self.gear_step -= 1;
                self.drive_by_pad = false;
            }
            KeyCode::Enter => self.missile_edge = true,
            KeyCode::Tab | KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_view = true,
            KeyCode::Char(']') => self.level_step += 1,
            KeyCode::Char('j') | KeyCode::Char('J') => self.switch_route = true,
            KeyCode::Char('t') | KeyCode::Char('T') => self.toggle_lock = true,
            KeyCode::F(9) => self.next_track = true,
            KeyCode::F(8) => self.toggle_sound = true,
            KeyCode::Char('c') | KeyCode::Char('C') if !k.modifiers.contains(KeyModifiers::CONTROL) => self.center_edge = true,
            KeyCode::Char(c @ ('u' | 'U' | ',' | '.' | 'g' | 'G' | 'x' | 'X' | 'r' | 'R' | 'z' | 'Z')) => self.unit_key = Some(c.to_ascii_lowercase()),
            KeyCode::Char(c @ '1'..='9') => self.build = Some(c as usize - '1' as usize),
            KeyCode::Char('0') => self.build = Some(9),
            KeyCode::Char('b') | KeyCode::Char('B') => self.build = Some(10),
            KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_step += 1,
            KeyCode::Char('-') | KeyCode::Char('_') => self.zoom_step -= 1,
            KeyCode::Char('[') => self.level_step -= 1,
            KeyCode::Esc => self.escape = true,
            KeyCode::F(1) | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => self.toggle_help = true,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
                self.exit_app = true;
            }
            _ => {}
        }
    }

    /// Open the gamepad layer (does nothing without the feature or a pad).
    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable_pad(&p))).collect();
                log::info!("gamepads (name, used): {all:?}");
                self.pad_name = all.into_iter().find(|(_, used)| *used).map(|(n, _)| n);
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    /// Drain gamepad events; call once per frame.
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            let mut events = Vec::new();
            while let Some(ev) = g.next_event() {
                events.push(ev);
            }
            for ev in events {
                let g = self.gilrs.as_ref().unwrap();
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                match ev.event {
                    EventType::Connected => {
                        self.pad_name = Some(pad.name().to_string());
                        log::info!("gamepad connected: {:?}", self.pad_name);
                    }
                    EventType::Disconnected => {
                        log::info!("gamepad disconnected");
                        self.pad_name = None;
                    }
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad button {b:?}");
                        let slots = crate::world::BUILDABLE.len();
                        match b {
                            Button::West => self.pad_rocket = true,
                            Button::RightThumb => self.toggle_lock = true,
                            Button::LeftThumb => self.center_edge = true,
                            Button::RightTrigger => self.zoom_step += 1,
                            Button::LeftTrigger => self.zoom_step -= 1,
                            Button::South => self.build = Some(self.build_cursor),
                            Button::East => self.switch_route = true,
                            Button::North => self.toggle_view = true,
                            Button::DPadLeft => self.build_cursor = (self.build_cursor + slots - 1) % slots,
                            Button::DPadRight => self.build_cursor = (self.build_cursor + 1) % slots,
                            Button::DPadUp => {
                                self.menu_moves += 1;
                                self.unit_key = Some('p');
                            }
                            Button::DPadDown => {
                                self.menu_moves -= 1;
                                self.unit_key = Some('x');
                            }
                            Button::Start => self.toggle_help = true,
                            Button::Select => self.next_track = true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
                let axis = |a: Axis| pad.axis_data(a).map_or(0.0, |d| d.value());
                let dead = |(x, y): (f32, f32)| if x.hypot(y) < STICK_DEADZONE { (0.0, 0.0) } else { (x, y) };
                let drive = dead((axis(Axis::LeftStickX), axis(Axis::LeftStickY)));
                if drive != (0.0, 0.0) {
                    self.drive_by_pad = true;
                }
                self.pad_drive = drive;
                self.pad_aim = dead((axis(Axis::RightStickX), axis(Axis::RightStickY)));
                self.pad_main = pad.is_pressed(Button::RightTrigger2);
                self.pad_chain = pad.is_pressed(Button::LeftTrigger2);
            }
        }
    }

    /// Controls for one simulation step; edges are consumed by the first step.
    pub fn controls(&mut self, now: Instant) -> Controls {
        let held = |k: Key| self.holds[k as usize].held(self.exact, now);
        let axis = |neg: Key, pos: Key| held(pos) as i8 as f32 - held(neg) as i8 as f32;
        let (keys_turn, turret) = (axis(Key::Left, Key::Right), axis(Key::TurretLeft, Key::TurretRight));
        if turret != 0.0 || self.pad_aim.0.hypot(self.pad_aim.1) >= AIM_THRESHOLD {
            self.mouse_aiming = false;
            self.mouse_aim = None;
        }
        let (main, chain) = (held(Key::Main), held(Key::Chain));
        // keys win when pressed; otherwise the stick steers
        let turn = if keys_turn != 0.0 { keys_turn } else { self.pad_drive.0 };
        let throttle = self.drive_by_pad.then_some(self.pad_drive.1);
        // stick up is positive, map up is negative y
        let (ax, ay) = self.pad_aim;
        let pad_aim = (ax.hypot(ay) >= AIM_THRESHOLD && turret == 0.0).then(|| (-ay).atan2(ax));
        let aim = pad_aim.or(self.mouse_aim.map(|m| m.0));
        let aim_range = if pad_aim.is_none() { self.mouse_aim.map(|m| m.1) } else { None };
        Controls {
            turn,
            turret,
            gear_step: std::mem::take(&mut self.gear_step),
            throttle,
            aim,
            fire_main: main || self.pad_main || self.mouse_main || std::mem::take(&mut self.main_edge),
            fire_chain: chain || self.pad_chain || self.mouse_chain,
            aim_range,
            fire_missile: std::mem::take(&mut self.missile_edge) || std::mem::take(&mut self.pad_rocket),
            center: std::mem::take(&mut self.center_edge),
        }
    }
}
