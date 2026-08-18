//! stdio トランスポートの MCP サーバー。inspect / read をツールとして公開する。
//! 処理本体は CLI と同じ純粋関数（inspect_json / read_json）を利用する。

use std::path::PathBuf;

use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    schemars, serve_server, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::error::AppError;
use crate::{inspect_json, read_json};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectArgs {
    #[schemars(description = "Officeファイル（xlsx/docx/pptx）のパス")]
    pub file: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadArgs {
    #[schemars(description = "Officeファイル（xlsx/docx/pptx）のパス")]
    pub file: String,
    #[schemars(description = "xlsx の対象シート名（省略時は全シート）")]
    pub sheet: Option<String>,
    #[schemars(description = "xlsx のセル範囲（例: A1:F50, A:C, 1:30）")]
    pub range: Option<String>,
    #[schemars(description = "docx の本文ブロック範囲（例: 1:10）")]
    pub para: Option<String>,
    #[schemars(description = "pptx のスライド範囲（例: 1:10）")]
    pub slide: Option<String>,
    #[schemars(description = "分解JSON全量をツール結果として返す（content.json は生成しない）")]
    pub stdout: Option<bool>,
    #[schemars(description = "メディア等の出力先（省略時は <ファイル名>.officedump/）")]
    pub out: Option<String>,
}

/// CLI のエラー契約と同じ kind/message をツール実行エラーとして返す。
fn error_result(err: &AppError) -> CallToolResult {
    let body = serde_json::json!({
        "error": { "kind": err.kind(), "message": err.to_string() }
    });
    CallToolResult::error(vec![ContentBlock::text(body.to_string())])
}

fn text_result(json: String) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(json)])
}

#[derive(Debug, Clone)]
pub struct OfficedumpServer {
    tool_router: ToolRouter<Self>,
}

impl OfficedumpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for OfficedumpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl OfficedumpServer {
    #[tool(
        description = "Officeファイル（xlsx/docx/pptx）の構造概要を返す。シート一覧・見出しアウトライン・スライドタイトル等、対象を絞り込むための情報を含む"
    )]
    fn inspect(
        &self,
        Parameters(InspectArgs { file }): Parameters<InspectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match inspect_json(&PathBuf::from(file)) {
            Ok(json) => Ok(text_result(json)),
            Err(err) => Ok(error_result(&err)),
        }
    }

    #[tool(
        description = "Officeファイルを忠実なJSONへ分解する。既定では content.json と media/ を出力先へ書き出し、manifest（content と mediaDir の絶対パス、件数要約）を返す。stdout: true を指定すると分解JSON全量をツール結果として返す。範囲引数（sheet/range は xlsx、para は docx、slide は pptx 専用）"
    )]
    fn read(&self, Parameters(args): Parameters<ReadArgs>) -> Result<CallToolResult, ErrorData> {
        let out = args.out.as_deref().map(PathBuf::from);
        match read_json(
            &PathBuf::from(args.file),
            args.sheet.as_deref(),
            args.range.as_deref(),
            args.para.as_deref(),
            args.slide.as_deref(),
            args.stdout.unwrap_or(false),
            out.as_deref(),
        ) {
            Ok(json) => Ok(text_result(json)),
            Err(err) => Ok(error_result(&err)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OfficedumpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Officeファイル（xlsx/docx/pptx）を解釈せず忠実にJSONへ分解するツールを提供します。\
                 まず inspect で構造を把握し、read で対象を絞り込んで読み出してください。",
        )
    }
}

/// stdio トランスポートで MCP サーバーを起動する。標準入力が閉じるまで返らない。
pub fn run_mcp() -> Result<(), AppError> {
    let runtime = tokio::runtime::Runtime::new().map_err(AppError::Io)?;
    runtime.block_on(async {
        let service = serve_server(OfficedumpServer::new(), stdio())
            .await
            .map_err(|e| AppError::Output(format!("MCPサーバーの起動に失敗: {e}")))?;
        service
            .waiting()
            .await
            .map_err(|e| AppError::Output(format!("MCPサーバーが異常終了: {e}")))?;
        Ok(())
    })
}
