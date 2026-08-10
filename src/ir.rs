//! 中間表現（IR）。構造は正規化し、属性は全保持し、意味の解釈は行わない。

use serde::Serialize;
use serde_json::Value;

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

/// 未知要素の生 XML 保持（escape hatch）
#[derive(Debug, Serialize)]
pub struct RawElement {
    pub name: String,
    pub xml: String,
    #[serde(skip_serializing_if = "is_false")]
    pub truncated: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Serialize)]
pub struct MediaItem {
    #[serde(rename = "ref")]
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<MediaAnchor>,
}

#[derive(Debug, Serialize)]
pub struct MediaAnchor {
    pub sheet: String,
    #[serde(rename = "anchorType")]
    pub anchor_type: String,
    /// xlsx の図はすべてセルに対して浮遊（フローティング）
    pub placement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<AnchorPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<AnchorPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<AnchorPos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<AnchorSize>,
}

/// アンカーの基点。col/row は 0 始まり、オフセットは EMU（生値）。
#[derive(Debug, Serialize)]
pub struct AnchorPoint {
    pub col: u32,
    pub row: u32,
    #[serde(rename = "colOffEmu")]
    pub col_off_emu: i64,
    #[serde(rename = "rowOffEmu")]
    pub row_off_emu: i64,
}

#[derive(Debug, Serialize)]
pub struct AnchorPos {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Serialize)]
pub struct AnchorSize {
    pub cx: i64,
    pub cy: i64,
}
