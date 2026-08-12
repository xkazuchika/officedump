//! xlsx（SpreadsheetML）プロファイル: zip + XML を自前で剥がし、IR を構築する。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde_json::Value;

use crate::error::AppError;
use crate::ir::{Cell, CellStyle, RawElement};
use crate::range::{RangeFilter, num_to_col, parse_cell_ref};
use crate::xmlutil::{attr_value, capture_element, local_name, raw_slice, text_content};

// ---------------------------------------------------------------------------
// workbook / sharedStrings / styles
// ---------------------------------------------------------------------------

pub struct SheetMeta {
    pub name: String,
    pub path: String,
}

pub fn parse_workbook(
    xml: &str,
    rels: &HashMap<String, String>,
) -> Result<Vec<SheetMeta>, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut sheets = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if local_name(e.name().as_ref()) == b"sheet" =>
            {
                let name = attr_value(&e, "name").unwrap_or_default();
                let rid = attr_value(&e, "r:id").ok_or_else(|| {
                    AppError::InvalidXlsx(format!("シート '{name}' に r:id がありません"))
                })?;
                let target = rels.get(&rid).ok_or_else(|| {
                    AppError::InvalidXlsx(format!(
                        "シート '{name}' のリレーション {rid} が見つかりません"
                    ))
                })?;
                sheets.push(SheetMeta {
                    name,
                    path: crate::xmlutil::resolve_target("xl", target),
                });
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("workbook.xml のパース失敗: {e}"))),
            _ => {}
        }
    }
    if sheets.is_empty() {
        return Err(AppError::InvalidXlsx("シートが1枚もありません".to_string()));
    }
    Ok(sheets)
}

/// 共有文字列テーブル。リッチテキスト（<r> 連結）は連結し、振り仮名（<rPh>）は除く。
pub fn parse_shared_strings(xml: &str) -> Result<Vec<String>, AppError> {
    let mut strings = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut in_si = false;
    let mut in_rph = false;
    let mut current = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"si" => {
                        in_si = true;
                        current.clear();
                    }
                    b"rPh" => in_rph = true,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"si" => {
                        strings.push(std::mem::take(&mut current));
                        in_si = false;
                    }
                    b"rPh" => in_rph = false,
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_si && !in_rph => {
                current.push_str(&text_content(&t));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AppError::Xml(format!(
                    "sharedStrings.xml のパース失敗: {e}"
                )));
            }
            _ => {}
        }
    }
    Ok(strings)
}

#[derive(Default)]
pub struct Styles {
    /// カスタム書式: numFmtId -> formatCode
    pub custom_fmts: HashMap<u32, String>,
    /// cellXfs: xf インデックス -> numFmtId
    pub cell_xfs: Vec<u32>,
}

pub fn parse_styles(xml: &str) -> Result<Styles, AppError> {
    let mut styles = Styles::default();
    let mut reader = Reader::from_str(xml);
    let mut in_cell_xfs = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"cellXfs" => {
                in_cell_xfs = true;
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == b"cellXfs" => {
                in_cell_xfs = false;
            }
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if local_name(e.name().as_ref()) == b"numFmt" =>
            {
                if let (Some(id), Some(code)) =
                    (attr_value(&e, "numFmtId"), attr_value(&e, "formatCode"))
                    && let Ok(id) = id.parse::<u32>()
                {
                    styles.custom_fmts.insert(id, code);
                }
            }
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if in_cell_xfs && local_name(e.name().as_ref()) == b"xf" =>
            {
                let id = attr_value(&e, "numFmtId")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                styles.cell_xfs.push(id);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("styles.xml のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(styles)
}

// ---------------------------------------------------------------------------
// worksheet
// ---------------------------------------------------------------------------

pub struct SheetParse {
    pub dimension: Option<String>,
    pub cells: Vec<Cell>,
    pub merged: Vec<String>,
    pub unhandled: Vec<RawElement>,
    pub drawing_rid: Option<String>,
}

#[derive(Default)]
struct CellDraft {
    reference: String,
    cell_type: String,
    xf: Option<u32>,
    formula: Option<String>,
    raw_value: String,
    inline_text: String,
    has_value: bool,
}

fn build_draft(e: &BytesStart, cur_row: &mut u32, last_col: &mut u32) -> CellDraft {
    let mut d = CellDraft {
        cell_type: "n".to_string(),
        ..Default::default()
    };
    if let Some(r) = attr_value(e, "r") {
        if let Ok((col, row)) = parse_cell_ref(&r) {
            *cur_row = row;
            *last_col = col;
        }
        d.reference = r;
    } else {
        // r 属性を持たないセルは直前セルの右隣とみなす
        *last_col += 1;
        d.reference = format!("{}{}", num_to_col(*last_col), *cur_row);
    }
    if let Some(t) = attr_value(e, "t") {
        d.cell_type = t;
    }
    d.xf = attr_value(e, "s").and_then(|s| s.parse().ok());
    d
}

fn parse_number(s: &str) -> Option<Value> {
    if let Ok(i) = s.parse::<i64>() {
        Some(Value::from(i))
    } else if let Ok(u) = s.parse::<u64>() {
        Some(Value::from(u))
    } else {
        s.parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
    }
}

fn cell_value(d: &CellDraft, shared: &[String]) -> Value {
    match d.cell_type.as_str() {
        "s" => d
            .raw_value
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i))
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
        "inlineStr" => Value::String(d.inline_text.clone()),
        "b" => Value::Bool(d.raw_value.trim() == "1"),
        "n" => {
            if d.has_value {
                parse_number(d.raw_value.trim()).unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        // str / e / d / その他の型: 生テキストを文字列のまま保持する（解釈しない）
        _ => {
            if d.has_value {
                Value::String(d.raw_value.clone())
            } else {
                Value::Null
            }
        }
    }
}

fn finalize_cell(
    d: CellDraft,
    shared: &[String],
    styles: &Styles,
    filter: &RangeFilter,
) -> Option<Cell> {
    let (col, row) = parse_cell_ref(&d.reference).ok()?;
    if !filter.contains(col, row) {
        return None;
    }
    let style = d.xf.map(|xf| {
        let num_fmt_id = styles.cell_xfs.get(xf as usize).copied().unwrap_or(0);
        let format_code = styles.custom_fmts.get(&num_fmt_id).cloned();
        CellStyle {
            xf,
            num_fmt_id,
            format_code,
        }
    });
    Some(Cell {
        reference: d.reference.clone(),
        cell_type: d.cell_type.clone(),
        value: cell_value(&d, shared),
        formula: d.formula,
        style,
    })
}

/// worksheet XML をパースする。
/// 既知の直接子要素（dimension / sheetData / mergeCells / drawing）以外は
/// 生 XML のまま `unhandled` に保持する（情報を落とさない）。
pub fn parse_worksheet(
    xml: &str,
    filter: &RangeFilter,
    shared: &[String],
    styles: &Styles,
) -> Result<SheetParse, AppError> {
    let mut result = SheetParse {
        dimension: None,
        cells: Vec::new(),
        merged: Vec::new(),
        unhandled: Vec::new(),
        drawing_rid: None,
    };

    let mut reader = Reader::from_str(xml);
    let mut depth: u32 = 0;
    let mut prev_pos: u64;

    let mut draft: Option<CellDraft> = None;
    let mut in_f = false;
    let mut in_v = false;
    let mut in_is = false;
    let mut cur_row: u32 = 0;
    let mut last_col_in_row: u32 = 0;

    loop {
        prev_pos = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if depth == 1 {
                    // worksheet の直接子
                    match lname {
                        b"sheetData" | b"mergeCells" => {}
                        b"dimension" => result.dimension = attr_value(&e, "ref"),
                        b"drawing" => result.drawing_rid = attr_value(&e, "r:id"),
                        _ => {
                            let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                            result.unhandled.push(raw);
                            continue; // サブツリーごと消費済みなので depth は動かさない
                        }
                    }
                } else if lname == b"row" {
                    cur_row = attr_value(&e, "r")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(cur_row + 1);
                    last_col_in_row = 0;
                } else if lname == b"c" && draft.is_none() {
                    draft = Some(build_draft(&e, &mut cur_row, &mut last_col_in_row));
                } else if draft.is_some() {
                    match lname {
                        b"f" => in_f = true,
                        b"v" => in_v = true,
                        b"is" => in_is = true,
                        _ => {}
                    }
                }
                depth += 1;
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if depth == 1 {
                    match lname {
                        b"dimension" => result.dimension = attr_value(&e, "ref"),
                        b"drawing" => result.drawing_rid = attr_value(&e, "r:id"),
                        b"sheetData" | b"mergeCells" | b"mergeCell" | b"row" | b"c" => {}
                        _ => {
                            let end = reader.buffer_position();
                            result.unhandled.push(raw_slice(xml, prev_pos, end, lname));
                        }
                    }
                } else if lname == b"mergeCell" {
                    if let Some(r) = attr_value(&e, "ref") {
                        result.merged.push(r);
                    }
                } else if lname == b"c" && draft.is_none() {
                    let d = build_draft(&e, &mut cur_row, &mut last_col_in_row);
                    if let Some(cell) = finalize_cell(d, shared, styles, filter) {
                        result.cells.push(cell);
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(d) = draft.as_mut() {
                    if in_v {
                        d.raw_value.push_str(&text_content(&t));
                        d.has_value = true;
                    } else if in_f {
                        d.formula
                            .get_or_insert_with(String::new)
                            .push_str(&text_content(&t));
                    } else if in_is {
                        d.inline_text.push_str(&text_content(&t));
                        d.has_value = true;
                    }
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"f" => in_f = false,
                    b"v" => in_v = false,
                    b"is" => in_is = false,
                    b"c" => {
                        if let Some(d) = draft.take()
                            && let Some(cell) = finalize_cell(d, shared, styles, filter)
                        {
                            result.cells.push(cell);
                        }
                    }
                    _ => {}
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("ワークシートのパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// inspect 用
// ---------------------------------------------------------------------------

pub fn sheet_dimension(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if local_name(e.name().as_ref()) == b"dimension" =>
            {
                return attr_value(&e, "ref");
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
    }
}

/// "A1:H1200" -> (1200, 8)
pub fn extent_from_dimension(dim: &str) -> Option<(u64, u64)> {
    let end = dim.rsplit(':').next()?;
    let (col, row) = parse_cell_ref(end).ok()?;
    Some((row as u64, col as u64))
}

/// dimension が無い場合のフォールバック: 全走査で最大行・最大列を確定する。
pub fn scan_extent(xml: &str) -> Result<(u64, u64), AppError> {
    let mut reader = Reader::from_str(xml);
    let mut max_row: u32 = 0;
    let mut max_col: u32 = 0;
    let mut cur_row: u32 = 0;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if lname == b"row" {
                    cur_row = attr_value(&e, "r")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(cur_row + 1);
                    max_row = max_row.max(cur_row);
                } else if lname == b"c"
                    && let Some(r) = attr_value(&e, "r")
                    && let Ok((col, row)) = parse_cell_ref(&r)
                {
                    max_col = max_col.max(col);
                    max_row = max_row.max(row);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("ワークシートの走査失敗: {e}"))),
            _ => {}
        }
    }
    Ok((max_row as u64, max_col as u64))
}
