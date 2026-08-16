use aviutl2_omt_live_output::media::{
    SessionClock, TICKS_PER_SECOND, audio_interval_ticks, planar_f32_le, rgba_to_tight_bgra,
    video_interval_ticks,
};

#[test]
fn padded_pitch_rgba_to_bgra_keeps_alpha() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4 + 8;
    let mut src = vec![0u8; (pitch * height) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let i = y * pitch as usize + x * 4;
            src[i] = 10;
            src[i + 1] = 20;
            src[i + 2] = 30;
            src[i + 3] = if x == 0 && y == 0 { 128 } else { 255 };
        }
    }

    let converted = rgba_to_tight_bgra(width, height, pitch, &src).expect("convert");
    assert_eq!(converted.width, 16);
    assert_eq!(converted.height, 16);
    assert_eq!(converted.stride, 16 * 4);
    assert!(converted.has_alpha);
    assert_eq!(&converted.bgra[0..4], &[30, 20, 10, 128]);
    assert_eq!(&converted.bgra[4..8], &[30, 20, 10, 255]);
}

#[test]
fn opaque_rgba_does_not_set_alpha() {
    let width = 16u32;
    let height = 16u32;
    let pitch = width * 4;
    let src = vec![255u8; (pitch * height) as usize];
    let converted = rgba_to_tight_bgra(width, height, pitch, &src).expect("convert");
    assert!(!converted.has_alpha);
}

#[test]
fn rejects_video_smaller_than_16() {
    let src = vec![0u8; 8 * 8 * 4];
    assert!(rgba_to_tight_bgra(8, 8, 32, &src).is_err());
}

#[test]
fn planar_f32_little_endian_stereo() {
    let left = [1.0f32, -1.0];
    let right = [0.5f32, 0.25];
    let bytes = planar_f32_le(&[&left, &right]).expect("audio");
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
    assert_eq!(&bytes[4..8], &(-1.0f32).to_le_bytes());
    assert_eq!(&bytes[8..12], &0.5f32.to_le_bytes());
    assert_eq!(&bytes[12..16], &0.25f32.to_le_bytes());
}

#[test]
fn timestamps_are_100ns_and_monotonic_across_loop_and_seek() {
    let mut clock = SessionClock::new();
    let first = clock.next_monotonic();
    assert!(first >= 0);
    let second = clock.next_monotonic();
    assert!(second > first);

    let mut samples = vec![first, second];
    for _ in 0..8 {
        samples.push(clock.next_monotonic());
    }
    for pair in samples.windows(2) {
        assert!(pair[1] > pair[0], "timestamp went backwards: {pair:?}");
    }

    assert_eq!(video_interval_ticks(60, 1), TICKS_PER_SECOND / 60);
    assert_eq!(audio_interval_ticks(480, 48_000), TICKS_PER_SECOND / 100);
}

#[test]
fn playhead_jumps_instead_of_catching_up() {
    use aviutl2_omt_live_output::media::{Playhead, playhead_frame};
    assert_eq!(playhead_frame(0.0, 30, 1, 99), Playhead::Frame(0));
    assert_eq!(playhead_frame(1.0, 30, 1, 99), Playhead::Frame(30));
    assert_eq!(playhead_frame(10.0, 30, 1, 99), Playhead::PastEnd);
}
