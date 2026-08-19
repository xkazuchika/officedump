## 1. IR 構造体の拡張（src/ir.rs）

- [x] 1.1 `XlsxRichRun`・`XlsxRunProps` 構造体を追加する（D1）
- [x] 1.2 `SharedString` 構造体を追加する（D6）
- [x] 1.3 `FormulaMeta` 構造体を追加し、`Cell` に `formulaMeta: Option<FormulaMeta>` と `runs: Option<Vec<XlsxRichRun>>` フィールドを追加する（D1, D2）
- [x] 1.4 `RowInfo` 構造体を追加し、`SheetDump` に `rows: Option<Vec<RowInfo>>` フィールドを追加する（D3）
- [x] 1.5 `WorkbookStyles`・`XfDef`・`AlignmentDef`・`ProtectionDef`・`CellStyleDef`・`NumFmtDef` 構造体を追加し、`ReadOutput` に `styles: Option<WorkbookStyles>` フィールドを追加する（D4）

## 2. 共有文字列パースの拡張（src/xlsx.rs）

- [x] 2.1 `parse_shared_strings` の戻り値を `Vec<String>` から `Vec<SharedString>` に変更する（D6）
- [x] 2.2 `<si>` 内の `<r>` ランを検出し、各ランの `<rPr>` 属性を `XlsxRunProps` に構造化して `SharedString.runs` に格納する
- [x] 2.3 `<rPh>` 振り仮名要素は従来どおり除外し、`runs` に含めない
- [x] 2.4 `<r>` ランを持たない `<si>`（`<t>` のみ）は `runs: None` とし、`text` のみを保持する

## 3. インライン文字列パースの拡張（src/xlsx.rs）

- [x] 3.1 `CellDraft` に `inline_runs: Option<Vec<XlsxRichRun>>` フィールドを追加する（D7）
- [x] 3.2 `parse_worksheet` で `<is>` 内の `<r>` ランを検出し、`<rPr>` とテキストを構造化して `inline_runs` に格納する
- [x] 3.3 `<is><t>` のみの場合は従来どおり `inline_text` を使い、`inline_runs` は `None` とする
- [x] 3.4 `finalize_cell` で `inline_runs` を `Cell.runs` に設定する

## 4. 数式属性パースの拡張（src/xlsx.rs）

- [x] 4.1 `CellDraft` に `formula_meta: Option<FormulaMeta>` フィールドを追加する
- [x] 4.2 `parse_worksheet` で `<f>` Start イベントの属性（`t`/`ref`/`si`/`aca`/`bx`/`ca`）を読み取り `formula_meta` に格納する
- [x] 4.3 `finalize_cell` で `formula_meta` を `Cell.formulaMeta` に設定する

## 5. 行属性パースの拡張（src/xlsx.rs）

- [x] 5.1 `parse_worksheet` で `<row>` Start イベントの属性（`spans`/`s`/`customFormat`/`ht`/`customHeight`/`hidden`/`outlineLevel`/`collapsed`/`thickTop`/`thickBottom`）を読み取る
- [x] 5.2 属性を持つ行を `RowInfo` に格納し、`SheetParse` に `rows: Vec<RowInfo>` フィールドを追加して蓄積する
- [x] 5.3 属性を持たない行は `rows` に含めない（出力サイズの膨張防止）
- [x] 5.4 `SheetDump.rows` に `SheetParse.rows` を反映する（`Vec` が空の場合は `None` にする）

## 6. スタイルパースの拡張（src/xlsx.rs）

- [x] 6.1 `Styles` 構造体に `fonts: Vec<RawElement>`・`fills: Vec<RawElement>`・`borders: Vec<RawElement>`・`cell_style_xfs: Vec<XfDef>`・`cell_styles: Vec<CellStyleDef>`・`num_fmts: Vec<NumFmtDef>` フィールドを追加する
- [x] 6.2 `parse_styles` で `<numFmt>` の `formatCode16` 属性を優先的に読み取る（D5）。`formatCode16` が存在する場合はそれを `formatCode` として採用する
- [x] 6.3 `parse_styles` で `<fonts>`/`<fills>`/`<borders>` の各子要素を `capture_element` で生 XML として捕捉する
- [x] 6.4 `parse_styles` で `<cellStyleXfs>` と `<cellXfs>` の各 `<xf>` 要素の全属性と `<alignment>`/`<protection>` 子要素を構造化して `XfDef` に格納する
- [x] 6.5 `parse_styles` で `<cellStyles>` の各 `<cellStyle>` 要素の属性を `CellStyleDef` に格納する

## 7. main.rs の統合（src/main.rs）

- [x] 7.1 `read_xlsx_json` で `parse_shared_strings` の戻り値型変更に対応する（`Vec<SharedString>` への参照）
- [x] 7.2 `finalize_cell` で共有文字列の `runs` を `Cell.runs` に設定する
- [x] 7.3 `read_xlsx_json` で `Styles` から `WorkbookStyles` IR を構築し、`ReadOutput.styles` に設定する
- [x] 7.4 `styles.xml` が存在しない場合は `ReadOutput.styles` を `None` とする

## 8. テストの追加（tests/integration.rs）

- [x] 8.1 共有文字列リッチテキストのフィクスチャを追加し、ランごとの書式とテキストが保持されることを検証する（Scenario: 共有文字列のリッチテキスト保持）
- [x] 8.2 インライン文字列リッチテキストのフィクスチャを追加し、ランごとの書式とテキストが保持されることを検証する（Scenario: インライン文字列のリッチテキスト保持）
- [x] 8.3 配列数式のフィクスチャを追加し、`t="array"` と `ref` が保持されることを検証する（Scenario: 配列数式のメタデータ保持）
- [x] 8.4 共有数式のフィクスチャを追加し、`t="shared"` と `si` が保持されることを検証する（Scenario: 共有数式のメタデータ保持）
- [x] 8.5 非表示行・アウトラインレベルのフィクスチャを追加し、行属性が保持されることを検証する（Scenario: 非表示行の保持、アウトラインレベルの保持）
- [x] 8.6 フォント・配置・名前付きスタイルを含むフィクスチャを追加し、スタイル情報が保持されることを検証する（Scenario: フォント・配置を含むスタイルの保持、名前付きスタイルの保持、セルのスタイルインデックス解決）
- [x] 8.7 `formatCode16` を持つフィクスチャを追加し、`formatCode` より優先されることを検証する（Scenario: formatCode16 の優先）
- [x] 8.8 既存テストが全て通過することを確認する（後方互換性）

## 9. ドキュメント更新

- [x] 9.1 README.md の対応状況表に xlsx 属性保持の拡充を追記する
- [x] 9.2 docs/agent-integration.md の xlsx セクションにリッチテキストラン・数式メタデータ・行属性・スタイル定義の IR 構造を追記する

## 10. 検証

- [x] 10.1 `cargo build` が通過することを確認する
- [x] 10.2 `cargo test` が全テスト通過することを確認する
- [x] 10.3 `cargo clippy` が警告なしであることを確認する
