//! XML パースの共通ヘルパー。

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::error::AppError;
use crate::ir::RawElement;

/// 未知要素の生 XML 保持の上限（これを超えたら truncated フラグを立てる）
pub(crate) const RAW_CAPTURE_LIMIT: usize = 100_000;

pub(crate) fn local_name(raw: &[u8]) -> &[u8] {
    match raw.iter().rposition(|&b| b == b':') {
        Some(i) => &raw[i + 1..],
        None => raw,
    }
}

pub(crate) fn attr_value(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key.as_bytes())
        .map(|a| {
            a.unescape_value()
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned())
        })
}

pub(crate) fn text_content(t: &quick_xml::events::BytesText) -> String {
    t.unescape()
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned())
}

/// リレーションシップの Target を zip 内パスへ正規化する。
/// 例: ("xl", "worksheets/sheet1.xml") -> "xl/worksheets/sheet1.xml"
///     ("word", "media/image1.png") -> "word/media/image1.png"
pub(crate) fn resolve_target(base_dir: &str, target: &str) -> String {
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_string();
    }
    let mut parts: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

pub(crate) fn dirname(path: &str) -> &str {
    path.rfind('/').map(|i| &path[..i]).unwrap_or("")
}

pub(crate) fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub(crate) fn rels_path_of(part_path: &str) -> String {
    format!("{}/_rels/{}.rels", dirname(part_path), basename(part_path))
}

/// `.rels` を Id -> Target の対応表にする。
pub(crate) fn parse_rels(xml: &str) -> Result<HashMap<String, String>, AppError> {
    let mut map = HashMap::new();
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if local_name(e.name().as_ref()) == b"Relationship" =>
            {
                if let (Some(id), Some(target)) = (attr_value(&e, "Id"), attr_value(&e, "Target")) {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AppError::Xml(format!(
                    "リレーションシップのパース失敗: {e}"
                )));
            }
            _ => {}
        }
    }
    Ok(map)
}

/// 位置指定で生 XML を切り出す（上限超過時は truncated フラグ）。
pub(crate) fn raw_slice(xml: &str, start: u64, end: u64, lname: &[u8]) -> RawElement {
    let raw = &xml[start as usize..(end as usize).min(xml.len())];
    let (text, truncated) = if raw.len() > RAW_CAPTURE_LIMIT {
        let mut cut = RAW_CAPTURE_LIMIT;
        while !raw.is_char_boundary(cut) {
            cut -= 1;
        }
        (raw[..cut].to_string(), true)
    } else {
        (raw.to_string(), false)
    };
    RawElement {
        name: String::from_utf8_lossy(lname).into_owned(),
        xml: text,
        truncated,
    }
}

/// 開始タグ位置から要素を丸ごと消費し、生 XML として保持する。
pub(crate) fn capture_element(
    reader: &mut Reader<&[u8]>,
    xml: &str,
    start_pos: u64,
    lname: &[u8],
) -> Result<RawElement, AppError> {
    let mut sub_depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => sub_depth += 1,
            Ok(Event::End(_)) => {
                sub_depth -= 1;
                if sub_depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => {
                return Err(AppError::Xml(format!(
                    "要素 {} が閉じられていません",
                    String::from_utf8_lossy(lname)
                )));
            }
            Err(e) => return Err(AppError::Xml(format!("XML パースエラー: {e}"))),
            _ => {}
        }
    }
    Ok(raw_slice(xml, start_pos, reader.buffer_position(), lname))
}
