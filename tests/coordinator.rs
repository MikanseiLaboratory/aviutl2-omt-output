use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use aviutl2_omt_live_output::sender::{LatestSlot, drain_audio_priority, drop_channel};

#[test]
fn video_latest_wins_and_audio_priority_on_overflow() {
    let video = LatestSlot::new();
    video.push(1u32);
    video.push(2u32);
    video.push(3u32);
    assert_eq!(video.drops(), 2);
    assert_eq!(video.take(), Some(3));

    let drops = Arc::new(AtomicU64::new(0));
    let (audio_tx, audio_rx) = drop_channel::<&'static str>(1, Arc::clone(&drops));
    assert!(audio_tx.try_send("a1"));
    assert!(!audio_tx.try_send("a2"));
    assert_eq!(drops.load(std::sync::atomic::Ordering::Relaxed), 1);

    let order = std::cell::RefCell::new(Vec::new());
    drain_audio_priority(
        &mut || audio_rx.try_recv().ok(),
        &mut || video.take(),
        |frame| order.borrow_mut().push(format!("audio:{frame}")),
        |frame| order.borrow_mut().push(format!("video:{frame}")),
    );
    assert_eq!(*order.borrow(), ["audio:a1"]);
    assert_eq!(audio_tx.drops(), 1);
}
