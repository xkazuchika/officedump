//! pptx（PresentationML）プロファイル。図形の読み順は決めず、zOrderとgeometryを保持する。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::AppError;
use crate::ir::{
    DocxRFonts, Geometry, PptxParagraph, PptxPlaceholder, PptxShape, PptxSlide, PptxTable,
    PptxTableCell, PptxTableRow, PptxTextFrame, TextRun,
};
use crate::xmlutil::{
    attr_value, capture_element, local_name, raw_slice, resolve_target, text_content,
};

pub struct SlideMeta {
    pub index: u32,
    pub path: String,
}

pub struct PptxMediaDraft {
    pub slide: u32,
    pub z_order: u32,
    pub geometry: Option<Geometry>,
    pub name: Option<String>,
    pub embed_rid: String,
}

pub struct SlideParse {
    pub slide: PptxSlide,
    pub media: Vec<PptxMediaDraft>,
}

pub fn parse_presentation(
    xml: &str,
    rels: &HashMap<String, String>,
) -> Result<Vec<SlideMeta>, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut slides = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e))
                if local_name(e.name().as_ref()) == b"sldId" =>
            {
                let rid = attr_value(&e, "r:id").ok_or_else(|| {
                    AppError::InvalidPptx("sldId に r:id がありません".to_string())
                })?;
                let target = rels.get(&rid).ok_or_else(|| {
                    AppError::InvalidPptx(format!("スライドのリレーション {rid} が見つかりません"))
                })?;
                slides.push(SlideMeta {
                    index: slides.len() as u32 + 1,
                    path: resolve_target("ppt", target),
                });
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("presentation.xml のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(slides)
}

pub fn parse_slide(xml: &str, index: u32) -> Result<SlideParse, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut depth = 0u32;
    let mut tree_depth = None;
    let mut shapes = Vec::new();
    let mut unhandled = Vec::new();
    let mut media = Vec::new();

    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if lname == b"spTree" {
                    depth += 1;
                    tree_depth = Some(depth);
                    continue;
                }
                if tree_depth == Some(depth) {
                    let z_order = shapes.len() as u32 + 1;
                    match lname {
                        b"sp" => {
                            let raw = capture_element(&mut reader, xml, start, lname)?;
                            shapes.push(parse_shape(&raw.xml, z_order)?);
                        }
                        b"pic" => {
                            let raw = capture_element(&mut reader, xml, start, lname)?;
                            let (shape, draft) = parse_picture(&raw.xml, z_order, index)?;
                            shapes.push(shape);
                            if let Some(draft) = draft {
                                media.push(draft);
                            }
                        }
                        b"graphicFrame" => {
                            let raw = capture_element(&mut reader, xml, start, lname)?;
                            shapes.push(parse_graphic_frame(&raw.xml, z_order)?);
                        }
                        _ => unhandled.push(capture_element(&mut reader, xml, start, lname)?),
                    }
                    continue;
                }
                depth += 1;
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                if tree_depth == Some(depth) {
                    unhandled.push(raw_slice(
                        xml,
                        start,
                        reader.buffer_position(),
                        local_name(qname.as_ref()),
                    ));
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                if local_name(qname.as_ref()) == b"spTree" {
                    tree_depth = None;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("slide XML のパース失敗: {e}"))),
            _ => {}
        }
    }

    Ok(SlideParse {
        slide: PptxSlide {
            index,
            shapes,
            unhandled,
        },
        media,
    })
}

fn parse_shape(xml: &str, z_order: u32) -> Result<PptxShape, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut name = None;
    let mut placeholder = None;
    let mut placeholder_detail = None;
    let mut prst_geom = None;
    let mut sppr_xml = None;
    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"cNvPr" => name = attr_value(&e, "name"),
                    b"ph" => {
                        let ph_type = attr_value(&e, "type");
                        let ph_idx = attr_value(&e, "idx").and_then(|v| v.parse().ok());
                        let ph_sz = attr_value(&e, "sz");
                        let ph_orient = attr_value(&e, "orient");
                        placeholder_detail = Some(PptxPlaceholder {
                            r#type: ph_type.clone().unwrap_or_else(|| "body".to_string()),
                            idx: ph_idx,
                            sz: ph_sz,
                            orient: ph_orient,
                        });
                        placeholder = Some(ph_type.unwrap_or_else(|| "body".to_string()));
                    }
                    b"prstGeom" => prst_geom = attr_value(&e, "prst"),
                    b"spPr" => {
                        let raw = capture_element(&mut reader, xml, start, b"spPr")?;
                        sppr_xml = Some(raw.xml.clone());
                        prst_geom = parse_prst_geom(&raw.xml);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("図形のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(PptxShape {
        shape_type: "shape".to_string(),
        z_order,
        name,
        placeholder,
        placeholder_detail,
        prst_geom,
        sppr_xml,
        geometry: parse_geometry(xml),
        text: parse_text_frame(xml)?,
        table: None,
    })
}

fn parse_picture(
    xml: &str,
    z_order: u32,
    slide: u32,
) -> Result<(PptxShape, Option<PptxMediaDraft>), AppError> {
    let mut reader = Reader::from_str(xml);
    let mut name = None;
    let mut embed_rid = None;
    let mut sppr_xml = None;
    let mut prst_geom = None;
    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"cNvPr" => name = attr_value(&e, "name"),
                    b"blip" => embed_rid = attr_value(&e, "r:embed"),
                    b"spPr" => {
                        let raw = capture_element(&mut reader, xml, start, b"spPr")?;
                        sppr_xml = Some(raw.xml.clone());
                        prst_geom = parse_prst_geom(&raw.xml);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("画像のパース失敗: {e}"))),
            _ => {}
        }
    }
    let geometry = parse_geometry(xml);
    let draft = embed_rid.map(|embed_rid| PptxMediaDraft {
        slide,
        z_order,
        geometry: geometry.clone(),
        name: name.clone(),
        embed_rid,
    });
    Ok((
        PptxShape {
            shape_type: "picture".to_string(),
            z_order,
            name,
            placeholder: None,
            placeholder_detail: None,
            prst_geom,
            sppr_xml,
            geometry,
            text: None,
            table: None,
        },
        draft,
    ))
}

fn parse_graphic_frame(xml: &str, z_order: u32) -> Result<PptxShape, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut name = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e))
                if local_name(e.name().as_ref()) == b"cNvPr" =>
            {
                name = attr_value(&e, "name");
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("graphicFrame のパース失敗: {e}"))),
            _ => {}
        }
    }
    let table = parse_table(xml)?;
    Ok(PptxShape {
        shape_type: "table".to_string(),
        z_order,
        name,
        placeholder: None,
        placeholder_detail: None,
        prst_geom: None,
        sppr_xml: None,
        geometry: parse_geometry(xml),
        text: None,
        table,
    })
}

fn parse_prst_geom(sppr_xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(sppr_xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e))
                if local_name(e.name().as_ref()) == b"prstGeom" =>
            {
                return attr_value(&e, "prst");
            }
            Ok(Event::Eof) => return None,
            _ => {}
        }
    }
}

fn parse_geometry(xml: &str) -> Option<Geometry> {
    let mut reader = Reader::from_str(xml);
    let mut x = None;
    let mut y = None;
    let mut cx = None;
    let mut cy = None;
    let mut rot = None;
    let mut flip_h = None;
    let mut flip_v = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"xfrm" => {
                        rot = attr_value(&e, "rot").and_then(|v| v.parse().ok());
                        flip_h = attr_value(&e, "flipH").map(|v| v == "1" || v == "true");
                        flip_v = attr_value(&e, "flipV").map(|v| v == "1" || v == "true");
                    }
                    b"off" if x.is_none() => {
                        x = attr_value(&e, "x").and_then(|v| v.parse().ok());
                        y = attr_value(&e, "y").and_then(|v| v.parse().ok());
                    }
                    b"ext" if cx.is_none() => {
                        cx = attr_value(&e, "cx").and_then(|v| v.parse().ok());
                        cy = attr_value(&e, "cy").and_then(|v| v.parse().ok());
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
    x.zip(y)
        .zip(cx.zip(cy))
        .map(|((x, y), (cx, cy))| Geometry {
            x,
            y,
            cx,
            cy,
            rot,
            flip_h,
            flip_v,
        })
}

fn parse_text_frame(xml: &str) -> Result<Option<PptxTextFrame>, AppError> {
    if !xml.contains(":txBody") && !xml.contains("<p:txBody") {
        return Ok(None);
    }
    let mut reader = Reader::from_str(xml);
    let mut paragraphs = Vec::new();
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut in_solid_fill = false;
    let mut in_ln_spc = false;
    let mut in_spc_bef = false;
    let mut in_spc_aft = false;
    let mut bold = false;
    let mut italic = false;
    let mut underline = false;
    let mut strike = false;
    let mut sz: Option<u32> = None;
    let mut color: Option<String> = None;
    let mut typeface: Option<String> = None;
    let mut current = String::new();
    let mut runs = Vec::new();
    let mut algn: Option<String> = None;
    let mut lvl: Option<u32> = None;
    let mut mar_l: Option<i64> = None;
    let mut indent: Option<i64> = None;
    let mut bu_char: Option<String> = None;
    let mut bu_auto_num: Option<String> = None;
    let mut ln_spc: Option<i64> = None;
    let mut spc_bef: Option<i64> = None;
    let mut spc_aft: Option<i64> = None;
    let mut bodypr_xml: Option<String> = None;
    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"bodyPr" => {
                        let raw = capture_element(&mut reader, xml, start, b"bodyPr")?;
                        bodypr_xml = Some(raw.xml);
                    }
                    b"p" => {
                        in_paragraph = true;
                        runs.clear();
                        algn = None;
                        lvl = None;
                        mar_l = None;
                        indent = None;
                        bu_char = None;
                        bu_auto_num = None;
                        ln_spc = None;
                        spc_bef = None;
                        spc_aft = None;
                    }
                    b"pPr" => {
                        algn = attr_value(&e, "algn");
                        lvl = attr_value(&e, "lvl").and_then(|v| v.parse().ok());
                        mar_l = attr_value(&e, "marL").and_then(|v| v.parse().ok());
                        indent = attr_value(&e, "indent").and_then(|v| v.parse().ok());
                    }
                    b"buChar" => bu_char = attr_value(&e, "char"),
                    b"buAutoNum" => bu_auto_num = attr_value(&e, "type"),
                    b"lnSpc" => in_ln_spc = true,
                    b"spcBef" => in_spc_bef = true,
                    b"spcAft" => in_spc_aft = true,
                    b"spcPct" | b"spcPts" if in_ln_spc || in_spc_bef || in_spc_aft => {
                        let val = attr_value(&e, "val").and_then(|v| v.parse().ok());
                        if in_ln_spc {
                            ln_spc = val;
                        } else if in_spc_bef {
                            spc_bef = val;
                        } else if in_spc_aft {
                            spc_aft = val;
                        }
                    }
                    b"r" => {
                        current.clear();
                        bold = false;
                        italic = false;
                        underline = false;
                        strike = false;
                        sz = None;
                        color = None;
                        typeface = None;
                    }
                    b"rPr" => {
                        bold = attr_value(&e, "b").as_deref() == Some("1");
                        italic = attr_value(&e, "i").as_deref() == Some("1");
                        underline = attr_value(&e, "u").map(|v| v != "none").unwrap_or(false);
                        strike = attr_value(&e, "strike").map(|v| v != "noStrike").unwrap_or(false);
                        sz = attr_value(&e, "sz").and_then(|v| v.parse().ok());
                    }
                    b"solidFill" => in_solid_fill = true,
                    b"srgbClr" if in_solid_fill => color = attr_value(&e, "val"),
                    b"latin" => typeface = attr_value(&e, "typeface"),
                    b"t" => in_text = true,
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"bodyPr" => {
                        let end = reader.buffer_position();
                        bodypr_xml = Some(raw_slice(xml, start, end, b"bodyPr").xml);
                    }
                    b"pPr" => {
                        algn = attr_value(&e, "algn");
                        lvl = attr_value(&e, "lvl").and_then(|v| v.parse().ok());
                        mar_l = attr_value(&e, "marL").and_then(|v| v.parse().ok());
                        indent = attr_value(&e, "indent").and_then(|v| v.parse().ok());
                    }
                    b"buChar" => bu_char = attr_value(&e, "char"),
                    b"buAutoNum" => bu_auto_num = attr_value(&e, "type"),
                    b"spcPct" | b"spcPts" if in_ln_spc || in_spc_bef || in_spc_aft => {
                        let val = attr_value(&e, "val").and_then(|v| v.parse().ok());
                        if in_ln_spc {
                            ln_spc = val;
                        } else if in_spc_bef {
                            spc_bef = val;
                        } else if in_spc_aft {
                            spc_aft = val;
                        }
                    }
                    b"rPr" => {
                        bold = attr_value(&e, "b").as_deref() == Some("1");
                        italic = attr_value(&e, "i").as_deref() == Some("1");
                        underline = attr_value(&e, "u").map(|v| v != "none").unwrap_or(false);
                        strike = attr_value(&e, "strike").map(|v| v != "noStrike").unwrap_or(false);
                        sz = attr_value(&e, "sz").and_then(|v| v.parse().ok());
                    }
                    b"srgbClr" if in_solid_fill => color = attr_value(&e, "val"),
                    b"latin" => typeface = attr_value(&e, "typeface"),
                    b"t" => {}
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"rPr" => {}
                    b"solidFill" => in_solid_fill = false,
                    b"pPr" => {}
                    b"lnSpc" => in_ln_spc = false,
                    b"spcBef" => in_spc_bef = false,
                    b"spcAft" => in_spc_aft = false,
                    b"t" => in_text = false,
                    b"r" => {
                        if !current.is_empty() {
                            runs.push(TextRun {
                                text: std::mem::take(&mut current),
                                style: None,
                                bold,
                                italic,
                                underline,
                                strike,
                                sz,
                                color: color.take(),
                                rfonts: typeface.take().map(|tf| DocxRFonts {
                                    ascii: Some(tf),
                                    h_ansi: None,
                                    east_asian: None,
                                    cs: None,
                                }),
                                vert_align: None,
                                spacing: None,
                                kern: None,
                                position: None,
                                rpr_xml: None,
                            });
                        }
                    }
                    b"p" if in_paragraph => {
                        paragraphs.push(PptxParagraph {
                            runs: std::mem::take(&mut runs),
                            algn: algn.take(),
                            lvl: lvl.take(),
                            bu_char: bu_char.take(),
                            bu_auto_num: bu_auto_num.take(),
                            mar_l: mar_l.take(),
                            indent: indent.take(),
                            ln_spc: ln_spc.take(),
                            spc_bef: spc_bef.take(),
                            spc_aft: spc_aft.take(),
                        });
                        in_paragraph = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_text => current.push_str(&text_content(&t)),
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("テキスト図形のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(Some(PptxTextFrame {
        paragraphs,
        bodypr_xml,
    }))
}

fn parse_table(xml: &str) -> Result<Option<PptxTable>, AppError> {
    if !xml.contains(":tbl") && !xml.contains("<a:tbl") {
        return Ok(None);
    }
    let mut reader = Reader::from_str(xml);
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut current_cells = Vec::new();
    let mut current_h: Option<i64> = None;
    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Empty(e)) if local_name(e.name().as_ref()) == b"gridCol" => {
                columns.push(
                    attr_value(&e, "w")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0),
                );
            }
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"tr" => {
                current_cells.clear();
                current_h = attr_value(&e, "h").and_then(|v| v.parse().ok());
            }
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"tc" => {
                let raw = capture_element(&mut reader, xml, start, b"tc")?;
                let grid_span = attr_value(&e, "gridSpan").and_then(|v| v.parse().ok());
                let row_span = attr_value(&e, "rowSpan").and_then(|v| v.parse().ok());
                let h_merge = attr_value(&e, "hMerge").map(|v| v == "1");
                let v_merge = attr_value(&e, "vMerge").map(|v| v == "1");
                current_cells.push(PptxTableCell {
                    text: parse_text_frame(&raw.xml)?
                        .unwrap_or(PptxTextFrame {
                            paragraphs: vec![],
                            bodypr_xml: None,
                        }),
                    grid_span,
                    row_span,
                    h_merge,
                    v_merge,
                });
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == b"tr" => {
                rows.push(PptxTableRow {
                    cells: std::mem::take(&mut current_cells),
                    h: current_h.take(),
                });
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("表のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(Some(PptxTable { columns, rows }))
}

pub fn title(slide: &PptxSlide) -> Option<String> {
    slide.shapes.iter().find_map(|shape| {
        matches!(
            shape.placeholder.as_deref(),
            Some("title") | Some("ctrTitle")
        )
        .then(|| shape.text.as_ref())
        .flatten()
        .map(|frame| {
            frame
                .paragraphs
                .iter()
                .flat_map(|p| p.runs.iter())
                .map(|r| r.text.as_str())
                .collect()
        })
        .filter(|text: &String| !text.is_empty())
    })
}
