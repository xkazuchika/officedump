## 1. IR 構造体の拡張（src/ir.rs）

- [ ] 1.1 `PptxPlaceholder` 構造体を追加する（D2）
- [ ] 1.2 `Geometry` に `rot`/`flip_h`/`flip_v` フィールドを追加する（D1）
- [ ] 1.3 `PptxShape` に `placeholder_detail`/`prst_geom`/`sppr_xml` フィールドを追加する（D2）
- [ ] 1.4 `TextRun` に `sz`/`color`/`rfonts`/`vert_align`/`spacing`/`kern`/`position`/`rpr_xml` フィールドを追加する（D3。docx change と共用。docx change が未適用の場合は pptx 側で追加）
- [ ] 1.5 `PptxParagraph` に `algn`/`lvl`/`bu_char`/`bu_auto_num`/`mar_l`/`indent`/`ln_spc`/`spc_bef`/`spc_aft` フィールドを追加する（D4）
- [ ] 1.6 `PptxTextFrame` に `bodypr_xml` フィールドを追加する（D5）
- [ ] 1.7 `PptxTableCell` に `grid_span`/`row_span`/`h_merge`/`v_merge` フィールドを追加する（D6）
- [ ] 1.8 `PptxTableRow` に `h` フィールドを追加する（D7）

## 2. Geometry パースの拡張（src/pptx.rs）

- [ ] 2.1 `parse_geometry` で `<a:xfrm>` 要素の `rot`/`flipH`/`flipV` 属性を読み取る
- [ ] 2.2 読み取った属性を `Geometry` の `rot`/`flip_h`/`flip_v` に設定する

## 3. 図形プロパティパースの拡張（src/pptx.rs）

- [ ] 3.1 `parse_shape` で `<a:prstGeom>` 要素の `prst` 属性を読み取り `PptxShape.prst_geom` に設定する
- [ ] 3.2 `parse_shape` で `<p:spPr>` 要素の生 XML を `capture_element` で捕捉し `PptxShape.sppr_xml` に設定する
- [ ] 3.3 `parse_shape` で `<p:ph>` 要素の `type`/`idx`/`sz`/`orient` 属性を読み取り `PptxPlaceholder` に構造化して `PptxShape.placeholder_detail` に設定する（D8）

## 4. テキストラン書式パースの拡張（src/pptx.rs）

- [ ] 4.1 `parse_text_frame` で `<a:rPr>` の `sz` 属性を読み取り `TextRun.sz` に設定する
- [ ] 4.2 `parse_text_frame` で `<a:rPr>` 内の `<a:solidFill><a:srgbClr val="...">` から色を読み取り `TextRun.color` に設定する
- [ ] 4.3 `parse_text_frame` で `<a:rPr>` 内の `<a:latin typeface="...">` からフォント名を読み取り `TextRun.rfonts`（`DocxRFonts.ascii`）に設定する
- [ ] 4.4 `parse_text_frame` で `<a:rPr>` の `strike` 属性を読み取り `TextRun.strike` に設定する（現在は常に `false`）

## 5. 段落プロパティパースの拡張（src/pptx.rs）

- [ ] 5.1 `parse_text_frame` で `<a:pPr>` 要素の `algn`/`lvl`/`marL`/`indent` 属性を読み取る
- [ ] 5.2 `parse_text_frame` で `<a:pPr>` 内の `<a:buChar>` の `char` 属性を読み取り `PptxParagraph.bu_char` に設定する
- [ ] 5.3 `parse_text_frame` で `<a:pPr>` 内の `<a:buAutoNum>` の `type` 属性を読み取り `PptxParagraph.bu_auto_num` に設定する
- [ ] 5.4 `parse_text_frame` で `<a:pPr>` 内の `<a:lnSpc>`/`<a:spcBef>`/`<a:spcAft>` 子要素の `val` を読み取り `PptxParagraph.ln_spc`/`spc_bef`/`spc_aft` に設定する

## 6. テキストボディプロパティ・表の拡張（src/pptx.rs）

- [ ] 6.1 `parse_text_frame` で `<a:bodyPr>` 要素の生 XML を `capture_element` で捕捉し `PptxTextFrame.bodypr_xml` に設定する
- [ ] 6.2 `parse_table` で `<a:tr>` 要素の `h` 属性を読み取り `PptxTableRow.h` に設定する
- [ ] 6.3 `parse_table` で `<a:tc>` 要素の `gridSpan`/`rowSpan`/`hMerge`/`vMerge` 属性を読み取り `PptxTableCell` の各フィールドに設定する

## 7. テストの追加（tests/integration.rs）

- [ ] 7.1 回転・反転を持つ図形のフィクスチャを追加し、geometry に保持されることを検証する（Scenario: 回転と反転の保持）
- [ ] 7.2 プリセット図形種別を持つ図形のフィクスチャを追加し、`prst_geom` に保持されることを検証する（Scenario: プリセット図形種別の保持）
- [ ] 7.3 プレースホルダー属性（type/idx/sz）を持つ図形のフィクスチャを追加し、`placeholder_detail` に保持されることを検証する（Scenario: プレースホルダー属性の保持）
- [ ] 7.4 テキストラン書式（sz/color/typeface）を持つフィクスチャを追加し、保持されることを検証する（Scenario: テキストランの書式プロパティ保持）
- [ ] 7.5 段落プロパティ（algn/lvl/buChar/marL/indent）を持つフィクスチャを追加し、保持されることを検証する（Scenario: 段落プロパティの保持）
- [ ] 7.6 テキストボディプロパティを持つフィクスチャを追加し、`bodypr_xml` に保持されることを検証する（Scenario: テキストボディプロパティの保持）
- [ ] 7.7 セル結合（gridSpan/rowSpan）を持つ表のフィクスチャを追加し、保持されることを検証する（Scenario: セル結合を持つ表の分解）
- [ ] 7.8 行の高さを持つ表のフィクスチャを追加し、保持されることを検証する（Scenario: 行の高さの保持）
- [ ] 7.9 既存テストが全て通過することを確認する（後方互換性）

## 8. ドキュメント更新

- [ ] 8.1 README.md の対応状況表に pptx 属性保持の拡充を追記する
- [ ] 8.2 docs/agent-integration.md の pptx セクションに geometry 拡張・図形プロパティ・テキスト書式・段落・表セルの IR 構造を追記する

## 9. 検証

- [ ] 9.1 `cargo build` が通過することを確認する
- [ ] 9.2 `cargo test` が全テスト通過することを確認する
- [ ] 9.3 `cargo clippy` が警告なしであることを確認する
