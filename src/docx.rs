//! docx（WordprocessingML）プロファイル: XML を構造化 IR に分解する。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::AppError;
use crate::ir::{
    AnchorSize, Block, DocxSection, NumProps, OutlineEntry, PosOffset, RawElement, RunNode,
    TableCell, TableRow, TextRun,
};
use crate::package::OfficePackage;
use crate::xmlutil::{
    attr_value, capture_element, local_name, parse_rels, raw_slice, resolve_target, text_content,
};

#[derive(Default)]
pub struct DocxStyles {
    styles: HashMap<String, (String, Option<u32>)>,
}

impl DocxStyles {
    pub fn style_name(&self, id: &str) -> String {
        self.styles
            .get(id)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub fn heading_level(&self, id: &str) -> Option<u32> {
        self.styles.get(id).and_then(|(_, level)| *level)
    }
}

pub fn parse_styles(xml: &str) -> Result<DocxStyles, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut styles = DocxStyles::default();
    let mut id: Option<String> = None;
    let mut name: Option<String> = None;
    let mut level: Option<u32> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"style" => {
                        id = attr_value(&e, "w:styleId");
                        name = None;
                        level = None;
                    }
                    b"name" if id.is_some() => name = attr_value(&e, "w:val"),
                    b"outlineLvl" if id.is_some() => {
                        level = attr_value(&e, "w:val").and_then(|v| v.parse().ok());
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == b"style" => {
                if let Some(style_id) = id.take() {
                    styles.styles.insert(
                        style_id.clone(),
                        (name.take().unwrap_or(style_id), level.take()),
                    );
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("styles.xml のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(styles)
}

pub struct DrawingInfo {
    pub kind: String,
    pub name: Option<String>,
    pub embed_rid: Option<String>,
    pub ext: Option<AnchorSize>,
    pub pos_h: Option<PosOffset>,
    pub pos_v: Option<PosOffset>,
}

pub struct DocxDrawingRef {
    pub section: String,
    pub block: u32,
    pub run: u32,
    pub info: DrawingInfo,
}

pub struct DocxDocument {
    pub sections: Vec<DocxSection>,
    pub unhandled: Vec<RawElement>,
    pub drawings: Vec<DocxDrawingRef>,
}

pub fn parse_drawing_xml(xml: &str) -> Result<DrawingInfo, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut kind = None;
    let mut name = None;
    let mut embed_rid = None;
    let mut ext = None;
    let mut pos_h = None;
    let mut pos_v = None;
    let mut h_relative = String::new();
    let mut v_relative = String::new();
    let mut h_align = None;
    let mut v_align = None;
    let mut h_offset = None;
    let mut v_offset = None;
    let mut in_h = false;
    let mut in_v = false;
    let mut child: Option<&str> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"inline" => kind = Some("inline".to_string()),
                    b"anchor" => kind = Some("anchor".to_string()),
                    b"extent" => {
                        ext = Some(AnchorSize {
                            cx: attr_value(&e, "cx")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            cy: attr_value(&e, "cy")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                        });
                    }
                    b"docPr" => name = attr_value(&e, "name"),
                    b"blip" => embed_rid = attr_value(&e, "r:embed"),
                    b"positionH" => {
                        in_h = true;
                        h_relative = attr_value(&e, "relativeFrom").unwrap_or_default();
                    }
                    b"positionV" => {
                        in_v = true;
                        v_relative = attr_value(&e, "relativeFrom").unwrap_or_default();
                    }
                    b"align" if in_h || in_v => child = Some("align"),
                    b"posOffset" if in_h || in_v => child = Some("offset"),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if child.is_some() => {
                let value = text_content(&t).trim().to_string();
                match (child, in_h, in_v) {
                    (Some("align"), true, _) => h_align = Some(value),
                    (Some("align"), _, true) => v_align = Some(value),
                    (Some("offset"), true, _) => h_offset = value.parse().ok(),
                    (Some("offset"), _, true) => v_offset = value.parse().ok(),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"align" | b"posOffset" => child = None,
                    b"positionH" => {
                        in_h = false;
                        pos_h = Some(PosOffset {
                            relative_from: std::mem::take(&mut h_relative),
                            align: h_align.take(),
                            offset_emu: h_offset.take(),
                        });
                    }
                    b"positionV" => {
                        in_v = false;
                        pos_v = Some(PosOffset {
                            relative_from: std::mem::take(&mut v_relative),
                            align: v_align.take(),
                            offset_emu: v_offset.take(),
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("drawing のパース失敗: {e}"))),
            _ => {}
        }
    }

    Ok(DrawingInfo {
        kind: kind.unwrap_or_else(|| "inline".to_string()),
        name,
        embed_rid,
        ext,
        pos_h,
        pos_v,
    })
}

pub fn parse_document(
    package: &mut OfficePackage,
    styles: &DocxStyles,
    para_range: Option<(u32, u32)>,
) -> Result<DocxDocument, AppError> {
    let document_xml = package.read_part("word/document.xml")?;
    let rels = parse_rels(&package.read_part("word/_rels/document.xml.rels")?)?;
    let mut drawings = Vec::new();
    let (mut body_blocks, mut unhandled) =
        parse_blocks(&document_xml, "body", &mut drawings, styles, &rels)?;

    if let Some((from, to)) = para_range {
        body_blocks.retain(|block| (from..=to).contains(&block.index()));
    }

    let mut sections = vec![DocxSection {
        section_type: "body".to_string(),
        blocks: body_blocks,
    }];
    let sect_pr: Vec<String> = unhandled
        .iter()
        .filter(|raw| raw.name == "sectPr")
        .map(|raw| raw.xml.clone())
        .collect();

    for sect in sect_pr {
        for (kind, reference_type, rid) in parse_header_footer_refs(&sect) {
            let Some(target) = rels.get(&rid) else {
                continue;
            };
            let part = resolve_target("word", target);
            let xml = package.read_part(&part)?;
            let section_type = format!("{kind}-{reference_type}");
            let (blocks, extra) = parse_blocks(&xml, &section_type, &mut drawings, styles, &rels)?;
            sections.push(DocxSection {
                section_type,
                blocks,
            });
            unhandled.extend(extra);
        }
    }

    Ok(DocxDocument {
        sections,
        unhandled,
        drawings,
    })
}

fn parse_header_footer_refs(xml: &str) -> Vec<(String, String, String)> {
    let mut reader = Reader::from_str(xml);
    let mut refs = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if (lname == b"headerReference" || lname == b"footerReference")
                    && let Some(rid) = attr_value(&e, "r:id")
                {
                    let kind = if lname == b"headerReference" {
                        "header"
                    } else {
                        "footer"
                    };
                    refs.push((
                        kind.to_string(),
                        attr_value(&e, "w:type").unwrap_or_else(|| "default".to_string()),
                        rid,
                    ));
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
    refs
}

fn parse_blocks(
    xml: &str,
    section: &str,
    drawings: &mut Vec<DocxDrawingRef>,
    styles: &DocxStyles,
    rels: &HashMap<String, String>,
) -> Result<(Vec<Block>, Vec<RawElement>), AppError> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut unhandled = Vec::new();

    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"document" | b"body" | b"hdr" | b"ftr" | b"tc" => {}
                    b"p" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        let index = blocks.len() as u32 + 1;
                        blocks.push(parse_paragraph(
                            &raw.xml, index, section, drawings, styles, rels,
                        )?);
                    }
                    b"tbl" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        let index = blocks.len() as u32 + 1;
                        blocks.push(parse_table(
                            &raw.xml, index, section, drawings, styles, rels,
                        )?);
                    }
                    _ => unhandled.push(capture_element(&mut reader, xml, start, lname)?),
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                unhandled.push(raw_slice(
                    xml,
                    start,
                    reader.buffer_position(),
                    local_name(qname.as_ref()),
                ));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("ブロックのパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok((blocks, unhandled))
}

#[derive(Default)]
struct RunAttrs {
    style: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}

fn flag(e: &quick_xml::events::BytesStart) -> bool {
    attr_value(e, "w:val")
        .map(|v| v != "0" && v != "false" && v != "none")
        .unwrap_or(true)
}

fn parse_paragraph(
    xml: &str,
    index: u32,
    section: &str,
    drawings: &mut Vec<DocxDrawingRef>,
    _styles: &DocxStyles,
    rels: &HashMap<String, String>,
) -> Result<Block, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut style = None;
    let mut num_id = None;
    let mut ilvl = None;
    let mut runs = Vec::new();
    let mut unhandled = Vec::new();
    let mut attrs = RunAttrs::default();
    let mut in_ppr = false;
    let mut in_numpr = false;
    let mut in_rpr = false;
    let mut in_run = false;
    let mut in_text = false;
    let mut in_instr = false;
    let mut text = String::new();
    let mut drawing_in_run = false;
    let mut hyperlink: Option<(Option<String>, Option<String>, Vec<TextRun>)> = None;
    let mut simple_field: Option<(String, String)> = None;
    let mut field_open = false;
    let mut field_separate = false;
    let mut field_instr = String::new();
    let mut field_text = String::new();

    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"p" => {}
                    b"pPr" => in_ppr = true,
                    b"numPr" => in_numpr = true,
                    b"pStyle" if in_ppr => style = attr_value(&e, "w:val"),
                    b"ilvl" if in_numpr => {
                        ilvl = attr_value(&e, "w:val").and_then(|v| v.parse().ok())
                    }
                    b"numId" if in_numpr => {
                        num_id = attr_value(&e, "w:val").and_then(|v| v.parse().ok())
                    }
                    b"r" => {
                        in_run = true;
                        attrs = RunAttrs::default();
                        text.clear();
                        drawing_in_run = false;
                    }
                    b"rPr" => in_rpr = true,
                    b"rStyle" if in_rpr => attrs.style = attr_value(&e, "w:val"),
                    b"b" if in_rpr => attrs.bold = flag(&e),
                    b"i" if in_rpr => attrs.italic = flag(&e),
                    b"u" if in_rpr => attrs.underline = flag(&e),
                    b"strike" if in_rpr => attrs.strike = flag(&e),
                    b"t" => in_text = true,
                    b"instrText" => in_instr = true,
                    b"tab" if in_run => text.push('\t'),
                    b"br" | b"cr" if in_run => text.push('\n'),
                    b"hyperlink" => {
                        let href = attr_value(&e, "r:id").and_then(|rid| rels.get(&rid).cloned());
                        hyperlink = Some((href, attr_value(&e, "w:anchor"), Vec::new()));
                    }
                    b"fldSimple" => {
                        simple_field =
                            Some((attr_value(&e, "w:instr").unwrap_or_default(), String::new()));
                    }
                    b"fldChar" => match attr_value(&e, "w:fldCharType").as_deref() {
                        Some("begin") => {
                            field_open = true;
                            field_separate = false;
                            field_instr.clear();
                            field_text.clear();
                        }
                        Some("separate") => field_separate = true,
                        Some("end") if field_open => {
                            runs.push(RunNode::Field {
                                instr: field_instr.trim().to_string(),
                                text: std::mem::take(&mut field_text),
                            });
                            field_open = false;
                        }
                        _ => {}
                    },
                    b"drawing" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        drawings.push(DocxDrawingRef {
                            section: section.to_string(),
                            block: index,
                            run: runs.len() as u32,
                            info: parse_drawing_xml(&raw.xml)?,
                        });
                        drawing_in_run = true;
                    }
                    _ => unhandled.push(capture_element(&mut reader, xml, start, lname)?),
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"p" => {}
                    b"pPr" => in_ppr = true,
                    b"numPr" => in_numpr = true,
                    b"pStyle" if in_ppr => style = attr_value(&e, "w:val"),
                    b"ilvl" if in_numpr => {
                        ilvl = attr_value(&e, "w:val").and_then(|v| v.parse().ok())
                    }
                    b"numId" if in_numpr => {
                        num_id = attr_value(&e, "w:val").and_then(|v| v.parse().ok())
                    }
                    b"r" => {
                        in_run = true;
                        attrs = RunAttrs::default();
                        text.clear();
                        drawing_in_run = false;
                    }
                    b"rPr" => in_rpr = true,
                    b"rStyle" if in_rpr => attrs.style = attr_value(&e, "w:val"),
                    b"b" if in_rpr => attrs.bold = flag(&e),
                    b"i" if in_rpr => attrs.italic = flag(&e),
                    b"u" if in_rpr => attrs.underline = flag(&e),
                    b"strike" if in_rpr => attrs.strike = flag(&e),
                    b"t" => in_text = true,
                    b"instrText" => in_instr = true,
                    b"tab" if in_run => text.push('\t'),
                    b"br" | b"cr" if in_run => text.push('\n'),
                    b"hyperlink" => {
                        let href = attr_value(&e, "r:id").and_then(|rid| rels.get(&rid).cloned());
                        hyperlink = Some((href, attr_value(&e, "w:anchor"), Vec::new()));
                    }
                    b"fldSimple" => {
                        simple_field =
                            Some((attr_value(&e, "w:instr").unwrap_or_default(), String::new()));
                    }
                    b"fldChar" => match attr_value(&e, "w:fldCharType").as_deref() {
                        Some("begin") => {
                            field_open = true;
                            field_separate = false;
                            field_instr.clear();
                            field_text.clear();
                        }
                        Some("separate") => field_separate = true,
                        Some("end") if field_open => {
                            runs.push(RunNode::Field {
                                instr: field_instr.trim().to_string(),
                                text: std::mem::take(&mut field_text),
                            });
                            field_open = false;
                        }
                        _ => {}
                    },
                    _ => unhandled.push(raw_slice(xml, start, reader.buffer_position(), lname)),
                }
            }
            Ok(Event::Text(t)) => {
                if in_text {
                    text.push_str(&text_content(&t));
                } else if in_instr {
                    field_instr.push_str(&text_content(&t));
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"pPr" => in_ppr = false,
                    b"numPr" => in_numpr = false,
                    b"rPr" => in_rpr = false,
                    b"t" => in_text = false,
                    b"instrText" => in_instr = false,
                    b"r" => {
                        in_run = false;
                        if !text.is_empty() || drawing_in_run {
                            let run = TextRun {
                                text: std::mem::take(&mut text),
                                style: attrs.style.clone(),
                                bold: attrs.bold,
                                italic: attrs.italic,
                                underline: attrs.underline,
                                strike: attrs.strike,
                            };
                            if let Some((_, _, items)) = hyperlink.as_mut() {
                                items.push(run);
                            } else if let Some((_, field_text_value)) = simple_field.as_mut() {
                                field_text_value.push_str(&run.text);
                            } else if field_open && field_separate {
                                field_text.push_str(&run.text);
                            } else if !field_open {
                                runs.push(RunNode::Text(run));
                            }
                        }
                    }
                    b"hyperlink" => {
                        if let Some((href, anchor, hyperlink_runs)) = hyperlink.take() {
                            runs.push(RunNode::Hyperlink {
                                href,
                                anchor,
                                runs: hyperlink_runs,
                            });
                        }
                    }
                    b"fldSimple" => {
                        if let Some((instr, text)) = simple_field.take() {
                            runs.push(RunNode::Field {
                                instr: instr.trim().to_string(),
                                text,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("段落のパース失敗: {e}"))),
            _ => {}
        }
    }

    Ok(Block::Paragraph {
        index,
        style,
        num: num_id
            .zip(ilvl)
            .map(|(num_id, ilvl)| NumProps { num_id, ilvl }),
        runs,
        unhandled,
    })
}

fn parse_table(
    xml: &str,
    index: u32,
    section: &str,
    drawings: &mut Vec<DocxDrawingRef>,
    styles: &DocxStyles,
    rels: &HashMap<String, String>,
) -> Result<Block, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut grid = Vec::new();
    let mut rows = Vec::new();
    let mut unhandled = Vec::new();
    let mut in_grid = false;

    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"tbl" => {}
                    b"tblGrid" => in_grid = true,
                    b"tr" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        rows.push(parse_row(&raw.xml, section, drawings, styles, rels)?);
                    }
                    _ => unhandled.push(capture_element(&mut reader, xml, start, lname)?),
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                if lname == b"gridCol" && in_grid {
                    grid.push(
                        attr_value(&e, "w:w")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                    );
                } else {
                    unhandled.push(raw_slice(xml, start, reader.buffer_position(), lname));
                }
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == b"tblGrid" => in_grid = false,
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("表のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(Block::Table {
        index,
        grid,
        rows,
        unhandled,
    })
}

fn parse_row(
    xml: &str,
    section: &str,
    drawings: &mut Vec<DocxDrawingRef>,
    styles: &DocxStyles,
    rels: &HashMap<String, String>,
) -> Result<TableRow, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut cells = Vec::new();
    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"tc" => {
                let raw = capture_element(&mut reader, xml, start, b"tc")?;
                cells.push(parse_cell(&raw.xml, section, drawings, styles, rels)?);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("表行のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(TableRow { cells })
}

fn parse_cell(
    xml: &str,
    section: &str,
    drawings: &mut Vec<DocxDrawingRef>,
    styles: &DocxStyles,
    rels: &HashMap<String, String>,
) -> Result<TableCell, AppError> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut unhandled = Vec::new();
    let mut grid_span = None;
    let mut v_merge = None;

    loop {
        let start = reader.buffer_position();
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"tc" => {}
                    b"tcPr" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        (grid_span, v_merge) = parse_tc_pr(&raw.xml);
                    }
                    b"p" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        let index = blocks.len() as u32 + 1;
                        blocks.push(parse_paragraph(
                            &raw.xml, index, section, drawings, styles, rels,
                        )?);
                    }
                    b"tbl" => {
                        let raw = capture_element(&mut reader, xml, start, lname)?;
                        let index = blocks.len() as u32 + 1;
                        blocks.push(parse_table(
                            &raw.xml, index, section, drawings, styles, rels,
                        )?);
                    }
                    _ => unhandled.push(capture_element(&mut reader, xml, start, lname)?),
                }
            }
            Ok(Event::Empty(e)) => {
                let qname = e.name();
                unhandled.push(raw_slice(
                    xml,
                    start,
                    reader.buffer_position(),
                    local_name(qname.as_ref()),
                ));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("表セルのパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(TableCell {
        grid_span,
        v_merge,
        blocks,
        unhandled,
    })
}

fn parse_tc_pr(xml: &str) -> (Option<u32>, Option<String>) {
    let mut reader = Reader::from_str(xml);
    let mut grid_span = None;
    let mut v_merge = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                match local_name(qname.as_ref()) {
                    b"gridSpan" => grid_span = attr_value(&e, "w:val").and_then(|v| v.parse().ok()),
                    b"vMerge" => {
                        v_merge =
                            Some(attr_value(&e, "w:val").unwrap_or_else(|| "continue".to_string()));
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
    (grid_span, v_merge)
}

pub fn collect_outline(blocks: &[Block], styles: &DocxStyles) -> Vec<OutlineEntry> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph {
                index,
                style: Some(style_id),
                runs,
                ..
            } => styles.heading_level(style_id).map(|level| OutlineEntry {
                index: *index,
                level,
                style: styles.style_name(style_id),
                text: runs_text(runs),
            }),
            _ => None,
        })
        .collect()
}

fn runs_text(runs: &[RunNode]) -> String {
    let mut text = String::new();
    for run in runs {
        match run {
            RunNode::Text(run) => text.push_str(&run.text),
            RunNode::Hyperlink { runs, .. } => {
                for run in runs {
                    text.push_str(&run.text);
                }
            }
            RunNode::Field {
                text: field_text, ..
            } => text.push_str(field_text),
        }
    }
    text
}
