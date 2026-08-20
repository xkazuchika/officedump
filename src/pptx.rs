//! pptx（PresentationML）プロファイル。図形の読み順は決めず、zOrderとgeometryを保持する。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::AppError;
use crate::ir::{
    Geometry, PptxParagraph, PptxShape, PptxSlide, PptxTable, PptxTableCell, PptxTableRow,
    PptxTextFrame, TextRun,
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
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"cNvPr" => name = attr_value(&e, "name"),
                    b"ph" => placeholder = attr_value(&e, "type").or(Some("body".to_string())),
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
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"cNvPr" => name = attr_value(&e, "name"),
                    b"blip" => embed_rid = attr_value(&e, "r:embed"),
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
        geometry: parse_geometry(xml),
        text: None,
        table,
    })
}

fn parse_geometry(xml: &str) -> Option<Geometry> {
    let mut reader = Reader::from_str(xml);
    let mut x = None;
    let mut y = None;
    let mut cx = None;
    let mut cy = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
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
        .map(|((x, y), (cx, cy))| Geometry { x, y, cx, cy })
}

fn parse_text_frame(xml: &str) -> Result<Option<PptxTextFrame>, AppError> {
    if !xml.contains(":txBody") && !xml.contains("<p:txBody") {
        return Ok(None);
    }
    let mut reader = Reader::from_str(xml);
    let mut paragraphs = Vec::new();
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut attrs = (false, false, false);
    let mut current = String::new();
    let mut runs = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"p" => {
                        in_paragraph = true;
                        runs.clear();
                    }
                    b"r" => {
                        current.clear();
                        attrs = (false, false, false);
                    }
                    b"rPr" => {
                        attrs = (
                            attr_value(&e, "b").as_deref() == Some("1"),
                            attr_value(&e, "i").as_deref() == Some("1"),
                            attr_value(&e, "u").map(|v| v != "none").unwrap_or(false),
                        );
                    }
                    b"t" => in_text = true,
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_text => current.push_str(&text_content(&t)),
            Ok(Event::End(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"t" => in_text = false,
                    b"r" => {
                        if !current.is_empty() {
                            runs.push(TextRun {
                                text: std::mem::take(&mut current),
                                style: None,
                                bold: attrs.0,
                                italic: attrs.1,
                                underline: attrs.2,
                                strike: false,
                                sz: None,
                                color: None,
                                rfonts: None,
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
                        });
                        in_paragraph = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("テキスト図形のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(Some(PptxTextFrame { paragraphs }))
}

fn parse_table(xml: &str) -> Result<Option<PptxTable>, AppError> {
    if !xml.contains(":tbl") && !xml.contains("<a:tbl") {
        return Ok(None);
    }
    let mut reader = Reader::from_str(xml);
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut current_cells = Vec::new();
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
            }
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"tc" => {
                let raw = capture_element(&mut reader, xml, start, b"tc")?;
                current_cells.push(PptxTableCell {
                    text: parse_text_frame(&raw.xml)?
                        .unwrap_or(PptxTextFrame { paragraphs: vec![] }),
                });
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == b"tr" => {
                rows.push(PptxTableRow {
                    cells: std::mem::take(&mut current_cells),
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
