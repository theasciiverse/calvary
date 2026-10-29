//! Music and sound effects from the original release (public domain), compiled
//! into the binary like the levels. Decoding is rodio's pure-Rust symphonia,
//! so the release's mpg123 decoder (non-profit licence only) never ships.
//!
//! Sound is optional: without an audio device, or with `--mute`, every call
//! here does nothing.

use rodio::{Decoder, MixerDeviceSink, Player, Source};
use std::collections::HashMap;
use std::io::Cursor;
use std::time::{Duration, Instant};

macro_rules! embed {
    ($dir:literal: $($file:literal),* $(,)?) => {
        &[$(($file, include_bytes!(concat!("../data/", $dir, "/", $file)) as &[u8])),*]
    };
}

/// The 15 original tracks, named for the places they were written for.
pub static MUSIC: &[(&str, &[u8])] = embed!("music":
    "Splash_Screen_Opus.mp3",
    "Zurich_Switzerland.mp3",
    "Munich_Germany.mp3",
    "Vienna_Austria.mp3",
    "Turin_Italy.mp3",
    "Rome_Italy.mp3",
    "Naples_Italy.mp3",
    "Greece.mp3",
    "Cairo_Egypt.mp3",
    "Al_Basrah.mp3",
    "Nakhayb_Iraq.mp3",
    "Jerusalem_Israel.mp3",
    "Helsinki_Sweden.mp3",
    "Finland.mp3",
    "Norway_Nephelim_Battle.mp3",
);

/// The effects the game uses (the whole sfx set is in `data/sfx`).
pub static SFX: &[(&str, &[u8])] = embed!("sfx":
    "fire/supertank/main1.mp3",
    "fire/supertank_acid_main_barrel_interrior_22khz.mp3",
    "fire/supertank_plazma_fireball_22khz.mp3",
    "fire/supertank_fires_rocket_22khz.mp3",
    "fire/machine_gun_fire_22khz_lp.wav",
    "fire/peon_tank.mp3",
    "fire/fire_one_22khz.mp3",
    "fire/fire_two.mp3",
    "fire/missile_truck.mp3",
    "fire/supergun.mp3",
    "fire/electric_tower_power_down_22khz.mp3",
    "explosion/generic.mp3",
    "explosion/ground_vehicle.mp3",
    "explosion/acid.mp3",
    "explosion/super_mortar.mp3",
    "explosion/old_generic.mp3",
    "narrative/bank_captured_22khz.mp3",
    "narrative/money_received_22khz.mp3",
    "narrative/enemy_base_destroyed_22khz.mp3",
    "narrative/turret_captured_22khz.mp3",
    "narrative/bank_lost_22khz.mp3",
    "narrative/supertank_lost_22khz.mp3",
    "narrative/mission_failed_22khz.mp3",
    "narrative/turret_lost_22khz.mp3",
    "narrative/building_captured_22khz.mp3",
    "assembled/peon_tank.mp3",
    "assembled/electric_car.mp3",
    "assembled/missile_truck.mp3",
    "assembled/helicopter.mp3",
    "assembled/jet.mp3",
    "assembled/engineering_vehicle.mp3",
    "assembled/kamikaze_trike.mp3",
    "assembled/vehicle_ready_22khz.mp3",
);

fn sfx(name: &str) -> Option<&'static [u8]> {
    SFX.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// Music track for a level, by the place its scenery comes from.
pub fn track_for_level(level: &str) -> &'static str {
    match level {
        "snow" => "Zurich_Switzerland.mp3",
        "egypt2" => "Cairo_Egypt.mp3",
        "test" => "Greece.mp3",
        _ => "Splash_Screen_Opus.mp3",
    }
}

/// Effect for a weapon going off.
pub fn fire_sound(weapon: &str) -> &'static str {
    match weapon {
        "b90mm" => "fire/peon_tank.mp3",
        "super_mortar" => "fire/supergun.mp3",
        "bolt" | "electric_beam" | "beam" | "plasma" => "fire/electric_tower_power_down_22khz.mp3",
        "chain_gun" => "fire/machine_gun_fire_22khz_lp.wav",
        w if w.contains("missile") || w.contains("rocket") => "fire/missile_truck.mp3",
        "b120mm" => "fire/fire_one_22khz.mp3",
        _ => "fire/fire_two.mp3",
    }
}

/// "Vehicle advancing" voice per unit, where the original recorded one.
pub fn ready_voice(kind: &str) -> &'static str {
    match kind {
        "peon_tank" => "assembled/peon_tank.mp3",
        "electric_car" => "assembled/electric_car.mp3",
        "rocket_tank" | "tank_buster" => "assembled/missile_truck.mp3",
        "helicopter" => "assembled/helicopter.mp3",
        "jet" => "assembled/jet.mp3",
        "engineer" => "assembled/engineering_vehicle.mp3",
        "trike" => "assembled/kamikaze_trike.mp3",
        _ => "assembled/vehicle_ready_22khz.mp3",
    }
}

/// How far (map cells) a sound carries from the supertank.
const HEARING: f32 = 28.0;
/// The same effect is not restarted within this time (machine guns, volleys).
const RETRIGGER: Duration = Duration::from_millis(70);

pub struct Audio {
    // the sink must outlive every player on its mixer
    sink: Option<MixerDeviceSink>,
    music: Option<Player>,
    track: usize,
    pub muted: bool,
    last: HashMap<&'static str, Instant>,
}

impl Audio {
    pub fn new(muted: bool) -> Audio {
        let sink = if muted {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        Audio { sink, music: None, track: 0, muted, last: HashMap::new() }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    pub fn track_name(&self) -> &'static str {
        MUSIC[self.track].0.trim_end_matches(".mp3")
    }

    /// Loop music track `name` (from MUSIC), replacing what plays.
    pub fn play_music(&mut self, name: &str) {
        if let Some(i) = MUSIC.iter().position(|(n, _)| *n == name) {
            self.track = i;
            self.start_track();
        }
    }

    /// F9 in the original: the next track.
    pub fn next_track(&mut self) {
        self.track = (self.track + 1) % MUSIC.len();
        self.start_track();
    }

    fn start_track(&mut self) {
        if let Some(old) = self.music.take() {
            old.stop();
        }
        let Some(sink) = &self.sink else { return };
        if self.muted {
            return;
        }
        match Decoder::new_looped(Cursor::new(MUSIC[self.track].1)) {
            Ok(src) => {
                let player = Player::connect_new(sink.mixer());
                player.set_volume(0.45);
                player.append(src);
                self.music = Some(player);
                log::info!("music: {}", MUSIC[self.track].0);
            }
            Err(e) => log::warn!("music {}: {e}", MUSIC[self.track].0),
        }
    }

    /// Silence the music (back in the map editor after a test-play).
    pub fn stop_music(&mut self) {
        if let Some(m) = self.music.take() {
            m.stop();
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            if let Some(m) = self.music.take() {
                m.stop();
            }
        } else {
            self.start_track();
        }
    }

    /// Play an effect at `volume` (0-1), unless the same one just started.
    pub fn play(&mut self, name: &'static str, volume: f32) {
        if self.muted || volume <= 0.02 {
            return;
        }
        let Some(sink) = &self.sink else { return };
        let now = Instant::now();
        if self.last.get(name).is_some_and(|t| now.duration_since(*t) < RETRIGGER) {
            return;
        }
        self.last.insert(name, now);
        let Some(bytes) = sfx(name) else {
            log::warn!("no such sound {name}");
            return;
        };
        match Decoder::new(Cursor::new(bytes)) {
            Ok(src) => sink.mixer().add(src.amplify(volume.min(1.0))),
            Err(e) => log::warn!("sound {name}: {e}"),
        }
    }

    /// An effect at map position (x, y), quieter with distance from (lx, ly).
    pub fn play_at(&mut self, name: &'static str, x: f32, y: f32, lx: f32, ly: f32, volume: f32) {
        let d = (x - lx).hypot(y - ly);
        self.play(name, volume * (1.0 - d / HEARING).clamp(0.0, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded file decodes (no audio device needed).
    #[test]
    fn all_embedded_audio_decodes() {
        for (name, bytes) in MUSIC.iter().chain(SFX.iter()) {
            let mut d = Decoder::new(Cursor::new(*bytes)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(d.next().is_some(), "{name} has no samples");
        }
    }
}
