use std::{fs::File, io::Read, io::BufReader, time::{Instant, Duration}, sync::Arc};
use rodio::{OutputStream, OutputStreamHandle, Sink, Decoder};
use rodio::Source;
use std::io::Cursor;

fn load_bytes(path: &str) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn load_bytes_any(paths: &[&str]) -> Option<Vec<u8>> {
    for p in paths {
        if let Some(b) = load_bytes(p) { return Some(b); }
    }
    None
}

pub struct AudioManager {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    bg_sink: Option<Sink>,
    sfx_sink: Sink,
    foot_sink: Sink,
    orb: Option<Arc<Vec<u8>>>,
    enemy_seen: Option<Arc<Vec<u8>>>,
    player_alert: Option<Arc<Vec<u8>>>,
    player_caught: Option<Arc<Vec<u8>>>,
    player_step: Option<Arc<Vec<u8>>>,
    enemy_step: Option<Arc<Vec<u8>>>,
    seen_loop_sink: Option<Sink>,
    player_alert_loop_sink: Option<Sink>,
    last_player_step: Instant,
    last_enemy_step: Instant,
    step_interval_player_walk: Duration,
    step_interval_player_sprint: Duration,
    step_interval_enemy: Duration,
    orb_volume: f32,
}

impl AudioManager {
    pub fn new() -> Option<Self> {
        let (_stream, handle) = OutputStream::try_default().ok()?;
        let sfx_sink = Sink::try_new(&handle).ok()?;
        let foot_sink = Sink::try_new(&handle).ok()?;
        Some(Self {
            _stream,
            handle,
            bg_sink: None,
            sfx_sink,
            foot_sink,
            orb: None,
            enemy_seen: None,
            player_alert: None,
            player_caught: None,
            player_step: None,
            enemy_step: None,
            seen_loop_sink: None,
            player_alert_loop_sink: None,
            last_player_step: Instant::now(),
            last_enemy_step: Instant::now(),
            step_interval_player_walk: Duration::from_millis(260),
            step_interval_player_sprint: Duration::from_millis(170),
            step_interval_enemy: Duration::from_millis(320),
            orb_volume: 0.65,
        })
    }

    pub fn load_sfx(&mut self, orb: &str, enemy_seen: &str, player_step: &str, enemy_step: &str) {
        self.orb = load_bytes(orb).map(Arc::new);
        self.enemy_seen = load_bytes(enemy_seen).map(Arc::new);
        self.player_step = load_bytes(player_step).map(Arc::new);
        self.enemy_step = load_bytes(enemy_step).map(Arc::new);
    }

    pub fn load_sfx_auto(&mut self) {
        self.orb = load_bytes_any(&[
            "assets/sfx_orb.wav",
            "assets/sounds/orb.wav",
            "assets/sounds/puffle.wav",
            "assets/sounds/key.wav",
        ]).map(Arc::new);
        self.enemy_seen = load_bytes_any(&[
            "assets/sfx_enemy_seen.wav",
            "assets/sounds/enemy_alert.wav",
            "assets/sounds/enemy_seen.wav",
            "assets/sounds/alert.wav",
        ]).map(Arc::new);
        self.player_alert = load_bytes_any(&[
            "assets/sfx_player_alert.wav",
            "assets/sounds/player_alert.wav",
            "assets/sounds/alert_player.wav",
        ]).map(Arc::new);
        self.player_step = load_bytes_any(&[
            "assets/sfx_player_step.wav",
            "assets/sounds/foot.wav",
            "assets/sounds/step.wav",
            "assets/sounds/footstep.wav",
        ]).map(Arc::new);
        self.enemy_step = load_bytes_any(&[
            "assets/sfx_enemy_step.wav",
            "assets/sounds/enemy_foot.wav",
            "assets/sounds/enemy_step.wav",
        ]).map(Arc::new);
        self.player_caught = load_bytes_any(&[
            "assets/sfx_player_caught.wav",
            "assets/sounds/caught.wav",
            "assets/sounds/caught.mp3",
        ]).map(Arc::new);
    }

    pub fn play_orb(&self) {
        // Play on its own sink so multiple pickups in the same frame all trigger immediately
        if let Some(d) = self.orb.clone() {
            if let Ok(dec) = Decoder::new(BufReader::new(Cursor::new(d.as_ref().clone()))) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(dec.amplify(self.orb_volume.clamp(0.0, 2.5)));
                    sink.detach(); // let it play independently even if we drop our handle to it
                }
            }
        }
    }
    pub fn play_enemy_seen(&self) { self.play_data(self.enemy_seen.clone()); }
    pub fn play_player_step(&mut self, sprinting: bool) {
        let interval = if sprinting { self.step_interval_player_sprint } else { self.step_interval_player_walk };
        if self.last_player_step.elapsed() >= interval {
            self.last_player_step = Instant::now();
            self.play_data_on_foot(self.player_step.clone());
        }
    }
    pub fn force_player_step(&mut self) {
        self.play_data_on_foot(self.player_step.clone());
        self.last_player_step = Instant::now();
    }
    pub fn stop_player_steps(&mut self) {
        // Immediately cut any queued/playing footstep audio
        self.foot_sink.stop();
        if let Ok(new_sink) = Sink::try_new(&self.handle) {
            self.foot_sink = new_sink;
        }
    }
    pub fn play_enemy_step(&mut self) {
        if self.last_enemy_step.elapsed() >= self.step_interval_enemy {
            self.last_enemy_step = Instant::now();
            self.play_data(self.enemy_step.clone());
        }
    }

    /// Enemy step with volume scaling (1.0 at base, louder when closer)
    pub fn play_enemy_step_with_volume(&mut self, volume: f32) {
        if self.last_enemy_step.elapsed() >= self.step_interval_enemy {
            self.last_enemy_step = Instant::now();
            self.play_data_with_volume(self.enemy_step.clone(), volume);
        }
    }

    fn play_data(&self, data: Option<Arc<Vec<u8>>>) {
        if let Some(d) = data {
            if let Ok(dec) = Decoder::new(BufReader::new(Cursor::new(d.as_ref().clone()))) {
                self.sfx_sink.append(dec);
            }
        }
    }

    fn play_data_with_volume(&self, data: Option<Arc<Vec<u8>>>, vol: f32) {
        if let Some(d) = data {
            if let Ok(dec) = Decoder::new(BufReader::new(Cursor::new(d.as_ref().clone()))) {
                let v = vol.clamp(0.0, 2.5);
                self.sfx_sink.append(dec.amplify(v));
            }
        }
    }

    pub fn play_player_alert(&self) {
        // Play player alert quieter
        self.play_data_with_volume(self.player_alert.clone(), 0.55);
    }

    fn play_data_on_foot(&self, data: Option<Arc<Vec<u8>>>) {
        if let Some(d) = data {
            if let Ok(dec) = Decoder::new(BufReader::new(Cursor::new(d.as_ref().clone()))) {
                self.foot_sink.append(dec);
            }
        }
    }

    pub fn play_music_loop(&mut self, path: &str) {
        if self.bg_sink.is_some() { return; }
        if let Some(bytes) = load_bytes(path) {
            if let Ok(dec) = Decoder::new_looped(Cursor::new(bytes)) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(dec);
                    sink.set_volume(0.35);
                    self.bg_sink = Some(sink);
                }
            }
        }
    }

    pub fn play_music_loop_auto(&mut self) {
        if self.bg_sink.is_some() { return; }
        let candidates = [
            "assets/music_bg.wav",
            "assets/sounds/music.wav",
            "assets/sounds/taylor.wav",
            "assets/sounds/bg.wav",
            "assets/sounds/loop.ogg",
        ];
        if let Some(bytes) = load_bytes_any(&candidates) {
            if let Ok(dec) = Decoder::new_looped(Cursor::new(bytes)) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(dec);
                    sink.set_volume(0.35);
                    self.bg_sink = Some(sink);
                }
            }
        }
    }

    pub fn update(&self) { /* sinks auto-play */ }

    pub fn play_player_caught(&self) {
        self.play_data(self.player_caught.clone());
    }

    // ===== Looped alerts while seen =====
    pub fn start_enemy_seen_loop(&mut self) {
        if self.seen_loop_sink.is_some() { return; }
        if let Some(bytes) = self.enemy_seen.clone() {
            if let Ok(dec) = Decoder::new_looped(Cursor::new(bytes.as_ref().clone())) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(dec);
                    sink.set_volume(0.85);
                    self.seen_loop_sink = Some(sink);
                }
            }
        }
    }
    pub fn stop_enemy_seen_loop(&mut self) {
        if let Some(s) = self.seen_loop_sink.take() { s.stop(); }
    }

    pub fn start_player_alert_loop(&mut self, volume: f32) {
        if self.player_alert_loop_sink.is_some() { return; }
        if let Some(bytes) = self.player_alert.clone() {
            if let Ok(dec) = Decoder::new_looped(Cursor::new(bytes.as_ref().clone())) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.append(dec);
                    sink.set_volume(volume.clamp(0.0, 1.5));
                    self.player_alert_loop_sink = Some(sink);
                }
            }
        }
    }
    pub fn stop_player_alert_loop(&mut self) {
        if let Some(s) = self.player_alert_loop_sink.take() { s.stop(); }
    }
}
