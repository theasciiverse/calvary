mod audio;
mod balance;
mod game;
mod input;
mod level;
mod render;
mod sprites;
mod tank;
mod terrain;
mod world;

use game::{Game, DT, LEVELS};
use input::Input;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::DefaultTerminal;
use std::io;
use std::time::{Duration, Instant};

const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);

fn main() -> io::Result<()> {
    // Session logging: RUST_LOG=debug calvary 2>calvary.log
    env_logger::init();
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--info") {
        print_info();
        return Ok(());
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let start = args.iter().find_map(|a| LEVELS.iter().position(|l| l.name == *a)).unwrap_or(0);
    // developer aid: `--at X,Y` starts the supertank at a map position
    let at = args.iter().position(|a| a == "--at").and_then(|i| args.get(i + 1)).and_then(|v| {
        let (x, y) = v.split_once(',')?;
        Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?))
    });
    // open audio before the alternate screen, so any ALSA chatter lands above it
    let mut audio = audio::Audio::new(args.iter().any(|a| a == "--mute"));
    log::info!("audio available {} muted {}", audio.available(), audio.muted);
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    // clicks switch routes at junctions, like the original's strategy mode
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal, &mut audio, start, at, kitty);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, audio: &mut audio::Audio, start: usize, at: Option<(f32, f32)>, kitty: bool) -> io::Result<()> {
    let mut game = Game::new(start).map_err(io::Error::other)?;
    audio.play_music(audio::track_for_level(LEVELS[start].name));
    if let Some((x, y)) = at {
        game.tank.x = x;
        game.tank.y = y;
    }
    let mut input = Input::new(kitty);
    input.open_pad();
    let tick = Duration::from_secs_f32(DT);
    let mut next = Instant::now() + tick;
    let mut next_snapshot = Instant::now();
    while !input.quit {
        render::follow_camera(&mut game, terminal.size()?.into());
        input.poll_pad();
        game.pad = input.pad_name.clone().map(|n| (n, input.build_cursor));
        terminal.draw(|f| render::draw(f, &game))?;
        if event::poll(next.saturating_duration_since(Instant::now()))? {
            loop {
                match event::read()? {
                    Event::Key(k) => {
                        log::debug!("key {:?}", k);
                        input.on_key(k, Instant::now());
                    }
                    Event::Mouse(m) if !game.help => {
                        let size: ratatui::layout::Rect = terminal.size()?.into();
                        on_mouse(&mut game, size, m);
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        if std::mem::take(&mut input.toggle_help) {
            game.help = !game.help;
            log::info!("help {}", game.help);
        }
        if std::mem::take(&mut input.escape) {
            if game.help {
                game.help = false;
                log::info!("help false");
            } else {
                input.quit = true;
            }
        }
        if std::mem::take(&mut input.toggle_lock) && !game.help {
            game.toggle_lock();
        }
        if std::mem::take(&mut input.switch_route) && !game.help {
            let (x, y) = (game.tank.x, game.tank.y);
            if game.world.switch_junction(x, y, 12.0).is_none() {
                game.world.say("No junction within 12 cells of the supertank");
            }
        }
        if let Some(k) = input.unit_key.take() {
            if !game.help {
                match k {
                    'u' => game.select_near_tank(12.0),
                    ',' => game.cycle_selection(-1),
                    '.' => game.cycle_selection(1),
                    'g' => game.rally(),
                    'x' => game.command(world::Command::Hold),
                    'r' => game.command(world::Command::Resume),
                    'z' => {
                        game.selected.clear();
                        game.world.say("Selection cleared");
                    }
                    'p' => {
                        game.select_near_tank(12.0);
                        if !game.selected.is_empty() {
                            game.rally();
                        }
                    }
                    _ => {}
                }
            }
        }
        game.prune_selection();
        if let Some(slot) = input.build.take() {
            if !game.help {
                game.order(slot);
            }
        }
        if input.zoom_step != 0 {
            game.zoom_by(std::mem::take(&mut input.zoom_step));
        }
        if std::mem::take(&mut input.toggle_view) {
            game.toggle_view();
        }
        if input.level_step != 0 {
            let n = LEVELS.len() as i32;
            let i = (game.level_index as i32 + std::mem::take(&mut input.level_step)).rem_euclid(n) as usize;
            let (view, help, zoom) = (game.view, game.help, game.zoom);
            game = Game::new(i).map_err(io::Error::other)?;
            audio.play_music(audio::track_for_level(LEVELS[i].name));
            game.view = view;
            game.help = help;
            game.zoom = zoom;
        }
        let now = Instant::now();
        let mut steps = 0;
        while now >= next && steps < 5 {
            let controls = input.controls(now);
            if !game.help {
                game.update(&controls);
            }
            next += tick;
            steps += 1;
        }
        if steps == 5 {
            next = now + tick;
        }
        if std::mem::take(&mut input.next_track) {
            audio.next_track();
            game.world.say(format!("Music: {}", audio.track_name().replace('_', " ")));
        }
        if std::mem::take(&mut input.toggle_sound) {
            audio.toggle_mute();
            game.world.say(if audio.muted { "Sound off" } else { "Sound on" });
        }
        let (lx, ly) = (game.tank.x, game.tank.y);
        for s in std::mem::take(&mut game.world.sounds) {
            match s.at {
                Some((x, y)) => audio.play_at(s.name, x, y, lx, ly, s.volume),
                None => audio.play(s.name, s.volume),
            }
        }
        if now >= next_snapshot {
            let t = &game.tank;
            log::debug!(
                "snapshot view={:?} pos=({:.2},{:.2}) hdg={:.2} turret={:.2} gear={} speed={:.2} blocked={} shots={} particles={}",
                game.view, t.x, t.y, t.heading, t.turret, t.gear_label(), t.speed, t.blocked, game.shots.len(), game.particles.len()
            );
            next_snapshot = now + SNAPSHOT_EVERY;
        }
    }
    Ok(())
}

/// Mouse: left click selects a unit (or switches a junction, or clears the
/// selection), left drag box-selects, right click sends the selection to a
/// spot or at an enemy.
fn on_mouse(game: &mut Game, size: ratatui::layout::Rect, m: event::MouseEvent) {
    let here = (m.column, m.row);
    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => game.drag = Some((here, here)),
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((start, _)) = game.drag {
                game.drag = Some((start, here));
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            let Some((start, _)) = game.drag.take() else { return };
            let moved = start.0.abs_diff(here.0) >= 2 || start.1.abs_diff(here.1) >= 1;
            if moved {
                let a = render::screen_to_map(game, size, start.0, start.1);
                let b = render::screen_to_map(game, size, here.0, here.1);
                if let (Some(a), Some(b)) = (a, b) {
                    let units = game.world.player_units_in((a.0, a.1), (b.0, b.1));
                    if units.is_empty() {
                        game.selected.clear();
                    }
                    game.select(units);
                }
                return;
            }
            let Some((x, y, reach)) = render::screen_to_map(game, size, here.0, here.1) else { return };
            log::debug!("click at ({x:.1},{y:.1})");
            if let Some(u) = game.world.player_unit_at(x, y, reach.max(0.8)) {
                game.select(vec![u]);
            } else if game.world.switch_junction(x, y, reach).is_none() && !game.selected.is_empty() {
                game.selected.clear();
                game.world.say("Selection cleared");
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            let Some((x, y, reach)) = render::screen_to_map(game, size, here.0, here.1) else { return };
            match game.world.enemy_at(x, y, reach.max(0.6)) {
                Some(t) => game.command(world::Command::Attack(t)),
                None => game.command(world::Command::Move(x, y)),
            }
        }
        _ => {}
    }
}

/// `calvary --info`: what the level reader sees in each bundled level.
fn print_info() {
    for src in LEVELS {
        match level::Level::parse(src.data) {
            Ok(l) => {
                let (lo, hi) = (l.vert_heights.iter().min().unwrap(), l.vert_heights.iter().max().unwrap());
                let mut used: Vec<u16> = l.cell_tiles.clone();
                used.sort_unstable();
                used.dedup();
                println!(
                    "{}: {}x{} cells, height {}..{}, {} tile types ({} used), {} object types",
                    src.name,
                    l.width,
                    l.height,
                    lo,
                    hi,
                    l.tiles.iter().filter(|t| !t.name.is_empty() || t.checksum != 0).count(),
                    used.len(),
                    l.object_types.len()
                );
            }
            Err(e) => println!("{}: {e}", src.name),
        }
    }
}
