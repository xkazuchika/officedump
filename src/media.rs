//! メディア抽出と drawing アンカーの IR 化。

use std::collections::{HashMap, HashSet};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::AppError;
use crate::ir::{AnchorPoint, AnchorPos, AnchorSize, MediaAnchor, MediaItem};
use crate::xlsx::{attr_value, local_name, resolve_target, text_content};

pub struct AnchorDraft {
    pub anchor_type: String,
    pub name: Option<String>,
    pub embed_rid: Option<String>,
    pub from: Option<AnchorPoint>,
    pub to: Option<AnchorPoint>,
    pub pos: Option<AnchorPos>,
    pub ext: Option<AnchorSize>,
}

struct PointDraft {
    col: u32,
    row: u32,
    col_off: i64,
    row_off: i64,
}

/// drawing XML（DrawingML）からアンカー情報を抽出する。
/// 座標・サイズの生値を保持し、読み順や意味の解釈は行わない。
pub fn parse_drawing(xml: &str) -> Result<Vec<AnchorDraft>, AppError> {
    let mut anchors = Vec::new();
    let mut reader = Reader::from_str(xml);

    let mut cur: Option<AnchorDraft> = None;
    let mut cur_point: Option<(bool, PointDraft)> = None; // bool: true=from, false=to
    let mut cur_field: Option<&'static str> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"twoCellAnchor" | b"oneCellAnchor" | b"absoluteAnchor" => {
                        cur = Some(AnchorDraft {
                            anchor_type: String::from_utf8_lossy(lname).into_owned(),
                            name: None,
                            embed_rid: None,
                            from: None,
                            to: None,
                            pos: None,
                            ext: None,
                        });
                    }
                    b"from" | b"to" if cur.is_some() => {
                        cur_point = Some((
                            lname == b"from",
                            PointDraft {
                                col: 0,
                                row: 0,
                                col_off: 0,
                                row_off: 0,
                            },
                        ));
                    }
                    b"col" | b"colOff" | b"row" | b"rowOff" if cur_point.is_some() => {
                        cur_field = Some(match lname {
                            b"col" => "col",
                            b"colOff" => "colOff",
                            b"row" => "row",
                            _ => "rowOff",
                        });
                    }
                    b"pos" => {
                        if let Some(a) = cur.as_mut() {
                            let x =
                                attr_value(&e, "x").and_then(|s| s.parse().ok()).unwrap_or(0);
                            let y =
                                attr_value(&e, "y").and_then(|s| s.parse().ok()).unwrap_or(0);
                            a.pos = Some(AnchorPos { x, y });
                        }
                    }
                    b"ext" => {
                        if let Some(a) = cur.as_mut() {
                            let cx =
                                attr_value(&e, "cx").and_then(|s| s.parse().ok()).unwrap_or(0);
                            let cy =
                                attr_value(&e, "cy").and_then(|s| s.parse().ok()).unwrap_or(0);
                            a.ext = Some(AnchorSize { cx, cy });
                        }
                    }
                    b"cNvPr" => {
                        if let Some(a) = cur.as_mut() {
                            a.name = attr_value(&e, "name");
                        }
                    }
                    b"blip" => {
                        if let Some(a) = cur.as_mut() {
                            a.embed_rid = attr_value(&e, "r:embed");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let (Some(field), Some((_, pt))) = (cur_field, cur_point.as_mut()) {
                    let v: i64 = text_content(&t).trim().parse().unwrap_or(0);
                    match field {
                        "col" => pt.col = v as u32,
                        "colOff" => pt.col_off = v,
                        "row" => pt.row = v as u32,
                        _ => pt.row_off = v,
                    }
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                let lname = local_name(qname.as_ref());
                match lname {
                    b"col" | b"colOff" | b"row" | b"rowOff" => cur_field = None,
                    b"from" | b"to" => {
                        if let (Some(a), Some((is_from, pt))) = (cur.as_mut(), cur_point.take())
                        {
                            let p = AnchorPoint {
                                col: pt.col,
                                row: pt.row,
                                col_off_emu: pt.col_off,
                                row_off_emu: pt.row_off,
                            };
                            if is_from {
                                a.from = Some(p);
                            } else {
                                a.to = Some(p);
                            }
                        }
                    }
                    b"twoCellAnchor" | b"oneCellAnchor" | b"absoluteAnchor" => {
                        if let Some(a) = cur.take() {
                            anchors.push(a);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Xml(format!("drawing XML のパース失敗: {e}"))),
            _ => {}
        }
    }
    Ok(anchors)
}

/// 抽出済みメディア一覧と drawing アンカーを突合して MediaItem 一覧を作る。
/// 整合性ルール:
/// - アンカーが参照するメディアが zip 内に存在しない → エラー（参照切れを出さない）
/// - どの drawing からも参照されないメディア → anchor: None の項目として残す（孤立ファイルを作らない）
pub fn build_media_items(
    extracted: &[String],
    anchors_with_rels: Vec<(String, AnchorDraft, HashMap<String, String>)>,
) -> Result<Vec<MediaItem>, AppError> {
    let extracted_set: HashSet<&str> = extracted.iter().map(|s| s.as_str()).collect();
    let mut items: Vec<MediaItem> = Vec::new();
    let mut referenced: HashSet<String> = HashSet::new();

    for (sheet_name, draft, rels) in anchors_with_rels {
        let embed_rid = match &draft.embed_rid {
            Some(r) => r.clone(),
            None => continue,
        };
        let target = rels.get(&embed_rid).ok_or_else(|| {
            AppError::InvalidXlsx(format!(
                "drawing のリレーション {embed_rid} が見つかりません"
            ))
        })?;
        let part = resolve_target("xl/drawings", target);
        let base = part.rsplit('/').next().unwrap_or(&part).to_string();
        let json_ref = format!("media/{base}");
        if !extracted_set.contains(json_ref.as_str()) {
            return Err(AppError::InvalidXlsx(format!(
                "drawing が参照するメディア {part} がパッケージ内に存在しません"
            )));
        }
        referenced.insert(json_ref.clone());
        items.push(MediaItem {
            path: json_ref,
            anchor: Some(MediaAnchor {
                sheet: sheet_name,
                anchor_type: draft.anchor_type,
                placement: "floating".to_string(),
                name: draft.name,
                from: draft.from,
                to: draft.to,
                pos: draft.pos,
                ext: draft.ext,
            }),
        });
    }

    for r in extracted {
        if !referenced.contains(r) {
            items.push(MediaItem {
                path: r.clone(),
                anchor: None,
            });
        }
    }

    items.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(items)
}
