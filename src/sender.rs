use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use openmediatransport::{
    Codec, ColorSpace, Discovery, FrameType, MediaFrame, Sender, SenderConfig, SenderInfo,
    VideoFlags,
};

use crate::config::PluginConfig;
use crate::controller::SharedState;

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub width: u32,
    pub height: u32,
    pub stride: i32,
    pub bgra: Vec<u8>,
    pub has_alpha: bool,
    pub timestamp: i64,
    pub fps_n: i32,
    pub fps_d: i32,
    pub color_space: ColorSpace,
}

#[derive(Debug, Clone)]
pub struct AudioJob {
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub sample_rate: i32,
    pub channels: i32,
    pub samples_per_channel: i32,
}

/// Depth-1 latest-wins slot. Pushing while occupied drops the older value.
#[derive(Debug)]
pub struct LatestSlot<T> {
    slot: Mutex<Option<T>>,
    drops: AtomicU64,
}

impl<T> LatestSlot<T> {
    pub fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            drops: AtomicU64::new(0),
        }
    }

    pub fn push(&self, item: T) {
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        if slot.replace(item).is_some() {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn take(&self) -> Option<T> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn clear(&self) {
        let _ = self.take();
    }

    pub fn drops(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DropSender<T> {
    tx: SyncSender<T>,
    drops: Arc<AtomicU64>,
}

impl<T: Send> DropSender<T> {
    pub fn try_send(&self, item: T) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.drops.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn drops(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

pub fn drop_channel<T: Send>(depth: usize, drops: Arc<AtomicU64>) -> (DropSender<T>, Receiver<T>) {
    drops.store(0, Ordering::Relaxed);
    let depth = depth.max(1);
    let (tx, rx) = sync_channel(depth);
    (DropSender { tx, drops }, rx)
}

/// Drain audio first, then at most one video frame. Used by the worker and tests.
pub fn drain_audio_priority<A, V>(
    audio: &mut dyn FnMut() -> Option<A>,
    video: &mut dyn FnMut() -> Option<V>,
    mut on_audio: impl FnMut(A),
    mut on_video: impl FnMut(V),
) {
    while let Some(frame) = audio() {
        on_audio(frame);
    }
    if let Some(frame) = video() {
        on_video(frame);
    }
}

pub struct SendSession {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl SendSession {
    pub fn start(
        shared: Arc<SharedState>,
        config: PluginConfig,
        audio_rx: Receiver<AudioJob>,
    ) -> Result<Self, String> {
        let mut sender = Sender::create_with_config(
            config.source_name.clone(),
            config.frame_types(),
            SenderConfig {
                send_queue_depth: config.send_queue_depth,
                ..SenderConfig::default()
            },
        )
        .map_err(|e| format!("OMT sender create failed: {e}"))?;

        sender.set_quality(config.quality.to_omt());
        sender.set_sender_info(SenderInfo::new(
            "AviUtl2 OMT Live Output",
            crate::config::PLUGIN_AUTHOR_JA,
            env!("CARGO_PKG_VERSION"),
        ));

        let port = sender.port();
        {
            let mut status = shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.port = port;
            status.last_error = None;
        }

        let mut discovery = Discovery::new().ok();
        if let Some(discovery) = discovery.as_mut() {
            if let Err(e) = discovery.register(&config.source_name, port) {
                shared.set_error(format!("DNS-SD register failed: {e}"));
            }
        } else {
            shared.set_error("DNS-SD discovery is unavailable; direct omt:// URLs still work");
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let shared_thread = Arc::clone(&shared);
        let source_name = config.source_name.clone();

        let join = thread::Builder::new()
            .name("omt-live-sender".into())
            .spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    sender_loop(
                        sender,
                        discovery,
                        &source_name,
                        shared_thread,
                        audio_rx,
                        stop_thread,
                    );
                }));
            })
            .map_err(|e| format!("failed to spawn OMT sender thread: {e}"))?;

        Ok(Self {
            stop,
            join: Some(join),
        })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for SendSession {
    fn drop(&mut self) {
        self.stop();
    }
}

fn sender_loop(
    mut sender: Sender,
    mut discovery: Option<Discovery>,
    source_name: &str,
    shared: Arc<SharedState>,
    audio_rx: Receiver<AudioJob>,
    stop: Arc<AtomicBool>,
) {
    let mut connections: u32 = 0;
    let mut last_sample = Instant::now();
    let mut last_frames: i64 = 0;
    let mut last_codec: i64 = 0;

    while !stop.load(Ordering::Acquire) {
        match sender.poll_accept() {
            Ok(true) => connections = connections.saturating_add(1),
            Ok(false) => {}
            Err(e) => shared.set_error(format!("poll_accept: {e}")),
        }
        if let Err(e) = sender.poll_peer_metadata() {
            shared.set_error(format!("poll_peer_metadata: {e}"));
        }

        let video_sub = sender.video_subscribed();
        let audio_sub = sender.audio_subscribed();
        {
            let mut status = shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.connections = connections;
            status.video_subscribed = video_sub;
            status.audio_subscribed = audio_sub;
        }
        while let Ok(job) = audio_rx.try_recv() {
            if let Err(e) = sender.send_audio(audio_frame(job)) {
                shared.set_error(format!("send_audio: {e}"));
            }
        }
        if let Some(job) = shared.video_slot.take()
            && let Err(e) = sender.send_video(video_frame(job))
        {
            shared.set_error(format!("send_video: {e}"));
        }

        let stats = sender.statistics();
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(last_sample).as_secs_f32();
        let mut send_fps = 0.0;
        let mut encode_us = 0i64;
        if elapsed >= 0.5 {
            let frame_delta = (stats.frames - last_frames).max(0) as f32;
            send_fps = if elapsed > 0.0 {
                frame_delta / elapsed
            } else {
                0.0
            };
            let codec_delta = (stats.codec_time - last_codec).max(0);
            encode_us = if frame_delta > 0.0 {
                (codec_delta as f32 / frame_delta) as i64
            } else {
                0
            };
            last_sample = now;
            last_frames = stats.frames;
            last_codec = stats.codec_time;
        }

        {
            let mut status = shared.status.lock().unwrap_or_else(|e| e.into_inner());
            status.connections = connections;
            status.video_subscribed = video_sub;
            status.audio_subscribed = audio_sub;
            status.port = sender.port();
            if elapsed >= 0.5 {
                status.send_fps = send_fps;
                status.encode_us = encode_us;
            }
            status.video_drops = shared.video_slot.drops();
            status.audio_drops = shared.audio_drops.load(Ordering::Relaxed);
            status.sender_drops = stats.frames_dropped;
        }

        thread::sleep(Duration::from_millis(1));
    }

    if let Some(discovery) = discovery.as_mut() {
        let _ = discovery.deregister(source_name);
    }
    let _ = sender.send_metadata(0, "<OMTMetadata />");
}

fn video_frame(job: VideoJob) -> MediaFrame {
    let flags = if job.has_alpha {
        VideoFlags::ALPHA
    } else {
        VideoFlags::NONE
    };
    MediaFrame {
        frame_type: FrameType::VIDEO,
        timestamp: job.timestamp,
        codec: Codec::Bgra as i32,
        width: job.width as i32,
        height: job.height as i32,
        stride: job.stride,
        flags,
        frame_rate_n: job.fps_n,
        frame_rate_d: job.fps_d,
        aspect_ratio: if job.height == 0 {
            1.0
        } else {
            job.width as f32 / job.height as f32
        },
        color_space: job.color_space,
        data: job.bgra,
        ..Default::default()
    }
}

fn audio_frame(job: AudioJob) -> MediaFrame {
    MediaFrame {
        frame_type: FrameType::AUDIO,
        timestamp: job.timestamp,
        codec: Codec::Fpa1 as i32,
        sample_rate: job.sample_rate,
        channels: job.channels,
        samples_per_channel: job.samples_per_channel,
        active_channels: 0,
        data: job.data,
        ..Default::default()
    }
}
