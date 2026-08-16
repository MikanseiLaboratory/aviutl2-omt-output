# 検証

CI: `cargo fmt --check`、`clippy -D warnings`、`cargo test --locked -- --test-threads=1`、リリースビルド。

## 自動テスト

- `tests/media.rs` — RGBA→BGRA、pitch、アルファ、planar f32、timestamp の単調性
- `tests/coordinator.rs` — 最新フレーム優先、音声優先、overflow
- `tests/omt_loopback.rs` — BGRA / ステレオ、timestamp、途中購読、再接続
- `src/player.rs` — 先頭ホールド、壁時計再生、最終フレーム固定、CUE 再押し

## 実機

AviUtl2 2.1.4+ で、出したいシーンを開いてから **描画開始 → CUE** を確認する。先頭ホールド、再生、最終ホールド、CUE 再押し、キャンセル、送出停止、映像/音声オン/オフ、受信者なし/途中接続を見る。

OBS / vMix などで 1080p59.94・48 kHz stereo を 10 分以上送り、同期・drop・encode 時間・CPU を見る。
