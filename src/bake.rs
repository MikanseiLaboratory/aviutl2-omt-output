use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use aviutl2::generic::EditHandle;

use crate::media::rgba_to_tight_bgra;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BakeStep {
    Video(u32),
    Audio(u32),
}

#[derive(Debug, Clone)]
pub struct BakedScene {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps_n: i32,
    pub fps_d: i32,
    pub sample_rate: i32,
    pub has_alpha: bool,
    pub video: Vec<Vec<u8>>,
    pub audio: Vec<(Vec<f32>, Vec<f32>)>,
}

impl BakedScene {
    pub fn last_frame(&self) -> u32 {
        self.video.len().saturating_sub(1) as u32
    }

    pub fn frame_count(&self) -> u32 {
        self.video.len() as u32
    }
}

pub struct BakeJob {
    handle: Arc<EditHandle>,
    last_frame: u32,
    step: BakeStep,
    scene: BakedScene,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU32>,
}

type OnDone = Box<dyn FnOnce(Result<BakedScene, String>) + Send>;

struct BakeCont {
    job: Option<BakeJob>,
    on_done: Option<OnDone>,
}

impl BakeCont {
    fn fail(&mut self, message: String) {
        if let Some(on_done) = self.on_done.take() {
            on_done(Err(message));
        }
    }
}

impl Drop for BakeCont {
    fn drop(&mut self) {
        if let Some(on_done) = self.on_done.take() {
            on_done(Err("描画に失敗しました".into()));
        }
    }
}

impl BakeJob {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        handle: Arc<EditHandle>,
        name: String,
        width: u32,
        height: u32,
        fps_n: i32,
        fps_d: i32,
        sample_rate: i32,
        last_frame: u32,
        cancel: Arc<AtomicBool>,
        progress: Arc<AtomicU32>,
    ) -> Self {
        let frames = last_frame.saturating_add(1) as usize;
        Self {
            handle,
            last_frame,
            step: BakeStep::Video(0),
            scene: BakedScene {
                name,
                width,
                height,
                fps_n,
                fps_d,
                sample_rate,
                has_alpha: false,
                video: Vec::with_capacity(frames),
                audio: Vec::with_capacity(frames),
            },
            cancel,
            progress,
        }
    }

    pub fn start(self, on_done: impl FnOnce(Result<BakedScene, String>) + Send + 'static) {
        request_step(self, Box::new(on_done));
    }
}

fn request_step(job: BakeJob, on_done: OnDone) {
    if job.cancel.load(Ordering::Acquire) {
        on_done(Err("描画をキャンセルしました".into()));
        return;
    }
    let mut cont = BakeCont {
        job: Some(job),
        on_done: Some(on_done),
    };
    let step = cont.job.as_ref().expect("job").step;
    let handle = Arc::clone(&cont.job.as_ref().expect("job").handle);
    match step {
        BakeStep::Video(frame) => {
            let _ = handle.rendering_scene_video(frame, move |video| {
                let mut job = cont.job.take().expect("job");
                if job.cancel.load(Ordering::Acquire) {
                    cont.fail("描画をキャンセルしました".into());
                    return;
                }
                match rgba_to_tight_bgra(video.width, video.height, video.pitch, video.buffer) {
                    Ok(converted) => {
                        if job.scene.video.is_empty() {
                            job.scene.width = converted.width;
                            job.scene.height = converted.height;
                        }
                        job.scene.has_alpha = job.scene.has_alpha || converted.has_alpha;
                        job.scene.video.push(converted.bgra);
                        job.step = BakeStep::Audio(frame);
                        let on_done = cont.on_done.take().expect("on_done");
                        request_step(job, on_done);
                    }
                    Err(e) => cont.fail(e.to_string()),
                }
            });
        }
        BakeStep::Audio(frame) => {
            let _ = handle.rendering_scene_audio(frame, move |audio| {
                let mut job = cont.job.take().expect("job");
                if job.cancel.load(Ordering::Acquire) {
                    cont.fail("描画をキャンセルしました".into());
                    return;
                }
                let mut left = audio.buffer0.to_vec();
                let mut right = audio.buffer1.to_vec();
                if right.is_empty() && !left.is_empty() {
                    right.clone_from(&left);
                }
                if left.is_empty() && !right.is_empty() {
                    left.clone_from(&right);
                }
                job.scene.audio.push((left, right));
                job.progress
                    .store(frame.saturating_add(1), Ordering::Release);
                let on_done = cont.on_done.take().expect("on_done");
                if frame < job.last_frame {
                    job.step = BakeStep::Video(frame.saturating_add(1));
                    request_step(job, on_done);
                } else if job.scene.video.len() != job.scene.audio.len() {
                    on_done(Err("映像と音声のフレーム数が一致しません".into()));
                } else {
                    on_done(Ok(job.scene));
                }
            });
        }
    }
}
