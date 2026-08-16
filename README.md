# AviUtl2 OMT Live Output

AviUtl2 の今開いているシーンを描画して、OMT で送出するプラグインです。再生が遅れたときは途中を飛ばして、いまの位置から送ります。

## 要件

- Windows x64、AviUtl2 2.1.4 以上
- SDR、8bit、ステレオ音声

## インストール

[Releases](https://github.com/MikanseiLaboratory/aviutl2-omt-output/releases) の `aviutl2-omt-output-v*.au2pkg.zip` を導入します。

- `Plugin/aviutl2_omt_live_output.aux2` → `C:\ProgramData\aviutl2\Plugin`
- `Language/*.aul2` → `C:\ProgramData\aviutl2\Language`

## 使い方

今開いているシーンだけ送れます。送りたいシーンを AviUtl2 で開いてください。

1. 送りたいシーンを開く
2. プラグインのウィンドウでソース名、品質、色空間を設定し、**描画開始** を押す。描画中はそのシーンを編集しない
3. 完了すると **CUE** が点灯し、先頭の映像を送出し始める
4. **CUE** ボタンで送出・再生を開始する（AviUtl2 の再生ボタンは使わない）
5. 再生が終わると最後の映像のまま送出する。CUE をもう一度押すと最初から再生。**停止** で描画データを捨てて送出を終える

受信側では `HOSTNAME (ソース名)` または `omt://IP:ポート` で受けます。ファイアウォールは UDP mDNS と TCP `6400`–`6600` を許可してください。

無圧縮なので、1080p は1フレームあたり約 8 MB 使います。

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
