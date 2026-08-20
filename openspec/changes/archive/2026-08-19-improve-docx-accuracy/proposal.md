## Why

officedump の設計原則「属性は全保持する」に照らし、docx 実装の情報欠落を修正する。現在 `TextRun` は bold/italic/underline/strike/style のみ保持で、サイズ・色・フォント・字間・縦位置等の `<rPr>` 子要素が未保持。段落プロパティ（`pPr`）は style/numId/ilvl のみで、配置・インデント・間隔等が未保持。表のプロパティ（`tblPr`/`trPr`/`tcPr`）は大部分が `unhandled` 行きで、行の高さ・セルの塗りつぶし・罫線・マージン等が構造化されていない。セクションプロパティ（`sectPr`）もページサイズ・マージン等が生 XML のままである。これらは「判断」ではなく「保持すべき属性」の欠落である。

## What Changes

**MVP（今回やる）**:

- **ラン書式プロパティの拡充**: `TextRun` に `sz`（サイズ）、`color`（色）、`rFonts`（フォント）、`vertAlign`（縦位置）、`spacing`（字間）、`kern`（カーニング）、`position`（位置）を追加する。`<rPr>` の未知の子要素は生 XML で保持する
- **段落プロパティの拡充**: `Block::Paragraph` に `jc`（配置）、`indent`（インデント情報）、`spacing`（段落間隔情報）を追加する。`<pPr>` の未知の子要素は生 XML で保持する
- **表プロパティの構造化**: `Block::Table` に `tblPr`（表プロパティ: 幅・罫線・セルマージン等）を生 XML で保持する。`TableRow` に `trHeight`・`cantSplit`・`tblHeader` を追加する。`TableCell` に `tcW`（セル幅）、`shd`（塗りつぶし）、`tcMar`（セルマージン）、`vAlign`（垂直配置）、`noWrap`、`tcBorders` を追加する
- **セクションプロパティの構造化**: `DocxSection` に `sectPr`（ページサイズ・マージン・列数等）を生 XML で保持する
- **ハイパーリンク属性の拡充**: `RunNode::Hyperlink` に `history`・`tooltip`・`tgtFrame` を追加する
- **フィールド属性の拡充**: `RunNode::Field` に `fldLock`・`dirty` を追加する

**MVP ではやらない（後続 change に回す）**:

- MS-DOCX 拡張のテキスト効果（glow/shadow/reflection/textOutline/textFill/scene3d/props3d/ligatures/numForm/numSpacing/stylisticSets/cntxtAlts）
- MS-DOCX 拡張の `collapsed`（pPr）、`footnoteColumns`（sectPr）、`paraId`/`textId`/`noSpellErr`（p/tr 属性）
- `settings.xml` の互換性設定
- 数式（OMML）、SmartArt、図形（DrawingML）の構造化

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `docx-decomposition`: 本文ブロックの忠実な分解要件（ラン書式・段落プロパティ拡充）、表の忠実な分解要件（表・行・セルプロパティ拡充）、ハイパーリンクとフィールドの解決要件（属性拡充）を拡張する。セクションプロパティ保持の新規要件を追加する

## Impact

- **コード**: `src/docx.rs`（パースロジックの拡張）、`src/ir.rs`（TextRun・Block::Paragraph・Block::Table・TableRow・TableCell・DocxSection・RunNode の構造体拡張）、`src/main.rs`（統合）を変更する
- **CLI**: 変更なし
- **依存**: 新規依存なし
- **出力 JSON**: 既存フィールドは維持し、新規フィールドを追加する（後方互換）
- **テスト**: `tests/integration.rs` に書式・段落・表・セクションプロパティの保持を検証するシナリオを追加する
- **ドキュメント**: README と docs/agent-integration.md を更新する
