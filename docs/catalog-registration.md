# AviUtl2 Catalog 登録

安定版 GitHub Release を公開し、導入・更新・削除を確認してから [`aviutl2-catalog-data`](https://github.com/Neosku/aviutl2-catalog-data) へ登録します。入力原案は `catalog/` です。

| 項目 | 値 |
| --- | --- |
| ID | `MikanseiLaboratory.aviutl2-omt-output` |
| 種類 | 汎用プラグイン |
| 作者 | 未完成成果物研究所 |
| ライセンス | MIT |
| リポジトリ | https://github.com/MikanseiLaboratory/aviutl2-omt-output |
| 概要 | 現在シーンをベイクし CUE で OMT 送出するプラグイン |
| 詳細 | `catalog/md/MikanseiLaboratory.aviutl2-omt-output.md` |
| サムネイル | `catalog/image/MikanseiLaboratory.aviutl2-omt-output_thumbnail.png` |

インストーラーは GitHub Releases（`MikanseiLaboratory/aviutl2-omt-output`、`^aviutl2-omt-output-v.*\.au2pkg\.zip$`）。install は download → extract → `Plugin` を `{dataDir}/Plugin` へ、`Language` を `{dataDir}/Language` へ copy。uninstall は配置した `.aux2` と付属データを削除。バージョン検出は `Plugin/aviutl2_omt_live_output.aux2` の XXH3-128。
