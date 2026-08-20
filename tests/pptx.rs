use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const P_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const FAKE_PNG: &[u8] = b"\x89PNG\r\n\x1a\npptx-image";

fn officedump(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_officedump"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("officedump の起動に失敗")
}

fn manifest_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {out:?}");
    serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない")
}

fn content_json(manifest: &Value) -> Value {
    serde_json::from_slice(
        &std::fs::read(manifest["content"].as_str().expect("contentがありません"))
            .expect("content.json を読めません"),
    )
    .expect("content.json が JSON ではない")
}

fn stderr_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stderr).expect("標準エラーが JSON ではない")
}

fn write_pptx(dir: &Path, name: &str, parts: Vec<(String, Vec<u8>)>) -> PathBuf {
    let path = dir.join(name);
    let mut writer = ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (part, bytes) in parts {
        writer.start_file(part, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
    path
}

fn strings(parts: Vec<(&str, String)>) -> Vec<(String, Vec<u8>)> {
    parts
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.into_bytes()))
        .collect()
}

fn presentation_xml(ids: &[(u32, &str)]) -> String {
    let list: String = ids
        .iter()
        .map(|(id, rid)| format!(r#"<p:sldId id="{id}" r:id="{rid}"/>"#))
        .collect();
    format!(
        r#"<p:presentation xmlns:p="{P_NS}" xmlns:r="{R_NS}"><p:sldIdLst>{list}</p:sldIdLst></p:presentation>"#
    )
}

fn presentation_rels(rels: &[(&str, &str)]) -> String {
    let values: String = rels
        .iter()
        .map(|(id, target)| format!(r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="{target}"/>"#))
        .collect();
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{values}</Relationships>"#
    )
}

fn text_shape(title: bool, name: &str, x: i64, y: i64, cx: i64, cy: i64, text: &str) -> String {
    let placeholder = if title {
        r#"<p:nvPr><p:ph type="title"/></p:nvPr>"#
    } else {
        ""
    };
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="{name}"/>{placeholder}</p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr b="1"/><a:t>{text}</a:t></a:r><a:r><a:t> tail</a:t></a:r></a:p></p:txBody></p:sp>"#
    )
}

fn table_shape() -> String {
    r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="表"/></p:nvGraphicFramePr><p:xfrm><a:off x="1000" y="1100"/><a:ext cx="1200" cy="1300"/></p:xfrm><a:graphic><a:graphicData><a:tbl><a:tblGrid><a:gridCol w="400"/><a:gridCol w="500"/></a:tblGrid><a:tr h="300"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>A1</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>B1</a:t></a:r></a:p></a:txBody></a:tc></a:tr><a:tr h="300"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>A2</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>B2</a:t></a:r></a:p></a:txBody></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#.to_string()
}

fn picture_shape() -> String {
    r#"<p:pic><p:nvPicPr><p:cNvPr id="5" name="画像"/></p:nvPicPr><p:blipFill><a:blip r:embed="rIdImage"/></p:blipFill><p:spPr><a:xfrm><a:off x="2000" y="2100"/><a:ext cx="2200" cy="2300"/></a:xfrm></p:spPr></p:pic>"#.to_string()
}

fn slide_xml(title: &str, detailed: bool) -> String {
    let content = if detailed {
        format!(
            "{}{}{}{}<p:unknownThing value=\"kept\"/>",
            text_shape(true, "タイトル", 100, 200, 300, 400, title),
            text_shape(false, "本文", 800, 900, 1000, 1100, "Body"),
            table_shape(),
            picture_shape(),
        )
    } else {
        text_shape(true, "タイトル", 100, 200, 300, 400, title)
    };
    format!(
        r#"<p:sld xmlns:p="{P_NS}" xmlns:a="{A_NS}" xmlns:r="{R_NS}"><p:cSld><p:spTree><p:nvGrpSpPr/><p:grpSpPr/>{content}</p:spTree></p:cSld></p:sld>"#
    )
}

fn comprehensive_parts() -> Vec<(String, Vec<u8>)> {
    // presentation順は slide2 -> slide1 -> slide3。ファイル名順に依存しないことを検証する。
    let mut parts = strings(vec![
        (
            "ppt/presentation.xml",
            presentation_xml(&[(256, "rId2"), (257, "rId1"), (258, "rId3")]),
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels(&[
                ("rId1", "slides/slide1.xml"),
                ("rId2", "slides/slide2.xml"),
                ("rId3", "slides/slide3.xml"),
            ]),
        ),
        ("ppt/slides/slide1.xml", slide_xml("Second", false)),
        ("ppt/slides/slide2.xml", slide_xml("First", true)),
        ("ppt/slides/slide3.xml", slide_xml("Third", false)),
        (
            "ppt/slides/_rels/slide2.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdImage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#.to_string(),
        ),
    ]);
    parts.push(("ppt/media/image1.png".to_string(), FAKE_PNG.to_vec()));
    parts
}

fn long_parts() -> Vec<(String, Vec<u8>)> {
    let mut ids = Vec::new();
    let mut rels = Vec::new();
    let mut parts = Vec::new();
    for index in 1..=20u32 {
        let rid = format!("rId{index}");
        let target = format!("slides/slide{index}.xml");
        ids.push((255 + index, rid));
        rels.push((format!("rId{index}"), target));
        parts.push((
            format!("ppt/slides/slide{index}.xml"),
            slide_xml(&format!("Slide {index}"), false).into_bytes(),
        ));
    }
    let id_refs: Vec<(u32, &str)> = ids.iter().map(|(id, rid)| (*id, rid.as_str())).collect();
    let rel_refs: Vec<(&str, &str)> = rels
        .iter()
        .map(|(id, target)| (id.as_str(), target.as_str()))
        .collect();
    parts.push((
        "ppt/presentation.xml".to_string(),
        presentation_xml(&id_refs).into_bytes(),
    ));
    parts.push((
        "ppt/_rels/presentation.xml.rels".to_string(),
        presentation_rels(&rel_refs).into_bytes(),
    ));
    parts
}

#[test]
fn inspect_preserves_presentation_slide_order_and_titles() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "deck.pptx", comprehensive_parts());
    let inspect = manifest_json(&officedump(
        &["inspect", file.to_str().unwrap()],
        dir.path(),
    ));

    assert_eq!(inspect["format"], "pptx");
    assert_eq!(inspect["slides"], 3);
    assert_eq!(inspect["titles"][0]["index"], 1);
    assert_eq!(inspect["titles"][0]["title"], "First tail");
    assert_eq!(inspect["titles"][1]["title"], "Second tail");
    assert_eq!(inspect["titles"][2]["title"], "Third tail");
}

#[test]
fn read_preserves_zorder_geometry_text_table_media_and_unknown_elements() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "deck.pptx", comprehensive_parts());
    let out = dir.path().join("out");
    let manifest = manifest_json(&officedump(
        &[
            "read",
            file.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        dir.path(),
    ));
    let content = content_json(&manifest);

    assert_eq!(manifest["summary"]["slides"], 3);
    assert!(manifest["summary"]["shapes"].as_u64().unwrap() >= 6);
    assert_eq!(content["slides"][0]["index"], 1);
    let shapes = content["slides"][0]["shapes"].as_array().unwrap();
    assert_eq!(shapes[0]["type"], "shape");
    assert_eq!(shapes[0]["zOrder"], 1);
    assert_eq!(shapes[0]["placeholder"], "title");
    assert_eq!(shapes[0]["geometry"]["x"], 100);
    assert_eq!(
        shapes[0]["text"]["paragraphs"][0]["runs"][0]["text"],
        "First"
    );
    assert_eq!(shapes[0]["text"]["paragraphs"][0]["runs"][0]["bold"], true);
    assert_eq!(shapes[1]["zOrder"], 2);
    assert_eq!(shapes[1]["geometry"]["x"], 800);
    assert_eq!(shapes[2]["type"], "table");
    assert_eq!(shapes[2]["table"]["columns"], serde_json::json!([400, 500]));
    assert_eq!(shapes[2]["table"]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(
        shapes[2]["table"]["rows"][0]["cells"][0]["text"]["paragraphs"][0]["runs"][0]["text"],
        "A1"
    );
    assert_eq!(shapes[3]["type"], "picture");
    assert_eq!(shapes[3]["zOrder"], 4);

    let unknown = content["slides"][0]["unhandledElements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "unknownThing")
        .expect("未知図形要素が保持されていません");
    assert!(unknown["xml"].as_str().unwrap().contains("value=\"kept\""));

    assert_eq!(
        std::fs::read(out.join("media/image1.png")).unwrap(),
        FAKE_PNG
    );
    let image = content["media"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["ref"] == "media/image1.png")
        .expect("画像参照がありません");
    assert_eq!(image["anchor"]["slide"], 1);
    assert_eq!(image["anchor"]["zOrder"], 4);
    assert_eq!(image["anchor"]["geometry"]["x"], 2000);
    assert_eq!(image["anchor"]["geometry"]["cy"], 2300);
}

#[test]
fn read_filters_slides_and_can_use_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "long.pptx", long_parts());
    let manifest = manifest_json(&officedump(
        &["read", file.to_str().unwrap(), "--slide", "5:8"],
        dir.path(),
    ));
    let content = content_json(&manifest);
    let slides = content["slides"].as_array().unwrap();
    assert_eq!(slides.len(), 4);
    assert_eq!(slides[0]["index"], 5);
    assert_eq!(slides[3]["index"], 8);

    let output = manifest_json(&officedump(
        &["read", file.to_str().unwrap(), "--slide", "1:1", "--stdout"],
        dir.path(),
    ));
    assert!(output["slides"].is_array());
    assert!(output.get("content").is_none());
}

#[test]
fn reports_invalid_pptx_as_json_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.pptx");
    std::fs::write(&file, b"not a zip").unwrap();
    let out = officedump(&["read", file.to_str().unwrap()], dir.path());
    assert!(!out.status.success());
    assert_eq!(stderr_json(&out)["error"]["kind"], "invalid_pptx");
}

// ---------------------------------------------------------------------------
// テスト: pptx 精度向上 (improve-pptx-accuracy)
// ---------------------------------------------------------------------------

fn stdout_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {out:?}");
    serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない")
}

fn accuracy_parts() -> Vec<(String, Vec<u8>)> {
    let slide = format!(
        r#"<p:sld xmlns:p="{P_NS}" xmlns:a="{A_NS}" xmlns:r="{R_NS}">
<p:cSld><p:spTree>
<p:sp><p:nvSpPr><p:cNvPr id="1" name="Title"/><p:nvPr><p:ph type="title" idx="0" sz="half"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm rot="2700000" flipH="1"><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></a:xfrm><a:prstGeom prst="roundRect"/></p:spPr><p:txBody><a:bodyPr vert="vert" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr" lvl="1" marL="457200" indent="228600"><a:buChar char="•"/></a:pPr><a:r><a:rPr b="1" sz="2400" strike="singleStrike"/><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="Arial"/><a:t>タイトル</a:t></a:r></a:p></p:txBody></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="表"/></p:nvGraphicFramePr><p:xfrm><a:off x="1000" y="1100"/><a:ext cx="1200" cy="1300"/></p:xfrm><a:graphic><a:graphicData><a:tbl><a:tblGrid><a:gridCol w="400"/><a:gridCol w="500"/></a:tblGrid><a:tr h="600"><a:tc gridSpan="2" rowSpan="1"><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>結合</a:t></a:r></a:p></a:txBody></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>
</p:spTree></p:cSld>
</p:sld>"#
    );
    strings(vec![
        ("ppt/presentation.xml", presentation_xml(&[(1, "rId1")])),
        ("ppt/_rels/presentation.xml.rels", presentation_rels(&[("rId1", "slides/slide1.xml")])),
        ("ppt/slides/slide1.xml", slide),
    ])
}

/// Scenario: 回転と反転の保持
#[test]
fn read_rotation_and_flip() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let shape = &content["slides"][0]["shapes"][0];
    assert_eq!(shape["geometry"]["rot"], 2700000);
    assert_eq!(shape["geometry"]["flipH"], true);
}

/// Scenario: プリセット図形種別の保持
#[test]
fn read_preset_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let shape = &content["slides"][0]["shapes"][0];
    assert_eq!(shape["prstGeom"], "roundRect");
}

/// Scenario: プレースホルダー属性の保持
#[test]
fn read_placeholder_detail() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let shape = &content["slides"][0]["shapes"][0];
    assert_eq!(shape["placeholderDetail"]["type"], "title");
    assert_eq!(shape["placeholderDetail"]["idx"], 0);
    assert_eq!(shape["placeholderDetail"]["sz"], "half");
}

/// Scenario: テキストランの書式プロパティ保持
#[test]
fn read_run_format_properties() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let run = &content["slides"][0]["shapes"][0]["text"]["paragraphs"][0]["runs"][0];
    assert_eq!(run["text"], "タイトル");
    assert_eq!(run["bold"], true);
    assert_eq!(run["sz"], 2400);
    assert_eq!(run["color"], "FF0000");
    assert_eq!(run["rfonts"]["ascii"], "Arial");
    assert_eq!(run["strike"], true);
}

/// Scenario: 段落プロパティの保持
#[test]
fn read_paragraph_properties() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let para = &content["slides"][0]["shapes"][0]["text"]["paragraphs"][0];
    assert_eq!(para["algn"], "ctr");
    assert_eq!(para["lvl"], 1);
    assert_eq!(para["marL"], 457200);
    assert_eq!(para["indent"], 228600);
    assert_eq!(para["buChar"], "•");
}

/// Scenario: テキストボディプロパティの保持
#[test]
fn read_body_properties() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let text = &content["slides"][0]["shapes"][0]["text"];
    assert!(text["bodyPrXml"].as_str().unwrap().contains("bodyPr"));
}

/// Scenario: セル結合を持つ表の分解
#[test]
fn read_table_cell_merge() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let cell = &content["slides"][0]["shapes"][1]["table"]["rows"][0]["cells"][0];
    assert_eq!(cell["gridSpan"], 2);
    assert_eq!(cell["rowSpan"], 1);
}

/// Scenario: 行の高さの保持
#[test]
fn read_row_height() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_pptx(dir.path(), "acc.pptx", accuracy_parts());
    let content = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--stdout"],
        dir.path(),
    ));
    let row = &content["slides"][0]["shapes"][1]["table"]["rows"][0];
    assert_eq!(row["h"], 600);
}
