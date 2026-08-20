## Why

officedump の設計原則「属性は全保持する」に照らし、pptx 実装の情報欠落を修正する。現在 `Geometry` は x/y/cx/cy のみで、回転（`rot`）・反転（`flipH`/`flipV`）が未保持。`PptxShape` は shape_type が固定文字列（"shape"/"picture"/"table"）で、実際の `prstGeom prst` 値（rect/roundRect/ellipse 等）が未保持。塗りつぶし・線・効果等の `spPr` 子要素が未保持。テキストランは bold/italic/underline/strike のみで、サイズ・色・フォント等が未保持。段落プロパティ（配置・箇条書き・間隔等）とテキストボディプロパティ（方向・配置・余白等）が未保持。表セルの結合（gridSpan/rowSpan/hMerge/vMerge）と行の高さが未保持。プレースホルダーの `idx`/`sz`/`orient` が未保持。

## What Changes

**MVP（今回やる）**:

- **Geometry の拡充**: `Geometry` に `rot`（回転）、`flipH`（水平反転）、`flipV`（垂直反転）を追加する
- **図形プロパティの拡充**: `PptxShape` に `prstGeom`（プリセット図形種別: rect/roundRect/ellipse 等）、`sppr_xml`（`spPr` の生 XML: 塗り・線・効果等）を追加する
- **テキストラン書式の拡充**: `TextRun`（pptx 共用）に `sz`（サイズ）、`color`（色）、`typeface`（フォント名）を追加する。pptx パースで `<a:rPr>` の属性・子要素からこれらを読み取る
- **段落プロパティの追加**: `PptxParagraph` に `algn`（配置）、`lvl`（レベル）、`bu_char`（箇条書き文字）、`bu_auto_num`（自動番号）、`mar_l`（左マージン）、`indent`（インデント）、`ln_spc`（行間）、`spc_bef`（段落前間隔）、`spc_aft`（段落後間隔）を追加する
- **テキストボディプロパティの追加**: `PptxTextFrame` に `bodypr_xml`（`bodyPr` の生 XML: 方向・配置・余白・ autofit 等）を追加する
- **表セルプロパティの追加**: `PptxTableCell` に `grid_span`、`row_span`、`h_merge`、`v_merge` を追加する
- **表行プロパティの追加**: `PptxTableRow` に `h`（行の高さ）を追加する
- **プレースホルダー属性の拡充**: `PptxShape` の `placeholder` を構造化し、`type`・`idx`・`sz`・`orient` を保持する

**MVP ではやらない（後続 change に回す）**:

- MS-PPTX 拡張の `cameo`/`unknown` プレースホルダー種別
- 図形の 3D プロパティ（`scene3d`/`sp3d`）
- テキスト効果（`effectLst`/`ln` on runs）
- カスタムジオメトリ（`custGeom`）のパス構造化
- SmartArt、チャート、コメントの構造化
- スライドトランジション・アニメーション

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `pptx-decomposition`: 図形の描画順と geometry の保持要件（回転・反転・プリセット図形種別・ spPr 保持）、テキスト図形の忠実な分解要件（ラン書式・段落プロパティ・ボディプロパティ拡充）、表図形の忠実な分解要件（セル結合・行の高さ拡充）、プレースホルダー属性拡充を拡張する

## Impact

- **コード**: `src/pptx.rs`（パースロジックの拡張）、`src/ir.rs`（Geometry・PptxShape・PptxParagraph・PptxTextFrame・PptxTableCell・PptxTableRow の構造体拡張。TextRun は docx と共有のため新規フィールドは docx 互換で追加）、`src/main.rs`（統合）を変更する
- **CLI**: 変更なし
- **依存**: 新規依存なし
- **出力 JSON**: 既存フィールドは維持し、新規フィールドを追加する（後方互換）
- **テスト**: `tests/integration.rs` に geometry 拡張・図形プロパティ・テキスト書式・段落・表セル・行の高さの保持を検証するシナリオを追加する
- **ドキュメント**: README と docs/agent-integration.md を更新する
