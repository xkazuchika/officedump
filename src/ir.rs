//! 中間表現（IR）。構造は正規化し、属性は全保持し、意味の解釈は行わない。

use serde::Serialize;
use serde_json::Value;

fn is_false(b: &bool) -> bool {
    !*b
}

// ---------------------------------------------------------------------------
// xlsx
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct InspectOutput {
    pub file: String,
    pub format: String,
    pub sheets: Vec<SheetSummary>,
}

#[derive(Debug, Serialize)]
pub struct SheetSummary {
    pub name: String,
    pub rows: u64,
    pub cols: u64,
}

#[derive(Debug, Serialize)]
pub struct ReadOutput {
    pub file: String,
    pub format: String,
    pub sheets: Vec<SheetDump>,
    pub media: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
pub struct SheetDump {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(rename = "mergedCells")]
    pub merged_cells: Vec<String>,
    pub cells: Vec<Cell>,
    #[serde(rename = "unhandledElements")]
    pub unhandled: Vec<RawElement>,
}

#[derive(Debug, Serialize)]
pub struct Cell {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(rename = "type")]
    pub cell_type: String,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<CellStyle>,
}

#[derive(Debug, Serialize)]
pub struct CellStyle {
    pub xf: u32,
    #[serde(rename = "numFmtId")]
    pub num_fmt_id: u32,
    #[serde(rename = "formatCode", skip_serializing_if = "Option::is_none")]
    pub format_code: Option<String>,
}

// ---------------------------------------------------------------------------
// docx
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DocxInspectOutput {
    pub file: String,
    pub format: String,
    pub sections: Vec<DocxSectionSummary>,
    pub outline: Vec<OutlineEntry>,
}

#[derive(Debug, Serialize)]
pub struct DocxSectionSummary {
    #[serde(rename = "type")]
    pub section_type: String,
    pub blocks: usize,
}

#[derive(Debug, Serialize)]
pub struct OutlineEntry {
    pub index: u32,
    pub level: u32,
    pub style: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct DocxReadOutput {
    pub file: String,
    pub format: String,
    pub sections: Vec<DocxSection>,
    pub media: Vec<MediaItem>,
    #[serde(rename = "unhandledElements")]
    pub unhandled: Vec<RawElement>,
}

#[derive(Debug, Serialize)]
pub struct DocxSection {
    #[serde(rename = "type")]
    pub section_type: String,
    pub blocks: Vec<Block>,
}

/// 文書のブロック。1始まりの `index` はメディアアンカー・部分読み出し・
/// 見出しアウトラインの共通キー。
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Block {
    #[serde(rename = "paragraph")]
    Paragraph {
        index: u32,
        /// 段落スタイルの styleId（生値）
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        num: Option<NumProps>,
        runs: Vec<RunNode>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        unhandled: Vec<RawElement>,
    },
    #[serde(rename = "table")]
    Table {
        index: u32,
        /// tblGrid の各列幅（twips 生値）
        grid: Vec<u32>,
        rows: Vec<TableRow>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        unhandled: Vec<RawElement>,
    },
}

impl Block {
    pub fn index(&self) -> u32 {
        match self {
            Block::Paragraph { index, .. } | Block::Table { index, .. } => *index,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NumProps {
    #[serde(rename = "numId")]
    pub num_id: u32,
    pub ilvl: u32,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum RunNode {
    #[serde(rename = "text")]
    Text(TextRun),
    #[serde(rename = "hyperlink")]
    Hyperlink {
        /// リレーション解決済みの対象 URL（文書内リンクの場合は None）
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
        /// 文書内リンクのアンカー（生値）
        #[serde(skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
        runs: Vec<TextRun>,
    },
    /// フィールド。命令テキストは原文のまま保持し、評価はしない。
    #[serde(rename = "field")]
    Field { instr: String, text: String },
}

#[derive(Debug, Serialize)]
pub struct TextRun {
    pub text: String,
    /// 文字スタイルの styleId（生値）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub strike: bool,
}

#[derive(Debug, Serialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Serialize)]
pub struct TableCell {
    #[serde(rename = "gridSpan", skip_serializing_if = "Option::is_none")]
    pub grid_span: Option<u32>,
    /// "restart" または "continue"
    #[serde(rename = "vMerge", skip_serializing_if = "Option::is_none")]
    pub v_merge: Option<String>,
    pub blocks: Vec<Block>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unhandled: Vec<RawElement>,
}

// ---------------------------------------------------------------------------
// 共通
// ---------------------------------------------------------------------------

/// 未知要素の生 XML 保持（escape hatch）
#[derive(Debug, Serialize)]
pub struct RawElement {
    pub name: String,
    pub xml: String,
    #[serde(skip_serializing_if = "is_false")]
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct MediaItem {
    #[serde(rename = "ref")]
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<MediaAnchor>,
}

/// read の既定標準出力。全 IR ではなく、次に読むべきファイルを示す。
#[derive(Debug, Serialize)]
pub struct ReadManifest {
    pub file: String,
    pub format: String,
    /// content.json の絶対パス
    pub content: String,
    #[serde(rename = "mediaDir")]
    /// media ディレクトリの絶対パス
    pub media_dir: String,
    pub summary: ReadSummary,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ReadSummary {
    Xlsx {
        sheets: usize,
        cells: usize,
        media: usize,
    },
    Docx {
        sections: usize,
        blocks: usize,
        media: usize,
    },
}

/// メディアの位置情報。xlsx は sheet/from/to、docx は section/block/run を基準にする。
/// 座標・サイズの生値を保持し、読み順や意味の解釈は行わない。
#[derive(Debug, Serialize)]
pub struct MediaAnchor {
    // xlsx
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<AnchorPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<AnchorPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<AnchorPos>,
    // docx
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<u32>,
    #[serde(rename = "posH", skip_serializing_if = "Option::is_none")]
    pub pos_h: Option<PosOffset>,
    #[serde(rename = "posV", skip_serializing_if = "Option::is_none")]
    pub pos_v: Option<PosOffset>,
    // 共通
    #[serde(rename = "anchorType")]
    pub anchor_type: String,
    /// "floating"（xlsx の図・docx の anchor）または "inline"（docx の inline）
    pub placement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// サイズ（EMU 生値）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<AnchorSize>,
}

/// アンカーの基点。col/row は 0 始まり、オフセットは EMU（生値）。
#[derive(Debug, Clone, Serialize)]
pub struct AnchorPoint {
    pub col: u32,
    pub row: u32,
    #[serde(rename = "colOffEmu")]
    pub col_off_emu: i64,
    #[serde(rename = "rowOffEmu")]
    pub row_off_emu: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnchorPos {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnchorSize {
    pub cx: i64,
    pub cy: i64,
}

/// docx フローティング配置の位置（posH/posV）
#[derive(Debug, Clone, Serialize)]
pub struct PosOffset {
    #[serde(rename = "relativeFrom")]
    pub relative_from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(rename = "offsetEmu", skip_serializing_if = "Option::is_none")]
    pub offset_emu: Option<i64>,
}
