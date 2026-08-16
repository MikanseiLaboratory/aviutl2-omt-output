use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use aviutl2::generic::EditHandle;
use openmediatransport::ColorSpace;

use crate::bake::{BakeJob, BakedScene};
use crate::config::{MAX_PREVIEW_WIDTH, PluginConfig};
use crate::media::{SessionClock, downscale_bgra_to_rgba, planar_f32_le, samples_per_frame};
use crate::player::{CuePlayer, Transport};
use crate::sender::{AudioJob, DropSender, LatestSlot, SendSession, VideoJob, drop_channel};

#[derive(Debug, Clone)]
pub struct PluginStatus {
    pub running: bool,
    pub port: u16,
    pub connections: u32,
    pub video_subscribed: bool,
    pub audio_subscribed: bool,
    pub send_fps: f32,
    pub encode_us: i64,
    pub video_drops: u64,
    pub audio_drops: u64,
    pub sender_drops: i64,
    pub last_error: Option<String>,
}

impl Default for PluginStatus {
    fn default() -> Self {
        Self {
            running: false,
            port: 0,
            connections: 0,
            video_subscribed: false,
            audio_subscribed: false,
            send_fps: 0.0,
            encode_us: 0,
            video_drops: 0,
            audio_drops: 0,
            sender_drops: 0,
            last_error: None,
        }
    }
}

pub struct SharedState {
    pub config: Mutex<PluginConfig>,
    pub config_epoch: AtomicU64,
    pub status: Mutex<PluginStatus>,
    pub running: AtomicBool,
    pub video_slot: LatestSlot<VideoJob>,
    pub audio_tx: Mutex<Option<DropSender<AudioJob>>>,
    pub audio_drops: Arc<AtomicU64>,
    pub clock: Mutex<SessionClock>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(PluginConfig::default()),
            config_epoch: AtomicU64::new(0),
            status: Mutex::new(PluginStatus::default()),
            running: AtomicBool::new(false),
            video_slot: LatestSlot::new(),
            audio_tx: Mutex::new(None),
            audio_drops: Arc::new(AtomicU64::new(0)),
            clock: Mutex::new(SessionClock::new()),
        }
    }

    pub fn config_snapshot(&self) -> PluginConfig {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .clamped()
    }

    pub fn set_config(&self, config: PluginConfig) {
        *self.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
        self.config_epoch.fetch_add(1, Ordering::Release);
    }

    pub fn store_draft(&self, config: PluginConfig) {
        *self.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
    }

    pub fn status_snapshot(&self) -> PluginStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_error(&self, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!("{message}");
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .last_error = Some(message);
    }

    fn next_timestamp(&self) -> i64 {
        self.clock
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .next_monotonic()
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LiveController {
    shared: Arc<SharedState>,
    player: Arc<CuePlayer>,
    edit: Mutex<Option<Arc<EditHandle>>>,
    session: Mutex<Option<SendSession>>,
    play_stop: Arc<AtomicBool>,
    play_join: Mutex<Option<JoinHandle<()>>>,
    scene_name_dirty: AtomicBool,
}

impl LiveController {
    pub fn new(shared: Arc<SharedState>) -> Self {
        Self {
            shared,
            player: Arc::new(CuePlayer::new()),
            edit: Mutex::new(None),
            session: Mutex::new(None),
            play_stop: Arc::new(AtomicBool::new(true)),
            play_join: Mutex::new(None),
            scene_name_dirty: AtomicBool::new(true),
        }
    }

    pub fn shared(&self) -> &Arc<SharedState> {
        &self.shared
    }

    pub fn player(&self) -> &Arc<CuePlayer> {
        &self.player
    }

    pub fn set_edit_handle(&self, handle: Arc<EditHandle>) {
        *self.edit.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        self.mark_scene_dirty();
        self.refresh_scene_info();
    }

    pub fn mark_scene_dirty(&self) {
        self.scene_name_dirty.store(true, Ordering::Release);
        self.refresh_scene_info();
    }

    pub fn refresh_scene_info(&self) {
        let Some(handle) = self.edit.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return;
        };
        if !handle.is_ready() {
            return;
        }
        let info = handle.get_edit_info();
        let mut scene = self.player.scene_snapshot();
        scene.width = info.width as u32;
        scene.height = info.height as u32;
        scene.fps_n = *info.fps.numer();
        scene.fps_d = (*info.fps.denom()).max(1);
        scene.frame_count = info.frame_max.saturating_add(1) as u32;
        scene.sample_rate = info.sample_rate as i32;
        self.player.set_scene(scene);
    }

    pub fn refresh_scene_name(&self) {
        if !self.scene_name_dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let Some(handle) = self.edit.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return;
        };
        if !handle.is_ready() {
            self.scene_name_dirty.store(true, Ordering::Release);
            return;
        }
        match handle.call_read_section(|section| section.get_scene_name()) {
            Ok(Ok(name)) => self.player.set_scene_name(name),
            Ok(Err(_)) | Err(_) => self.player.set_scene_name("—".into()),
        }
    }

    pub fn start_bake(self: &Arc<Self>) {
        if self.player.transport() == Transport::Baking {
            return;
        }
        self.refresh_scene_info();
        self.scene_name_dirty.store(true, Ordering::Release);
        self.refresh_scene_name();

        let scene = self.player.scene_snapshot();
        if scene.width == 0 || scene.height == 0 || scene.frame_count == 0 {
            self.shared
                .set_error("現在シーンの解像度またはフレーム数が無効です");
            return;
        }
        let Some(handle) = self.edit.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            self.shared
                .set_error("編集ハンドルがまだ準備できていません");
            return;
        };
        if !handle.is_ready() {
            self.shared
                .set_error("編集ハンドルがまだ準備できていません");
            return;
        }

        *self.shared.config.lock().unwrap_or_else(|e| e.into_inner()) =
            self.shared.config_snapshot();
        self.stop_output();
        self.player.begin_bake(scene.frame_count);

        let last_frame = scene.last_frame();
        let job = BakeJob::new(
            handle,
            scene.name,
            scene.width,
            scene.height,
            scene.fps_n,
            scene.fps_d,
            scene.sample_rate,
            last_frame,
            self.player.bake_cancel_flag(),
            self.player.bake_progress_flag(),
        );
        let this = Arc::clone(self);
        job.start(move |result| this.on_bake_done(result));
    }

    fn on_bake_done(self: Arc<Self>, result: Result<BakedScene, String>) {
        if self.player.cancel_requested() {
            self.player.abort_bake();
            self.shared.set_error("描画をキャンセルしました");
            return;
        }
        match result {
            Ok(scene) => {
                if scene.video.is_empty() {
                    self.player.abort_bake();
                    self.shared.set_error("描画結果が空です");
                    return;
                }
                self.player.finish_bake(scene);
                if let Err(e) = self.start_output() {
                    self.player.abort_bake();
                    self.shared.set_error(e);
                }
            }
            Err(e) => {
                self.player.abort_bake();
                self.shared.set_error(e);
            }
        }
    }

    pub fn cancel_bake(&self) {
        if self.player.transport() != Transport::Baking {
            return;
        }
        self.player.request_cancel();
    }

    pub fn cue(&self) {
        if !self.player.cue_enabled() {
            return;
        }
        self.player.cue();
    }

    pub fn stop(&self) {
        self.player.request_cancel();
        self.stop_output();
        self.player.clear();
        self.shared
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .running = false;
    }

    fn start_output(&self) -> Result<(), String> {
        let config = self.shared.config_snapshot();
        self.shared.video_slot.clear();
        self.shared.audio_drops.store(0, Ordering::Relaxed);
        *self.shared.clock.lock().unwrap_or_else(|e| e.into_inner()) = SessionClock::new();

        let (audio_tx, audio_rx) = drop_channel::<AudioJob>(
            config.send_queue_depth,
            Arc::clone(&self.shared.audio_drops),
        );
        *self
            .shared
            .audio_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(audio_tx);

        let session = SendSession::start(Arc::clone(&self.shared), config, audio_rx)?;
        *self.session.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);

        self.shared.running.store(true, Ordering::Release);
        {
            let mut status = self.shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.running = true;
            status.connections = 0;
            status.video_subscribed = false;
            status.audio_subscribed = false;
            status.send_fps = 0.0;
            status.encode_us = 0;
            status.video_drops = 0;
            status.audio_drops = 0;
            status.sender_drops = 0;
            status.last_error = None;
        }

        self.start_playback_thread();
        Ok(())
    }

    fn stop_output(&self) {
        self.play_stop.store(true, Ordering::Release);
        if let Some(join) = self
            .play_join
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = join.join();
        }
        self.shared.running.store(false, Ordering::Release);
        *self
            .shared
            .audio_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        if let Some(mut session) = self
            .session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            session.stop();
        }
        self.shared.video_slot.clear();
        self.shared
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .running = false;
    }

    fn start_playback_thread(&self) {
        self.play_stop.store(false, Ordering::Release);
        let stop = Arc::clone(&self.play_stop);
        let shared = Arc::clone(&self.shared);
        let player = Arc::clone(&self.player);
        let join = thread::Builder::new()
            .name("omt-cue-play".into())
            .spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    playback_loop(shared, player, stop);
                }));
            });
        match join {
            Ok(join) => {
                *self.play_join.lock().unwrap_or_else(|e| e.into_inner()) = Some(join);
            }
            Err(e) => self
                .shared
                .set_error(format!("failed to spawn playback thread: {e}")),
        }
    }
}

impl Drop for LiveController {
    fn drop(&mut self) {
        self.stop();
    }
}

fn playback_loop(shared: Arc<SharedState>, player: Arc<CuePlayer>, stop: Arc<AtomicBool>) {
    let mut last_preview: Option<u32> = None;
    while !stop.load(Ordering::Acquire) {
        let Some(baked) = player.baked() else {
            thread::sleep(Duration::from_millis(8));
            continue;
        };
        let period = frame_period(baked.fps_n, baked.fps_d);
        let tick_start = Instant::now();
        if let Some((index, live_audio)) = player.output_frame() {
            push_output(&shared, &baked, index, live_audio);
            if last_preview != Some(index) {
                update_preview(&player, &baked, index);
                last_preview = Some(index);
            }
        }
        let elapsed = tick_start.elapsed();
        if elapsed < period {
            thread::sleep(period - elapsed);
        }
    }
}

fn frame_period(fps_n: i32, fps_d: i32) -> Duration {
    let n = fps_n.max(1) as u128;
    let d = fps_d.max(1) as u128;
    Duration::from_nanos(((1_000_000_000u128 * d) / n) as u64)
}

fn push_output(shared: &SharedState, baked: &BakedScene, index: u32, live_audio: bool) {
    let config = shared.config_snapshot();
    let idx = index as usize;
    let Some(bgra) = baked.video.get(idx) else {
        return;
    };
    let timestamp = shared.next_timestamp();
    let color_space: ColorSpace = config.color_space.to_omt();
    if config.send_video {
        shared.video_slot.push(VideoJob {
            width: baked.width,
            height: baked.height,
            stride: (baked.width as i32).saturating_mul(4),
            bgra: bgra.clone(),
            has_alpha: baked.has_alpha,
            timestamp,
            fps_n: baked.fps_n,
            fps_d: baked.fps_d,
            color_space,
        });
    }
    if config.send_audio {
        let samples = expected_samples(baked);
        let (left, right) = if live_audio {
            baked
                .audio
                .get(idx)
                .cloned()
                .unwrap_or_else(|| (vec![0.0; samples], vec![0.0; samples]))
        } else {
            (vec![0.0; samples], vec![0.0; samples])
        };
        let left = if left.is_empty() {
            vec![0.0; samples]
        } else {
            left
        };
        let right = if right.is_empty() {
            vec![0.0; samples]
        } else {
            right
        };
        match planar_f32_le(&[&left, &right]) {
            Ok(data) => {
                let job = AudioJob {
                    data,
                    timestamp,
                    sample_rate: baked.sample_rate,
                    channels: 2,
                    samples_per_channel: left.len() as i32,
                };
                if let Some(tx) = shared
                    .audio_tx
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                {
                    let _ = tx.try_send(job);
                }
            }
            Err(e) => shared.set_error(e.to_string()),
        }
    }
}

fn expected_samples(baked: &BakedScene) -> usize {
    baked
        .audio
        .first()
        .map(|(left, right)| left.len().max(right.len()))
        .filter(|n| *n > 0)
        .unwrap_or_else(|| {
            samples_per_frame(baked.sample_rate.max(1) as u32, baked.fps_n, baked.fps_d) as usize
        })
}

fn update_preview(player: &CuePlayer, baked: &BakedScene, index: u32) {
    let Some(bgra) = baked.video.get(index as usize) else {
        return;
    };
    if let Some((width, height, preview)) =
        downscale_bgra_to_rgba(baked.width, baked.height, bgra, MAX_PREVIEW_WIDTH)
    {
        player.set_preview(width, height, preview);
    }
}
