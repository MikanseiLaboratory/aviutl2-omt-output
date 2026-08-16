use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::bake::BakedScene;
use crate::media::{Playhead, estimate_bake_bytes, playhead_frame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Idle,
    Baking,
    HoldFirst,
    Playing,
    HoldLast,
}

impl Transport {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Idle => "待機",
            Self::Baking => "描画中",
            Self::HoldFirst => "先頭ホールド",
            Self::Playing => "再生中",
            Self::HoldLast => "最終ホールド",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SceneSummary {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps_n: i32,
    pub fps_d: i32,
    pub frame_count: u32,
    pub sample_rate: i32,
}

impl Default for SceneSummary {
    fn default() -> Self {
        Self {
            name: "—".to_string(),
            width: 0,
            height: 0,
            fps_n: 30,
            fps_d: 1,
            frame_count: 0,
            sample_rate: 48000,
        }
    }
}

impl SceneSummary {
    pub fn fps_label(&self) -> String {
        if self.fps_d <= 1 {
            format!("{}", self.fps_n.max(1))
        } else {
            format!("{:.3}", self.fps_n as f64 / self.fps_d.max(1) as f64)
        }
    }

    pub fn last_frame(&self) -> u32 {
        self.frame_count.saturating_sub(1)
    }

    pub fn estimated_bytes(&self) -> u64 {
        estimate_bake_bytes(
            self.width,
            self.height,
            self.frame_count,
            self.sample_rate.max(0) as u32,
            self.fps_n,
            self.fps_d,
        )
    }
}

#[derive(Debug, Clone)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub generation: u64,
}

pub struct CuePlayer {
    transport: Mutex<Transport>,
    play_origin: Mutex<Option<Instant>>,
    baked: Mutex<Option<Arc<BakedScene>>>,
    bake_current: Arc<AtomicU32>,
    bake_total: AtomicU32,
    cancel_bake: Arc<AtomicBool>,
    scene: Mutex<SceneSummary>,
    preview: Mutex<Option<PreviewFrame>>,
    preview_gen: AtomicU32,
}

impl CuePlayer {
    pub fn new() -> Self {
        Self {
            transport: Mutex::new(Transport::Idle),
            play_origin: Mutex::new(None),
            baked: Mutex::new(None),
            bake_current: Arc::new(AtomicU32::new(0)),
            bake_total: AtomicU32::new(0),
            cancel_bake: Arc::new(AtomicBool::new(false)),
            scene: Mutex::new(SceneSummary::default()),
            preview: Mutex::new(None),
            preview_gen: AtomicU32::new(0),
        }
    }

    pub fn scene_snapshot(&self) -> SceneSummary {
        self.scene.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set_scene(&self, scene: SceneSummary) {
        *self.scene.lock().unwrap_or_else(|e| e.into_inner()) = scene;
    }

    pub fn set_scene_name(&self, name: String) {
        self.scene.lock().unwrap_or_else(|e| e.into_inner()).name = name;
    }

    pub fn transport(&self) -> Transport {
        *self.transport.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn baked(&self) -> Option<Arc<BakedScene>> {
        self.baked.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn bake_cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_bake)
    }

    pub fn bake_progress_flag(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.bake_current)
    }

    pub fn bake_progress(&self) -> (u32, u32) {
        (
            self.bake_current.load(Ordering::Acquire),
            self.bake_total.load(Ordering::Acquire),
        )
    }

    pub fn set_bake_progress(&self, current: u32, total: u32) {
        self.bake_current.store(current, Ordering::Release);
        self.bake_total.store(total, Ordering::Release);
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel_bake.load(Ordering::Acquire)
    }

    pub fn request_cancel(&self) {
        self.cancel_bake.store(true, Ordering::Release);
    }

    pub fn begin_bake(&self, total: u32) {
        self.cancel_bake.store(false, Ordering::Release);
        self.set_bake_progress(0, total);
        *self.play_origin.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.baked.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.preview.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Transport::Baking;
    }

    pub fn abort_bake(&self) {
        *self.baked.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.play_origin.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.preview.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Transport::Idle;
        self.set_bake_progress(0, 0);
    }

    pub fn finish_bake(&self, scene: BakedScene) {
        *self.baked.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(scene));
        *self.play_origin.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Transport::HoldFirst;
    }

    pub fn cue(&self) {
        let Some(baked) = self.baked() else {
            return;
        };
        if baked.frame_count() == 0 {
            return;
        }
        *self.play_origin.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Transport::Playing;
    }

    pub fn clear(&self) {
        self.cancel_bake.store(true, Ordering::Release);
        self.abort_bake();
    }

    pub fn output_frame(&self) -> Option<(u32, bool)> {
        let baked = self.baked()?;
        let last = baked.last_frame();
        let mut transport = self.transport.lock().unwrap_or_else(|e| e.into_inner());
        match *transport {
            Transport::Idle | Transport::Baking => None,
            Transport::HoldFirst => Some((0, false)),
            Transport::HoldLast => Some((last, false)),
            Transport::Playing => {
                let origin = self
                    .play_origin
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .unwrap_or_else(Instant::now);
                match playhead_frame(
                    origin.elapsed().as_secs_f64(),
                    baked.fps_n,
                    baked.fps_d,
                    last,
                ) {
                    Playhead::Frame(index) => Some((index, true)),
                    Playhead::PastEnd => {
                        *transport = Transport::HoldLast;
                        Some((last, false))
                    }
                }
            }
        }
    }

    pub fn cue_enabled(&self) -> bool {
        matches!(
            self.transport(),
            Transport::HoldFirst | Transport::HoldLast | Transport::Playing
        )
    }

    pub fn sending(&self) -> bool {
        matches!(
            self.transport(),
            Transport::HoldFirst | Transport::HoldLast | Transport::Playing
        )
    }

    pub fn current_index(&self) -> usize {
        self.output_frame()
            .map(|(index, _)| index as usize)
            .unwrap_or(0)
    }

    pub fn set_preview(&self, width: u32, height: u32, rgba: Vec<u8>) {
        let generation = u64::from(self.preview_gen.fetch_add(1, Ordering::Relaxed)) + 1;
        *self.preview.lock().unwrap_or_else(|e| e.into_inner()) = Some(PreviewFrame {
            width,
            height,
            rgba,
            generation,
        });
    }

    pub fn preview_snapshot(&self) -> Option<PreviewFrame> {
        self.preview
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Default for CuePlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn play_index_holds_first_then_jumps_to_last() {
        let player = CuePlayer::new();
        player.finish_bake(BakedScene {
            name: "t".into(),
            width: 16,
            height: 16,
            fps_n: 1000,
            fps_d: 1,
            sample_rate: 48000,
            has_alpha: false,
            video: vec![vec![0u8; 16]; 10],
            audio: vec![(vec![0.0], vec![0.0]); 10],
        });
        assert_eq!(player.output_frame(), Some((0, false)));
        assert_eq!(player.transport(), Transport::HoldFirst);
        player.cue();
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(player.output_frame(), Some((9, false)));
        assert_eq!(player.transport(), Transport::HoldLast);
        player.cue();
        assert_eq!(player.output_frame(), Some((0, true)));
    }
}
