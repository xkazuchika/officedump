# Tasks: add-pptx-decomposition

## 1. 共通層と CLI

- [x] 1.1 `OfficeFormat::Pptx` と `ppt/media/` の共通抽出対応を追加する
- [x] 1.2 pptx IR を追加する（Presentation / Slide / Shape / TextFrame / Table、zOrder、geometry、画像アンカー）
- [x] 1.3 `.pptx` の拡張子ディスパッチ、`--slide N:M`、形式専用オプションの usage_error を追加する
- [x] 1.4 file-output-mode の manifest summary に slides/shapes/media を追加する

## 2. pptx 分解コア

- [x] 2.1 `presentation.xml` と rels をパースし、sldIdLst に従うスライド順と1始まりの索引を確定する
- [x] 2.2 slide XML の図形ツリーをパースし、基本図形・画像・graphicFrame に zOrder と x/y/cx/cy の EMU 生値を保持する
- [x] 2.3 テキスト図形を分解する（段落・ラン・書式属性、title/ctrTitle プレースホルダー種別）
- [x] 2.4 graphicFrame の表を分解する（列幅・行・セル・セル内テキスト）
- [x] 2.5 slide rels から画像を解決し、pptx メディアアンカー（slide/zOrder/geometry）を構築する
- [x] 2.6 未処理の図形ツリー要素をスライド単位の escape hatch へ保持する

## 3. 出力と検証

- [x] 3.1 pptx `inspect` を実装する（スライド数、タイトルプレースホルダーの一覧）
- [x] 3.2 pptx `read` を実装する（`--slide N:M` フィルタ、content.json / manifest / --stdout）
- [x] 3.3 pptx フィクスチャを用意する（タイトル、混在書式図形、表、画像、geometry、未知要素、破損ファイル）
- [x] 3.4 specs の全シナリオを統合テストし、xlsx/docx 回帰を確認する
- [x] 3.5 README と agent-integration.md を pptx 対応へ更新し、`cargo fmt --check`、`cargo test`、`cargo clippy -- -D warnings`、`openspec validate add-pptx-decomposition --strict` を実行する
