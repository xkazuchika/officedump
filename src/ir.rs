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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<WorkbookStyles>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<RowInfo>>,
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
    #[serde(rename = "formulaMeta", skip_serializing_if = "Option::is_none")]
    pub formula_meta: Option<FormulaMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<CellStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs: Option<Vec<XlsxRichRun>>,
}

#[derive(Debug, Serialize)]
pub struct CellStyle {
    pub xf: u32,
    #[serde(rename = "numFmtId")]
    pub num_fmt_id: u32,
    #[serde(rename = "formatCode", skip_serializing_if = "Option::is_none")]
    pub format_code: Option<String>,
}

/// 数式のメタデータ。数式テキストは Cell.formula に保持し、ここには属性のみ保持する。
#[derive(Debug, Serialize)]
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

/// リッチテキストラン。共有文字列・インライン文字列の <r> に対応する。
#[derive(Debug, Clone, Serialize)]
pub struct XlsxRichRun {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpr: Option<XlsxRunProps>,
}

/// ラン書式プロパティ（<rPr>）。生値を保持し、解決は行わない。
#[derive(Debug, Default, Clone, Serialize)]
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
    #[serde(rename = "vertAlign", skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condense: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend: Option<bool>,
}

/// 行の構造属性。存在する属性のみ保持する。
#[derive(Debug, Serialize)]
pub struct RowInfo {
    pub r: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spans: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u32>,
    #[serde(rename = "customFormat", skip_serializing_if = "Option::is_none")]
    pub custom_format: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ht: Option<f64>,
    #[serde(rename = "customHeight", skip_serializing_if = "Option::is_none")]
    pub custom_height: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(rename = "outlineLevel", skip_serializing_if = "Option::is_none")]
    pub outline_level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(rename = "thickTop", skip_serializing_if = "Option::is_none")]
    pub thick_top: Option<bool>,
    #[serde(rename = "thickBottom", skip_serializing_if = "Option::is_none")]
    pub thick_bottom: Option<bool>,
}

/// ワークブックのスタイル定義。生値を保持し、解決・適用は行わない。
#[derive(Debug, Default, Clone, Serialize)]
pub struct WorkbookStyles {
    #[serde(rename = "numFmts")]
    pub num_fmts: Vec<NumFmtDef>,
    pub fonts: Vec<RawElement>,
    pub fills: Vec<RawElement>,
    pub borders: Vec<RawElement>,
    #[serde(rename = "cellStyleXfs")]
    pub cell_style_xfs: Vec<XfDef>,
    #[serde(rename = "cellXfs")]
    pub cell_xfs: Vec<XfDef>,
    #[serde(rename = "cellStyles")]
    pub cell_styles: Vec<CellStyleDef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NumFmtDef {
    #[serde(rename = "numFmtId")]
    pub id: u32,
    #[serde(rename = "formatCode")]
    pub code: String,
}

/// xf 要素の定義。全属性と alignment/protection 子要素を保持する。
#[derive(Debug, Default, Clone, Serialize)]
pub struct XfDef {
    #[serde(rename = "numFmtId", skip_serializing_if = "Option::is_none")]
    pub num_fmt_id: Option<u32>,
    #[serde(rename = "fontId", skip_serializing_if = "Option::is_none")]
    pub font_id: Option<u32>,
    #[serde(rename = "fillId", skip_serializing_if = "Option::is_none")]
    pub fill_id: Option<u32>,
    #[serde(rename = "borderId", skip_serializing_if = "Option::is_none")]
    pub border_id: Option<u32>,
    #[serde(rename = "xfId", skip_serializing_if = "Option::is_none")]
    pub xf_id: Option<u32>,
    #[serde(rename = "applyNumberFormat", skip_serializing_if = "Option::is_none")]
    pub apply_number_format: Option<bool>,
    #[serde(rename = "applyFont", skip_serializing_if = "Option::is_none")]
    pub apply_font: Option<bool>,
    #[serde(rename = "applyFill", skip_serializing_if = "Option::is_none")]
    pub apply_fill: Option<bool>,
    #[serde(rename = "applyBorder", skip_serializing_if = "Option::is_none")]
    pub apply_border: Option<bool>,
    #[serde(rename = "applyAlignment", skip_serializing_if = "Option::is_none")]
    pub apply_alignment: Option<bool>,
    #[serde(rename = "applyProtection", skip_serializing_if = "Option::is_none")]
    pub apply_protection: Option<bool>,
    #[serde(rename = "quotePrefix", skip_serializing_if = "Option::is_none")]
    pub quote_prefix: Option<bool>,
    #[serde(rename = "pivotButton", skip_serializing_if = "Option::is_none")]
    pub pivot_button: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<AlignmentDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection: Option<ProtectionDef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlignmentDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizontal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical: Option<String>,
    #[serde(rename = "wrapText", skip_serializing_if = "Option::is_none")]
    pub wrap_text: Option<bool>,
    #[serde(rename = "textRotation", skip_serializing_if = "Option::is_none")]
    pub text_rotation: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<u32>,
    #[serde(rename = "relativeIndent", skip_serializing_if = "Option::is_none")]
    pub relative_indent: Option<i32>,
    #[serde(rename = "shrinkToFit", skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<bool>,
    #[serde(rename = "justifyLastLine", skip_serializing_if = "Option::is_none")]
    pub justify_last_line: Option<bool>,
    #[serde(rename = "readingOrder", skip_serializing_if = "Option::is_none")]
    pub reading_order: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtectionDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CellStyleDef {
    pub name: String,
    #[serde(rename = "xfId")]
    pub xf_id: u32,
    #[serde(rename = "builtinId", skip_serializing_if = "Option::is_none")]
    pub builtin_id: Option<u32>,
    #[serde(rename = "customBuiltin", skip_serializing_if = "Option::is_none")]
    pub custom_builtin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(rename = "customLocked", skip_serializing_if = "Option::is_none")]
    pub custom_locked: Option<bool>,
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

// ---------------------------------------------------------------------------
// pptx
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PptxInspectOutput {
    pub file: String,
    pub format: String,
    pub slides: usize,
    pub titles: Vec<SlideTitle>,
}

#[derive(Debug, Serialize)]
pub struct SlideTitle {
    pub index: u32,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct PptxReadOutput {
    pub file: String,
    pub format: String,
    pub slides: Vec<PptxSlide>,
    pub media: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
pub struct PptxSlide {
    pub index: u32,
    pub shapes: Vec<PptxShape>,
    #[serde(rename = "unhandledElements")]
    pub unhandled: Vec<RawElement>,
}

/// 図形ツリーの順序とgeometryを保つ。読み順の解釈はしない。
#[derive(Debug, Serialize)]
pub struct PptxShape {
    #[serde(rename = "type")]
    pub shape_type: String,
    #[serde(rename = "zOrder")]
    pub z_order: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(rename = "placeholderDetail", skip_serializing_if = "Option::is_none")]
    pub placeholder_detail: Option<PptxPlaceholder>,
    #[serde(rename = "prstGeom", skip_serializing_if = "Option::is_none")]
    pub prst_geom: Option<String>,
    #[serde(rename = "spPrXml", skip_serializing_if = "Option::is_none")]
    pub sppr_xml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<PptxTextFrame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<PptxTable>,
}

#[derive(Debug, Serialize)]
pub struct PptxPlaceholder {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orient: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Geometry {
    pub x: i64,
    pub y: i64,
    pub cx: i64,
    pub cy: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rot: Option<i64>,
    #[serde(rename = "flipH", skip_serializing_if = "Option::is_none")]
    pub flip_h: Option<bool>,
    #[serde(rename = "flipV", skip_serializing_if = "Option::is_none")]
    pub flip_v: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PptxTextFrame {
    pub paragraphs: Vec<PptxParagraph>,
    #[serde(rename = "bodyPrXml", skip_serializing_if = "Option::is_none")]
    pub bodypr_xml: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PptxParagraph {
    pub runs: Vec<TextRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lvl: Option<u32>,
    #[serde(rename = "buChar", skip_serializing_if = "Option::is_none")]
    pub bu_char: Option<String>,
    #[serde(rename = "buAutoNum", skip_serializing_if = "Option::is_none")]
    pub bu_auto_num: Option<String>,
    #[serde(rename = "marL", skip_serializing_if = "Option::is_none")]
    pub mar_l: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<i64>,
    #[serde(rename = "lnSpc", skip_serializing_if = "Option::is_none")]
    pub ln_spc: Option<i64>,
    #[serde(rename = "spcBef", skip_serializing_if = "Option::is_none")]
    pub spc_bef: Option<i64>,
    #[serde(rename = "spcAft", skip_serializing_if = "Option::is_none")]
    pub spc_aft: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PptxTable {
    pub columns: Vec<i64>,
    pub rows: Vec<PptxTableRow>,
}

#[derive(Debug, Serialize)]
pub struct PptxTableRow {
    pub cells: Vec<PptxTableCell>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PptxTableCell {
    pub text: PptxTextFrame,
    #[serde(rename = "gridSpan", skip_serializing_if = "Option::is_none")]
    pub grid_span: Option<u32>,
    #[serde(rename = "rowSpan", skip_serializing_if = "Option::is_none")]
    pub row_span: Option<u32>,
    #[serde(rename = "hMerge", skip_serializing_if = "Option::is_none")]
    pub h_merge: Option<bool>,
    #[serde(rename = "vMerge", skip_serializing_if = "Option::is_none")]
    pub v_merge: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DocxSection {
    #[serde(rename = "type")]
    pub section_type: String,
    pub blocks: Vec<Block>,
    #[serde(rename = "sectPrXml", skip_serializing_if = "Option::is_none")]
    pub sectpr_xml: Option<String>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        jc: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ind: Option<DocxInd>,
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<DocxSpacing>,
        #[serde(rename = "pPrXml", skip_serializing_if = "Option::is_none")]
        ppr_xml: Option<String>,
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
        #[serde(rename = "tblPrXml", skip_serializing_if = "Option::is_none")]
        tblpr_xml: Option<String>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        history: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tooltip: Option<String>,
        #[serde(rename = "tgtFrame", skip_serializing_if = "Option::is_none")]
        tgt_frame: Option<String>,
    },
    /// フィールド。命令テキストは原文のまま保持し、評価はしない。
    #[serde(rename = "field")]
    Field {
        instr: String,
        text: String,
        #[serde(rename = "fldLock", skip_serializing_if = "Option::is_none")]
        fld_lock: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dirty: Option<bool>,
    },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rfonts: Option<DocxRFonts>,
    #[serde(rename = "vertAlign", skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kern: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(rename = "rPrXml", skip_serializing_if = "Option::is_none")]
    pub rpr_xml: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DocxRFonts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ascii: Option<String>,
    #[serde(rename = "hAnsi", skip_serializing_if = "Option::is_none")]
    pub h_ansi: Option<String>,
    #[serde(rename = "eastAsia", skip_serializing_if = "Option::is_none")]
    pub east_asian: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cs: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DocxInd {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<i32>,
    #[serde(rename = "firstLine", skip_serializing_if = "Option::is_none")]
    pub first_line: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hanging: Option<i32>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DocxSpacing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    #[serde(rename = "lineRule", skip_serializing_if = "Option::is_none")]
    pub line_rule: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    #[serde(rename = "trHeight", skip_serializing_if = "Option::is_none")]
    pub tr_height: Option<DocxTrHeight>,
    #[serde(rename = "cantSplit", skip_serializing_if = "Option::is_none")]
    pub cant_split: Option<bool>,
    #[serde(rename = "tblHeader", skip_serializing_if = "Option::is_none")]
    pub tbl_header: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocxTrHeight {
    pub val: i32,
    #[serde(rename = "hRule", skip_serializing_if = "Option::is_none")]
    pub h_rule: Option<String>,
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
    #[serde(rename = "tcW", skip_serializing_if = "Option::is_none")]
    pub tcw: Option<DocxTcW>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shd: Option<String>,
    #[serde(rename = "tcMar", skip_serializing_if = "Option::is_none")]
    pub tc_mar: Option<String>,
    #[serde(rename = "vAlign", skip_serializing_if = "Option::is_none")]
    pub v_align: Option<String>,
    #[serde(rename = "noWrap", skip_serializing_if = "Option::is_none")]
    pub no_wrap: Option<bool>,
    #[serde(rename = "tcBorders", skip_serializing_if = "Option::is_none")]
    pub tc_borders: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocxTcW {
    pub w: i32,
    #[serde(rename = "type")]
    pub type_: String,
}

// ---------------------------------------------------------------------------
// 共通
// ---------------------------------------------------------------------------

/// 未知要素の生 XML 保持（escape hatch）
#[derive(Debug, Clone, Serialize)]
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
    Pptx {
        slides: usize,
        shapes: usize,
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
    // pptx
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slide: Option<u32>,
    #[serde(rename = "zOrder", skip_serializing_if = "Option::is_none")]
    pub z_order: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
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
