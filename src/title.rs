//! Opening and title screen.
//!
//! The opening follows the original's own: the Shakespeare epigraph from the
//! crack.com Golgotha pages (Richard II, IV.i), then the spinning globe
//! against a sea of stars that their Act 1 Scene 1 fades into, with the
//! Golgotha site marked. Then the title, the menu, and the credits of the
//! people who made Golgotha, as Crack dot Com published them.

use crate::audio::Audio;
use crate::campaign;
use crate::game::LEVELS;
use crate::input::Input;
use crate::render;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
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

/// Scene timing, seconds from start.
const EPIGRAPH_END: f32 = 10.0;
const GLOBE_END: f32 = 19.0;

const EPIGRAPH: [&str; 6] = [
    "Peace shall go sleep with Turks and infidels,",
    "And in this seat of peace tumultuous wars",
    "Shall kin with kin and kind with kind confound;",
    "Disorder, horror, fear, and mutiny",
    "Shall here inhabit, and this land be call'd",
    "The field of Golgotha and dead men's skulls.",
];


/// Where things are, for the globe: the bundled levels' scenery and the
/// campaign's cities (CAMPAIGN.md), by the names maps are likely to carry.
const PLACES: &[(&str, &str, f32, f32)] = &[
    ("test", "Piraeus, Greece", 37.94, 23.65),
    ("piraeus", "Piraeus, Greece", 37.94, 23.65),
    ("snow", "Zurich, Switzerland", 47.37, 8.54),
    ("zurich", "Zurich, Switzerland", 47.37, 8.54),
    ("egypt2", "Cairo, Egypt", 30.04, 31.24),
    ("cairo", "Cairo, Egypt", 30.04, 31.24),
    ("basrah", "Al Basrah, Iraq", 30.51, 47.81),
    // Qurna, where the Tigris and Euphrates meet to form the Shatt al-Arab
    ("shatt", "Shatt al-Arab, Iraq", 31.00, 47.43),
    ("ur", "Ur, Iraq", 30.96, 46.10),
    // Nasiriyah, on the Euphrates
    ("euphrates", "The Euphrates, Iraq", 31.04, 46.26),
    ("nakhayb", "Nakhayb, Iraq", 32.04, 42.27),
    ("babylon", "Babylon, Iraq", 32.54, 44.42),
    ("nineveh", "Nineveh, Iraq", 36.36, 43.15),
    ("palmyra", "Palmyra, Syria", 34.55, 38.27),
    // Kerak, on the King's Highway
    ("highway", "The King's Highway, Jordan", 31.18, 35.70),
    ("sinai", "Sinai", 29.50, 33.80),
    ("suez", "The Suez Canal", 30.58, 32.27),
    ("giza", "Giza, Egypt", 29.98, 31.13),
    ("alexandria", "Alexandria, Egypt", 31.20, 29.92),
    ("delphi", "Delphi, Greece", 38.48, 22.50),
    ("ionian", "The Ionian Islands", 38.45, 20.60),
    ("naples", "Naples, Italy", 40.85, 14.27),
    ("pompeii", "Pompeii, Italy", 40.75, 14.49),
    ("rome", "Rome, Italy", 41.90, 12.50),
    ("turin", "Turin, Italy", 45.07, 7.69),
    ("gotthard", "The Gotthard Pass", 46.56, 8.57),
    ("chartres", "Chartres, France", 48.45, 1.49),
    ("loire", "The Loire, France", 47.40, 1.00),
    ("munich", "Munich, Germany", 48.14, 11.58),
    ("vienna", "Vienna, Austria", 48.21, 16.37),
    ("berlin", "Berlin, Germany", 52.52, 13.40),
    ("helsinki", "Helsinki, Finland", 60.17, 24.94),
    ("lapland", "Lapland, Finland", 67.90, 26.00),
    ("svalbard", "Svalbard, Norway", 78.22, 15.65),
    ("norway", "Norway", 62.00, 7.00),
    ("cyprus", "Cyprus", 35.13, 33.43),
    ("jaffa", "Jaffa", 32.05, 34.75),
    ("jerusalem", "Jerusalem", 31.78, 35.23),
];

/// Golgotha itself, where the menu turns when nothing else is picked.
pub(crate) const GOLGOTHA: (&str, f32, f32) = ("Golgotha, Jerusalem", 31.78, 35.23);

pub(crate) fn place_of(name: &str) -> Option<(&'static str, f32, f32)> {
    let n = name.to_lowercase();
    PLACES.iter().find(|p| n == p.0 || n.contains(p.0) && p.0.len() > 3).map(|p| (p.1, p.2, p.3))
}

pub enum Choice {
    /// The campaign, from this mission (an index into `campaign::MISSIONS`).
    Campaign(usize),
    Level(usize),
    Map(String),
    Quit,
}

enum Item {
    Campaign,
    Level(usize, &'static str),
    Map(String),
    Controls,
    Credits,
    Quit,
}

impl Item {
    /// The place on the globe this entry stands for.
    fn place(&self, mission: usize) -> (&'static str, f32, f32) {
        let found = match self {
            Item::Campaign => Some(campaign::place(&campaign::MISSIONS[mission])),
            Item::Level(i, _) => place_of(LEVELS[*i].name),
            Item::Map(p) => place_of(p.trim_start_matches("maps/").trim_end_matches(".cmap")),
            _ => None,
        };
        found.unwrap_or(GOLGOTHA)
    }

    fn label(&self, mission: usize) -> String {
        match self {
            Item::Campaign => {
                let m = &campaign::MISSIONS[mission];
                // left and right pick among the missions reached
                let (l, r) = (if mission > 0 { "◂ " } else { "" }, if mission + 1 < campaign::reached() { " ▸" } else { "" });
                format!("{l}Campaign: mission {}, {}{r}", m.number, m.title)
            }
            Item::Level(_, l) => l.to_string(),
            Item::Map(p) => format!("Map: {}", p.trim_start_matches("maps/").trim_end_matches(".cmap")),
            Item::Controls => "Controls".into(),
            Item::Credits => "Credits".into(),
            Item::Quit => "Quit".into(),
        }
    }
}

fn menu() -> Vec<Item> {
    let names = |n: &str| match n {
        "test" => "Battle: the Greek coast",
        "snow" => "Skirmish: the Swiss valley",
        "egypt2" => "Sandbox: the walled town (Egypt)",
        _ => "Level",
    };
    let mut items = vec![Item::Campaign];
    items.extend(LEVELS.iter().enumerate().map(|(i, l)| Item::Level(i, names(l.name))));
    if let Ok(dir) = std::fs::read_dir("maps") {
        let mut maps: Vec<String> = dir.flatten().map(|e| e.path().display().to_string()).filter(|p| p.ends_with(".cmap")).collect();
        maps.sort();
        items.extend(maps.into_iter().map(Item::Map));
    }
    items.extend([Item::Controls, Item::Credits, Item::Quit]);
    items
}

#[derive(PartialEq)]
enum Page {
    Main,
    Controls,
    Credits,
}

/// Run the opening (unless `skip_intro`) and the title menu.
pub fn run(terminal: &mut DefaultTerminal, audio: &mut Audio, input: &mut Input, skip_intro: bool) -> io::Result<Choice> {
    audio.play_music("Splash_Screen_Opus.mp3");
    let skipped = || Instant::now() - Duration::from_secs_f32(GLOBE_END);
    let mut start = if skip_intro { skipped() } else { Instant::now() };
    let items = menu();
    let mut pick = 0usize;
    // the campaign entry offers the furthest mission reached
    let mut mission = campaign::reached() - 1;
    let mut page = Page::Main;
    // globe orientation (spin = longitude facing us, tilt = latitude), eased
    // toward the selected entry's place once the title is up
    let mut view = (0.0f32, 0.0f32);
    let mut credits_at = Instant::now();
    let mut last = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        let dt = last.elapsed().as_secs_f32().min(0.1);
        last = Instant::now();
        if t < GLOBE_END {
            view = (((t - EPIGRAPH_END).max(0.0) * 0.3 - 1.2), 0.25);
        } else {
            let (_, lat, lon) = items[pick].place(mission);
            let (want_spin, want_tilt) = (lon.to_radians(), (lat * 0.8).to_radians());
            let d = (want_spin - view.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let k = (dt * 2.5).min(1.0);
            view.0 += d * k;
            view.1 += (want_tilt - view.1) * k;
        }
        let mut rows: Vec<Rect> = Vec::new();
        terminal.draw(|f| {
            rows = draw(f, t, &items, pick, mission, &page, view, credits_at.elapsed().as_secs_f32());
        })?;
        input.poll_pad();
        // the pad: d-pad up/down move, A or Start picks, B backs out
        let mut moves = std::mem::take(&mut input.menu_moves);
        input.unit_key = None;
        input.zoom_step = 0;
        let mut confirm = input.build.take().is_some() || std::mem::take(&mut input.toggle_help);
        let mut back = std::mem::take(&mut input.switch_route);
        if !event::poll(Duration::from_millis(33))? {
            if moves == 0 && !confirm && !back {
                continue;
            }
        } else {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if t < GLOBE_END {
                        // any key skips the opening
                        start = skipped();
                        continue;
                    }
                    match k.code {
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') => moves += 1,
                        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => moves -= 1,
                        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') if matches!(items[pick], Item::Campaign) => mission = mission.saturating_sub(1),
                        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') if matches!(items[pick], Item::Campaign) => {
                            mission = (mission + 1).min(campaign::reached() - 1)
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => confirm = true,
                        KeyCode::Esc | KeyCode::Char('q') => back = true,
                        _ => {}
                    }
                }
                Event::Mouse(m) if matches!(m.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) && t >= GLOBE_END && page == Page::Main => {
                    moves += if m.kind == MouseEventKind::ScrollUp { 1 } else { -1 };
                }
                Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                    if t < GLOBE_END {
                        start = skipped();
                        continue;
                    }
                    if page != Page::Main {
                        back = true;
                    } else if let Some(i) = rows.iter().position(|r| r.y == m.row && m.column >= r.x && m.column < r.x + r.width) {
                        pick = i;
                        confirm = true;
                    }
                }
                _ => {}
            }
        }
        if t < GLOBE_END && (moves != 0 || confirm || back) {
            start = skipped();
            continue;
        }
        if page != Page::Main {
            if confirm || back {
                if page == Page::Credits {
                    audio.play_music("Splash_Screen_Opus.mp3");
                }
                page = Page::Main;
            }
            continue;
        }
        if moves != 0 {
            pick = (pick as i32 - moves).rem_euclid(items.len() as i32) as usize;
        }
        if back {
            return Ok(Choice::Quit);
        }
        if confirm {
            match &items[pick] {
                Item::Campaign => return Ok(Choice::Campaign(mission)),
                Item::Level(i, _) => return Ok(Choice::Level(*i)),
                Item::Map(p) => return Ok(Choice::Map(p.clone())),
                Item::Controls => page = Page::Controls,
                Item::Credits => {
                    page = Page::Credits;
                    credits_at = Instant::now();
                    // the end of the road in the campaign
                    audio.play_music("Jerusalem_Israel.mp3");
                }
                Item::Quit => return Ok(Choice::Quit),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(f: &mut Frame, t: f32, items: &[Item], pick: usize, mission: usize, page: &Page, view: (f32, f32), credits_t: f32) -> Vec<Rect> {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
    if *page == Page::Credits && t >= GLOBE_END {
        draw_credits(f, area, credits_t);
        return Vec::new();
    }
    if t < EPIGRAPH_END {
        draw_epigraph(f, area, t);
        return Vec::new();
    }
    if t < GLOBE_END {
        draw_globe(f.buffer_mut(), area, area.height, t - EPIGRAPH_END, 1.0, view, None);
        let k = t - EPIGRAPH_END;
        let caption = if k < 3.0 { "2048" } else { "The Golgotha Transmission" };
        let fade = ((k % 3.0) / 0.8).min(1.0);
        let c = Color::Rgb((255.0 * fade) as u8, (180.0 * fade) as u8, (40.0 * fade) as u8);
        let y = area.y + area.height.saturating_sub(4);
        f.render_widget(Paragraph::new(Line::from(Span::styled(caption, Style::default().fg(c).add_modifier(Modifier::BOLD))).centered()), Rect { y, height: 1, ..area });
        return Vec::new();
    }
    // the title: the globe keeps turning behind it, dimmed
    let place = (*page == Page::Main).then(|| items[pick].place(mission));
    // the Earth fills the screen, centred, as in the opening
    draw_globe(f.buffer_mut(), area, area.height, t - EPIGRAPH_END, 0.45, view, place.map(|p| (p.1, p.2)));
    // breathing room above the title, more on taller windows
    let top = area.y + (area.height / 14).clamp(1, 5);
    let title_h = draw_big_title(f.buffer_mut(), area, top, t - GLOBE_END);
    let mut y = top + title_h + 1;
    // text over the globe gets a black backing on its own cells only, so it
    // stays readable without cutting a band across the Earth
    let line = |f: &mut Frame, y: u16, text: Line| {
        if y < area.y + area.height {
            let spans: Vec<Span> = text
                .spans
                .into_iter()
                .map(|sp| {
                    let style = sp.style;
                    if style.bg.is_none() { sp.style(style.bg(Color::Black)) } else { sp }
                })
                .collect();
            f.render_widget(Paragraph::new(Line::from(spans)).centered(), Rect { y, height: 1, ..area });
        }
    };
    line(f, y, Line::from(Span::styled("T H E   G O L G O T H A   T R A N S M I S S I O N", Style::default().fg(BONE))));
    let mut rows = Vec::new();
    match page {
        Page::Main => {
            // the menu sits low, just above the dedication, leaving the
            // middle of the screen to the Earth
            let foot = area.y + area.height.saturating_sub(3 + (area.height / 12).clamp(2, 6));
            y = foot.saturating_sub(items.len() as u16 + 2).max(y + 2);
            if let Some((name, lat, lon)) = place {
                let ns = if lat >= 0.0 { 'N' } else { 'S' };
                let ew = if lon >= 0.0 { 'E' } else { 'W' };
                let caption = format!("{name}   {:.1}°{ns} {:.1}°{ew}", lat.abs(), lon.abs());
                line(f, y.saturating_sub(2), Line::from(Span::styled(caption, Style::default().fg(AMBER))));
            }
            for (i, item) in items.iter().enumerate() {
                let label = item.label(mission);
                let selected = i == pick;
                let text = if selected { format!("> {label} <") } else { label.clone() };
                let style = if selected { Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD) } else { Style::default().fg(GREEN) };
                line(f, y, Line::from(Span::styled(format!(" {text} "), style)));
                let w = text.chars().count() as u16 + 2;
                rows.push(Rect { x: area.x + area.width.saturating_sub(w) / 2, y, width: w, height: 1 });
                y += 1;
            }
        }
        Page::Controls => render::draw_help(f, area),
        Page::Credits => {}
    }
    // the dedication, on every title page
    // the dedication sits low, two rows above the bottom edge
    let thanks = ["For Dave Taylor, Jonathan Clark, Carly Taylor and the whole team.", "Twenty-eight years on, it is finished."];
    // on one line when it fits, else on two, the whole block a row higher
    let wide = area.width as usize > thanks[0].len() + thanks[1].len() + 1;
    let rows_of_thanks: Vec<String> = if wide { vec![thanks.join(" ")] } else { thanks.iter().map(|t| t.to_string()).collect() };
    let foot = area.y + area.height.saturating_sub(4 + rows_of_thanks.len() as u16);
    line(f, foot, Line::from(Span::styled("Built on GOLGOTHA (1998) by Crack dot Com, given to the world in the public domain.", Style::default().fg(DIM))));
    for (k, t) in rows_of_thanks.iter().enumerate() {
        line(f, foot + 1 + k as u16, Line::from(Span::styled(t.clone(), Style::default().fg(DIM))));
    }
    let last = foot + 1 + rows_of_thanks.len() as u16;
    line(f, last, Line::from(vec![Span::styled("ASCIIVERSE", Style::default().fg(AMBER)), Span::styled("  2026", Style::default().fg(DIM))]));
    rows
}

fn draw_epigraph(f: &mut Frame, area: Rect, t: f32) {
    // one line every 1.2 s, typed in; the attribution after the last
    let h = EPIGRAPH.len() as u16 + 3;
    let y0 = area.y + area.height.saturating_sub(h) / 2;
    for (i, text) in EPIGRAPH.iter().enumerate() {
        let shown = ((t - 0.6 - i as f32 * 1.2) * 40.0).clamp(0.0, text.len() as f32) as usize;
        if shown == 0 {
            continue;
        }
        let style = if i == EPIGRAPH.len() - 1 { Style::default().fg(AMBER).add_modifier(Modifier::BOLD) } else { Style::default().fg(BONE) };
        let mut s: String = text.chars().take(shown).collect();
        s.extend(std::iter::repeat_n(' ', text.len() - shown));
        f.render_widget(Paragraph::new(Line::from(Span::styled(s, style))).centered(), Rect { y: y0 + i as u16, height: 1, ..area });
    }
    if t > 0.6 + EPIGRAPH.len() as f32 * 1.2 + 0.6 {
        let y = y0 + EPIGRAPH.len() as u16 + 1;
        f.render_widget(Paragraph::new(Line::from(Span::styled("William Shakespeare, Richard II", Style::default().fg(DIM)))).centered(), Rect { y, height: 1, ..area });
    }
}

/// NASA's Blue Marble (public domain, see data/IMAGES.txt), 512x256, as PPM.
static EARTH: &[u8] = include_bytes!("../data/earth.ppm");

/// The texture's pixels (RGB) and width/height, parsed once.
/// "P6\n<w> <h>\n255\n" then RGB bytes.
pub(crate) fn parse_ppm(bytes: &[u8]) -> (Vec<u8>, usize, usize) {
    let mut fields = Vec::new();
    let mut pos = 0;
    while fields.len() < 4 {
        while bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        fields.push(std::str::from_utf8(&bytes[start..pos]).unwrap_or("0").to_string());
    }
    let (w, h) = (fields[1].parse().unwrap_or(1), fields[2].parse().unwrap_or(1));
    (bytes[pos + 1..].to_vec(), w, h)
}

fn earth() -> &'static (Vec<u8>, usize, usize) {
    static TEX: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    TEX.get_or_init(|| parse_ppm(EARTH))
}

/// The Golgotha team on a supertank, from the 1998 release's textures
/// (see data/IMAGES.txt).
static TEAM: &[u8] = include_bytes!("../data/team.ppm");

fn team() -> &'static (Vec<u8>, usize, usize) {
    static IMG: std::sync::OnceLock<(Vec<u8>, usize, usize)> = std::sync::OnceLock::new();
    IMG.get_or_init(|| parse_ppm(TEAM))
}

fn sample_earth(lat: f32, lon: f32) -> (f32, f32, f32) {
    let (px, w, h) = earth();
    let u = ((lon / std::f32::consts::TAU + 0.5).rem_euclid(1.0) * *w as f32) as usize % w;
    let v = (((0.5 - lat / std::f32::consts::PI) * *h as f32) as usize).min(h - 1);
    let i = (v * w + u) * 3;
    (px[i] as f32, px[i + 1] as f32, px[i + 2] as f32)
}

/// Terminal cell height over width, from the terminal when it says, else 2.
fn cell_aspect() -> f32 {
    match ratatui::crossterm::terminal::window_size() {
        Ok(w) if w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0 => (w.height as f32 / w.rows as f32) / (w.width as f32 / w.columns as f32),
        _ => 2.0,
    }
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h.wrapping_mul(0x2c1b_3c6d) >> 8
}

/// The spinning Earth against the stars, the original opening's image: the
/// Blue Marble texture on a sphere in half-block pixels (each its own
/// colour), lit from the upper left, with a thin atmosphere rim; Golgotha
/// (Jerusalem, 31.8 N 35.2 E) pulses red on the near side. Kept round for
/// the terminal's real cell shape.
/// `band` is how many rows from the top the globe may use (the stars fill
/// the whole area): the title keeps the globe above the dedication.
pub(crate) fn draw_globe(buf: &mut Buffer, area: Rect, band: u16, t: f32, bright: f32, view: (f32, f32), marker: Option<(f32, f32)>) {
    let (w, h) = (area.width as usize, area.height as usize);
    let bh = band.min(area.height) as f32 * 2.0;
    let (pw, ph) = (w, h * 2);
    // a half-block pixel is one column wide and half a cell tall
    let px_aspect = cell_aspect() / 2.0;
    let r = (bh * 0.46 * px_aspect).min(pw as f32 * 0.45);
    let (rx, ry) = (r, r / px_aspect);
    let (cx, cy) = (pw as f32 / 2.0, bh / 2.0);
    let _ = ph;
    let (spin, tilt) = view;
    let (ct, st) = (tilt.cos(), tilt.sin());
    let mark = marker.map(|(la, lo)| (la.to_radians(), lo.to_radians()));
    let pulse = ((t * 4.0).sin() * 0.5 + 0.5) * 0.7 + 0.3;
    let (glat, glon) = (31.8f32.to_radians(), 35.2f32.to_radians());
    let pixel = |x: usize, y: usize| -> Option<Color> {
        let nx = (x as f32 + 0.5 - cx) / rx;
        let ny = (y as f32 + 0.5 - cy) / ry;
        let d2 = nx * nx + ny * ny;
        if d2 > 1.0 {
            // atmosphere rim
            let d = d2.sqrt();
            return (d < 1.035).then(|| {
                let k = (1.0 - (d - 1.0) / 0.035) * 0.6 * bright;
                Color::Rgb((70.0 * k) as u8, (130.0 * k) as u8, (220.0 * k) as u8)
            });
        }
        let z = (1.0 - d2).sqrt();
        // tilt the globe about the screen's x axis so `tilt` latitude faces us
        let (vy, vz) = (-ny, z);
        let (wy, wz) = (vy * ct + vz * st, -vy * st + vz * ct);
        let lat = wy.clamp(-1.0, 1.0).asin();
        let lon = nx.atan2(wz) + spin;
        if let Some((ml, mn)) = mark {
            let d = (lat.sin() * ml.sin() + lat.cos() * ml.cos() * (lon - mn).cos()).clamp(-1.0, 1.0).acos();
            // a pulsing amber ring around the selected place
            if (d - (0.06 + 0.02 * pulse)).abs() < 0.012 {
                return Some(Color::Rgb(255, (180.0 * pulse.max(0.6)) as u8, 40));
            }
        }
        let dist = (lat.sin() * glat.sin() + lat.cos() * glat.cos() * (lon - glon).cos()).clamp(-1.0, 1.0).acos();
        if dist < 0.03 + 0.015 * pulse {
            return Some(Color::Rgb((255.0 * pulse.max(bright)) as u8, (50.0 * bright) as u8, (40.0 * bright) as u8));
        }
        let (r_, g_, b_) = sample_earth(lat, lon);
        let light = (0.12 + 0.95 * (z * 0.75 - nx * 0.45 - ny * 0.25).max(0.0)).min(1.0) * bright;
        Some(Color::Rgb((r_ * light) as u8, (g_ * light) as u8, (b_ * light) as u8))
    };
    for row in 0..h {
        for col in 0..w {
            let (top, bottom) = (pixel(col, row * 2), pixel(col, row * 2 + 1));
            let Some(cell) = buf.cell_mut((area.x + col as u16, area.y + row as u16)) else { continue };
            match (top, bottom) {
                (None, None) => {
                    // stars, twinkling
                    let s = hash(col as u32, row as u32);
                    if s % 41 == 0 && (t * 1.5 + (s % 11) as f32).sin() > -0.3 {
                        let k = (150.0 + (s % 100) as f32) * bright;
                        cell.set_char(if s % 3 == 0 { '+' } else { '·' }).set_fg(Color::Rgb(k as u8, k as u8, (k * 0.95) as u8)).set_bg(Color::Black);
                    }
                }
                (t_, b_) => {
                    cell.set_char('▀').set_fg(t_.unwrap_or(Color::Black)).set_bg(b_.unwrap_or(Color::Black));
                }
            }
        }
    }
}

/// Block capitals, 5 wide and 5 tall, for the title.
fn glyph(c: char) -> [&'static str; 5] {
    match c {
        'C' => [" ####", "#    ", "#    ", "#    ", " ####"],
        'A' => [" ### ", "#   #", "#####", "#   #", "#   #"],
        'L' => ["#    ", "#    ", "#    ", "#    ", "#####"],
        'V' => ["#   #", "#   #", "#   #", " # # ", "  #  "],
        'R' => ["#### ", "#   #", "#### ", "#  # ", "#   #"],
        'Y' => ["#   #", " # # ", "  #  ", "  #  ", "  #  "],
        _ => ["     "; 5],
    }
}

/// CALVARY in block letters, wiped in from the left over the first second;
/// returns the rows used.
fn draw_big_title(buf: &mut Buffer, area: Rect, top: u16, t: f32) -> u16 {
    let word = "CALVARY";
    // each glyph column becomes two terminal columns, each glyph row one row
    let (gw, gap) = (10u16, 2u16);
    let width = word.len() as u16 * (gw + gap) - gap;
    if width > area.width || top + 5 > area.y + area.height {
        // too small for block letters: plain text
        let x0 = area.x + area.width.saturating_sub(7) / 2;
        for (i, ch) in word.chars().enumerate() {
            if let Some(cell) = buf.cell_mut((x0 + i as u16, top)) {
                cell.set_char(ch).set_fg(AMBER);
            }
        }
        return 1;
    }
    let x0 = area.x + (area.width - width) / 2;
    let reveal = (t / 1.0).clamp(0.0, 1.0) * width as f32;
    for (i, ch) in word.chars().enumerate() {
        for (row, bits) in glyph(ch).iter().enumerate() {
            for (col, b) in bits.chars().enumerate() {
                if b != '#' {
                    continue;
                }
                for half in 0..2u16 {
                    let x = x0 + i as u16 * (gw + gap) + col as u16 * 2 + half;
                    if (x - x0) as f32 > reveal {
                        continue;
                    }
                    // a little vertical shading: lighter on top
                    let shade = 1.0 - row as f32 * 0.09;
                    let c = Color::Rgb((255.0 * shade) as u8, (180.0 * shade) as u8, (40.0 * shade) as u8);
                    if let Some(cell) = buf.cell_mut((x, top + row as u16)) {
                        cell.set_char('█').set_fg(c).set_bg(Color::Black);
                    }
                }
            }
        }
    }
    5
}

/// The credits, Nintendo style: the team photo, then the key people one by
/// one over it, then everyone rolling upward.
const PHOTO_END: f32 = 6.0;
const CARD: f32 = 3.2;
const CARDS: [(&str, &str); 4] = [
    ("Design and business", "Dave Taylor"),
    ("Programming", "Jonathan Clark"),
    ("The name, the plot, the voices", "Carly Taylor"),
    ("Sound and music", "Gene Rosenburg"),
];
/// Roll speed in rows per second; the dot font moves a quarter row at a time.
const ROLL_SPEED: f32 = 4.0;

/// The rolling credits: (style, text). Style 0 heading, 1 name, 2 note.
/// Lines stay short so the dot font fits a 120 column terminal.
const ROLL: &[(u8, &str)] = &[
    (0, "GOLGOTHA"),
    (2, "Crack dot Com, 1996-1998"),
    (2, ""),
    (2, "Design and business"),
    (1, "Dave Taylor"),
    (2, ""),
    (2, "Programming"),
    (1, "Jonathan Clark"),
    (1, "Oliver Yu"),
    (1, "Trey Harrision"),
    (2, ""),
    (2, "Artwork and level design"),
    (1, "Kevin Tyler"),
    (1, "Stephen Wilson"),
    (2, ""),
    (2, "Sound and music"),
    (1, "Gene Rosenburg"),
    (2, ""),
    (2, "System administration"),
    (1, "Jacob Leverich"),
    (2, ""),
    (2, "Whose work did not make"),
    (2, "the 1998 release"),
    (1, "Josh Prykril, the intro movie"),
    (1, "Ken Baird, programming"),
    (1, "Carly Taylor, the name Golgotha,"),
    (1, "the plot, and the voice direction"),
    (2, "and all the volunteer voice talent,"),
    (2, "and everyone who made"),
    (2, "a Golgotha fan site"),
    (2, ""),
    (2, "With Crack dot Com's thanks to"),
    (1, "TelStar  Red Hat  Origin  Bungie"),
    (1, "Octogon  AMD  SGI  DEC  Microsoft"),
    (1, "3Dfx  3Dlabs  Matrox  PowerVR"),
    (1, "Aureal  Diamond  Creative Labs"),
    (1, "S3  Logitech  id Software"),
    (1, "Eclipse Games  IDT  Cyrix"),
    (2, ""),
    (2, "Given to the world"),
    (2, "in the public domain"),
    (1, "October 22, 1998"),
    (2, ""),
    (2, ""),
    (0, "GOLGOTHA FOREVER"),
    (1, "Patrick Grawehr"),
    (2, "and the revival team,"),
    (2, "who kept the game alive"),
    (2, ""),
    (2, ""),
    (0, "CALVARY"),
    (2, "Asciiverse, 2026"),
    (2, ""),
    (2, "Their game, their maps,"),
    (2, "their music,"),
    (2, "finished in a terminal."),
    (2, ""),
    (2, "Music: Gene Rosenburg's"),
    (2, "Golgotha soundtrack"),
    (2, "The Earth: NASA Blue Marble"),
    (2, "Story: after Carly Taylor's"),
    (2, "Golgotha Transmission"),
];

/// Game pieces rising with the roll: (roll line they sit beside, unit kind,
/// enemy colours, side: -1 left, 1 right).
const ILLUSTRATIONS: [(&str, &str, bool, f32); 10] = [
    ("GOLGOTHA", "stank", false, -1.0),
    ("Programming", "peon_tank", true, 1.0),
    ("Artwork and level design", "helicopter", false, -1.0),
    ("Sound and music", "cobra_tank", false, 1.0),
    ("Whose work did not make", "electric_car", true, -1.0),
    ("With Crack dot Com's thanks to", "bomber", false, 1.0),
    ("Given to the world", "jet", true, -1.0),
    ("GOLGOTHA FOREVER", "rocket_tank", false, 1.0),
    ("CALVARY", "stank", false, -1.0),
    ("CALVARY", "stank", true, 1.0),
];

/// The public domain X11 misc-fixed 5x8 font: per glyph 8 rows of 5 bits,
/// the high bit leftmost.
fn font() -> &'static std::collections::HashMap<char, [u8; 8]> {
    static FONT: std::sync::OnceLock<std::collections::HashMap<char, [u8; 8]>> = std::sync::OnceLock::new();
    FONT.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        // the first line is a comment; glyph rows may start with '#' too
        let mut lines = include_str!("../data/font5x8.txt").lines().skip(1);
        while let Some(code) = lines.next() {
            let Some(c) = code.trim().parse::<u32>().ok().and_then(char::from_u32) else { break };
            let mut rows = [0u8; 8];
            for r in &mut rows {
                *r = lines.next().unwrap_or("").bytes().fold(0, |acc, b| acc << 1 | (b == b'#') as u8);
            }
            map.insert(c, rows);
        }
        map
    })
}

/// Width of `text` in font pixels at `scale` (6 per glyph, the last gap cut).
fn text_px(text: &str, scale: usize) -> usize {
    (text.chars().count() * 6).saturating_sub(1) * scale
}

/// Call `put(x, y)` for every lit font pixel of `text`, top-left at (x0, y0).
fn font_pixels(text: &str, scale: usize, x0: i32, y0: i32, mut put: impl FnMut(i32, i32)) {
    let f = font();
    for (i, ch) in text.chars().enumerate() {
        let Some(rows) = f.get(&ch) else { continue };
        for (r, bits) in rows.iter().enumerate() {
            for c in 0..5 {
                if bits >> (4 - c) & 1 == 1 {
                    for s in 0..scale * scale {
                        put(x0 + ((i * 6 + c) * scale + s % scale) as i32, y0 + (r * scale + s / scale) as i32);
                    }
                }
            }
        }
    }
}

/// Braille dots over a screen area, one colour per cell.
struct Dots {
    w: usize,
    h: usize,
    cells: Vec<(u8, Color)>,
}

impl Dots {
    fn new(area: Rect) -> Dots {
        let (w, h) = (area.width as usize, area.height as usize);
        Dots { w, h, cells: vec![(0, Color::Black); w * h] }
    }

    fn plot(&mut self, x: i32, y: i32, c: Color) {
        if x < 0 || y < 0 || x as usize >= self.w * 2 || y as usize >= self.h * 4 {
            return;
        }
        const BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
        let cell = &mut self.cells[y as usize / 4 * self.w + x as usize / 2];
        cell.0 |= BITS[x as usize % 2][y as usize % 4];
        cell.1 = c;
    }

    fn flush(&self, buf: &mut Buffer, area: Rect) {
        for (i, &(bits, c)) in self.cells.iter().enumerate() {
            if bits != 0 {
                if let Some(cell) = buf.cell_mut((area.x + (i % self.w) as u16, area.y + (i / self.w) as u16)) {
                    cell.set_char(char::from_u32(0x2800 + bits as u32).unwrap_or(' ')).set_fg(c).set_bg(Color::Black);
                }
            }
        }
    }
}

/// How the roll is laid out on this screen: drawn in dots, or (on a
/// narrow terminal) as plain text a row at a time.
struct RollLayout {
    dotted: bool,
    /// Per line: top in dots from the roll's start, and font scale.
    lines: Vec<(i32, usize)>,
    /// Total height in dots.
    total: i32,
}

fn roll_layout(area: Rect) -> RollLayout {
    let fits = |text: &str, scale: usize| text_px(text, scale) <= area.width as usize * 2;
    let dotted = ROLL.iter().all(|&(_, text)| fits(text, 1));
    let mut lines = Vec::with_capacity(ROLL.len());
    let mut y = 0;
    for &(style, text) in ROLL {
        let scale = if dotted && style == 0 && fits(text, 2) { 2 } else { 1 };
        lines.push((y, scale));
        y += match (dotted, text.is_empty()) {
            (false, _) => 4,
            (true, true) => 5,
            (true, false) => 8 * scale as i32 + 2 * scale as i32,
        };
    }
    RollLayout { dotted, lines, total: y }
}

/// Draw the illustration sprites for the roll; `row_of(line)` gives the
/// screen row (from the area's top) a roll line's middle is at.
fn draw_illustrations(buf: &mut Buffer, area: Rect, row_of: impl Fn(usize) -> f32, ct: f32) {
    use crate::sprites::{self, PixelLayer, Pose};
    // 6 columns per map cell: a supertank is about 10 columns long
    let cols = 6.0;
    let mut layer = PixelLayer::new(area, (0.0, 0.0), cols);
    let bottom = area.height as f32;
    for (i, &(anchor, kind, enemy, side)) in ILLUSTRATIONS.iter().enumerate() {
        let Some(line) = ROLL.iter().position(|&(_, t)| t == anchor) else { continue };
        let row = row_of(line);
        if row < -4.0 || row > bottom + 4.0 {
            continue;
        }
        let col = area.width as f32 * (0.5 + side * 0.36);
        let (x, y) = (col / cols, row * 2.0 / cols);
        let pal = if enemy { sprites::ENEMY } else { sprites::PLAYER };
        // each turns slowly; the two supertanks at the end face each other
        let last_pair = i >= ILLUSTRATIONS.len() - 2;
        let heading = if last_pair { if side < 0.0 { 0.0 } else { std::f32::consts::PI } } else { ct * 0.5 + i as f32 * 1.3 };
        if crate::world::flies(kind) {
            sprites::paint_aircraft_at(&mut layer, kind, x, y, heading, pal, false, ct);
        } else {
            let pose = Pose { x, y, heading, aim: heading + (ct * 0.7 + i as f32).sin() * 0.6, odometer: ct * 1.5, muzzle: 0.0, hit: 0.0 };
            sprites::paint_vehicle(&mut layer, sprites::look(kind), pal, pose);
        }
    }
    layer.flush_black(buf);
}

/// The finale's background: a battle playing itself out in the follow view,
/// muted, behind the thanks. Kept between frames.
struct Finale {
    game: crate::game::Game,
    /// Seconds simulated so far.
    sim: f32,
    /// Where the supertank was at the last progress check, and when.
    mark: (f32, f32, f32),
    /// Backing out of a dead end until this time.
    reverse_until: f32,
    /// Playing film number `.0`, next tick `.1` (None: the autopilot battle).
    film: Option<(usize, usize)>,
}

/// Films for the finale, recorded with F12 in a battle and picked from
/// `films/` into `data/films/`. With none bundled, the finale falls back to
/// the autopilot battle.
const FILMS: &[&[u8]] = &[];

fn films() -> &'static [crate::film::Film] {
    static PARSED: std::sync::OnceLock<Vec<crate::film::Film>> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| {
        FILMS
            .iter()
            .filter_map(|bytes| crate::film::Film::parse(bytes).map_err(|e| log::warn!("bundled film: {e}")).ok())
            .collect()
    })
}

thread_local! {
    static FINALE: std::cell::RefCell<Option<Finale>> = const { std::cell::RefCell::new(None) };
}

impl Finale {
    fn new() -> Option<Finale> {
        if let Some(first) = films().first() {
            let game = first.stage().ok()?;
            return Some(Finale { game, sim: 0.0, mark: (0.0, 0.0, 0.0), reverse_until: 0.0, film: Some((0, 0)) });
        }
        // Switzerland, one of the two levels Crack dot Com showed playable;
        // both sides build there (Cairo has no garage of ours)
        let level = LEVELS.iter().position(|l| l.name == "snow").unwrap_or(0);
        let mut game = crate::game::Game::new(level).ok()?;
        game.show_aim = false;
        let (x, y) = (game.tank.x, game.tank.y);
        Some(Finale { game, sim: 0.0, mark: (x, y, 0.0), reverse_until: 0.0, film: None })
    }

    /// Brightness of the picture: films fade in and out at their cuts.
    fn fade(&self) -> f32 {
        match self.film {
            Some((n, tick)) => {
                let len = films()[n].len() as f32;
                let s = |ticks: f32| (ticks * crate::game::DT / 0.8).clamp(0.0, 1.0);
                s(tick as f32).min(s(len - tick as f32))
            }
            None => 1.0,
        }
    }

    /// Play films up to `te` seconds, one after another, looping. Every
    /// tick is applied (a film stores only what changed), so a stalled
    /// frame catches up rather than skipping.
    fn advance_film(&mut self, te: f32) {
        let dt = crate::game::DT;
        while self.sim + dt <= te {
            self.sim += dt;
            let Some((mut n, mut tick)) = self.film else { return };
            let list = films();
            if tick >= list[n].len() {
                n = (n + 1) % list.len();
                tick = 0;
                match list[n].stage() {
                    Ok(g) => self.game = g,
                    Err(e) => {
                        log::warn!("film {n}: {e}");
                        return;
                    }
                }
            }
            list[n].apply(tick, &mut self.game);
            self.film = Some((n, tick + 1));
        }
    }

    /// Run the battle up to `te` seconds: our garage builds, the supertank
    /// rides behind the lead of the convoy, fires at what comes near, and
    /// backs out when stuck.
    fn advance(&mut self, te: f32) {
        use crate::world::{Class, Team};
        if self.film.is_some() {
            return self.advance_film(te);
        }
        let dt = crate::game::DT;
        // a stalled frame skips ahead rather than racing to catch up
        self.sim = self.sim.max(te - 1.0);
        while self.sim + dt <= te {
            self.sim += dt;
            let g = &mut self.game;
            if g.world.outcome != crate::world::Outcome::Playing {
                if let Some(fresh) = Finale::new() {
                    *self = Finale { sim: self.sim, ..fresh };
                }
                continue;
            }
            // our garage keeps a convoy coming, so there is a front to follow
            let k = (self.sim / 5.0) as usize;
            if ((self.sim - dt) / 5.0) as usize != k {
                g.world.order(Team::Player, crate::world::BUILDABLE[k % 4]);
            }
            let t = &g.tank;
            let mut c = crate::tank::Controls::default();
            let near = |team: Team, ground: bool| {
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.team == team && e.class != Class::Prop && (!ground || (e.class == Class::Vehicle && !crate::world::flies(&e.kind))))
                    .map(|e| (e.x - t.x, e.y - t.y))
                    .min_by(|a, b| (a.0.hypot(a.1)).total_cmp(&b.0.hypot(b.1)))
            };
            // the lead vehicle of our convoy: the one furthest from home
            let home = g.world.find(Class::Hq, Team::Player).map(|i| (g.world.entities[i].x, g.world.entities[i].y));
            let lead = home.and_then(|(hx, hy)| {
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.team == Team::Player && e.class == Class::Vehicle && !crate::world::flies(&e.kind))
                    .max_by(|a, b| (a.x - hx).hypot(a.y - hy).total_cmp(&(b.x - hx).hypot(b.y - hy)))
                    .map(|e| (e.x - t.x, e.y - t.y))
            });
            let foe = near(Team::Enemy, false);
            if let Some((dx, dy)) = foe {
                let dist = dx.hypot(dy);
                c.fire_main = dist < 16.0 && (self.sim % 1.1) < dt;
                c.fire_chain = dist < 9.0 && (self.sim % 3.0) < 0.8;
            }
            if self.sim < self.reverse_until {
                c.throttle = Some(-0.6);
                c.turn = 1.0;
            } else if let Some((dx, dy)) = lead {
                // ride a few cells behind the lead, stop when close
                let dist = dx.hypot(dy);
                let d = (dy.atan2(dx) - t.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                c.turn = (d * 1.5).clamp(-1.0, 1.0);
                c.throttle = Some(if dist < 4.0 { 0.0 } else if d.abs() > 1.2 { 0.2 } else { (dist / 10.0).clamp(0.3, 0.8) });
            }
            g.update(&c);
            g.world.sounds.clear();
            let (x, y) = (g.tank.x, g.tank.y);
            if self.sim - self.mark.2 > 2.0 {
                if (x - self.mark.0).hypot(y - self.mark.1) < 1.0 && c.throttle.unwrap_or(0.0) > 0.1 {
                    self.reverse_until = self.sim + 1.5;
                }
                self.mark = (x, y, self.sim);
            }
        }
    }
}

/// The finale, `te` seconds in: the muted battle, framed like a picture,
/// and the big thanks under it.
fn draw_finale(f: &mut Frame, area: Rect, te: f32) {
    // the thanks: the largest of the half-block letter sizes that fits
    let lines = ["Thank you", "for playing."];
    let (w, h) = (area.width as usize, area.height as usize * 2);
    let widest = lines.iter().map(|l| text_px(l, 1)).max().unwrap_or(1);
    let scale = (1..=2).rev().find(|&s| widest * s <= w * 9 / 10 && 19 * s <= h * 4 / 10).unwrap_or(0);
    let text_rows = if scale == 0 { 1 } else { (19 * scale as u16).div_ceil(2) };
    // the battle as a framed picture above it, about 2:1, black around
    let inner_h = area.height.saturating_sub(text_rows + 6);
    let inner_w = (inner_h as f32 * 3.6).min(area.width as f32 * 0.85) as u16;
    let framed = inner_h >= 6 && inner_w >= 20;
    let pic = if framed {
        Rect { x: area.x + (area.width - inner_w) / 2, y: area.y + 2, width: inner_w, height: inner_h }
    } else {
        area
    };
    let buf = f.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ').set_fg(Color::Black).set_bg(Color::Black);
            }
        }
    }
    let mut fade = 1.0;
    FINALE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Finale::new();
        }
        if let Some(fin) = slot.as_mut() {
            fin.advance(te);
            crate::chase::draw(buf, pic, &fin.game);
            fade = fin.fade();
        }
    });
    // muted: mostly grey, darkened, fading up from black
    let k = (te / 2.5).min(1.0) * fade;
    let mute = |c: Color| match c {
        Color::Rgb(r, g, b) => {
            let l = 0.3 * r as f32 + 0.59 * g as f32 + 0.11 * b as f32;
            let m = |v: u8| ((v as f32 * 0.3 + l * 0.7) * k * 0.55) as u8;
            Color::Rgb(m(r), m(g), m(b).saturating_add((6.0 * k) as u8))
        }
        other => other,
    };
    for y in pic.y..pic.y + pic.height {
        for x in pic.x..pic.x + pic.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let (fg, bg) = (mute(cell.fg), mute(cell.bg));
                cell.set_fg(fg).set_bg(bg);
            }
        }
    }
    if framed {
        // a thin gilt frame hugging the picture, in half-block edges
        let gold = faded((150, 118, 60), k);
        let (x0, y0, x1, y1) = (pic.x - 1, pic.y - 1, pic.x + pic.width, pic.y + pic.height);
        let mut edge = |x: u16, y: u16, ch: char| {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(ch).set_fg(gold).set_bg(Color::Black);
            }
        };
        for x in pic.x..x1 {
            edge(x, y0, '▄');
            edge(x, y1, '▀');
        }
        for y in pic.y..y1 {
            edge(x0, y, '▐');
            edge(x1, y, '▌');
        }
        edge(x0, y0, '▗');
        edge(x1, y0, '▖');
        edge(x0, y1, '▝');
        edge(x1, y1, '▘');
    }

    let shown = ((te - 0.8) / 1.5).clamp(0.0, 1.0);
    let text_top = if framed { pic.y + pic.height + 2 } else { area.y + area.height * 2 / 5 };
    if scale == 0 {
        f.render_widget(Paragraph::new(Line::from(Span::styled("Thank you for playing.", Style::default().fg(faded((255, 180, 40), shown)).add_modifier(Modifier::BOLD)))).centered(), Rect { y: text_top, height: 1, ..area });
    } else {
        let top = ((text_top - area.y) * 2) as i32;
        let buf = f.buffer_mut();
        let mut put = |x: i32, y: i32, c: Color| {
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                return;
            }
            if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + (y / 2) as u16)) {
                if cell.symbol() != "▀" {
                    let bg = cell.bg;
                    cell.set_char('▀').set_fg(bg);
                }
                if y % 2 == 0 {
                    cell.set_fg(c);
                } else {
                    cell.set_bg(c);
                }
            }
        };
        for (i, text) in lines.iter().enumerate() {
            let x0 = (w as i32 - text_px(text, scale) as i32) / 2;
            let y0 = top + (i * (8 + 3) * scale) as i32;
            if !framed {
                font_pixels(text, scale, x0 + 1, y0 + 1, |x, y| put(x, y, Color::Rgb(0, 0, 0)));
            }
            font_pixels(text, scale, x0, y0, |x, y| {
                // lighter on top, like the title logo
                let shade = 1.0 - ((y - y0) as f32 / (8 * scale) as f32) * 0.35;
                put(x, y, faded(((255.0 * shade) as u8, (180.0 * shade) as u8, (40.0 * shade) as u8), shown));
            });
        }
    }
    let foot = area.y + area.height.saturating_sub(1);
    if foot > text_top + text_rows {
        let hint = Span::styled("any key to return", Style::default().fg(faded((70, 110, 80), shown)));
        f.render_widget(Paragraph::new(Line::from(hint)).centered(), Rect { y: foot, height: 1, ..area });
    }
}

fn faded(c: (u8, u8, u8), k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::Rgb((c.0 as f32 * k) as u8, (c.1 as f32 * k) as u8, (c.2 as f32 * k) as u8)
}

/// The team photo in half-block pixels, fitted into `area`'s top part.
fn draw_photo(buf: &mut Buffer, area: Rect, bright: f32) -> Rect {
    draw_picture(buf, area, team(), bright, (0.72, 0.62, 0.08))
}

/// A picture in half-block pixels, centred across `area`: `fit` is the
/// largest share of the width and height it may take, and where its top
/// sits, as fractions of the area.
pub(crate) fn draw_picture(buf: &mut Buffer, area: Rect, img: &(Vec<u8>, usize, usize), bright: f32, fit: (f32, f32, f32)) -> Rect {
    let (px, iw, ih) = img;
    // a half-block pixel is about square: one column wide, half a row tall
    let max_w = (area.width as f32 * fit.0) as usize;
    let max_h = (area.height as f32 * fit.1) as usize * 2;
    let scale = (max_w as f32 / *iw as f32).min(max_h as f32 / *ih as f32);
    let (w, h) = ((*iw as f32 * scale) as usize, (*ih as f32 * scale) as usize / 2 * 2);
    let x0 = area.x + (area.width - w as u16) / 2;
    let y0 = area.y + (area.height as f32 * fit.2) as u16;
    let at = |x: usize, y: usize| {
        let (sx, sy) = (((x as f32 + 0.5) / scale) as usize, ((y as f32 + 0.5) / scale) as usize);
        let i = (sy.min(ih - 1) * iw + sx.min(iw - 1)) * 3;
        faded((px[i], px[i + 1], px[i + 2]), bright)
    };
    for row in 0..h / 2 {
        for col in 0..w {
            if let Some(cell) = buf.cell_mut((x0 + col as u16, y0 + row as u16)) {
                cell.set_char('▀').set_fg(at(col, row * 2)).set_bg(at(col, row * 2 + 1));
            }
        }
    }
    Rect { x: x0, y: y0, width: w as u16, height: (h / 2) as u16 }
}

fn draw_credits(f: &mut Frame, area: Rect, ct: f32) {
    let roll_at = PHOTO_END + CARDS.len() as f32 * CARD;
    let layout = roll_layout(area);
    // dots per second: the plain text fallback moves a whole row at a time,
    // so it goes slower to stay readable
    let speed = if layout.dotted { ROLL_SPEED * 4.0 } else { 2.6 * 4.0 };
    let (bottom, mid) = (area.height as i32 * 4, area.height as i32 / 2 * 4);
    // the finale once the last line has risen to the middle
    let roll_end = roll_at + (bottom + layout.total - mid) as f32 / speed;
    if ct >= roll_end {
        draw_finale(f, area, ct - roll_end);
        return;
    }
    FINALE.with(|cell| cell.borrow_mut().take());
    // the photo: fades in, holds, dims behind the cards, fades as the roll rises
    let bright = if ct < 2.5 {
        ct / 2.5
    } else if ct < PHOTO_END {
        1.0
    } else if ct < roll_at {
        0.35
    } else {
        (0.35 - (ct - roll_at) / 4.0 * 0.25).max(0.1)
    };
    let photo = draw_photo(f.buffer_mut(), area, bright);
    let center = |f: &mut Frame, y: u16, text: &str, c: Color, bold: bool| {
        if y >= area.y && y < area.y + area.height {
            let mut st = Style::default().fg(c).bg(Color::Black);
            if bold {
                st = st.add_modifier(Modifier::BOLD);
            }
            f.render_widget(Paragraph::new(Line::from(Span::styled(text.to_string(), st))).centered(), Rect { y, height: 1, ..area });
        }
    };
    let below = photo.y + photo.height + 2;
    if ct < PHOTO_END {
        let k = ((ct - 1.0) / 1.5).clamp(0.0, 1.0);
        center(f, below, "Crack dot Com, 1998", faded((255, 180, 40), k), true);
        center(f, below + 1, "the Golgotha team", faded((70, 110, 80), k), false);
    } else if ct < roll_at {
        let i = ((ct - PHOTO_END) / CARD) as usize;
        let local = (ct - PHOTO_END) - i as f32 * CARD;
        let k = (local / 0.6).min((CARD - local) / 0.6).clamp(0.0, 1.0);
        let (role, name) = CARDS[i.min(CARDS.len() - 1)];
        let spaced: String = name.to_uppercase().chars().flat_map(|c| [c, ' ']).collect();
        let mid = photo.y + photo.height / 2;
        center(f, mid - 1, role, faded((150, 190, 160), k), false);
        center(f, mid + 1, spaced.trim_end(), faded((255, 180, 40), k), true);
    } else {
        // whole dots: the roll steps a quarter row at a time
        let rise = ((ct - roll_at) * speed) as i32;
        let top_of = |i: usize| bottom - rise + layout.lines[i].0;
        let colour = |style: u8| match style {
            0 => (255, 180, 40),
            1 => (80, 220, 120),
            _ => (150, 170, 155),
        };
        draw_illustrations(
            f.buffer_mut(),
            area,
            |i| (top_of(i) + 4 * layout.lines[i].1 as i32) as f32 / 4.0,
            ct,
        );
        if layout.dotted {
            let mut dots = Dots::new(area);
            for (i, &(style, text)) in ROLL.iter().enumerate() {
                let (top, scale) = (top_of(i), layout.lines[i].1);
                if top + 8 * scale as i32 <= 0 || top >= bottom {
                    continue;
                }
                let c = colour(style);
                let c = Color::Rgb(c.0, c.1, c.2);
                let x0 = (area.width as i32 * 2 - text_px(text, scale) as i32) / 2;
                font_pixels(text, scale, x0, top, |x, y| dots.plot(x, y, c));
            }
            dots.flush(f.buffer_mut(), area);
        } else {
            for (i, &(style, text)) in ROLL.iter().enumerate() {
                let y = top_of(i) / 4;
                if y >= 0 && y < area.height as i32 {
                    center(f, area.y + y as u16, text, faded(colour(style), 1.0), style == 0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// The whole credits timeline renders at any size: photo, cards, roll
    /// with illustrations, and the battle finale.
    #[test]
    fn credits_render_from_start_to_finale() {
        for (w, h) in [(20, 8), (80, 24), (160, 50), (300, 100)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            let mut ct = 0.0;
            while ct < 140.0 {
                term.draw(|f| draw_credits(f, f.area(), ct)).unwrap();
                ct += 0.37;
            }
        }
    }
}
