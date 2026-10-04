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
use ratatui::crossterm::event::{self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
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
    let start = arg.and_then(|a| LEVELS.iter().position(|l| l.name == a)).unwrap_or(0);
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    let result = run(&mut terminal, start, kitty);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, start: usize, kitty: bool) -> io::Result<()> {
    let mut game = Game::new(start).map_err(io::Error::other)?;
    let mut input = Input::new(kitty);
    let tick = Duration::from_secs_f32(DT);
    let mut next = Instant::now() + tick;
    let mut next_snapshot = Instant::now();
    while !input.quit {
        render::follow_camera(&mut game, terminal.size()?.into());
        terminal.draw(|f| render::draw(f, &game))?;
        if event::poll(next.saturating_duration_since(Instant::now()))? {
            loop {
                if let Event::Key(k) = event::read()? {
                    log::debug!("key {:?}", k);
                    input.on_key(k, Instant::now());
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
        if std::mem::take(&mut input.switch_route) && !game.help {
            let (x, y) = (game.tank.x, game.tank.y);
            if game.world.switch_junction(x, y, 12.0).is_none() {
                game.world.say("No junction within 12 cells of the supertank");
            }
        }
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
