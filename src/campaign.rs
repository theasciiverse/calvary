//! The campaign (CAMPAIGN.md): missions in order, each a `.cmap` built into
//! the binary with its own win and lose rules, a briefing over the globe
//! turned to its place, and the original's own result screens after it
//! (`youwin.jpg` and `youlose.jpg` from the 1998 release). The furthest
//! mission reached is saved, so the title can continue from it.

use crate::audio::Audio;
use crate::input::Input;
use crate::title;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::io;
use std::time::{Duration, Instant};

const AMBER: Color = Color::Rgb(255, 180, 40);
const GREEN: Color = Color::Rgb(80, 220, 120);
const DIM: Color = Color::Rgb(70, 110, 80);
const BONE: Color = Color::Rgb(226, 220, 204);

pub struct Act {
    pub name: &'static str,
    /// Kruese's press conference, the "Golgotha Transmission", opens each act.
    pub transmission: &'static [&'static str],
}

pub struct Mission {
    /// Index into `ACTS`.
    pub act: usize,
    pub number: u32,
    pub title: &'static str,
    /// The map, built in (`maps/*.cmap`).
    pub map: &'static str,
    /// Sergeant Reid Charles on the radio.
    pub briefing: &'static [&'static str],
    pub objective: &'static str,
}

pub const ACTS: &[Act] = &[Act {
    name: "Act I: The Incursion",
    transmission: &["We went in because they told us an American was dead.", "She was. They lied about who killed her."],
}];

pub const MISSIONS: &[Mission] = &[
    Mission {
        act: 0,
        number: 1,
        title: "Al Basrah",
        map: include_str!("../maps/al-basrah.cmap"),
        briefing: &[
            "Commander, Reid here. We are ashore on the Shatt al-Arab, south of the",
            "old port, and the HQ is down. Command says the people who killed",
            "Dr. Calloway came through Basrah, and the road north is held: towers",
            "along the waterfront, armour in the date groves, more of it coming.",
            "No garage until the engineers land, sir. It is you and the supertank.",
            "When the guns run dry, come home: the HQ rearms you and patches you up.",
        ],
        objective: "Destroy every enemy tank and tower. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 2,
        title: "Shatt al-Arab",
        map: include_str!("../maps/shatt-al-arab.cmap"),
        briefing: &[
            "Commander, Reid. The engineers are ashore and we have a garage at last.",
            "North of here the marsh is cut by three channels, and every bridge on",
            "both causeways is blown. Peon tanks will hold the line, but nothing",
            "crosses water without a bridger: send one up a causeway and it lays",
            "itself over the first gap it finds. Their base sits on the levee to",
            "the north-east. Shell the far banks before you send the bridgers in.",
        ],
        objective: "Bridge the channels and destroy the enemy HQ. Keep ours standing.",
    },
    Mission {
        act: 0,
        number: 3,
        title: "Ur",
        map: include_str!("../maps/ur.cmap"),
        briefing: &[
            "Commander, this is Ur. Dr. Calloway's first dig is at the foot of the",
            "ziggurat, and Command wants it in our hands before anyone else goes",
            "through her trenches. Hold the dig for ninety seconds with nothing of",
            "theirs on it. There are banks along both roads: an engineer takes one,",
            "and our money planes fly its crates home. Their engineers know that too.",
            "Guns on the ziggurat's first terrace cover the dig. Mind them, sir.",
        ],
        objective: "Hold Calloway's dig for 90 seconds. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 4,
        title: "Nakhayb",
        map: include_str!("../maps/nakhayb.cmap"),
        briefing: &[
            "Commander, Reid. Nakhayb, out in the western desert. This was her camp:",
            "Calloway worked here alone for two seasons, and she left in a hurry.",
            "It sits where the two wadis meet. Get an engineer onto the pad in the",
            "middle and the camp is ours, with a turret to keep it. Theirs will try",
            "the same: if they get there first, knock their turret down and go again.",
            "The mesas are sheer, sir. Stay in the wadis.",
        ],
        objective: "Take Calloway's camp: an engineer on the takeover pad. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 5,
        title: "Babylon",
        map: include_str!("../maps/babylon.cmap"),
        briefing: &[
            "Commander. Her notebook names a second site, east of the old city of",
            "Babylon, and they got there first. Sir, I have seen it from the ridge.",
            "It is bones. A skull the size of a truck, ribs taller than our tanks,",
            "lying in a pit in the open. They have ringed it with electric towers.",
            "Bring those towers down, or send engineers to take them. Our garage",
            "can build electric cars now: short reach, but they burn through armour.",
        ],
        objective: "Destroy or take every electric tower round the pit. Keep the HQ standing.",
    },
    Mission {
        act: 0,
        number: 6,
        title: "The Euphrates line",
        map: include_str!("../maps/euphrates.cmap"),
        briefing: &[
            "Commander, Reid. Six trucks for the forward post on the Euphrates,",
            "and the bones from Babylon are in two of them. Command has changed",
            "the route twice since dusk, so choose for yourself, sir: J switches",
            "the junction nearest you, and the crossovers let you change your mind.",
            "Both roads are watched. Select the trucks and X holds them while you",
            "clear the way; R sends them on. Four must get through. Lose three and",
            "it is over.",
        ],
        objective: "Bring four of the six supply trucks to the forward post. Keep the HQ standing.",
    },
];

/// The mission's map, parsed.
pub fn map(m: &Mission) -> Result<(crate::level::Level, String, Option<String>), String> {
    crate::cmap::parse(m.map).map_err(|e| format!("mission {} ({}): {e}", m.number, m.title))
}

/// Where the mission is, for the globe.
pub fn place(m: &Mission) -> (&'static str, f32, f32) {
    title::place_of(m.title).unwrap_or(title::GOLGOTHA)
}

fn progress_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "calvary").map(|d| d.data_dir().join("campaign.txt"))
}

/// How many missions are open to play: the first, plus every one after a
/// mission won.
pub fn reached() -> usize {
    let n = progress_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("reached = ").and_then(|v| v.trim().parse::<usize>().ok())))
        .unwrap_or(1);
    n.clamp(1, MISSIONS.len())
}

/// Open the mission after `index` (a no-op when it already is).
pub fn unlock_after(index: usize) {
    let n = (index + 2).min(MISSIONS.len());
    if n <= reached() {
        return;
    }
    let Some(path) = progress_path() else { return };
    let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, format!("reached = {n}\n")));
    match saved {
        Ok(()) => log::info!("campaign: {n} missions open ({})", path.display()),
        Err(e) => log::warn!("campaign progress not saved to {}: {e}", path.display()),
    }
}

/// What the player chose on a campaign screen.
#[derive(PartialEq, Debug)]
pub enum Pick {
    Go,
    Back,
}

/// One key, click or pad button: `Some(true)` goes on, `Some(false)` backs out.
fn read_choice(input: &mut Input, wait: Duration) -> io::Result<Option<bool>> {
    input.poll_pad();
    input.unit_key = None;
    input.zoom_step = 0;
    input.menu_moves = 0;
    if input.build.take().is_some() || std::mem::take(&mut input.toggle_help) {
        return Ok(Some(true));
    }
    if std::mem::take(&mut input.switch_route) {
        return Ok(Some(false));
    }
    if !event::poll(wait)? {
        return Ok(None);
    }
    Ok(match event::read()? {
        Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                input.exit_app = true;
                Some(false)
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => Some(false),
            _ => Some(true),
        },
        Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => Some(true),
        _ => None,
    })
}

/// Kruese's words in quotation marks, opening on the first line.
fn quoted(lines: &[&str]) -> Vec<String> {
    let n = lines.len();
    lines.iter().enumerate().map(|(i, l)| format!("{}{l}{}", if i == 0 { "\u{201c}" } else { "" }, if i + 1 == n { "\u{201d}" } else { "" })).collect()
}

/// Lines typed out left-aligned in a block centred on the screen, so a line
/// being typed stays put; each line on its own black backing.
fn block(f: &mut Frame, area: Rect, y: u16, full: &[String], typed: &[String], style: Style) -> u16 {
    let w = full.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    for (i, l) in typed.iter().enumerate() {
        let row = y + i as u16;
        if row < area.y + area.height && !l.is_empty() {
            let r = Rect { x, y: row, width: (l.chars().count() as u16).min(area.width), height: 1 };
            f.render_widget(Paragraph::new(Span::styled(l.clone(), style.bg(Color::Black))), r);
        }
    }
    full.len() as u16
}

/// Text over the globe, each line on its own black backing.
fn centred(f: &mut Frame, area: Rect, y: u16, spans: Vec<Span>) {
    if y >= area.y + area.height {
        return;
    }
    let spans: Vec<Span> = spans.into_iter().map(|sp| {
            let style = sp.style;
            if style.bg.is_none() { sp.style(style.bg(Color::Black)) } else { sp }
        }).collect();
    f.render_widget(Paragraph::new(Line::from(spans)).centered(), Rect { y, height: 1, ..area });
}

/// The briefing: the globe turns to the mission's place while the act's
/// transmission (on its first mission) and Reid's radio call type out.
pub fn briefing(terminal: &mut DefaultTerminal, audio: &mut Audio, input: &mut Input, index: usize) -> io::Result<Pick> {
    let m = &MISSIONS[index];
    let act = &ACTS[m.act];
    let opens_act = MISSIONS.iter().position(|x| x.act == m.act) == Some(index);
    if let Ok((_, _, Some(music))) = map(m) {
        audio.play_music(&music);
    }
    let (place, lat, lon) = place(m);
    // the globe swings in from Golgotha
    let mut view = (title::GOLGOTHA.2.to_radians(), (title::GOLGOTHA.1 * 0.8).to_radians());
    let start = Instant::now();
    let mut last = Instant::now();
    let mut typed_all = false;
    loop {
        let t = start.elapsed().as_secs_f32();
        let k = (last.elapsed().as_secs_f32() * 1.5).min(1.0);
        last = Instant::now();
        let d = (lon.to_radians() - view.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        view.0 += d * k;
        view.1 += ((lat * 0.8).to_radians() - view.1) * k;
        // characters typed so far: the transmission, then the radio call
        let typed = if typed_all { usize::MAX } else { (t * 55.0) as usize };
        let total: usize = quoted(act.transmission).iter().map(|l| l.chars().count()).sum::<usize>() + m.briefing.iter().map(|l| l.chars().count()).sum::<usize>();
        terminal.draw(|f| draw_briefing(f, m, act, opens_act, (place, lat, lon), view, t, typed))?;
        match read_choice(input, Duration::from_millis(33))? {
            Some(true) if !typed_all && typed < total => typed_all = true,
            Some(true) => return Ok(Pick::Go),
            Some(false) => return Ok(Pick::Back),
            None => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_briefing(f: &mut Frame, m: &Mission, act: &Act, opens_act: bool, place: (&str, f32, f32), view: (f32, f32), t: f32, typed: usize) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
    title::draw_globe(f.buffer_mut(), area, area.height, t, 0.4, view, Some((place.1, place.2)));
    let mut left = typed;
    let mut type_out = |text: &str| -> String {
        let n = text.chars().count().min(left);
        left -= n;
        text.chars().take(n).collect()
    };
    let words = if opens_act { quoted(act.transmission) } else { Vec::new() };
    let transmission: Vec<String> = words.iter().map(|l| type_out(l)).collect();
    let call: Vec<String> = m.briefing.iter().map(|l| l.to_string()).collect();
    let radio: Vec<String> = call.iter().map(|l| type_out(l)).collect();
    let done = left > 0;
    // laid out from a top that leaves the block centred low on the screen
    let rows = 3 + if opens_act { act.transmission.len() as u16 + 3 } else { 0 } + 2 + radio.len() as u16 + 4;
    let mut y = area.y + area.height.saturating_sub(rows + 1).max(1);
    let bold = |c: Color| Style::default().fg(c).add_modifier(Modifier::BOLD);
    centred(f, area, y, vec![Span::styled(act.name.to_uppercase(), bold(AMBER))]);
    y += 2;
    if opens_act {
        y += block(f, area, y, &words, &transmission, Style::default().fg(BONE).add_modifier(Modifier::ITALIC));
        centred(f, area, y, vec![Span::styled("Lt. Elaine Kruese, the Golgotha Transmission", Style::default().fg(DIM))]);
        y += 2;
    }
    let ns = if place.1 >= 0.0 { 'N' } else { 'S' };
    let ew = if place.2 >= 0.0 { 'E' } else { 'W' };
    centred(
        f,
        area,
        y,
        vec![
            Span::styled(format!("MISSION {}: {}", m.number, m.title.to_uppercase()), bold(GREEN)),
            Span::styled(format!("   {:.1}°{ns} {:.1}°{ew}", place.1.abs(), place.2.abs()), Style::default().fg(AMBER)),
        ],
    );
    y += 2;
    y += block(f, area, y, &call, &radio, Style::default().fg(GREEN));
    centred(f, area, y, vec![Span::styled("Sgt. Reid Charles, on the radio", Style::default().fg(DIM))]);
    y += 2;
    if done {
        centred(f, area, y, vec![Span::styled("Objective: ", bold(AMBER)), Span::styled(m.objective, Style::default().fg(BONE))]);
        centred(f, area, y + 2, vec![Span::styled("Enter: move out    F1 in battle: controls    Esc: back to the title", Style::default().fg(DIM))]);
    }
}

static YOU_WIN: &[u8] = include_bytes!("../data/youwin.ppm");
static YOU_LOSE: &[u8] = include_bytes!("../data/youlose.ppm");

/// After a mission: the original's own screen ("MISSION COMPLETED!" or
/// "YOULOSE"), why, and what next. Going on means the next mission after a
/// win, another try after a loss.
pub fn result(terminal: &mut DefaultTerminal, input: &mut Input, index: usize, won: bool, verdict: &str) -> io::Result<Pick> {
    static WIN: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    static LOSE: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    let img = if won { WIN.get_or_init(|| title::parse_ppm(YOU_WIN)) } else { LOSE.get_or_init(|| title::parse_ppm(YOU_LOSE)) };
    let last = index + 1 == MISSIONS.len();
    let next = match (won, last) {
        (true, false) => format!("Enter: on to mission {}, {}", MISSIONS[index + 1].number, MISSIONS[index + 1].title),
        (true, true) => "The road north is not built yet. Enter or Esc: back to the title".to_string(),
        (false, _) => format!("Enter: try {} again    Esc: back to the title", MISSIONS[index].title),
    };
    let start = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
            let pic = title::draw_picture(f.buffer_mut(), area, img, (t / 0.8).min(1.0), (0.8, 0.72, 0.04));
            let y = pic.y + pic.height + 1;
            let color = if won { AMBER } else { Color::Rgb(235, 70, 55) };
            centred(f, area, y, vec![Span::styled(verdict.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD))]);
            if t > 1.0 {
                centred(f, area, y + 2, vec![Span::styled(next.clone(), Style::default().fg(DIM))]);
            }
        })?;
        // a second's grace, so a key still held from the battle does not skip it
        match read_choice(input, Duration::from_millis(33))? {
            Some(go) if t > 1.0 => return Ok(if go && !(won && last) { Pick::Go } else { Pick::Back }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Class, Outcome, Team, World};

    /// Every mission's map loads, has our HQ and supertank start, and its
    /// rules all apply (none already true before the battle).
    #[test]
    fn missions_load() {
        for m in MISSIONS {
            let (level, name, music) = map(m).unwrap();
            assert!(music.is_some(), "{name}: no music");
            assert_ne!(place(m).0, title::GOLGOTHA.0, "{name}: not on the globe");
            let w = World::from_level(&level);
            assert!(w.find(Class::Hq, Team::Player).is_some(), "{name}: no HQ");
            assert!(w.player_tank.is_some(), "{name}: no supertank start");
            for c in level.rules.win.iter().chain(&level.rules.lose) {
                assert!(!c.holds(&w), "{name}: {} holds before the battle", c.text());
            }
        }
    }

    /// A won mission opens the next and the save remembers it; nothing goes
    /// past the last mission or backwards.
    #[test]
    fn progress_is_saved() {
        let dir = std::env::temp_dir().join(format!("calvary-progress-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &dir);
        assert_eq!(reached(), 1);
        unlock_after(0);
        assert_eq!(reached(), 2.min(MISSIONS.len()));
        unlock_after(MISSIONS.len() + 5);
        assert_eq!(reached(), MISSIONS.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Al Basrah is won by clearing the field and lost with the HQ.
    #[test]
    fn al_basrah_rules() {
        let (level, _, _) = map(&MISSIONS[0]).unwrap();
        let mut w = World::from_level(&level);
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Playing);
        for e in w.entities.iter_mut().filter(|e| e.team == Team::Enemy) {
            e.alive = false;
        }
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Won);

        let mut w = World::from_level(&level);
        let hq = w.find(Class::Hq, Team::Player).unwrap();
        w.entities[hq].alive = false;
        w.step(1.0 / 30.0);
        assert_eq!(w.outcome, Outcome::Lost);
        assert!(w.verdict.contains("base"), "{}", w.verdict);
    }

    /// Shatt al-Arab: six gaps, only peons and bridgers, and a bridger sent
    /// up the first causeway spans its first channel.
    #[test]
    fn shatt_al_arab_needs_bridgers() {
        let (level, _, _) = map(&MISSIONS[1]).unwrap();
        let mut w = World::from_level(&level);
        assert_eq!((0..w.nodes.len()).filter(|&n| w.gap(n)).count(), 6);
        assert!(w.offers(Team::Player, "bridger") && w.offers(Team::Player, "peon_tank") && !w.offers(Team::Player, "rocket_tank"));
        assert!(w.order(Team::Player, "bridger"));
        for _ in 0..30 * 120 {
            w.step(1.0 / 30.0);
        }
        assert_eq!((0..w.nodes.len()).filter(|&n| w.gap(n)).count(), 5, "one channel bridged");
        assert_eq!(w.outcome, Outcome::Playing);
    }

    /// Balance probe for a mission: `MISSION=3 cargo test --release
    /// mission_report -- --ignored --nocapture` (default mission 2). A
    /// scripted player orders a bridger a minute where the map offers them,
    /// and every 20 s the next of the map's other units; with `SUPERTANK=1`
    /// the enemy towers and armour are cleared every minute from 2:00 and
    /// the supertank parks on the map's site to hold, and supply trucks wait
    /// at home until 2:00, standing in for the supertank's work. `RUST_LOG=info` shows the battle's log.
    #[test]
    #[ignore]
    fn mission_report() {
        let _ = env_logger::builder().is_test(true).try_init();
        let n: usize = std::env::var("MISSION").ok().and_then(|m| m.parse().ok()).unwrap_or(2);
        let (level, _, _) = map(&MISSIONS[n - 1]).unwrap();
        let mut w = World::from_level(&level);
        let supertank = std::env::var("SUPERTANK").is_ok();
        let units: Vec<&'static str> = crate::world::BUILDABLE.iter().copied().filter(|k| *k != "bridger" && w.offers(Team::Player, k)).collect();
        for s in 0..900 {
            if s % 60 == 5 {
                w.order(Team::Player, "bridger");
            }
            if s % 20 == 0 && !units.is_empty() {
                w.order(Team::Player, units[(s / 20) % units.len()]);
            }
            // the supply trucks wait at home until the road is cleared
            let trucks: Vec<usize> = (0..w.entities.len()).filter(|&i| w.entities[i].kind == "car" && w.entities[i].team == Team::Player).collect();
            if supertank && s == 0 {
                w.command(&trucks, crate::world::Command::Hold);
            }
            if supertank && s == 121 {
                w.command(&trucks, crate::world::Command::Resume);
            }
            if supertank && s >= 120 && s % 60 == 0 {
                for e in w.entities.iter_mut().filter(|e| e.team == Team::Enemy && matches!(e.class, Class::Tower | Class::Vehicle)) {
                    e.alive = false;
                }
                if let (Some(t), Some(site)) = (w.player_tank, w.sites().first().copied()) {
                    (w.entities[t].x, w.entities[t].y) = (site.1, site.2);
                }
            }
            for _ in 0..30 {
                w.step(1.0 / 30.0);
            }
            if s % 60 == 59 {
                let units = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Vehicle && e.team == t).count();
                let banks = |t: Team| w.entities.iter().filter(|e| e.alive && e.class == Class::Bank && e.team == t).count();
                let gaps = (0..w.nodes.len()).filter(|&n| w.gap(n)).count();
                let towers = w.entities.iter().filter(|e| e.alive && e.class == Class::Tower).count();
                let held: Vec<String> = w.sites().iter().map(|s| format!("{:.0}/{:.0}", s.5, s.4)).collect();
                println!(
                    "{:>3}s ours {:>2} theirs {:>2} banks {}/{} gaps {gaps} towers {towers} held {held:?} money {:?} {:?}",
                    s + 1,
                    units(Team::Player),
                    units(Team::Enemy),
                    banks(Team::Player),
                    banks(Team::Enemy),
                    w.money,
                    w.outcome
                );
            }
        }
    }
}
