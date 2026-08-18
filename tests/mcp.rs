//! MCP サーバー（officedump mcp）の統合テスト。
//! stdio 上で JSON-RPC を直接やり取りしてプロトコル契約を検証する。

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::Value;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const MAIN_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const P_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

// ---------------------------------------------------------------------------
// MCP セッションハーネス
// ---------------------------------------------------------------------------

struct McpSession {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: Receiver<Value>,
    next_id: u64,
}

impl McpSession {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_officedump"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("officedump mcp の起動に失敗");
        let stdin = child.stdin.take().expect("stdin を取得できない");
        let stdout = child.stdout.take().expect("stdout を取得できない");
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        if tx.send(value).is_err() {
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            responses: rx,
            next_id: 0,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin が閉じています");
        writeln!(stdin, "{message}").expect("メッセージの送信に失敗");
        stdin.flush().expect("stdin の flush に失敗");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let message =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send(&message);
        loop {
            let response = self
                .responses
                .recv_timeout(Duration::from_secs(30))
                .expect("サーバー応答がタイムアウトしました");
            if response.get("id") == Some(&serde_json::json!(id)) {
                return response;
            }
        }
    }

    fn notify(&mut self, method: &str) {
        let message = serde_json::json!({"jsonrpc": "2.0", "method": method});
        self.send(&message);
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "officedump-test", "version": "0.0.0"}
            }),
        );
        assert!(
            response.get("error").is_none(),
            "initialize が失敗しました: {response}"
        );
        self.notify("notifications/initialized");
        response
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request(
            "tools/call",
            serde_json::json!({"name": name, "arguments": arguments}),
        );
        assert!(
            response.get("error").is_none(),
            "tools/call がプロトコルエラー: {response}"
        );
        response["result"].clone()
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        let stdin = self.stdin.take();
        drop(stdin);
        let _ = self.child.wait();
    }
}

fn tool_text(result: &Value) -> Value {
    let text = result["content"][0]["text"].as_str().expect("text がない");
    serde_json::from_str(text).expect("ツール結果の text が JSON ではない")
}

// ---------------------------------------------------------------------------
// フィクスチャ
// ---------------------------------------------------------------------------

fn write_zip(path: &std::path::Path, parts: Vec<(&str, String)>) {
    let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in parts {
        writer.start_file(name, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

fn xlsx_fixture(path: &std::path::Path) {
    write_zip(
        path,
        vec![
            (
                "xl/workbook.xml",
                format!(
                    r#"<workbook xmlns="{MAIN_NS}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
                ),
            ),
            (
                "xl/_rels/workbook.xml.rels",
                format!(
                    r#"<Relationships xmlns="{RELS_NS}"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#
                ),
            ),
            (
                "xl/worksheets/sheet1.xml",
                format!(
                    r#"<worksheet xmlns="{MAIN_NS}"><dimension ref="A1:B2"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Hi</t></is></c></row></sheetData></worksheet>"#
                ),
            ),
        ],
    );
}

fn docx_fixture(path: &std::path::Path) {
    write_zip(
        path,
        vec![
            (
                "word/document.xml",
                format!(
                    r#"<w:document xmlns:w="{W_NS}"><w:body><w:p><w:r><w:t>こんにちは</w:t></w:r></w:p><w:p><w:r><w:t>二段落目</w:t></w:r></w:p></w:body></w:document>"#
                ),
            ),
            (
                "word/_rels/document.xml.rels",
                format!(r#"<Relationships xmlns="{RELS_NS}"/>"#),
            ),
        ],
    );
}

fn pptx_fixture(path: &std::path::Path) {
    write_zip(
        path,
        vec![
            (
                "ppt/presentation.xml",
                format!(
                    r#"<p:presentation xmlns:p="{P_NS}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#
                ),
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                format!(
                    r#"<Relationships xmlns="{RELS_NS}"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#
                ),
            ),
            (
                "ppt/slides/slide1.xml",
                format!(
                    r#"<p:sld xmlns:p="{P_NS}" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr/><p:grpSpPr/></p:spTree></p:cSld></p:sld>"#
                ),
            ),
        ],
    );
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[test]
fn mcp_initialize_lists_tools() {
    let mut session = McpSession::spawn();
    let init = session.initialize();
    assert!(
        init["result"]["serverInfo"]["name"].is_string(),
        "serverInfo がありません: {init}"
    );

    let list = session.request("tools/list", serde_json::json!({}));
    let tools = list["result"]["tools"]
        .as_array()
        .expect("tools 一覧がありません");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("ツール名がない"))
        .collect();
    assert!(
        names.contains(&"inspect"),
        "inspect がありません: {names:?}"
    );
    assert!(names.contains(&"read"), "read がありません: {names:?}");
}

#[test]
fn mcp_inspect_returns_structure() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("book.xlsx");
    xlsx_fixture(&file);

    let mut session = McpSession::spawn();
    session.initialize();
    let result = session.call_tool(
        "inspect",
        serde_json::json!({"file": file.to_string_lossy()}),
    );
    assert!(
        result["isError"].is_null() || result["isError"] == false,
        "inspect がエラー: {result}"
    );
    let inspect = tool_text(&result);
    assert_eq!(inspect["format"], "xlsx");
    assert_eq!(inspect["sheets"][0]["name"], "Sheet1");
    assert_eq!(inspect["sheets"][0]["rows"], 2);
}

#[test]
fn mcp_read_docx_writes_files_and_returns_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("letter.docx");
    docx_fixture(&file);
    let out = dir.path().join("docx-out");

    let mut session = McpSession::spawn();
    session.initialize();
    let result = session.call_tool(
        "read",
        serde_json::json!({
            "file": file.to_string_lossy(),
            "out": out.to_string_lossy(),
        }),
    );
    assert!(
        result["isError"].is_null() || result["isError"] == false,
        "read がエラー: {result}"
    );
    let manifest = tool_text(&result);
    assert_eq!(manifest["format"], "docx");
    assert!(
        manifest["content"]
            .as_str()
            .expect("content パスがない")
            .contains("docx-out/content.json")
    );
    assert_eq!(manifest["summary"]["sections"], 1);
    assert_eq!(manifest["summary"]["blocks"], 2);

    let content_path = std::path::PathBuf::from(manifest["content"].as_str().unwrap());
    assert!(content_path.is_file(), "content.json が生成されていない");
    let media_dir = std::path::PathBuf::from(manifest["mediaDir"].as_str().unwrap());
    assert!(media_dir.is_dir(), "media ディレクトリが生成されていない");
}

#[test]
fn mcp_read_pptx_rejects_sheet_argument() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("deck.pptx");
    pptx_fixture(&file);

    let mut session = McpSession::spawn();
    session.initialize();
    let result = session.call_tool(
        "read",
        serde_json::json!({
            "file": file.to_string_lossy(),
            "sheet": "Sheet1",
        }),
    );
    assert_eq!(result["isError"], true, "エラーになるべき: {result}");
    let error = tool_text(&result);
    assert_eq!(error["error"]["kind"], "usage_error");
}

#[test]
fn mcp_read_broken_file_reports_error_and_server_continues() {
    let dir = tempfile::tempdir().unwrap();
    let broken = dir.path().join("broken.xlsx");
    std::fs::write(&broken, b"this is not a zip file").unwrap();
    let valid = dir.path().join("book.xlsx");
    xlsx_fixture(&valid);

    let mut session = McpSession::spawn();
    session.initialize();
    let result = session.call_tool(
        "read",
        serde_json::json!({
            "file": broken.to_string_lossy(),
            "out": dir.path().join("broken-out").to_string_lossy(),
        }),
    );
    assert_eq!(result["isError"], true, "エラーになるべき: {result}");
    let error = tool_text(&result);
    assert_eq!(error["error"]["kind"], "invalid_xlsx");

    // サーバーは後続のツール呼び出しを受け付け続ける
    let inspect = session.call_tool(
        "inspect",
        serde_json::json!({"file": valid.to_string_lossy()}),
    );
    assert!(
        inspect["isError"].is_null() || inspect["isError"] == false,
        "エラー後もサーバーが継続していない: {inspect}"
    );
    assert_eq!(tool_text(&inspect)["format"], "xlsx");
}
