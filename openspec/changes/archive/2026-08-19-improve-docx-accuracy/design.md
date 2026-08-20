## Context

現在の `src/docx.rs` は段落・表・ヘッダー/フッターをブロック単位で分解するが、`TextRun` の書式属性、段落プロパティ、表プロパティ、セルプロパティの大部分が未保持である。既存の `quick_xml::Reader` イベント駆動パースと `serde` IR 出力のアーキテクチャを維持し、IR 構造体とパースロジックを拡張する。

## Goals / Non-Goals

**Goals:**

- ラン書式プロパティ（sz/color/rFonts/vertAlign/spacing/kern/position）を IR に含める
- 段落プロパティ（jc/ind/spacing）を構造化して IR に含める
- 表プロパティ（tblPr）を生 XML で IR に含める
- 行プロパティ（trHeight/cantSplit/tblHeader）を IR に含める
- セルプロパティ（tcW/shd/tcMar/vAlign/noWrap/tcBorders）を IR に含める
- セクションプロパティ（sectPr）を生 XML で IR に含める
- ハイパーリンク属性（history/tooltip/tgtFrame）を IR に含める
- フィールド属性（fldLock/dirty）を IR に含める
- 既存の出力 JSON のフィールドを維持し、後方互換性を保つ

**Non-Goals:**

- MS-DOCX 拡張のテキスト効果（glow/shadow/reflection/textOutline/textFill/scene3d/props3d/ligatures/numForm/numSpacing/stylisticSets/cntxtAlts）
- MS-DOCX 拡張の `collapsed`（pPr）、`footnoteColumns`（sectPr）、`paraId`/`textId`/`noSpellErr`（p/tr 属性）
- `settings.xml` の互換性設定
- 数式（OMML）、SmartArt、図形（DrawingML）の構造化
- スタイルの解決・適用（生定義を保持し、解決は LLM の仕事）

## Decisions

### D1: TextRun の拡張

`TextRun` に新規フィールドを追加する。既存の `bold`/`italic`/`underline`/`strike` は維持し、新規フィールドは `Option` + `skip_serializing_if` で省略可能とする。

```rust
pub struct TextRun {
    pub text: String,
    pub style: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,      // 既存: bool。u val="single" 等の区別は rpr_xml で保持
    pub strike: bool,
    // 新規
    pub sz: Option<u32>,           // 半ポイント（sz="24" = 12pt）
    pub color: Option<String>,     // color val="FF0000" の値
    pub rfonts: Option<DocxRFonts>,// rFonts の ascii/hAnsi/eastAsia/cs
    pub vert_align: Option<String>,// vertAlign val="superscript"/"subscript"
    pub spacing: Option<i32>,      // spacing val（1/20 pt）
    pub kern: Option<u32>,         // kern val（半ポイント）
    pub position: Option<i32>,     // position val（1/20 pt）
    pub rpr_xml: Option<String>,   // rPr 全体の生 XML（未知子要素の escape hatch）
}

pub struct DocxRFonts {
    pub ascii: Option<String>,
    pub h_ansi: Option<String>,
    pub east_asian: Option<String>,
    pub cs: Option<String>,
}
```

**代替案**: `underline` を `Option<String>` に変更して val 値を保持する。→ 既存テストが `bool` を前提とするため不採用。val 値は `rpr_xml` で参照可能。

### D2: Block::Paragraph の拡張

```rust
Block::Paragraph {
    index: u32,
    style: Option<String>,
    num: Option<NumProps>,
    // 新規
    jc: Option<String>,            // jc val="center"/"left"/"right"/"both"
    ind: Option<DocxInd>,          // ind の left/right/firstLine/hanging
    spacing: Option<DocxSpacing>,  // spacing の before/after/line/lineRule
    ppr_xml: Option<String>,       // pPr 全体の生 XML（未知子要素の escape hatch）
    runs: Vec<RunNode>,
    unhandled: Vec<RawElement>,
}

pub struct DocxInd {
    pub left: Option<i32>,
    pub right: Option<i32>,
    pub first_line: Option<i32>,
    pub hanging: Option<i32>,
}

pub struct DocxSpacing {
    pub before: Option<i32>,
    pub after: Option<i32>,
    pub line: Option<i32>,
    pub line_rule: Option<String>,
}
```

### D3: Block::Table の拡張

```rust
Block::Table {
    index: u32,
    grid: Vec<u32>,
    rows: Vec<TableRow>,
    // 新規
    tblpr_xml: Option<String>,     // tblPr の生 XML
    unhandled: Vec<RawElement>,
}
```

`tblPr` は子要素構造が複雑（tblW/tblInd/tblBorders/tblCellMar/tblLook/tblLayout 等）のため、生 XML で保持する。`font/fill/border` の xlsx 方針と同じく「複雑な子要素は生 XML」アプローチ。

### D4: TableRow の拡張

```rust
pub struct TableRow {
    pub cells: Vec<TableCell>,
    // 新規
    pub tr_height: Option<DocxTrHeight>,
    pub cant_split: Option<bool>,
    pub tbl_header: Option<bool>,
}

pub struct DocxTrHeight {
    pub val: i32,
    pub h_rule: Option<String>,   // "atLeast"/"exact"/"auto"
}
```

### D5: TableCell の拡張

```rust
pub struct TableCell {
    pub grid_span: Option<u32>,
    pub v_merge: Option<String>,
    pub blocks: Vec<Block>,
    pub unhandled: Vec<RawElement>,
    // 新規
    pub tcw: Option<DocxTcW>,
    pub shd: Option<String>,        // shd の生 XML
    pub tc_mar: Option<String>,    // tcMar の生 XML
    pub v_align: Option<String>,   // vAlign val
    pub no_wrap: Option<bool>,
    pub tc_borders: Option<String>,// tcBorders の生 XML
}

pub struct DocxTcW {
    pub w: i32,
    pub type_: String,             // "dxa"/"pct"/"auto"
}
```

`shd`/`tcMar`/`tcBorders` は子要素構造が複雑なため生 XML で保持する。

### D6: DocxSection の拡張

```rust
pub struct DocxSection {
    pub section_type: String,
    pub blocks: Vec<Block>,
    // 新規
    pub sectpr_xml: Option<String>,  // sectPr の生 XML
}
```

現在 `sectPr` は `unhandled` に退避されているが、独立フィールドとして構造化する。ヘッダー/フッター参照の読み取りは従来どおり `sectpr_xml` から行う。

### D7: RunNode::Hyperlink の拡張

```rust
RunNode::Hyperlink {
    href: Option<String>,
    anchor: Option<String>,
    runs: Vec<TextRun>,
    // 新規
    history: Option<String>,
    tooltip: Option<String>,
    tgt_frame: Option<String>,
}
```

### D8: RunNode::Field の拡張

```rust
RunNode::Field {
    instr: String,
    text: String,
    // 新規
    fld_lock: Option<bool>,
    dirty: Option<bool>,
}
```

`fldLock`/`dirty` は `fldChar` 要素の属性として読み取る。

## Risks / Trade-offs

- **出力サイズの増大**: 書式・プロパティ追加により JSON サイズが増加する。→ `skip_serializing_if` で未設定フィールドを省略し、影響を最小化する。
- **生 XML 保持のノイズ**: `tblPr`/`sectPr`/`shd`/`tcMar`/`tcBorders`/`rpr_xml`/`ppr_xml` は名前空間付き生 XML のままで LLM が解析しにくい可能性がある。→ 構造化フィールドで主要属性を抽出しつつ、生 XML を escape hatch として併存させる。
- **既存テストの互換性**: 既存フィールドの型を変更しないため、既存テストはそのまま通過する。新規フィールドのテストを追加する。
- **パース性能**: 追加の属性読み取りによりパース時間が増加する可能性がある。→ イベント駆動パースの枠組み内で追加するため、O(n) のまま維持される。
