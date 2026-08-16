# AviUtl2 OMT Live Output

AviUtl2 の現在シーンをメモリへ描画し、OMT でキュー送出します。遅れた場合は溜めずに最新位置へ進みます。

## 要件

- Windows x64、AviUtl2 2.1.4 以上
- SDR / 8-bit RGBA / ステレオ f32

## インストール

[Releases](https://github.com/MikanseiLaboratory/aviutl2-omt-output/releases) の `aviutl2-omt-output-v*.au2pkg.zip` を導入します。

- `Plugin/aviutl2_omt_live_output.aux2` → `C:\ProgramData\aviutl2\Plugin`
- `Language/*.aul2` → `C:\ProgramData\aviutl2\Language`

## 使い方

SDK にシーン一覧やタブ切替はないため、描けるのは今開いているシーンだけです。

1. AviUtl2 側で出したいシーンを開く
2. ドッキングウィンドウでソース名、品質、色空間を設定し、**描画開始** を押す。ベイク中はそのシーンを編集しない
3. 完了すると **CUE** が点灯し、先頭フレーム固定で OMT 送出が始まる
4. **CUE** で壁時計のシーン fps に従って再生する（AviUtl2 の再生ボタンは使わない）
5. 終了後は最終フレームでホールド。CUE 再押しで先頭から再生。**停止** でバッファ破棄

受信側では `HOSTNAME (ソース名)` または `omt://IP:ポート` を購読します。ファイアウォールは UDP mDNS と TCP `6400`–`6600` を許可してください。

無圧縮のため 1080p は約 8 MB/frame です。

## 開発

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --test-threads=1
cargo build --release --locked --target x86_64-pc-windows-msvc
./scripts/package.ps1
```

## ライセンス

MIT。第三者通知は `THIRD_PARTY_NOTICES.md`。
