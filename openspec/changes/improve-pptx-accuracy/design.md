## Context

現在の `src/pptx.rs` はスライドの図形ツリーを図形・画像・表に分解するが、geometry の回転・反転、プリセット図形種別、`spPr` の塗り・線・効果、テキストランのサイズ・色・フォント、段落プロパティ、テキストボディプロパティ、表セルの結合、行の高さ、プレースホルダーの詳細属性が未保持である。既存のイベント駆動パースアーキテクチャを維持し、IR 構造体とパースロジックを拡張する。

## Goals / Non-Goals

**Goals:**

- Geometry に回転・反転を追加する
- プリセット図形種別（`prstGeom prst`）を保持する
- `spPr` の生 XML を保持する
- テキストランにサイズ・色・フォント名を追加する
- 段落プロパティ（配置・レベル・箇条書き・マージン・インデント・間隔）を保持する
- テキストボディプロパティ（`bodyPr`）を生 XML で保持する
- 表セルの結合（gridSpan/rowSpan/hMerge/vMerge）を保持する
- 表行の高さを保持する
- プレースホルダーの type/idx/sz/orient を構造化して保持する
- 既存の出力 JSON のフィールドを維持し、後方互換性を保つ

**Non-Goals:**

- MS-PPTX 拡張の `cameo`/`unknown` プレースホルダー種別
- 図形の 3D プロパティ（`scene3d`/`sp3d`）
- テキスト効果（`effectLst`/`ln` on runs）
- カスタムジオメトリ（`custGeom`）のパス構造化
- SmartArt、チャート、コメントの構造化
- スライドトランジション・アニメーション

## Decisions

### D1: Geometry の拡張

```rust
pub struct Geometry {
    pub x: i64,
    pub y: i64,
    pub cx: i64,
    pub cy: i64,
    // 新規
    pub rot: Option<i64>,      // 1/60000度単位
    pub flip_h: Option<bool>,
    pub flip_v: Option<bool>,
}
```

`a:xfrm` 要素の属性から読み取る。既存の `parse_geometry` を拡張する。

### D2: PptxShape の拡張

```rust
pub struct PptxShape {
    pub shape_type: String,     // 既存: "shape"/"picture"/"table"
    pub z_order: u32,
    pub name: Option<String>,
    pub placeholder: Option<String>,  // 既存: type のみ（後方互換のため維持）
    // 新規
    pub placeholder_detail: Option<PptxPlaceholder>,
    pub prst_geom: Option<String>,     // prstGeom prst 値
    pub sppr_xml: Option<String>,      // spPr の生 XML
    pub geometry: Option<Geometry>,
    pub text: Option<PptxTextFrame>,
    pub table: Option<PptxTable>,
}

pub struct PptxPlaceholder {
    pub r#type: String,    // ph type
    pub idx: Option<u32>,
    pub sz: Option<String>,
    pub orient: Option<String>,
}
```

`placeholder`（既存の `Option<String>`）は後方互換のため維持し、`placeholder_detail` に構造化データを追加する。

### D3: TextRun の拡張（pptx 共用）

`TextRun` は docx と共用のため、docx の改善 change で追加されるフィールド（`sz`/`color`/`rfonts`/`vert_align`/`spacing`/`kern`/`position`/`rpr_xml`）と協調する。pptx パースでは以下を読み取る:

- `sz`: `a:rPr` の `sz` 属性（1/100 pt単位）
- `color`: `a:rPr` 内の `a:solidFill/a:srgbClr val` 値
- `typeface`: `a:rPr` 内の `a:latin typeface` 値

```rust
pub struct TextRun {
    pub text: String,
    pub style: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    // 新規（docx 共用）
    pub sz: Option<u32>,
    pub color: Option<String>,
    pub rfonts: Option<DocxRFonts>,   // docx 用。pptx の typeface は rfonts.ascii に入れる
    pub vert_align: Option<String>,
    pub spacing: Option<i32>,
    pub kern: Option<u32>,
    pub position: Option<i32>,
    pub rpr_xml: Option<String>,
}
```

pptx の `typeface` は `rfonts.ascii` に格納し、フィールドを共用する。

### D4: PptxParagraph の拡張

```rust
pub struct PptxParagraph {
    pub runs: Vec<TextRun>,
    // 新規
    pub algn: Option<String>,
    pub lvl: Option<u32>,
    pub bu_char: Option<String>,
    pub bu_auto_num: Option<String>,
    pub mar_l: Option<i64>,
    pub indent: Option<i64>,
    pub ln_spc: Option<i64>,
    pub spc_bef: Option<i64>,
    pub spc_aft: Option<i64>,
}
```

`a:pPr` 要素の属性・子要素から読み取る。`lnSpc`/`spcBef`/`spcAft` は `a:spcPct`/`a:spcPts` 子要素の値も読み取る。

### D5: PptxTextFrame の拡張

```rust
pub struct PptxTextFrame {
    pub paragraphs: Vec<PptxParagraph>,
    // 新規
    pub bodypr_xml: Option<String>,   // bodyPr の生 XML
}
```

`a:bodyPr` 要素の生 XML を保持する。方向・配置・余白・autofit 等の複雑な構造は生 XML で保持し、LLM が解釈する。

### D6: PptxTableCell の拡張

```rust
pub struct PptxTableCell {
    pub text: PptxTextFrame,
    // 新規
    pub grid_span: Option<u32>,
    pub row_span: Option<u32>,
    pub h_merge: Option<bool>,
    pub v_merge: Option<bool>,
}
```

### D7: PptxTableRow の拡張

```rust
pub struct PptxTableRow {
    pub cells: Vec<PptxTableCell>,
    // 新規
    pub h: Option<i64>,   // 行の高さ（EMU）
}
```

### D8: プレースホルダー属性のパース

`parse_shape` で `<p:ph>` 要素の `type`/`idx`/`sz`/`orient` 属性を読み取る。`type` が省略された場合は従来どおり `"body"` とするが、`placeholder_detail` では省略を区別するため `None` とする。

## Risks / Trade-offs

- **出力サイズの増大**: 書式・プロパティ追加により JSON サイズが増加する。→ `skip_serializing_if` で未設定フィールドを省略し、影響を最小化する。
- **TextRun 共用のリスク**: `TextRun` は docx と共用のため、両方の change で同時に編集すると競合する可能性がある。→ pptx change は docx change の適用後に実行することを推奨。フィールド定義は協調する。
- **生 XML 保持のノイズ**: `sppr_xml`/`bodypr_xml` は名前空間付き生 XML のままで LLM が解析しにくい可能性がある。→ 主要属性は構造化フィールドで抽出しつつ、生 XML を escape hatch として併存させる。
- **既存テストの互換性**: 既存フィールドの型を変更しないため、既存テストはそのまま通過する。新規フィールドのテストを追加する。
- **パース性能**: 追加の属性読み取りによりパース時間が増加する可能性がある。→ イベント駆動パースの枠組み内で追加するため、O(n) のまま維持される。
