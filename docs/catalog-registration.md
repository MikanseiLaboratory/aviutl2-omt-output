# AviUtl2 Catalog 登録

公開中のカタログアプリ（v0.3.3）の「JSON入力」は、[template.json](https://github.com/Neosku/aviutl2-catalog-data/blob/main/template.json) と同じ **配列** です。先頭は `[`、中身はパッケージ1件のオブジェクトです。単体の `{ ... }` を貼ると「入力形式が不正です」になります。

貼るファイルは `catalog/package.json` です。

| 項目 | 値 |
| --- | --- |
| ID | `MikanseiLaboratory.OMTPlugin` |
| パッケージ名 | OMTライブ送出 |
| 種類 | 出力プラグイン |
| 作者 | 未完成成果物研究所 |
| ライセンス | MIT |
| リポジトリ | https://github.com/MikanseiLaboratory/aviutl2-omt-output |
| 概要 | AviUtl2にOMTの映像送出機能を追加します。 |
| 詳細 | `https://raw.githubusercontent.com/MikanseiLaboratory/aviutl2-omt-output/refs/heads/main/README.md` |
| サムネイル | `catalog/image/MikanseiLaboratory.OMTPlugin_thumbnail.png`（登録画面で添付。JSON の images は空） |

貼り付け後に登録画面で行うこと:

1. サムネイル画像を添付する
2. バージョン欄で `aviutl2_omt_live_output.aux2` を選び、XXH3-128 を計算する（JSON の `0000…` は仮値）
3. GitHub Release の `.au2pkg.zip` を公開してからインストーラーテストする

インストーラーは GitHub Releases（`MikanseiLaboratory/aviutl2-omt-output`、`^aviutl2-omt-output-v.*\.au2pkg\.zip$`）。zip ルートは `package.ini` + `Plugin/` + `Language/` です。install は download → extract → `{tmp}/Plugin` を `{pluginsDir}` へ、`{tmp}/Language` を `{dataDir}/Language` へ copy（フォルダ指定は中身をコピー）。
