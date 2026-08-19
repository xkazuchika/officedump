## Context

現在の `src/xlsx.rs` は sharedStrings・styles・worksheet を最小限の構造でパースしている。`[MS-XLSX]-260519.txt` および ISO/IEC29500 と照合すると、リッチテキストの `<rPr>` 書式、数式属性、行属性、スタイルの `fonts`/`fills`/`borders`/`alignment`/`cellStyles` が未保持である。既存の `quick_xml::Reader` を用いたイベント駆動パースと `serde` による IR 出力のアーキテクチャは維持し、IR 構造体とパースロジックを拡張する。

## Goals / Non-Goals

**Goals:**

- リッチテキストランの書式属性を情報損失なく IR に含める
- 数式のメタデータ（種別・範囲・グループインデックス）を IR に含める
- 行の構造属性を IR に含める
- スタイル定義（fonts/fills/borders/cellXfs/cellStyleXfs/cellStyles）を IR に含める
- `formatCode16` を `formatCode` より優先して読み取る
- 既存の出力 JSON のフィールドを維持し、後方互換性を保つ

**Non-Goals:**

- リッチデータ層（Modern Data Types、モダンエラー値）の対応
- セルメタデータ（cm/vm/ph）と Metadata パートの解釈
- MS-XLSX 拡張属性（dyDescent/knownFonts/xfComplement/misleadingFormat）の対応
- 組み込み numFmtId（0-49）の formatCode 展開
- 1900/1904 日付システム属性の読み取り
- スタイルの解決・適用（ツールは生定義を保持し、解決は LLM の仕事）

## Decisions

### D1: リッチテキストランの IR 構造

共有文字列とインライン文字列のリッチテキストランを `XlsxRichRun` で表現する。`text`（連結テキスト）と `runs`（ラン配列）を分けて保持し、`runs` はランを持たない場合は `None` とする。

```rust
pub struct XlsxRichRun {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpr: Option<XlsxRunProps>,
}

pub struct XlsxRunProps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rfont: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertAlign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condense: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend: Option<bool>,
}
```

`Cell` 構造体に `runs: Option<Vec<XlsxRichRun>>` を追加する。`t="s"` と `t="inlineStr"` のセルでリッチテキストランが存在する場合に `runs` を設定し、`value` は従来どおり連結テキストの文字列を返す。

**代替案**: `value` をオブジェクトに変更して `text` と `runs` を統一する。→ 既存の `value` が文字列であることを前提とする消費者（テスト含む）との後方互換性が壊れるため不採用。

**代替案**: `<rPr>` を生 XML として保持する。→ 属性が正規化されず名前空間ノイズが残るため不採用。構造化フィールドで保持する。

### D2: 数式属性の IR 構造

`Cell.formula` は従来どおり `Option<String>`（数式テキスト）を維持し、新規に `formulaMeta: Option<FormulaMeta>` を追加する。

```rust
pub struct FormulaMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub si: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aca: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bx: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca: Option<bool>,
}
```

**代替案**: `formula` を `Option<Formula>` 構造体に変更してテキストと属性を統一する。→ 既存の `formula` が文字列であることを前提とするテストとの互換性が壊れるため不採用。

### D3: 行属性の IR 構造

`SheetDump` に `rows: Option<Vec<RowInfo>>` を追加する。`RowInfo` は行番号と、存在する属性のみを保持する。デフォルト値の属性は出力に含めない（出力サイズの膨張を防ぐ）。

```rust
pub struct RowInfo {
    pub r: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spans: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customFormat: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ht: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customHeight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlineLevel: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickTop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickBottom: Option<bool>,
}
```

`None`（属性なしの行のみ）の場合は `rows` フィールド自体を省略する。これにより、行属性を持たないシートでは出力が変化しない。

### D4: スタイル定義の IR 構造

`ReadOutput` に `styles: Option<WorkbookStyles>` を追加する。`WorkbookStyles` は各スタイル構成要素の配列を持つ。`cellXfs` と `cellStyleXfs` は構造化された `XfDef` で保持し、`fonts`/`fills`/`borders` は各要素の生 XML を `RawElement` として保持する（複雑な子要素構造を情報損失なく保持するため）。

```rust
pub struct WorkbookStyles {
    pub numFmts: Vec<NumFmtDef>,
    pub fonts: Vec<RawElement>,
    pub fills: Vec<RawElement>,
    pub borders: Vec<RawElement>,
    pub cellStyleXfs: Vec<XfDef>,
    pub cellXfs: Vec<XfDef>,
    pub cellStyles: Vec<CellStyleDef>,
}

pub struct XfDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numFmtId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fontId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fillId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borderId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xfId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyNumberFormat: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyFont: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyFill: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyBorder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyAlignment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applyProtection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotePrefix: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivotButton: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<AlignmentDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection: Option<ProtectionDef>,
}

pub struct AlignmentDef {
    // horizontal, vertical, wrapText, textRotation, indent, relativeIndent, shrinkToFit, justifyLastLine, readingOrder
    // 全て Option で skip_serializing_if
}

pub struct ProtectionDef {
    // locked, hidden — Option で skip_serializing_if
}

pub struct CellStyleDef {
    pub name: String,
    #[serde(rename = "xfId")]
    pub xf_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtinId: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customBuiltin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customLocked: Option<bool>,
}

pub struct NumFmtDef {
    #[serde(rename = "numFmtId")]
    pub id: u32,
    #[serde(rename = "formatCode")]
    pub code: String,
}
```

`styles` は `styles.xml` が存在しない場合は `None` とする。

**代替案**: `fonts`/`fills`/`borders` も構造化する。→ `<color>` や `<gradientFill>` 等の子要素が複雑で、構造化すると属性欠落のリスクが高まるため不採用。生 XML 保持が「属性は全保持」の原則に最も合致する。

**代替案**: `CellStyle`（セル単位）に全スタイル情報をインライン展開する。→ 同じスタイルを複数セルが参照するため出力が肥大化するため不採用。スタイル定義を独立させ、セルはインデックスで参照する。

### D5: formatCode16 の読み取り

`parse_styles` の `numFmt` パース処理で `formatCode16` 属性を先に探し、存在する場合はそれを `formatCode` として採用する。`formatCode16` が存在しない場合は従来どおり `formatCode` を読む。

### D6: 共有文字列のパース拡張

`parse_shared_strings` の戻り値を `Vec<String>` から `Vec<SharedString>` に変更する。`SharedString` は連結テキストとオプションのラン配列を持つ。

```rust
pub struct SharedString {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs: Option<Vec<XlsxRichRun>>,
}
```

`cell_value` は `SharedString` から `text` を取り出して `Value::String` を返し、`finalize_cell` で `runs` を `Cell.runs` に設定する。`SharedString` に `runs` がない場合は `Cell.runs` を `None` とする。

### D7: インライン文字列のパース拡張

`CellDraft` に `inline_runs: Option<Vec<XlsxRichRun>>` を追加する。`parse_worksheet` で `<is>` 内の `<r>` ランを検出した場合、各ランの `<rPr>` とテキストを構造化して `inline_runs` に格納する。`<is><t>` のみの場合は従来どおり `inline_text` を使い、`runs` は `None` とする。

## Risks / Trade-offs

- **出力サイズの増大**: スタイル定義とリッチテキストランの追加により、JSON 出力サイズが増加する。→ `skip_serializing_if` で未設定フィールドを省略し、影響を最小化する。行属性も属性を持つ行のみを出力する。
- **fonts/fills/borders の生 XML 保持**: 生 XML は名前空間付きのまま保持されるため、LLM が解析しにくい可能性がある。→ `local_name` による要素名の正規化は `RawElement.name` で行い、`xml` フィールドには元の XML を保持する。LLM は `name` で要素種別を判別し、`xml` で詳細を読める。
- **既存テストの互換性**: `formula` 文字列と `value` 文字列を維持するため、既存テストはそのまま通過する。新規フィールドのテストを追加する。
- **パース性能**: 追加の属性読み取りとスタイルパースにより、パース時間が増加する可能性がある。→ 既存のイベント駆動パースの枠組み内で追加するため、O(n) のまま維持される。
