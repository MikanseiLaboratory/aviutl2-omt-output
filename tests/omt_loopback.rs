//! OMT sender/receiver loopback for BGRA video and stereo audio.

use std::thread;
use std::time::Duration;

use aviutl2_omt_live_output::media::{planar_f32_le, rgba_to_tight_bgra};
use openmediatransport::{
    Codec, FrameType, MediaFrame, ReceiverConfig, ReceiverSession, Sender, VideoFlags,
};

fn wait_for_subscribe(sender: &mut Sender, video: bool, audio: bool) {
    for _ in 0..80 {
        let _ = sender.poll_accept();
        let _ = sender.poll_peer_metadata();
        if (!video || sender.video_subscribed()) && (!audio || sender.audio_subscribed()) {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    sender.force_subscribe(video, audio, true);
}

#[test]
fn bgra_stereo_loopback_timestamps_and_reconnect() {
    let mut sender = Sender::create(
        "aviutl2-omt-loopback",
        FrameType::VIDEO | FrameType::AUDIO | FrameType::METADATA,
    )
    .expect("sender");
    let port = sender.port();
    let url = format!("omt://127.0.0.1:{port}");

    let session = ReceiverSession::connect(
        url.clone(),
        ReceiverConfig {
            frame_types: FrameType::VIDEO | FrameType::AUDIO | FrameType::METADATA,
            connect_timeout: Duration::from_secs(5),
            auto_reconnect: false,
            ..ReceiverConfig::default()
        },
    )
    .expect("connect");

    wait_for_subscribe(&mut sender, true, true);
    assert!(sender.video_subscribed());
    assert!(sender.audio_subscribed());

    let width = 32u32;
    let height = 32u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = 10;
        pixel[1] = 20;
        pixel[2] = 30;
        pixel[3] = 255;
    }
    let converted = rgba_to_tight_bgra(width, height, width * 4, &rgba).expect("convert");
    let video_ts = 1_000_000i64;
    sender
        .send_video(MediaFrame {
            frame_type: FrameType::VIDEO,
            timestamp: video_ts,
            codec: Codec::Bgra as i32,
            width: converted.width as i32,
            height: converted.height as i32,
            stride: converted.stride,
            flags: if converted.has_alpha {
                VideoFlags::ALPHA
            } else {
                VideoFlags::NONE
            },
            frame_rate_n: 60,
            frame_rate_d: 1,
            color_space: openmediatransport::ColorSpace::Bt709,
            data: converted.bgra,
            ..Default::default()
        })
        .expect("send_video");

    let left = vec![0.25f32; 480];
    let right = vec![-0.5f32; 480];
    let pcm = planar_f32_le(&[&left, &right]).expect("audio");
    let audio_ts = 1_000_100i64;
    sender
        .send_audio(MediaFrame {
            frame_type: FrameType::AUDIO,
            timestamp: audio_ts,
            codec: Codec::Fpa1 as i32,
            sample_rate: 48_000,
            channels: 2,
            samples_per_channel: 480,
            active_channels: 0,
            data: pcm,
            ..Default::default()
        })
        .expect("send_audio");

    let video = session
        .recv_video_timeout(Duration::from_secs(3))
        .expect("decoded video");
    assert_eq!(video.width, 32);
    assert_eq!(video.height, 32);
    assert_eq!(video.timestamp, video_ts);
    assert_eq!(video.pixels[3], 255);

    let mut audio = None;
    for _ in 0..80 {
        let _ = sender.poll_accept();
        let _ = sender.poll_peer_metadata();
        if let Some(frame) = session.try_recv_audio() {
            audio = Some(frame);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let audio = audio.expect("decoded audio");
    assert_eq!(audio.channels, 2);
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.timestamp, audio_ts);

    session.disconnect();

    let session2 = ReceiverSession::connect(
        url,
        ReceiverConfig {
            frame_types: FrameType::VIDEO | FrameType::AUDIO,
            connect_timeout: Duration::from_secs(5),
            auto_reconnect: false,
            ..ReceiverConfig::default()
        },
    )
    .expect("reconnect");
    wait_for_subscribe(&mut sender, true, false);
    assert!(sender.video_subscribed());

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[1, 2, 3, 255]);
    }
    let converted = rgba_to_tight_bgra(width, height, width * 4, &rgba).expect("convert");
    sender
        .send_video(MediaFrame {
            frame_type: FrameType::VIDEO,
            timestamp: 2_000_000,
            codec: Codec::Bgra as i32,
            width: converted.width as i32,
            height: converted.height as i32,
            stride: converted.stride,
            frame_rate_n: 60,
            frame_rate_d: 1,
            data: converted.bgra,
            ..Default::default()
        })
        .expect("send after reconnect");
    let video = session2
        .recv_video_timeout(Duration::from_secs(3))
        .expect("video after reconnect");
    assert_eq!(video.timestamp, 2_000_000);
    session2.disconnect();
}

#[test]
fn mid_subscribe_receives_later_frames() {
    let mut sender = Sender::create(
        "aviutl2-omt-mid-subscribe",
        FrameType::VIDEO | FrameType::METADATA,
    )
    .expect("sender");
    let port = sender.port();

    for _ in 0..4 {
        let _ = sender.poll_accept();
        let _ = sender.send_video(solid_frame(16, 0));
        thread::sleep(Duration::from_millis(10));
    }

    let session = ReceiverSession::connect(
        format!("omt://127.0.0.1:{port}"),
        ReceiverConfig {
            frame_types: FrameType::VIDEO,
            connect_timeout: Duration::from_secs(5),
            auto_reconnect: false,
            ..ReceiverConfig::default()
        },
    )
    .expect("late connect");
    wait_for_subscribe(&mut sender, true, false);

    sender
        .send_video(solid_frame(16, 9_000_000))
        .expect("send after subscribe");
    let video = session
        .recv_video_timeout(Duration::from_secs(3))
        .expect("frame after mid-subscribe");
    assert_eq!(video.timestamp, 9_000_000);
    session.disconnect();
}

fn solid_frame(size: u32, timestamp: i64) -> MediaFrame {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[40, 50, 60, 255]);
    }
    let converted = rgba_to_tight_bgra(size, size, size * 4, &rgba).expect("convert");
    MediaFrame {
        frame_type: FrameType::VIDEO,
        timestamp,
        codec: Codec::Bgra as i32,
        width: converted.width as i32,
        height: converted.height as i32,
        stride: converted.stride,
        frame_rate_n: 30,
        frame_rate_d: 1,
        data: converted.bgra,
        ..Default::default()
    }
}
