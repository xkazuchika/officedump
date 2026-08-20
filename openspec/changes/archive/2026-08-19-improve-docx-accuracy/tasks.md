## 1. IR 構造体の拡張（src/ir.rs）

- [x] 1.1 `DocxRFonts` 構造体を追加する（D1）
- [x] 1.2 `DocxInd`・`DocxSpacing` 構造体を追加する（D2）
- [x] 1.3 `DocxTrHeight` 構造体を追加する（D4）
- [x] 1.4 `DocxTcW` 構造体を追加する（D5）
- [x] 1.5 `TextRun` に `sz`/`color`/`rfonts`/`vert_align`/`spacing`/`kern`/`position`/`rpr_xml` フィールドを追加する（D1）
- [x] 1.6 `Block::Paragraph` に `jc`/`ind`/`spacing`/`ppr_xml` フィールドを追加する（D2）
- [x] 1.7 `Block::Table` に `tblpr_xml` フィールドを追加する（D3）
- [x] 1.8 `TableRow` に `tr_height`/`cant_split`/`tbl_header` フィールドを追加する（D4）
- [x] 1.9 `TableCell` に `tcw`/`shd`/`tc_mar`/`v_align`/`no_wrap`/`tc_borders` フィールドを追加する（D5）
- [x] 1.10 `DocxSection` に `sectpr_xml` フィールドを追加する（D6）
- [x] 1.11 `RunNode::Hyperlink` に `history`/`tooltip`/`tgt_frame` フィールドを追加する（D7）
- [x] 1.12 `RunNode::Field` に `fld_lock`/`dirty` フィールドを追加する（D8）

## 2. ラン書式プロパティパースの拡張（src/docx.rs）

- [x] 2.1 `<rPr>` 内の `<sz>` 要素の `val` 属性を読み取り `TextRun.sz` に設定する
- [x] 2.2 `<rPr>` 内の `<color>` 要素の `val` 属性を読み取り `TextRun.color` に設定する
- [x] 2.3 `<rPr>` 内の `<rFonts>` 要素の `ascii`/`hAnsi`/`eastAsia`/`cs` 属性を読み取り `TextRun.rfonts` に設定する
- [x] 2.4 `<rPr>` 内の `<vertAlign>` 要素の `val` 属性を読み取り `TextRun.vert_align` に設定する
- [x] 2.5 `<rPr>` 内の `<spacing>` 要素の `val` 属性を読み取り `TextRun.spacing` に設定する
- [x] 2.6 `<rPr>` 内の `<kern>` 要素の `val` 属性を読み取り `TextRun.kern` に設定する
- [x] 2.7 `<rPr>` 内の `<position>` 要素の `val` 属性を読み取り `TextRun.position` に設定する
- [x] 2.8 `<rPr>` 要素全体の生 XML を `TextRun.rpr_xml` に保持する（未知子要素の escape hatch）

## 3. 段落プロパティパースの拡張（src/docx.rs）

- [x] 3.1 `<pPr>` 内の `<jc>` 要素の `val` 属性を読み取り `Block::Paragraph.jc` に設定する
- [x] 3.2 `<pPr>` 内の `<ind>` 要素の `left`/`right`/`firstLine`/`hanging` 属性を読み取り `DocxInd` に構造化して `Block::Paragraph.ind` に設定する
- [x] 3.3 `<pPr>` 内の `<spacing>` 要素の `before`/`after`/`line`/`lineRule` 属性を読み取り `DocxSpacing` に構造化して `Block::Paragraph.spacing` に設定する
- [x] 3.4 `<pPr>` 要素全体の生 XML を `Block::Paragraph.ppr_xml` に保持する（未知子要素の escape hatch）

## 4. 表プロパティパースの拡張（src/docx.rs）

- [x] 4.1 `<tblPr>` 要素の生 XML を `Block::Table.tblpr_xml` に保持する
- [x] 4.2 `<trPr>` 内の `<trHeight>` 要素の `val`/`hRule` 属性を読み取り `DocxTrHeight` に構造化して `TableRow.tr_height` に設定する
- [x] 4.3 `<trPr>` 内の `<cantSplit>` 要素を `TableRow.cant_split` に、`<tblHeader>` 要素を `TableRow.tbl_header` に設定する
- [x] 4.4 `<tcPr>` 内の `<tcW>` 要素の `w`/`type` 属性を読み取り `DocxTcW` に構造化して `TableCell.tcw` に設定する
- [x] 4.5 `<tcPr>` 内の `<shd>`/`<tcMar>`/`<tcBorders>` 要素の生 XML を `TableCell.shd`/`tc_mar`/`tc_borders` に保持する
- [x] 4.6 `<tcPr>` 内の `<vAlign>` 要素の `val` 属性を `TableCell.v_align` に、`<noWrap>` 要素を `TableCell.no_wrap` に設定する

## 5. セクションプロパティ・ハイパーリンク・フィールドの拡張（src/docx.rs）

- [x] 5.1 `<sectPr>` 要素の生 XML を `DocxSection.sectpr_xml` に保持する（既存の header/footer 参照読み取りは sectpr_xml から行うよう移行）
- [x] 5.2 `<hyperlink>` 要素の `history`/`tooltip`/`tgtFrame` 属性を読み取り `RunNode::Hyperlink` の各フィールドに設定する
- [x] 5.3 `<fldChar>` 要素の `fldLock`/`dirty` 属性を読み取り `RunNode::Field` の各フィールドに設定する

## 6. テストの追加（tests/integration.rs）

- [x] 6.1 ラン書式プロパティ（sz/color/rFonts/vertAlign/spacing）のフィクスチャを追加し、保持されることを検証する（Scenario: ラン書式プロパティの保持）
- [x] 6.2 段落プロパティ（jc/ind/spacing）のフィクスチャを追加し、保持されることを検証する（Scenario: 段落プロパティの保持）
- [x] 6.3 表プロパティ（tblPr）のフィクスチャを追加し、生 XML で保持されることを検証する（Scenario: 表プロパティの保持）
- [x] 6.4 行プロパティ（trHeight/tblHeader）のフィクスチャを追加し、保持されることを検証する（Scenario: 行プロパティの保持）
- [x] 6.5 セルプロパティ（shd/vAlign/tcMar）のフィクスチャを追加し、保持されることを検証する（Scenario: セルプロパティの保持）
- [x] 6.6 セクションプロパティ（pgSz/pgMar）のフィクスチャを追加し、生 XML で保持されることを検証する（Scenario: ページサイズとマージンの保持）
- [x] 6.7 ハイパーリンク属性（tooltip/tgtFrame）のフィクスチャを追加し、保持されることを検証する（Scenario: ハイパーリンクの追加属性保持）
- [x] 6.8 フィールド属性（dirty/fldLock）のフィクスチャを追加し、保持されることを検証する（Scenario: フィールドの属性保持）
- [x] 6.9 既存テストが全て通過することを確認する（後方互換性）

## 7. ドキュメント更新

- [x] 7.1 README.md の対応状況表に docx 属性保持の拡充を追記する
- [x] 7.2 docs/agent-integration.md の docx セクションにラン書式・段落・表・セクションプロパティの IR 構造を追記する

## 8. 検証

- [x] 8.1 `cargo build` が通過することを確認する
- [x] 8.2 `cargo test` が全テスト通過することを確認する
- [x] 8.3 `cargo clippy` が警告なしであることを確認する
