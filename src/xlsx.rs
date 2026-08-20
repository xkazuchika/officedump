//! xlsx（SpreadsheetML）プロファイル: zip + XML を自前で剥がし、IR を構築する。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde_json::Value;

use crate::error::AppError;
use crate::ir::{
    AlignmentDef, Cell, CellStyle, CellStyleDef, FormulaMeta, NumFmtDef, ProtectionDef, RawElement,
    RowInfo, WorkbookStyles, XfDef, XlsxRichRun, XlsxRunProps,
};
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

/// 共有文字列テーブルの1エントリ。連結テキストと、リッチテキストランを持つ。
pub struct SharedString {
    pub text: String,
    pub runs: Option<Vec<XlsxRichRun>>,
}

/// 共有文字列テーブル。リッチテキストランの書式を保持し、振り仮名（<rPh>）は除く。
pub fn parse_shared_strings(xml: &str) -> Result<Vec<SharedString>, AppError> {
    let mut strings = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut in_si = false;
    let mut in_r = false;
    let mut in_rph = false;
    let mut in_t = false;
    let mut current_text = String::new();
    let mut runs: Vec<XlsxRichRun> = Vec::new();
    let mut run_text = String::new();
    let mut run_rpr: Option<XlsxRunProps> = None;
    let mut has_runs = false;
    loop {
        let prev_pos = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"si" => {
                        in_si = true;
                        current_text.clear();
                        runs.clear();
                        has_runs = false;
                    }
                    b"r" if in_si && !in_rph => {
                        in_r = true;
                        run_text.clear();
                        run_rpr = None;
                        has_runs = true;
                    }
                    b"rPr" if in_r && !in_rph => {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        run_rpr = Some(parse_rpr(&raw.xml));
                        continue;
                    }
                    b"t" if in_si && !in_rph => {
                        in_t = true;
                    }
                    b"rPh" => in_rph = true,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"si" => {
                        let text = std::mem::take(&mut current_text);
                        let r = if has_runs {
                            Some(std::mem::take(&mut runs))
                        } else {
                            None
                        };
                        strings.push(SharedString { text, runs: r });
                        in_si = false;
                    }
                    b"r" if in_r => {
                        let text = std::mem::take(&mut run_text);
                        let rpr = run_rpr.take();
                        runs.push(XlsxRichRun { text, rpr });
                        in_r = false;
                    }
                    b"t" if in_t => in_t = false,
                    b"rPh" => in_rph = false,
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_si && !in_rph && in_t => {
                let text = text_content(&t);
                if in_r {
                    run_text.push_str(&text);
                }
                current_text.push_str(&text);
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

pub type Styles = WorkbookStyles;

#[derive(Default, PartialEq)]
enum StyleSection {
    #[default]
    None,
    Fonts,
    Fills,
    Borders,
    CellStyleXfs,
    CellXfs,
    CellStyles,
}

pub fn parse_styles(xml: &str) -> Result<Styles, AppError> {
    let mut styles = Styles::default();
    let mut reader = Reader::from_str(xml);
    let mut section = StyleSection::None;
    loop {
        let prev_pos = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"fonts" => section = StyleSection::Fonts,
                    b"fills" => section = StyleSection::Fills,
                    b"borders" => section = StyleSection::Borders,
                    b"cellStyleXfs" => section = StyleSection::CellStyleXfs,
                    b"cellXfs" => section = StyleSection::CellXfs,
                    b"cellStyles" => section = StyleSection::CellStyles,
                    b"numFmt" => {
                        push_num_fmt(&e, &mut styles);
                    }
                    b"font" if section == StyleSection::Fonts => {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        styles.fonts.push(raw);
                        continue;
                    }
                    b"fill" if section == StyleSection::Fills => {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        styles.fills.push(raw);
                        continue;
                    }
                    b"border" if section == StyleSection::Borders => {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        styles.borders.push(raw);
                        continue;
                    }
                    b"xf"
                        if section == StyleSection::CellStyleXfs
                            || section == StyleSection::CellXfs =>
                    {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        let xf_def = parse_xf(&raw.xml);
                        if section == StyleSection::CellStyleXfs {
                            styles.cell_style_xfs.push(xf_def);
                        } else {
                            styles.cell_xfs.push(xf_def);
                        }
                        continue;
                    }
                    b"cellStyle" if section == StyleSection::CellStyles => {
                        let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                        styles.cell_styles.push(parse_cell_style(&raw.xml));
                        continue;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"numFmt" => {
                        push_num_fmt(&e, &mut styles);
                    }
                    b"xf" if section == StyleSection::CellStyleXfs => {
                        styles.cell_style_xfs.push(read_xf_attrs(&e));
                    }
                    b"xf" if section == StyleSection::CellXfs => {
                        styles.cell_xfs.push(read_xf_attrs(&e));
                    }
                    b"cellStyle" if section == StyleSection::CellStyles => {
                        styles
                            .cell_styles
                            .push(read_cell_style_attrs(&e));
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if matches!(
                    lname,
                    b"fonts" | b"fills" | b"borders" | b"cellStyleXfs" | b"cellXfs" | b"cellStyles"
                ) {
                    section = StyleSection::None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("styles.xml のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(styles)
}

fn push_num_fmt(e: &BytesStart, styles: &mut Styles) {
    if let Some(id) = attr_value(e, "numFmtId").and_then(|s| s.parse::<u32>().ok()) {
        let code = attr_value(e, "formatCode16")
            .or_else(|| attr_value(e, "formatCode"))
            .unwrap_or_default();
        styles.num_fmts.push(NumFmtDef { id, code });
    }
}

fn attr_bool_opt(e: &BytesStart, key: &str) -> Option<bool> {
    attr_value(e, key).map(|v| v == "1" || v == "true")
}

fn read_xf_attrs(e: &BytesStart) -> XfDef {
    XfDef {
        num_fmt_id: attr_value(e, "numFmtId").and_then(|s| s.parse().ok()),
        font_id: attr_value(e, "fontId").and_then(|s| s.parse().ok()),
        fill_id: attr_value(e, "fillId").and_then(|s| s.parse().ok()),
        border_id: attr_value(e, "borderId").and_then(|s| s.parse().ok()),
        xf_id: attr_value(e, "xfId").and_then(|s| s.parse().ok()),
        apply_number_format: attr_bool_opt(e, "applyNumberFormat"),
        apply_font: attr_bool_opt(e, "applyFont"),
        apply_fill: attr_bool_opt(e, "applyFill"),
        apply_border: attr_bool_opt(e, "applyBorder"),
        apply_alignment: attr_bool_opt(e, "applyAlignment"),
        apply_protection: attr_bool_opt(e, "applyProtection"),
        quote_prefix: attr_bool_opt(e, "quotePrefix"),
        pivot_button: attr_bool_opt(e, "pivotButton"),
        alignment: None,
        protection: None,
    }
}

fn parse_xf(xf_xml: &str) -> XfDef {
    let mut reader = Reader::from_str(xf_xml);
    let mut xf = XfDef::default();
    let mut found = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if lname == b"xf" && !found {
                    found = true;
                    xf = read_xf_attrs(&e);
                } else if lname == b"alignment" {
                    xf.alignment = Some(read_alignment_attrs(&e));
                } else if lname == b"protection" {
                    xf.protection = Some(read_protection_attrs(&e));
                }
            }
            Ok(Event::End(e)) => {
                if local_name(e.name().as_ref()) == b"xf" && found {
                    return xf;
                }
            }
            Ok(Event::Eof) => return xf,
            Err(_) => return xf,
            _ => {}
        }
    }
}

fn read_alignment_attrs(e: &BytesStart) -> AlignmentDef {
    AlignmentDef {
        horizontal: attr_value(e, "horizontal"),
        vertical: attr_value(e, "vertical"),
        wrap_text: attr_bool_opt(e, "wrapText"),
        text_rotation: attr_value(e, "textRotation").and_then(|s| s.parse().ok()),
        indent: attr_value(e, "indent").and_then(|s| s.parse().ok()),
        relative_indent: attr_value(e, "relativeIndent").and_then(|s| s.parse().ok()),
        shrink_to_fit: attr_bool_opt(e, "shrinkToFit"),
        justify_last_line: attr_bool_opt(e, "justifyLastLine"),
        reading_order: attr_value(e, "readingOrder").and_then(|s| s.parse().ok()),
    }
}

fn read_protection_attrs(e: &BytesStart) -> ProtectionDef {
    ProtectionDef {
        locked: attr_bool_opt(e, "locked"),
        hidden: attr_bool_opt(e, "hidden"),
    }
}

fn read_cell_style_attrs(e: &BytesStart) -> CellStyleDef {
    CellStyleDef {
        name: attr_value(e, "name").unwrap_or_default(),
        xf_id: attr_value(e, "xfId").and_then(|s| s.parse().ok()).unwrap_or(0),
        builtin_id: attr_value(e, "builtinId").and_then(|s| s.parse().ok()),
        custom_builtin: attr_bool_opt(e, "customBuiltin"),
        hidden: attr_bool_opt(e, "hidden"),
        custom_locked: attr_bool_opt(e, "customLocked"),
    }
}

fn parse_cell_style(cs_xml: &str) -> CellStyleDef {
    let mut reader = Reader::from_str(cs_xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == b"cellStyle" {
                    return read_cell_style_attrs(&e);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    CellStyleDef::default()
}

fn parse_rpr(rpr_xml: &str) -> XlsxRunProps {
    let mut props = XlsxRunProps::default();
    let mut reader = Reader::from_str(rpr_xml);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"b" => props.b = Some(attr_bool_val(&e, true)),
                    b"i" => props.i = Some(attr_bool_val(&e, true)),
                    b"u" => props.u = Some(attr_value(&e, "val").unwrap_or_else(|| "single".to_string())),
                    b"strike" => {
                        props.strike = Some(attr_value(&e, "val").unwrap_or_else(|| "true".to_string()))
                    }
                    b"sz" => props.sz = attr_value(&e, "val").and_then(|s| s.parse().ok()),
                    b"color" => {
                        props.color = attr_value(&e, "rgb")
                            .or_else(|| attr_value(&e, "theme"))
                            .or_else(|| attr_value(&e, "indexed"))
                            .or_else(|| attr_value(&e, "auto"))
                    }
                    b"rFont" => props.rfont = attr_value(&e, "val"),
                    b"family" => props.family = attr_value(&e, "val").and_then(|s| s.parse().ok()),
                    b"charset" => props.charset = attr_value(&e, "val").and_then(|s| s.parse().ok()),
                    b"scheme" => props.scheme = attr_value(&e, "val"),
                    b"vertAlign" => props.vert_align = attr_value(&e, "val"),
                    b"outline" => props.outline = Some(attr_bool_val(&e, true)),
                    b"shadow" => props.shadow = Some(attr_bool_val(&e, true)),
                    b"condense" => props.condense = Some(attr_bool_val(&e, true)),
                    b"extend" => props.extend = Some(attr_bool_val(&e, true)),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    props
}

fn attr_bool_val(e: &BytesStart, default: bool) -> bool {
    match attr_value(e, "val") {
        Some(v) => v != "0" && v != "false",
        None => default,
    }
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
    pub rows: Vec<RowInfo>,
}

#[derive(Default)]
struct CellDraft {
    reference: String,
    cell_type: String,
    xf: Option<u32>,
    formula: Option<String>,
    formula_meta: Option<FormulaMeta>,
    raw_value: String,
    inline_text: String,
    inline_runs: Option<Vec<XlsxRichRun>>,
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

fn build_row_info(e: &BytesStart, r_val: u32) -> Option<RowInfo> {
    let info = RowInfo {
        r: r_val,
        spans: attr_value(e, "spans"),
        s: attr_value(e, "s").and_then(|s| s.parse().ok()),
        custom_format: attr_bool_opt(e, "customFormat"),
        ht: attr_value(e, "ht").and_then(|s| s.parse().ok()),
        custom_height: attr_bool_opt(e, "customHeight"),
        hidden: attr_bool_opt(e, "hidden"),
        outline_level: attr_value(e, "outlineLevel").and_then(|s| s.parse().ok()),
        collapsed: attr_bool_opt(e, "collapsed"),
        thick_top: attr_bool_opt(e, "thickTop"),
        thick_bottom: attr_bool_opt(e, "thickBottom"),
    };
    if info.spans.is_some()
        || info.s.is_some()
        || info.custom_format.is_some()
        || info.ht.is_some()
        || info.custom_height.is_some()
        || info.hidden.is_some()
        || info.outline_level.is_some()
        || info.collapsed.is_some()
        || info.thick_top.is_some()
        || info.thick_bottom.is_some()
    {
        Some(info)
    } else {
        None
    }
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

fn cell_value(d: &CellDraft, shared: &[SharedString]) -> Value {
    match d.cell_type.as_str() {
        "s" => d
            .raw_value
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i))
            .map(|s| Value::String(s.text.clone()))
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
    mut d: CellDraft,
    shared: &[SharedString],
    styles: &Styles,
    filter: &RangeFilter,
) -> Option<Cell> {
    let (col, row) = parse_cell_ref(&d.reference).ok()?;
    if !filter.contains(col, row) {
        return None;
    }
    let style = d.xf.map(|xf| {
        let num_fmt_id = styles
            .cell_xfs
            .get(xf as usize)
            .and_then(|xf_def| xf_def.num_fmt_id)
            .unwrap_or(0);
        let format_code = styles
            .num_fmts
            .iter()
            .find(|f| f.id == num_fmt_id)
            .map(|f| f.code.clone());
        CellStyle {
            xf,
            num_fmt_id,
            format_code,
        }
    });

    let runs = if d.cell_type == "s" {
        d.raw_value
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i))
            .and_then(|s| s.runs.clone())
    } else {
        d.inline_runs.take()
    };

    let value = cell_value(&d, shared);

    Some(Cell {
        reference: d.reference,
        cell_type: d.cell_type,
        value,
        formula: d.formula,
        formula_meta: d.formula_meta,
        style,
        runs,
    })
}

/// worksheet XML をパースする。
/// 既知の直接子要素（dimension / sheetData / mergeCells / drawing）以外は
/// 生 XML のまま `unhandled` に保持する（情報を落とさない）。
pub fn parse_worksheet(
    xml: &str,
    filter: &RangeFilter,
    shared: &[SharedString],
    styles: &Styles,
) -> Result<SheetParse, AppError> {
    let mut result = SheetParse {
        dimension: None,
        cells: Vec::new(),
        merged: Vec::new(),
        unhandled: Vec::new(),
        drawing_rid: None,
        rows: Vec::new(),
    };

    let mut reader = Reader::from_str(xml);
    let mut depth: u32 = 0;
    let mut prev_pos: u64;

    let mut draft: Option<CellDraft> = None;
    let mut in_f = false;
    let mut in_v = false;
    let mut in_is = false;
    let mut in_is_r = false;
    let mut in_is_t = false;
    let mut cur_row: u32 = 0;
    let mut last_col_in_row: u32 = 0;
    let mut is_runs: Vec<XlsxRichRun> = Vec::new();
    let mut is_run_text = String::new();
    let mut is_run_rpr: Option<XlsxRunProps> = None;

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
                    let r_val = attr_value(&e, "r")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(cur_row + 1);
                    cur_row = r_val;
                    last_col_in_row = 0;
                    if let Some(info) = build_row_info(&e, r_val) {
                        result.rows.push(info);
                    }
                } else if lname == b"c" && draft.is_none() {
                    draft = Some(build_draft(&e, &mut cur_row, &mut last_col_in_row));
                } else if lname == b"mergeCell" && depth == 2 {
                    if let Some(r) = attr_value(&e, "ref") {
                        result.merged.push(r);
                    }
                } else if draft.is_some() {
                    match lname {
                        b"f" => {
                            in_f = true;
                            if let Some(d) = draft.as_mut() {
                                d.formula_meta = Some(FormulaMeta {
                                    t: attr_value(&e, "t"),
                                    r#ref: attr_value(&e, "ref"),
                                    si: attr_value(&e, "si").and_then(|s| s.parse().ok()),
                                    aca: attr_bool_opt(&e, "aca"),
                                    bx: attr_bool_opt(&e, "bx"),
                                    ca: attr_bool_opt(&e, "ca"),
                                });
                            }
                        }
                        b"v" => in_v = true,
                        b"is" => {
                            in_is = true;
                            is_runs.clear();
                        }
                        b"r" if in_is => {
                            in_is_r = true;
                            is_run_text.clear();
                            is_run_rpr = None;
                        }
                        b"rPr" if in_is_r => {
                            let raw = capture_element(&mut reader, xml, prev_pos, lname)?;
                            is_run_rpr = Some(parse_rpr(&raw.xml));
                            continue;
                        }
                        b"t" if in_is => {
                            in_is_t = true;
                        }
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
                } else if lname == b"f" && draft.is_some()
                    && let Some(d) = draft.as_mut()
                {
                    d.formula_meta = Some(FormulaMeta {
                        t: attr_value(&e, "t"),
                        r#ref: attr_value(&e, "ref"),
                        si: attr_value(&e, "si").and_then(|s| s.parse().ok()),
                        aca: attr_bool_opt(&e, "aca"),
                        bx: attr_bool_opt(&e, "bx"),
                        ca: attr_bool_opt(&e, "ca"),
                    });
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
                    } else if in_is && in_is_t {
                        let text = text_content(&t);
                        if in_is_r {
                            is_run_text.push_str(&text);
                        }
                        d.inline_text.push_str(&text);
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
                    b"is" => {
                        in_is = false;
                        if !is_runs.is_empty()
                            && let Some(d) = draft.as_mut()
                        {
                            d.inline_runs = Some(std::mem::take(&mut is_runs));
                        }
                        in_is_r = false;
                        in_is_t = false;
                    }
                    b"t" if in_is_t => in_is_t = false,
                    b"r" if in_is_r => {
                        let text = std::mem::take(&mut is_run_text);
                        let rpr = is_run_rpr.take();
                        is_runs.push(XlsxRichRun { text, rpr });
                        in_is_r = false;
                    }
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
