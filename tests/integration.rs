//! spec の全シナリオに対応する統合テスト。
//! フィクスチャは最小限の OOXML パーツをコードから生成する（バイナリ blob を持たない）。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

// ---------------------------------------------------------------------------
// ヘルパー
// ---------------------------------------------------------------------------

fn officedump(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_officedump"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("officedump の起動に失敗")
}

fn stdout_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {:?}", out);
    let output: Value = serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない");
    if let Some(content) = output.get("content").and_then(Value::as_str) {
        return serde_json::from_slice(&std::fs::read(content).expect("content.json を読めません"))
            .expect("content.json が JSON ではない");
    }
    output
}

fn manifest_json(out: &Output) -> Value {
    assert!(out.status.success(), "コマンドが失敗: {:?}", out);
    serde_json::from_slice(&out.stdout).expect("標準出力が JSON ではない")
}

fn stderr_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stderr).expect("標準エラーが JSON ではない")
}

fn write_xlsx(dir: &Path, name: &str, parts: Vec<(String, Vec<u8>)>) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zw = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (n, data) in parts {
        zw.start_file(n, opts).unwrap();
        zw.write_all(&data).unwrap();
    }
    zw.finish().unwrap();
    path
}

fn find_cell<'a>(json: &'a Value, sheet: &str, reference: &str) -> &'a Value {
    json["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == sheet)
        .and_then(|s| {
            s["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["ref"] == reference)
        })
        .unwrap_or_else(|| panic!("セル {sheet}!{reference} が見つかりません"))
}

// ---------------------------------------------------------------------------
// フィクスチャ部品
// ---------------------------------------------------------------------------

const MAIN_NS: &str = "xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"";
const R_NS: &str =
    "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"";

fn workbook_xml(names: &[&str]) -> String {
    let sheets: String = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                r#"<sheet name="{n}" sheetId="{}" r:id="rId{}"/>"#,
                i + 1,
                i + 1
            )
        })
        .collect();
    format!(r#"<workbook {MAIN_NS} {R_NS}><sheets>{sheets}</sheets></workbook>"#)
}

fn workbook_rels_xml(count: usize) -> String {
    let rels: String = (1..=count)
        .map(|i| {
            format!(
                r#"<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>"#
            )
        })
        .collect();
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
    )
}

fn parts(mut v: Vec<(&str, String)>) -> Vec<(String, Vec<u8>)> {
    v.drain(..)
        .map(|(n, s)| (n.to_string(), s.into_bytes()))
        .collect()
}

/// 3シート（売上 1200x8 / 経費 340x5 / 空）の inspect 用フィクスチャ
fn multi_sheet_parts() -> Vec<(String, Vec<u8>)> {
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["売上", "経費", "空"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(3)),
        (
            "xl/worksheets/sheet1.xml",
            format!(r#"<worksheet {MAIN_NS}><dimension ref="A1:H1200"/><sheetData/></worksheet>"#),
        ),
        (
            "xl/worksheets/sheet2.xml",
            format!(r#"<worksheet {MAIN_NS}><dimension ref="A1:E340"/><sheetData/></worksheet>"#),
        ),
        (
            // dimension 欠落 → 走査フォールバックの対象
            "xl/worksheets/sheet3.xml",
            format!(r#"<worksheet {MAIN_NS}><sheetData/></worksheet>"#),
        ),
    ])
}

/// 数式・日付書式・結合セル・未知要素を含むフィクスチャ（シート名: 計算）
fn calc_parts() -> Vec<(String, Vec<u8>)> {
    let shared = format!(r#"<sst {MAIN_NS}><si><t>日付</t></si></sst>"#);
    let styles = format!(
        r#"<styleSheet {MAIN_NS}><numFmts count="1"><numFmt numFmtId="164" formatCode="yyyy/mm/dd"/></numFmts><cellXfs count="3"><xf numFmtId="0"/><xf numFmtId="14"/><xf numFmtId="164" applyNumberFormat="1"/></cellXfs></styleSheet>"#
    );
    // A2:A11 = 10,20,...,100（合計 550）
    let mut rows = String::from(
        r#"<row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" s="1"><v>45000</v></c><c r="D1" s="2"><v>45000</v></c></row>"#,
    );
    for r in 2..=11u32 {
        rows.push_str(&format!(
            r#"<row r="{r}"><c r="A{r}"><v>{}</v></c></row>"#,
            (r - 1) * 10
        ));
    }
    rows.push_str(r#"<row r="11"><c r="B11"><f>SUM(A2:A11)</f><v>550</v></c></row>"#);
    let sheet = format!(
        r#"<worksheet {MAIN_NS} {R_NS}><dimension ref="A1:D11"/><sheetData>{rows}</sheetData><mergeCells count="1"><mergeCell ref="A1:C1"/></mergeCells><dataValidations count="1"><dataValidation type="list" sqref="B1"><formula1>"a,b"</formula1></dataValidation></dataValidations></worksheet>"#
    );
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["計算"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/sharedStrings.xml", shared),
        ("xl/styles.xml", styles),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

const FAKE_PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake-image-bytes";

/// 画像2点（twoCellAnchor + absoluteAnchor）を含むフィクスチャ（シート名: 図）
fn media_parts() -> Vec<(String, Vec<u8>)> {
    let sheet = format!(
        r#"<worksheet {MAIN_NS} {R_NS}><dimension ref="A1:J20"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>画像テスト</t></is></c></row></sheetData><drawing r:id="rId5"/></worksheet>"#
    );
    let sheet_rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#.to_string();
    let drawing = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>3</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>10</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:pic>
      <xdr:nvPicPr><xdr:cNvPr id="2" name="図1"/></xdr:nvPicPr>
      <xdr:blipFill><a:blip r:embed="rId1"/></xdr:blipFill>
    </xdr:pic>
  </xdr:twoCellAnchor>
  <xdr:absoluteAnchor>
    <xdr:pos x="914400" y="914400"/>
    <xdr:ext cx="1828800" cy="914400"/>
    <xdr:pic>
      <xdr:nvPicPr><xdr:cNvPr id="3" name="図2"/></xdr:nvPicPr>
      <xdr:blipFill><a:blip r:embed="rId2"/></xdr:blipFill>
    </xdr:pic>
  </xdr:absoluteAnchor>
</xdr:wsDr>"#
        .to_string();
    let drawing_rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image2.png"/></Relationships>"#.to_string();
    let mut v = parts(vec![
        ("xl/workbook.xml", workbook_xml(&["図"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet),
        ("xl/worksheets/_rels/sheet1.xml.rels", sheet_rels),
        ("xl/drawings/drawing1.xml", drawing),
        ("xl/drawings/_rels/drawing1.xml.rels", drawing_rels),
    ]);
    v.push(("xl/media/image1.png".to_string(), FAKE_PNG.to_vec()));
    v.push(("xl/media/image2.png".to_string(), FAKE_PNG.to_vec()));
    v
}

/// 1200行×3列の範囲読み出し用フィクスチャ（シート名: データ）
fn rows_parts() -> Vec<(String, Vec<u8>)> {
    let mut sd = String::new();
    for r in 1..=1200u32 {
        sd.push_str(&format!(
            r#"<row r="{r}"><c r="A{r}"><v>{r}</v></c><c r="B{r}"><v>{}</v></c><c r="C{r}" t="inlineStr"><is><t>行{r}</t></is></c></row>"#,
            r * 2
        ));
    }
    let sheet = format!(
        r#"<worksheet {MAIN_NS}><dimension ref="A1:C1200"/><sheetData>{sd}</sheetData></worksheet>"#
    );
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["データ"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

// ---------------------------------------------------------------------------
// テスト: xlsx-decomposition
// ---------------------------------------------------------------------------

/// Scenario: 複数シートの概要取得
#[test]
fn inspect_multi_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "multi.xlsx", multi_sheet_parts());
    let out = officedump(&["inspect", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    assert_eq!(json["format"], "xlsx");
    let sheets = json["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 3);
    assert_eq!(sheets[0]["name"], "売上");
    assert_eq!(sheets[0]["rows"], 1200);
    assert_eq!(sheets[0]["cols"], 8);
    assert_eq!(sheets[1]["name"], "経費");
    assert_eq!(sheets[1]["rows"], 340);
    assert_eq!(sheets[1]["cols"], 5);
    // dimension 欠落シートは走査で 0x0
    assert_eq!(sheets[2]["name"], "空");
    assert_eq!(sheets[2]["rows"], 0);
    assert_eq!(sheets[2]["cols"], 0);
    // 概要にセルデータを含まない
    assert!(sheets[0].get("cells").is_none());
}

/// Scenario: 数式セルの分解
#[test]
fn read_formula_cell() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let cell = find_cell(&json, "計算", "B11");
    assert_eq!(cell["formula"], "SUM(A2:A11)");
    assert_eq!(cell["value"], 550); // 評価せずキャッシュ値を保持
}

/// Scenario: 日付書式セルの分解
#[test]
fn read_date_format_cell() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    // 組み込み書式 numFmtId=14: 生シリアル値と書式 ID を保持（日付文字列化しない）
    let c1 = find_cell(&json, "計算", "C1");
    assert_eq!(c1["value"], 45000);
    assert_eq!(c1["style"]["numFmtId"], 14);

    // カスタム書式 numFmtId=164: formatCode も保持
    let d1 = find_cell(&json, "計算", "D1");
    assert_eq!(d1["value"], 45000);
    assert_eq!(d1["style"]["numFmtId"], 164);
    assert_eq!(d1["style"]["formatCode"], "yyyy/mm/dd");
}

/// 共有文字列の解決
#[test]
fn read_shared_string_cell() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let a1 = find_cell(&json, "計算", "A1");
    assert_eq!(a1["type"], "s");
    assert_eq!(a1["value"], "日付");
}

/// Scenario: 結合セルを含むシートの分解
#[test]
fn read_merged_cells() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let merged = json["sheets"][0]["mergedCells"].as_array().unwrap();
    assert!(merged.iter().any(|m| m == "A1:C1"));
}

/// 未知要素の escape hatch（生 XML 保持）
#[test]
fn read_unhandled_elements() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let unhandled = json["sheets"][0]["unhandledElements"].as_array().unwrap();
    let dv = unhandled
        .iter()
        .find(|e| e["name"] == "dataValidations")
        .expect("dataValidations が保持されていません");
    assert!(dv["xml"].as_str().unwrap().contains("<dataValidations"));
}

/// Scenario: 巨大シートの部分読み出し
#[test]
fn read_range_filter() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rows.xlsx", rows_parts());
    let out = officedump(
        &["read", f.to_str().unwrap(), "--range", "1:30"],
        dir.path(),
    );
    let json = stdout_json(&out);

    let cells = json["sheets"][0]["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 90); // 30行 × 3列
    for c in cells {
        let r = c["ref"].as_str().unwrap();
        let row: u32 = r
            .trim_start_matches(|ch: char| ch.is_ascii_alphabetic())
            .parse()
            .unwrap();
        assert!(row <= 30, "範囲外のセルが含まれています: {r}");
    }
}

/// --sheet フィルタと、存在しないシートのエラー
#[test]
fn read_sheet_filter_and_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "multi.xlsx", multi_sheet_parts());

    let out = officedump(
        &["read", f.to_str().unwrap(), "--sheet", "経費"],
        dir.path(),
    );
    let json = stdout_json(&out);
    let sheets = json["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0]["name"], "経費");

    let out = officedump(
        &["read", f.to_str().unwrap(), "--sheet", "存在しない"],
        dir.path(),
    );
    assert!(!out.status.success());
    let err = stderr_json(&out);
    assert_eq!(err["error"]["kind"], "sheet_not_found");
}

/// Scenario: 破損ファイルの処理
#[test]
fn read_corrupt_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("corrupt.xlsx");
    std::fs::write(&f, b"this is not a zip file").unwrap();

    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    assert!(!out.status.success());
    let err = stderr_json(&out);
    assert_eq!(err["error"]["kind"], "invalid_xlsx");
    assert!(err["error"]["message"].is_string());
}

// ---------------------------------------------------------------------------
// テスト: xlsx 精度向上 (improve-xlsx-accuracy)
// ---------------------------------------------------------------------------

/// リッチテキストランを含む共有文字列・インライン文字列のフィクスチャ（シート名: リッチ）
fn rich_text_parts() -> Vec<(String, Vec<u8>)> {
    let shared = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si>
    <r>
      <rPr><b/><sz val="14"/><color rgb="FFFF0000"/><rFont val="Arial"/></rPr>
      <t>太字</t>
    </r>
    <r>
      <rPr><i/><u val="single"/></rPr>
      <t>斜体</t>
    </r>
  </si>
  <si><t>プレーン</t></si>
</sst>"#;
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:C1"/>
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="s"><v>1</v></c>
      <c r="C1" t="inlineStr"><is>
        <r><rPr><b/></rPr><t>Bold</t></r>
        <r><rPr><i/></rPr><t>Italic</t></r>
      </is></c>
    </row>
  </sheetData>
</worksheet>"#;
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["リッチ"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/sharedStrings.xml", shared.to_string()),
        ("xl/worksheets/sheet1.xml", sheet.to_string()),
    ])
}

/// Scenario: 共有文字列のリッチテキスト保持
#[test]
fn read_shared_string_rich_text() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rich.xlsx", rich_text_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    // リッチテキストランを持つ共有文字列
    let a1 = find_cell(&json, "リッチ", "A1");
    assert_eq!(a1["type"], "s");
    assert_eq!(a1["value"], "太字斜体");
    let runs = a1["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["text"], "太字");
    assert_eq!(runs[0]["rpr"]["b"], true);
    assert_eq!(runs[0]["rpr"]["sz"], 14.0);
    assert_eq!(runs[0]["rpr"]["color"], "FFFF0000");
    assert_eq!(runs[0]["rpr"]["rfont"], "Arial");
    assert_eq!(runs[1]["text"], "斜体");
    assert_eq!(runs[1]["rpr"]["i"], true);
    assert_eq!(runs[1]["rpr"]["u"], "single");

    // プレーン文字列（ランなし）
    let b1 = find_cell(&json, "リッチ", "B1");
    assert_eq!(b1["value"], "プレーン");
    assert!(b1.get("runs").is_none() || b1["runs"].is_null());
}

/// Scenario: インライン文字列のリッチテキスト保持
#[test]
fn read_inline_string_rich_text() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rich.xlsx", rich_text_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let c1 = find_cell(&json, "リッチ", "C1");
    assert_eq!(c1["type"], "inlineStr");
    assert_eq!(c1["value"], "BoldItalic");
    let runs = c1["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["text"], "Bold");
    assert_eq!(runs[0]["rpr"]["b"], true);
    assert_eq!(runs[1]["text"], "Italic");
    assert_eq!(runs[1]["rpr"]["i"], true);
}

/// 配列数式・共有数式のメタデータを含むフィクスチャ（シート名: 数式）
fn formula_meta_parts() -> Vec<(String, Vec<u8>)> {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:C2"/>
  <sheetData>
    <row r="1">
      <c r="A1"><v>1</v></c>
      <c r="B1"><v>2</v></c>
      <c r="C1"><v>3</v></c>
    </row>
    <row r="2">
      <c r="A2"><f t="array" ref="A2:C2">A1:C1*2</f><v>2</v></c>
      <c r="B2"><f t="shared" si="0" ref="B2">A2*2</f><v>4</v></c>
    </row>
  </sheetData>
</worksheet>"#;
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["数式"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet.to_string()),
    ])
}

/// Scenario: 配列数式のメタデータ保持
#[test]
fn read_array_formula_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "formula.xlsx", formula_meta_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let a2 = find_cell(&json, "数式", "A2");
    assert_eq!(a2["formula"], "A1:C1*2");
    assert_eq!(a2["formulaMeta"]["t"], "array");
    assert_eq!(a2["formulaMeta"]["ref"], "A2:C2");
}

/// Scenario: 共有数式のメタデータ保持
#[test]
fn read_shared_formula_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "formula.xlsx", formula_meta_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let b2 = find_cell(&json, "数式", "B2");
    assert_eq!(b2["formula"], "A2*2");
    assert_eq!(b2["formulaMeta"]["t"], "shared");
    assert_eq!(b2["formulaMeta"]["si"], 0);
}

/// 行属性（非表示・アウトライン）を含むフィクスチャ（シート名: 行属性）
fn row_attrs_parts() -> Vec<(String, Vec<u8>)> {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:A3"/>
  <sheetData>
    <row r="1" hidden="1" ht="0" customHeight="1"><c r="A1"><v>1</v></c></row>
    <row r="2" outlineLevel="2" collapsed="1"><c r="A2"><v>2</v></c></row>
    <row r="3"><c r="A3"><v>3</v></c></row>
  </sheetData>
</worksheet>"#;
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["行属性"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet.to_string()),
    ])
}

/// Scenario: 非表示行の保持
#[test]
fn read_hidden_row() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rows.xlsx", row_attrs_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let rows = json["sheets"][0]["rows"].as_array().unwrap();
    let r1 = rows.iter().find(|r| r["r"] == 1).expect("行1がありません");
    assert_eq!(r1["hidden"], true);
    assert_eq!(r1["customHeight"], true);
    assert_eq!(r1["ht"], 0.0);
}

/// Scenario: アウトラインレベルの保持
#[test]
fn read_outline_level() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rows.xlsx", row_attrs_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let rows = json["sheets"][0]["rows"].as_array().unwrap();
    let r2 = rows.iter().find(|r| r["r"] == 2).expect("行2がありません");
    assert_eq!(r2["outlineLevel"], 2);
    assert_eq!(r2["collapsed"], true);
    // 行3は属性なし -> rowsに含まれない
    assert!(rows.iter().all(|r| r["r"] != 3));
}

/// フォント・配置・名前付きスタイル・formatCode16 を含むフィクスチャ（シート名: スタイル）
fn full_styles_parts() -> Vec<(String, Vec<u8>)> {
    let styles = r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <numFmts count="1">
    <numFmt numFmtId="164" formatCode="yyyy/m/d" formatCode16="yyyy/m/d;[$-ja-JP,0x01104011]"/>
  </numFmts>
  <fonts count="2">
    <font><sz val="11"/><name val="Calibri"/></font>
    <font><b/><sz val="14"/><color rgb="FFFF0000"/><name val="Arial"/></font>
  </fonts>
  <fills count="2">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
  </fills>
  <borders count="1">
    <border><left/><right/><top/><bottom/><diagonal/></border>
  </borders>
  <cellStyleXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
  </cellStyleXfs>
  <cellXfs count="3">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1" applyAlignment="1">
      <alignment horizontal="center" vertical="top" wrapText="1"/>
    </xf>
    <xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
  </cellXfs>
  <cellStyles count="1">
    <cellStyle name="見出し 1" xfId="0" builtinId="0"/>
  </cellStyles>
</styleSheet>"#;
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:A3"/>
  <sheetData>
    <row r="1">
      <c r="A1"><v>1</v></c>
      <c r="A2" s="1"><v>2</v></c>
      <c r="A3" s="2"><v>45000</v></c>
    </row>
  </sheetData>
</worksheet>"#;
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["スタイル"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/styles.xml", styles.to_string()),
        ("xl/worksheets/sheet1.xml", sheet.to_string()),
    ])
}

/// Scenario: フォント・配置を含むスタイルの保持
#[test]
fn read_styles_with_font_and_alignment() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "styles.xlsx", full_styles_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let styles = &json["styles"];
    assert!(styles["fonts"].is_array());
    assert_eq!(styles["fonts"].as_array().unwrap().len(), 2);
    assert_eq!(styles["fills"].as_array().unwrap().len(), 2);
    assert_eq!(styles["borders"].as_array().unwrap().len(), 1);

    let cell_xfs = styles["cellXfs"].as_array().unwrap();
    assert_eq!(cell_xfs.len(), 3);
    let xf1 = &cell_xfs[1];
    assert_eq!(xf1["fontId"], 1);
    assert_eq!(xf1["applyFont"], true);
    assert_eq!(xf1["applyAlignment"], true);
    assert_eq!(xf1["alignment"]["horizontal"], "center");
    assert_eq!(xf1["alignment"]["vertical"], "top");
    assert_eq!(xf1["alignment"]["wrapText"], true);
}

/// Scenario: 名前付きスタイルの保持
#[test]
fn read_named_styles() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "styles.xlsx", full_styles_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    let cell_styles = json["styles"]["cellStyles"].as_array().unwrap();
    assert_eq!(cell_styles.len(), 1);
    assert_eq!(cell_styles[0]["name"], "見出し 1");
    assert_eq!(cell_styles[0]["xfId"], 0);
}

/// Scenario: セルのスタイルインデックス解決
#[test]
fn read_cell_style_index_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "styles.xlsx", full_styles_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    // A2 は s="1" -> cellXfs[1] の numFmtId=0
    let a2 = find_cell(&json, "スタイル", "A2");
    assert_eq!(a2["style"]["numFmtId"], 0);
    // A3 は s="2" -> cellXfs[2] の numFmtId=164
    let a3 = find_cell(&json, "スタイル", "A3");
    assert_eq!(a3["style"]["numFmtId"], 164);
}

/// Scenario: formatCode16 の優先
#[test]
fn read_format_code16_priority() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "styles.xlsx", full_styles_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);

    // numFmt 164 は formatCode16 を持つ -> formatCode より優先
    let a3 = find_cell(&json, "スタイル", "A3");
    assert_eq!(a3["style"]["formatCode"], "yyyy/m/d;[$-ja-JP,0x01104011]");
}

// ---------------------------------------------------------------------------
// テスト: media-extraction
// ---------------------------------------------------------------------------

/// Scenario: 画像を含む xlsx の分解 / フローティング画像の分解 / 整合性の検証
#[test]
fn read_media_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "media.xlsx", media_parts());
    let out_dir = dir.path().join("out");
    let out = officedump(
        &[
            "read",
            f.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ],
        dir.path(),
    );
    let json = stdout_json(&out);

    // 子フォルダに画像2点が抽出されている
    let img1 = out_dir.join("media/image1.png");
    let img2 = out_dir.join("media/image2.png");
    assert!(img1.exists(), "image1.png が抽出されていません");
    assert!(img2.exists(), "image2.png が抽出されていません");
    assert_eq!(std::fs::read(&img1).unwrap(), FAKE_PNG); // 元バイナリのまま

    let media = json["media"].as_array().unwrap();
    assert_eq!(media.len(), 2);

    // twoCellAnchor: アンカー情報と座標の保持
    let m1 = media
        .iter()
        .find(|m| m["ref"] == "media/image1.png")
        .expect("image1 の項目がありません");
    let a1 = &m1["anchor"];
    assert_eq!(a1["sheet"], "図");
    assert_eq!(a1["anchorType"], "twoCellAnchor");
    assert_eq!(a1["placement"], "floating");
    assert_eq!(a1["name"], "図1");
    assert_eq!(a1["from"]["col"], 1);
    assert_eq!(a1["from"]["row"], 3);
    assert_eq!(a1["to"]["col"], 4);
    assert_eq!(a1["to"]["row"], 10);

    // absoluteAnchor: pos/ext の保持
    let m2 = media
        .iter()
        .find(|m| m["ref"] == "media/image2.png")
        .expect("image2 の項目がありません");
    let a2 = &m2["anchor"];
    assert_eq!(a2["anchorType"], "absoluteAnchor");
    assert_eq!(a2["pos"]["x"], 914400);
    assert_eq!(a2["ext"]["cx"], 1828800);

    // 整合性: JSON の全参照が実在し、抽出ファイルに JSON 未参照のものがない
    for m in media {
        let p = out_dir.join(m["ref"].as_str().unwrap());
        assert!(p.exists(), "参照切れ: {}", m["ref"]);
    }
    let on_disk: Vec<String> = std::fs::read_dir(out_dir.join("media"))
        .unwrap()
        .map(|e| format!("media/{}", e.unwrap().file_name().to_string_lossy()))
        .collect();
    for p in on_disk {
        assert!(
            media.iter().any(|m| m["ref"] == p),
            "JSON から参照されていない抽出ファイル: {p}"
        );
    }
}

/// --out 省略時の既定値: <ファイル名>.officedump/
#[test]
fn read_media_default_out_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_xlsx(dir.path(), "media.xlsx", media_parts());
    let out = officedump(&["read", "media.xlsx"], dir.path());
    stdout_json(&out);
    assert!(
        dir.path()
            .join("media.officedump/media/image1.png")
            .exists(),
        "既定出力先に抽出されていません"
    );
}

/// 既定 read はcontent.jsonへ全IRを書き、stdoutには小さいmanifestだけを返す。
#[test]
fn read_uses_file_output_manifest_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
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

    assert_eq!(manifest["format"], "xlsx");
    assert!(std::path::Path::new(manifest["content"].as_str().unwrap()).is_absolute());
    assert!(std::path::Path::new(manifest["mediaDir"].as_str().unwrap()).is_absolute());
    assert_eq!(manifest["summary"]["sheets"], 1);
    assert!(manifest["summary"]["cells"].as_u64().unwrap() > 0);
    assert!(out_dir.join("content.json").exists());
    assert!(out_dir.join("media").is_dir());
    let content: Value =
        serde_json::from_slice(&std::fs::read(out_dir.join("content.json")).unwrap()).unwrap();
    assert!(content["sheets"].is_array());
}

/// --stdout はcontent.jsonを書かず、従来どおり全IRを標準出力へ返す。
#[test]
fn read_stdout_mode_omits_content_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_xlsx(dir.path(), "calc.xlsx", calc_parts());
    let out_dir = dir.path().join("stdout-result");
    let output = manifest_json(&officedump(
        &[
            "read",
            file.to_str().unwrap(),
            "--stdout",
            "--out",
            out_dir.to_str().unwrap(),
        ],
        dir.path(),
    ));

    assert!(output["sheets"].is_array());
    assert!(output.get("content").is_none());
    assert!(!out_dir.join("content.json").exists());
    assert!(out_dir.join("media").is_dir());
}

// ---------------------------------------------------------------------------
// レビュー指摘に対する回帰テスト
// ---------------------------------------------------------------------------

/// <mergeCell> が Start/End 形式でも読める
fn start_end_merge_parts() -> Vec<(String, Vec<u8>)> {
    let sheet = format!(
        r#"<worksheet {MAIN_NS}><dimension ref="A1:C1"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>A</t></is></c></row></sheetData><mergeCells count="1"><mergeCell ref="A1:C1"></mergeCell></mergeCells></worksheet>"#
    );
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["結合"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

#[test]
fn read_start_end_merge_cell() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "merge.xlsx", start_end_merge_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);
    let merged = json["sheets"][0]["mergedCells"].as_array().unwrap();
    assert!(merged.iter().any(|m| m == "A1:C1"));
}

/// 逆転した範囲を正規化して読める
#[test]
fn read_reversed_range() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "rows.xlsx", rows_parts());
    let out = officedump(
        &["read", f.to_str().unwrap(), "--range", "B1:A1"],
        dir.path(),
    );
    let json = stdout_json(&out);
    let refs: Vec<&str> = json["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["ref"].as_str().unwrap())
        .collect();
    assert!(refs.contains(&"A1"));
    assert!(refs.contains(&"B1"));
}

/// 画像を含まない drawing でも .rels がなくても読める
fn empty_drawing_parts() -> Vec<(String, Vec<u8>)> {
    let sheet = format!(
        r#"<worksheet {MAIN_NS} {R_NS}><dimension ref="A1:A1"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Hi</t></is></c></row></sheetData><drawing r:id="rId5"/></worksheet>"#
    );
    let sheet_rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#.to_string();
    let drawing = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <xdr:absoluteAnchor><xdr:pos x="0" y="0"/><xdr:ext cx="100000" cy="100000"/></xdr:absoluteAnchor>
</xdr:wsDr>"#
        .to_string();
    parts(vec![
        ("xl/workbook.xml", workbook_xml(&["空図"])),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml(1)),
        ("xl/worksheets/sheet1.xml", sheet),
        ("xl/worksheets/_rels/sheet1.xml.rels", sheet_rels),
        ("xl/drawings/drawing1.xml", drawing),
    ])
}

#[test]
fn read_empty_drawing_without_rels() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_xlsx(dir.path(), "empty_draw.xlsx", empty_drawing_parts());
    let out = officedump(&["read", f.to_str().unwrap()], dir.path());
    let json = stdout_json(&out);
    assert!(json["media"].as_array().unwrap().is_empty());
}
