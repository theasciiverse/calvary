//! Opening and title screen.
//!
//! The opening follows the original's own: the Shakespeare epigraph from the
//! crack.com Golgotha pages (Richard II, IV.i), then the spinning globe
//! against a sea of stars that their Act 1 Scene 1 fades into, with the
//! Golgotha site marked. Then the title, the menu, and the credits of the
//! people who made Golgotha, as Crack dot Com published them.

use crate::audio::Audio;
use crate::game::LEVELS;
use crate::input::Input;
use crate::render;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
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

/// The Golgotha team, as crack.com's credits page lists them (names as
/// published there).
const CREDITS: &[(&str, &str)] = &[
    ("Design and business", "Dave Taylor"),
    ("Programming", "Jonathan Clark, Oliver Yu, Trey Harrision"),
    ("Artwork and level design", "Kevin Tyler, Stephen Wilson"),
    ("Sound and music", "Gene Rosenburg"),
    ("System administration", "Jacob Leverich"),
    ("Intro movie", "Josh Prykril"),
    ("Programming", "Ken Baird"),
    ("The name, the plot, the voices", "Carly Taylor"),
];

pub enum Choice {
    Level(usize),
    Map(String),
    Quit,
}

enum Item {
    Level(usize, &'static str),
    Map(String),
    Controls,
    Credits,
    Quit,
}

impl Item {
    fn label(&self) -> String {
        match self {
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
    let mut items: Vec<Item> = LEVELS.iter().enumerate().map(|(i, l)| Item::Level(i, names(l.name))).collect();
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
    let mut page = Page::Main;
    loop {
        let t = start.elapsed().as_secs_f32();
        let mut rows: Vec<Rect> = Vec::new();
        terminal.draw(|f| {
            rows = draw(f, t, &items, pick, &page);
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
                        KeyCode::Enter | KeyCode::Char(' ') => confirm = true,
                        KeyCode::Esc | KeyCode::Char('q') => back = true,
                        _ => {}
                    }
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
                Item::Level(i, _) => return Ok(Choice::Level(*i)),
                Item::Map(p) => return Ok(Choice::Map(p.clone())),
                Item::Controls => page = Page::Controls,
                Item::Credits => page = Page::Credits,
                Item::Quit => return Ok(Choice::Quit),
            }
        }
    }
}

fn draw(f: &mut Frame, t: f32, items: &[Item], pick: usize, page: &Page) -> Vec<Rect> {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
    if t < EPIGRAPH_END {
        draw_epigraph(f, area, t);
        return Vec::new();
    }
    if t < GLOBE_END {
        draw_globe(f.buffer_mut(), area, t - EPIGRAPH_END, 1.0);
        let k = t - EPIGRAPH_END;
        let caption = if k < 3.0 { "2048" } else { "The Golgotha Transmission" };
        let fade = ((k % 3.0) / 0.8).min(1.0);
        let c = Color::Rgb((255.0 * fade) as u8, (180.0 * fade) as u8, (40.0 * fade) as u8);
        let y = area.y + area.height.saturating_sub(4);
        f.render_widget(Paragraph::new(Line::from(Span::styled(caption, Style::default().fg(c).add_modifier(Modifier::BOLD))).centered()), Rect { y, height: 1, ..area });
        return Vec::new();
    }
    // the title: the globe keeps turning behind it, dimmed
    draw_globe(f.buffer_mut(), area, t - EPIGRAPH_END, 0.35);
    let top = area.y + 1;
    let title_h = draw_big_title(f.buffer_mut(), area, top, t - GLOBE_END);
    let mut y = top + title_h + 1;
    let line = |f: &mut Frame, y: u16, text: Line| {
        if y < area.y + area.height {
            f.render_widget(Paragraph::new(text).centered(), Rect { y, height: 1, ..area });
        }
    };
    line(f, y, Line::from(Span::styled("T H E   G O L G O T H A   T R A N S M I S S I O N", Style::default().fg(BONE))));
    y += 2;
    let mut rows = Vec::new();
    match page {
        Page::Main => {
            for (i, item) in items.iter().enumerate() {
                let label = item.label();
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
        Page::Credits => draw_credits(f, area),
    }
    // the dedication, on every title page
    let foot = area.y + area.height.saturating_sub(3);
    line(f, foot, Line::from(Span::styled("Built on GOLGOTHA (1998) by Crack dot Com, given to the world in the public domain.", Style::default().fg(DIM))));
    line(
        f,
        foot + 1,
        Line::from(Span::styled("For Dave Taylor, Jonathan Clark, Carly Taylor and the whole team. Twenty-eight years on, it is finished.", Style::default().fg(DIM))),
    );
    line(f, foot + 2, Line::from(vec![Span::styled("ASCIIVERSE", Style::default().fg(AMBER)), Span::styled("  2026", Style::default().fg(DIM))]));
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

/// Rough continents as ellipses in (latitude, longitude, half-height,
/// half-width) degrees: enough for a globe made of Braille dots.
const LAND: [(f32, f32, f32, f32); 14] = [
    (5.0, 20.0, 33.0, 21.0),
    (50.0, 15.0, 12.0, 22.0),
    (60.0, 60.0, 14.0, 40.0),
    (45.0, 95.0, 18.0, 40.0),
    (25.0, 45.0, 10.0, 11.0),
    (21.0, 78.0, 11.0, 8.0),
    (5.0, 105.0, 8.0, 13.0),
    (-25.0, 134.0, 11.0, 17.0),
    (48.0, -100.0, 20.0, 32.0),
    (15.0, -90.0, 7.0, 9.0),
    (-15.0, -60.0, 23.0, 14.0),
    (72.0, -40.0, 8.0, 14.0),
    (64.0, 15.0, 6.0, 8.0),
    (-82.0, 0.0, 8.0, 180.0),
];

fn land(lat: f32, lon: f32) -> bool {
    LAND.iter().any(|&(la, lo, rh, rw)| {
        let dlon = ((lon - lo + 540.0) % 360.0) - 180.0;
        ((lat - la) / rh).powi(2) + (dlon / rw).powi(2) <= 1.0
    })
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h.wrapping_mul(0x2c1b_3c6d) >> 8
}

/// The spinning globe against the stars, in Braille dots; Golgotha
/// (Jerusalem, 31.8 N 35.2 E) pulses red when it is on the near side.
fn draw_globe(buf: &mut Buffer, area: Rect, t: f32, bright: f32) {
    let (w, h) = (area.width as usize, area.height as usize);
    let (dw, dh) = (w * 2, h * 4);
    // Braille dots are square on screen (2 per column, 4 per row of a 1:2 cell)
    let r = (dh as f32 * 0.42).min(dw as f32 * 0.45);
    let (cx, cy) = (dw as f32 / 2.0, dh as f32 / 2.0 - dh as f32 * 0.04);
    let spin = t * 0.35 - 0.9;
    let pulse = ((t * 4.0).sin() * 0.5 + 0.5) * 0.7 + 0.3;
    let (glat, glon) = (31.8f32.to_radians(), 35.2f32.to_radians());
    let mut bits = vec![0u8; w * h];
    let mut color = vec![Color::Black; w * h];
    let set = |bits: &mut [u8], color: &mut [Color], dx: usize, dy: usize, c: Color| {
        let i = (dy / 4) * w + dx / 2;
        let bit = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]][dx % 2][dy % 4];
        bits[i] |= bit;
        color[i] = c;
    };
    let scale = |c: (f32, f32, f32), k: f32| Color::Rgb((c.0 * k) as u8, (c.1 * k) as u8, (c.2 * k) as u8);
    for dy in 0..dh {
        for dx in 0..dw {
            let x = (dx as f32 - cx) / r;
            let y = (dy as f32 - cy) / r;
            let d2 = x * x + y * y;
            if d2 > 1.0 {
                // stars, twinkling
                let s = hash(dx as u32, dy as u32);
                if s % 173 == 0 && ((t * 2.0 + (s % 7) as f32).sin() > -0.2) {
                    set(&mut bits, &mut color, dx, dy, scale((200.0, 200.0, 190.0), bright));
                }
                continue;
            }
            let z = (1.0 - d2).sqrt();
            let lat = (-y).asin();
            let lon = x.atan2(z) + spin;
            let light = (0.35 + 0.65 * (z * 0.8 - x * 0.4).max(0.0)) * bright;
            let (la, lo) = (lat.to_degrees(), ((lon.to_degrees() + 540.0) % 360.0) - 180.0);
            let grid = (la % 30.0).abs() < 0.9 || (lo % 30.0).abs() < 1.2 / lat.cos().max(0.2);
            // Golgotha
            let (gl, gn) = (glat, glon);
            let dist = (lat.sin() * gl.sin() + lat.cos() * gl.cos() * (lon - gn).cos()).clamp(-1.0, 1.0).acos();
            if dist < 0.035 + 0.02 * pulse {
                set(&mut bits, &mut color, dx, dy, Color::Rgb((255.0 * pulse.max(bright)) as u8, (60.0 * bright) as u8, (40.0 * bright) as u8));
            } else if land(la, lo) {
                if (dx + dy) % 2 == 0 || light > 0.5 {
                    set(&mut bits, &mut color, dx, dy, scale((80.0, 220.0, 120.0), light));
                }
            } else if grid {
                set(&mut bits, &mut color, dx, dy, scale((40.0, 110.0, 70.0), light));
            }
        }
    }
    for row in 0..h {
        for col in 0..w {
            let i = row * w + col;
            if bits[i] == 0 {
                continue;
            }
            if let Some(cell) = buf.cell_mut((area.x + col as u16, area.y + row as u16)) {
                cell.set_char(char::from_u32(0x2800 + bits[i] as u32).unwrap()).set_fg(color[i]).set_bg(Color::Black);
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

fn draw_credits(f: &mut Frame, full: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(" GOLGOTHA, Crack dot Com, 1996-1998", Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" as credited on crack.com", Style::default().fg(DIM))),
        Line::from(""),
    ];
    for (role, names) in CREDITS {
        lines.push(Line::from(vec![Span::styled(format!(" {role:<32}"), Style::default().fg(DIM)), Span::styled(names.to_string(), Style::default().fg(GREEN))]));
    }
    lines.extend([
        Line::from(""),
        Line::from(Span::styled(" Released to the public domain on October 22, 1998:", Style::default().fg(BONE))),
        Line::from(Span::styled(" the source, the graphics, the music and the levels.", Style::default().fg(BONE))),
        Line::from(""),
        Line::from(Span::styled(" CALVARY, Asciiverse, 2026", Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" Their game, their maps, their music, finished in a terminal.", Style::default().fg(GREEN))),
        Line::from(Span::styled(" With thanks to Patrick Grawehr and the Golgotha Forever revival.", Style::default().fg(GREEN))),
    ]);
    let w = 78.min(full.width);
    let h = (lines.len() as u16 + 2).min(full.height);
    let area = Rect { x: full.x + (full.width - w) / 2, y: full.y + (full.height - h) / 2, width: w, height: h };
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(GREEN))
        .title(Span::styled(" CREDITS ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(" any key to go back ", Style::default().fg(DIM)))
        .style(Style::default().bg(Color::Black));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
}
