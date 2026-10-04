mod audio;
mod balance;
mod chase;
mod cmap;
mod editor;
mod game;
mod input;
mod level;
mod render;
mod sprites;
mod tank;
mod terrain;
mod title;
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
    // `--export NAME OUT.cmap`: write a bundled level in Calvary's own format
    if let Some(i) = args.iter().position(|a| a == "--export") {
        let (Some(name), Some(out)) = (args.get(i + 1), args.get(i + 2)) else {
            eprintln!("usage: calvary --export LEVEL OUT.cmap   (levels: {})", LEVELS.iter().map(|l| l.name).collect::<Vec<_>>().join(", "));
            std::process::exit(2);
        };
        let src = LEVELS.iter().find(|l| l.name == name).ok_or_else(|| io::Error::other(format!("no bundled level {name}")))?;
        let level = level::Level::parse(src.data).map_err(io::Error::other)?;
        let text = cmap::export(&level, src.name, Some(audio::track_for_level(src.name)), terrain::tile_name_for).map_err(io::Error::other)?;
        std::fs::write(out, text)?;
        println!("{name} -> {out}");
        return Ok(());
    }
    // `--edit MAP.cmap` or `--new NAME WxH TILE [PATH]`: the map editor
    let edit = if let Some(i) = args.iter().position(|a| a == "--edit") {
        let path = args.get(i + 1).ok_or_else(|| io::Error::other("usage: calvary --edit MAP.cmap"))?;
        let (level, name, music) = cmap::parse(&std::fs::read_to_string(path)?).map_err(|e| io::Error::other(format!("{path}: {e}")))?;
        Some(editor::Editor::new(path, level, name, music))
    } else if let Some(i) = args.iter().position(|a| a == "--new") {
        let usage = || io::Error::other("usage: calvary --new NAME WxH TILE [PATH]   e.g. --new delphi 120x120 grassfull");
        let name = args.get(i + 1).ok_or_else(usage)?;
        let (w, h) = args.get(i + 2).and_then(|d| d.split_once('x')).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))).ok_or_else(usage)?;
        let tile = args.get(i + 3).ok_or_else(usage)?;
        let path = args.get(i + 4).cloned().unwrap_or_else(|| format!("maps/{name}.cmap"));
        if std::path::Path::new(&path).exists() {
            return Err(io::Error::other(format!("{path} exists; open it with --edit")));
        }
        Some(editor::Editor::new(&path, editor::Editor::blank(w, h, tile), name.clone(), None))
    } else {
        None
    };
    if let Some(ed) = edit {
        let mut terminal = ratatui::init();
        execute!(io::stdout(), EnableMouseCapture)?;
        let result = editor::run(&mut terminal, ed);
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
        return result;
    }
    // a .cmap path plays that map
    let map = match args.iter().find(|a| a.ends_with(".cmap")) {
        Some(path) => {
            let text = std::fs::read_to_string(path)?;
            Some(cmap::parse(&text).map_err(|e| io::Error::other(format!("{path}: {e}")))?)
        }
        None => None,
    };
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
    let mut input = Input::new(kitty);
    input.open_pad();
    // a level, map or position on the command line starts that battle
    // directly; otherwise the title, and back to it after each battle
    let direct = map.is_some() || at.is_some() || args.iter().any(|a| LEVELS.iter().any(|l| l.name == *a));
    let result = if direct {
        run(&mut terminal, &mut audio, &mut input, start, map, at)
    } else {
        let mut first = !args.iter().any(|a| a == "--no-intro");
        loop {
            let choice = title::run(&mut terminal, &mut audio, &mut input, !first);
            first = false;
            let r = match choice {
                Ok(title::Choice::Level(i)) => run(&mut terminal, &mut audio, &mut input, i, None, None),
                Ok(title::Choice::Map(path)) => match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| cmap::parse(&t)) {
                    Ok(m) => run(&mut terminal, &mut audio, &mut input, 0, Some(m), None),
                    Err(e) => Err(io::Error::other(format!("{path}: {e}"))),
                },
                Ok(title::Choice::Quit) => break Ok(()),
                Err(e) => break Err(e),
            };
            if r.is_err() || input.exit_app {
                break r;
            }
            input.quit = false;
            input.escape = false;
        }
    };
    let _ = execute!(io::stdout(), DisableMouseCapture);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(
    terminal: &mut DefaultTerminal,
    audio: &mut audio::Audio,
    input: &mut Input,
    start: usize,
    map: Option<(level::Level, String, Option<String>)>,
    at: Option<(f32, f32)>,
) -> io::Result<()> {
    let mut game = match &map {
        Some((level, name, music)) => {
            audio.play_music(music.as_deref().unwrap_or(audio::track_for_level(name)));
            Game::from_level(name, level)
        }
        None => {
            audio.play_music(audio::track_for_level(LEVELS[start].name));
            Game::new(start).map_err(io::Error::other)?
        }
    };
    if let Some((x, y)) = at {
        game.tank.x = x;
        game.tank.y = y;
    }
    let tick = Duration::from_secs_f32(DT);
    let mut next = Instant::now() + tick;
    let mut next_snapshot = Instant::now();
    while !input.quit {
        render::follow_camera(&mut game, terminal.size()?.into());
        input.poll_pad();
        input.menu_moves = 0;
        mouse_aim(&mut game, input, terminal.size()?.into());
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
                        on_mouse(&mut game, input, size, m);
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
        // with the help window open, the pad's A or B closes it (Start toggles it)
        if game.help && input.pad_name.is_some() && (input.build.is_some() || input.switch_route) {
            input.build = None;
            input.switch_route = false;
            game.help = false;
            log::info!("help false (pad)");
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
        if std::mem::take(&mut input.cycle_view) {
            game.cycle_view();
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

/// Mouse, after the original's action mode (manual 3.3): the turret follows
/// the pointer in the tactical view; left button fires the main gun, unless
/// it is on one of your units (select) or a junction (switch); left drag
/// box-selects; right button commands the selection (go there, or attack
/// what is under it), or fires the machine gun when nothing is selected.
/// On the strategy map the mouse only selects and commands.
fn on_mouse(game: &mut Game, input: &mut Input, size: ratatui::layout::Rect, m: event::MouseEvent) {
    let here = (m.column, m.row);
    let tactical = game.view == game::View::Tactical;
    input.mouse_at = Some(here);
    match m.kind {
        MouseEventKind::Moved => {
            if tactical {
                input.mouse_aiming = true;
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            game.drag = Some((here, here));
            if !tactical {
                return;
            }
            input.mouse_aiming = true;
            let Some((x, y, _)) = render::screen_to_map(game, size, here.0, here.1) else { return };
            if let Some(u) = game.world.player_unit_at(x, y, 0.8) {
                game.select(vec![u]);
            } else if game.world.switch_junction(x, y, 0.8).is_none() {
                input.mouse_main = true;
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((start, _)) = game.drag {
                game.drag = Some((start, here));
                // a drag is a box selection, not a burst of fire
                if start.0.abs_diff(here.0) >= 2 || start.1.abs_diff(here.1) >= 1 {
                    input.mouse_main = false;
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            input.mouse_main = false;
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
            if tactical {
                return;
            }
            // strategy map: a click selects, switches a junction or clears
            let Some((x, y, reach)) = render::screen_to_map(game, size, here.0, here.1) else { return };
            if let Some(u) = game.world.player_unit_at(x, y, reach.max(0.8)) {
                game.select(vec![u]);
            } else if game.world.switch_junction(x, y, reach).is_none() && !game.selected.is_empty() {
                game.selected.clear();
                game.world.say("Selection cleared");
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            let Some((x, y, reach)) = render::screen_to_map(game, size, here.0, here.1) else { return };
            if game.selected.is_empty() {
                if tactical {
                    input.mouse_chain = true;
                }
                return;
            }
            match game.world.enemy_at(x, y, reach.max(0.6)) {
                Some(t) => game.command(world::Command::Attack(t)),
                None => game.command(world::Command::Move(x, y)),
            }
        }
        MouseEventKind::Up(MouseButton::Right) => input.mouse_chain = false,
        // the wheel zooms the tactical view
        MouseEventKind::ScrollUp => game.zoom_by(1),
        MouseEventKind::ScrollDown => game.zoom_by(-1),
        _ => {}
    }
}

/// Turn the pointer into a turret aim (angle and distance from the tank) and
/// a crosshair, while the mouse is aiming in the tactical view.
fn mouse_aim(game: &mut Game, input: &mut Input, size: ratatui::layout::Rect) {
    game.crosshair = None;
    input.mouse_aim = None;
    if !input.mouse_aiming || game.view != game::View::Tactical || game.help {
        return;
    }
    let Some(pos) = input.mouse_at else { return };
    let Some((x, y, _)) = render::screen_to_map(game, size, pos.0, pos.1) else { return };
    let (dx, dy) = (x - game.tank.x, y - game.tank.y);
    if dx.hypot(dy) < 0.6 {
        return;
    }
    input.mouse_aim = Some((dy.atan2(dx), dx.hypot(dy)));
    game.crosshair = Some(pos);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyModifiers, MouseEvent};
    use ratatui::layout::Rect;

    fn mouse(kind: MouseEventKind, at: (u16, u16)) -> MouseEvent {
        MouseEvent { kind, column: at.0, row: at.1, modifiers: KeyModifiers::NONE }
    }

    #[test]
    fn mouse_aims_fires_and_selects() {
        let size = Rect::new(0, 0, 160, 45);
        let mut game = Game::new(0).unwrap();
        game.help = false;
        let mut input = Input::new(false);
        render::follow_camera(&mut game, size);
        let (map, _) = render::split(size);
        // a point 6 cells east of the tank
        let (tx, ty) = (game.tank.x, game.tank.y);
        let east = render::map_to_screen(&game, map, tx + 6.0, ty).unwrap();

        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Moved, east));
        mouse_aim(&mut game, &mut input, size);
        let (angle, range) = input.mouse_aim.expect("aiming");
        assert!(angle.abs() < 0.2, "aim east, got {angle}");
        assert!((range - 6.0).abs() < 1.0, "range {range}");
        let c = input.controls(std::time::Instant::now());
        assert!(c.aim.is_some() && c.aim_range.is_some());

        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Down(MouseButton::Left), east));
        assert!(input.controls(std::time::Instant::now()).fire_main, "left button fires on open ground");
        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Up(MouseButton::Left), east));
        assert!(!input.controls(std::time::Instant::now()).fire_main);

        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Down(MouseButton::Right), east));
        assert!(input.controls(std::time::Instant::now()).fire_chain, "right button: machine gun with nothing selected");
        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Up(MouseButton::Right), east));

        // a unit of ours under the pointer is selected, not shot
        game.world.order(world::Team::Player, "peon_tank");
        game.world.step(1.0 / 30.0);
        let u = game.world.entities.iter().rposition(|e| e.kind == "peon_tank").unwrap();
        game.world.entities[u].x = tx + 6.0;
        game.world.entities[u].y = ty;
        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::Down(MouseButton::Left), east));
        assert!(!input.controls(std::time::Instant::now()).fire_main);
        assert_eq!(game.selected, vec![u]);

        // the wheel zooms
        let z = game.zoom;
        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::ScrollUp, east));
        assert_eq!(game.zoom, z + 1);
        on_mouse(&mut game, &mut input, size, mouse(MouseEventKind::ScrollDown, east));
        assert_eq!(game.zoom, z);
    }
}
