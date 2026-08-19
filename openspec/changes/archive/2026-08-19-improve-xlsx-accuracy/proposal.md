## Why

officedump の設計原則は「構造は正規化する、属性は全保持する、判断はしない」だが、xlsx 実装は `[MS-XLSX]-260519.txt` および ISO/IEC29500 と照合すると複数の箇所で情報を落としている。共有文字列のリッチテキスト書式、数式メタデータ（配列数式・共有数式の範囲・種別）、行の構造属性、スタイルの完全な情報（フォント・塗りつぶし・罫線・配置）、`formatCode16`（MS-XLSX 拡張で `formatCode` より優先される）が未保持である。これらは「判断」ではなく「保持すべき属性」の欠落であり、LLM が元のファイル情報を復元できない状況を生んでいる。

## What Changes

**MVP（今回やる）**:

- **共有文字列のリッチテキスト保持**: `<si>` の `<r>` ランごとに `<rPr>`（フォント書式）とテキストを構造化して保持する。従来の連結テキストも `text` フィールドに残し、後方互換性を維持する
- **インライン文字列のリッチテキスト保持**: `<is>` 内の `<r>` ランも同様に構造化する
- **数式属性の保持**: `<f>` 要素の `t`（normal/array/shared/dataTable）・`ref`（配列・共有数式の範囲）・`si`（共有数式グループインデックス）・`aca`・`bx`・`ca` 属性を保持する
- **行属性の保持**: `<row>` の `hidden`・`ht`・`customHeight`・`outlineLevel`・`collapsed`・`spans`・`customFormat`・`s`（スタイルインデックス）・`thickBottom`・`thickTop` を構造化して保持する
- **スタイル情報の拡充**: `cellXfs` の `<xf>` 要素について全属性（`fontId`・`fillId`・`borderId`・`xfId`・`applyFont`・`applyFill`・`applyBorder`・`applyAlignment`・`applyProtection`・`quotePrefix`）と子要素（`alignment`・`protection`）を保持する。`fonts`・`fills`・`borders`・`cellStyleXfs`・`cellStyles`（名前付きスタイル）を読み、IR に含める
- **`formatCode16` の読み取り**: MS-XLSX 拡張である `formatCode16` 属性が存在する場合は `formatCode` より優先して保持する

**MVP ではやらない（後続 change に回す）**:

- リッチデータ層（Modern Data Types: Linked Entity、画像URL、配列データ型、`#SPILL!`/`#BLOCKED!` 等のモダンエラー値）
- セルメタデータ（`cm`/`vm`/`ph`）と Metadata パートの解釈
- `dyDescent`（行のタイポグラフィ情報）、`knownFonts`、`xfComplement`、`misleadingFormat` の MS-XLSX 拡張属性
- 組み込み numFmtId（0-49）の formatCode への展開
- 1900/1904 日付システム属性の読み取り

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `xlsx-decomposition`: セルデータの忠実な分解要件を拡張する。共有文字列・インライン文字列のリッチテキスト書式保持、数式属性保持、行属性保持、スタイル情報の完全な保持を要件に追加する

## Impact

- **コード**: `src/xlsx.rs`（共有文字列パース、スタイルパース、ワークシートパースの拡張）、`src/ir.rs`（IR 構造体の拡張: リッチテキストラン、行属性、スタイル詳細）、`src/main.rs`（スタイル読み出しの統合）を変更する
- **CLI**: 変更なし（`inspect`/`read` のインターフェースは同じ）
- **依存**: 新規依存なし（zip + quick-xml + serde + clap のまま）
- **出力 JSON**: 既存フィールドは維持し、新規フィールドを追加する（後方互換）。共有文字列の `value` は従来どおり文字列を返し、新規に `runs` フィールドを追加する。行属性とスタイル詳細は新規フィールドとして追加する
- **テスト**: `tests/integration.rs` にリッチテキスト・数式属性・行属性・スタイル情報の保持を検証するシナリオを追加する
- **ドキュメント**: README と docs/agent-integration.md の IR 構造説明を更新する
