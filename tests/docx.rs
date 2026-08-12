use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const FAKE_PNG: &[u8] = b"\x89PNG\r\n\x1a\ndocx-image";

fn officedump(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_officedump"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("officedump の起動に失敗")
}

fn stdout_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {out:?}");
    let output: Value = serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない");
    if let Some(content) = output.get("content").and_then(Value::as_str) {
        return serde_json::from_slice(&std::fs::read(content).expect("content.json を読めません"))
            .expect("content.json が JSON ではない");
    }
    output
}

fn manifest_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {out:?}");
    serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない")
}

fn stderr_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stderr).expect("標準エラーが JSON ではない")
}

fn write_docx(dir: &Path, name: &str, parts: Vec<(String, Vec<u8>)>) -> PathBuf {
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
        .map(|(path, content)| (path.to_string(), content.into_bytes()))
        .collect()
}

fn styles_xml() -> String {
    format!(
        r#"<w:styles xmlns:w="{W_NS}">
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
</w:styles>"#
    )
}

fn document_rels() -> String {
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId10" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
<Relationship Id="rId11" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png"/>
<Relationship Id="rId20" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
<Relationship Id="rId30" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#
        .to_string()
}

fn inline_drawing() -> String {
    format!(
        r#"<w:drawing><wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><wp:extent cx="3000000" cy="2000000"/><wp:docPr id="1" name="画像1"/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:blipFill><a:blip r:embed="rId10" xmlns:r="{R_NS}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#
    )
}

fn floating_drawing() -> String {
    format!(
        r#"<w:drawing><wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><wp:positionH relativeFrom="margin"><wp:posOffset>100000</wp:posOffset></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:align>bottom</wp:align></wp:positionV><wp:extent cx="1000000" cy="500000"/><wp:docPr id="2" name="画像2"/><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:blipFill><a:blip r:embed="rId11" xmlns:r="{R_NS}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing>"#
    )
}

fn comprehensive_parts() -> Vec<(String, Vec<u8>)> {
    let mut filler = String::new();
    for index in 9..=50 {
        filler.push_str(&format!(r#"<w:p><w:r><w:t>本文{index}</w:t></w:r></w:p>"#));
    }
    let document = format!(
        r#"<w:document xmlns:w="{W_NS}" xmlns:r="{R_NS}"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>導入</w:t></w:r></w:p>
<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>太字</w:t></w:r><w:r><w:t>通常</w:t></w:r><w:bookmarkStart w:id="0" w:name="anchor"/></w:p>
<w:tbl><w:tblGrid><w:gridCol w:w="4000"/><w:gridCol w:w="4000"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>結合セル</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>詳細</w:t></w:r></w:p>
<w:p><w:hyperlink r:id="rId30"><w:r><w:t>例へのリンク</w:t></w:r></w:hyperlink><w:fldSimple w:instr=" PAGE "><w:r><w:t>1</w:t></w:r></w:fldSimple><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> NUMPAGES </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>5</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>
<w:p><w:r>{}</w:r></w:p>
<w:p><w:r>{}</w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>まとめ</w:t></w:r></w:p>
{filler}
<w:unknownBlock w:foo="bar"/><w:sectPr><w:headerReference w:type="default" r:id="rId20"/></w:sectPr>
</w:body></w:document>"#,
        inline_drawing(),
        floating_drawing(),
    );
    let header =
        format!(r#"<w:hdr xmlns:w="{W_NS}"><w:p><w:r><w:t>ヘッダー本文</w:t></w:r></w:p></w:hdr>"#);
    let mut parts = strings(vec![
        ("word/document.xml", document),
        ("word/_rels/document.xml.rels", document_rels()),
        ("word/styles.xml", styles_xml()),
        ("word/header1.xml", header),
    ]);
    parts.push(("word/media/image1.png".to_string(), FAKE_PNG.to_vec()));
    parts.push(("word/media/image2.png".to_string(), FAKE_PNG.to_vec()));
    parts
}

fn long_document_parts() -> Vec<(String, Vec<u8>)> {
    let mut body = String::new();
    for index in 1..=100 {
        body.push_str(&format!(r#"<w:p><w:r><w:t>段落{index}</w:t></w:r></w:p>"#));
    }
    strings(vec![
        (
            "word/document.xml",
            format!(r#"<w:document xmlns:w="{W_NS}"><w:body>{body}</w:body></w:document>"#),
        ),
        (
            "word/_rels/document.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#.to_string(),
        ),
    ])
}

#[test]
fn inspect_reports_sections_and_heading_outline() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_docx(dir.path(), "report.docx", comprehensive_parts());
    let json = stdout_json(&officedump(
        &["inspect", file.to_str().unwrap()],
        dir.path(),
    ));

    assert_eq!(json["format"], "docx");
    assert_eq!(json["sections"][0]["type"], "body");
    assert_eq!(json["sections"][0]["blocks"], 50);
    assert_eq!(json["sections"][1]["type"], "header-default");
    assert_eq!(json["sections"][1]["blocks"], 1);
    assert!(json["sections"][0].get("runs").is_none());
    let outline = json["outline"].as_array().unwrap();
    assert_eq!(outline.len(), 3);
    assert_eq!(outline[0]["index"], 1);
    assert_eq!(outline[0]["level"], 0);
    assert_eq!(outline[0]["style"], "heading 1");
    assert_eq!(outline[0]["text"], "導入");
    assert_eq!(outline[1]["index"], 4);
    assert_eq!(outline[2]["index"], 8);
}

#[test]
fn read_preserves_runs_tables_headers_links_fields_and_unknown_elements() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_docx(dir.path(), "report.docx", comprehensive_parts());
    let json = stdout_json(&officedump(&["read", file.to_str().unwrap()], dir.path()));
    let body = &json["sections"][0]["blocks"];

    assert_eq!(body[1]["type"], "paragraph");
    assert_eq!(body[1]["runs"][0]["text"], "太字");
    assert_eq!(body[1]["runs"][0]["bold"], true);
    assert_eq!(body[1]["runs"][1]["text"], "通常");
    assert!(body[1]["runs"][1].get("bold").is_none());
    assert_eq!(body[1]["unhandled"][0]["name"], "bookmarkStart");

    assert_eq!(body[2]["type"], "table");
    assert_eq!(body[2]["grid"], serde_json::json!([4000, 4000]));
    assert_eq!(body[2]["rows"][0]["cells"][0]["gridSpan"], 2);
    assert_eq!(
        body[2]["rows"][0]["cells"][0]["blocks"][0]["runs"][0]["text"],
        "結合セル"
    );
    assert_eq!(body[2]["rows"][1]["cells"][0]["vMerge"], "restart");

    assert_eq!(body[4]["runs"][0]["kind"], "hyperlink");
    assert_eq!(body[4]["runs"][0]["href"], "https://example.com");
    assert_eq!(body[4]["runs"][0]["runs"][0]["text"], "例へのリンク");
    assert_eq!(body[4]["runs"][1]["kind"], "field");
    assert_eq!(body[4]["runs"][1]["instr"], "PAGE");
    assert_eq!(body[4]["runs"][1]["text"], "1");
    assert_eq!(body[4]["runs"][2]["kind"], "field");
    assert_eq!(body[4]["runs"][2]["instr"], "NUMPAGES");
    assert_eq!(body[4]["runs"][2]["text"], "5");

    assert_eq!(json["sections"][1]["type"], "header-default");
    assert_eq!(
        json["sections"][1]["blocks"][0]["runs"][0]["text"],
        "ヘッダー本文"
    );
    assert!(
        body.as_array()
            .unwrap()
            .iter()
            .all(|block| block.to_string().contains("ヘッダー本文") == false)
    );

    let unknown = json["unhandledElements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "unknownBlock")
        .expect("未知要素が保持されていません");
    assert!(unknown["xml"].as_str().unwrap().contains("w:foo"));
}

#[test]
fn read_extracts_inline_and_floating_media_with_anchors() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_docx(dir.path(), "report.docx", comprehensive_parts());
    let out = dir.path().join("out");
    let json = stdout_json(&officedump(
        &[
            "read",
            file.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        dir.path(),
    ));

    assert_eq!(
        std::fs::read(out.join("media/image1.png")).unwrap(),
        FAKE_PNG
    );
    assert!(out.join("media/image2.png").exists());
    let media = json["media"].as_array().unwrap();
    let inline = media
        .iter()
        .find(|item| item["ref"] == "media/image1.png")
        .unwrap();
    assert_eq!(inline["anchor"]["placement"], "inline");
    assert_eq!(inline["anchor"]["block"], 6);
    assert_eq!(inline["anchor"]["ext"]["cx"], 3000000);
    let floating = media
        .iter()
        .find(|item| item["ref"] == "media/image2.png")
        .unwrap();
    assert_eq!(floating["anchor"]["placement"], "floating");
    assert_eq!(floating["anchor"]["posH"]["relativeFrom"], "margin");
    assert_eq!(floating["anchor"]["posH"]["offsetEmu"], 100000);
    assert_eq!(floating["anchor"]["posV"]["align"], "bottom");
    for item in media {
        assert!(out.join(item["ref"].as_str().unwrap()).exists());
    }
}

#[test]
fn read_filters_body_by_block_range() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_docx(dir.path(), "long.docx", long_document_parts());
    let json = stdout_json(&officedump(
        &["read", file.to_str().unwrap(), "--para", "1:10"],
        dir.path(),
    ));
    let blocks = json["sections"][0]["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 10);
    assert_eq!(blocks[0]["index"], 1);
    assert_eq!(blocks[9]["index"], 10);
}

#[test]
fn rejects_unsupported_extension_with_json_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("unsupported.pptx");
    std::fs::write(&file, b"not relevant").unwrap();
    let out = officedump(&["read", file.to_str().unwrap()], dir.path());
    assert!(!out.status.success());
    assert_eq!(stderr_json(&out)["error"]["kind"], "unsupported_format");
}

#[test]
fn read_docx_uses_manifest_and_stdout_mode() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_docx(dir.path(), "report.docx", comprehensive_parts());
    let out_dir = dir.path().join("result");
    let manifest = manifest_json(&officedump(
        &[
            "read",
            file.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ],
        dir.path(),
    ));

    assert_eq!(manifest["format"], "docx");
    assert!(std::path::Path::new(manifest["content"].as_str().unwrap()).is_absolute());
    assert_eq!(manifest["summary"]["sections"], 2);
    assert!(manifest["summary"]["blocks"].as_u64().unwrap() > 0);
    assert!(out_dir.join("content.json").exists());
    assert!(out_dir.join("media").is_dir());

    let stdout_dir = dir.path().join("stdout-result");
    let output = manifest_json(&officedump(
        &[
            "read",
            file.to_str().unwrap(),
            "--stdout",
            "--out",
            stdout_dir.to_str().unwrap(),
        ],
        dir.path(),
    ));
    assert!(output["sections"].is_array());
    assert!(output.get("content").is_none());
    assert!(!stdout_dir.join("content.json").exists());
    assert!(stdout_dir.join("media").is_dir());
}
