# Tasks: add-mcp-server

## 1. 依存追加と処理本体の再利用化

- [x] 1.1 `rmcp`（features: server, transport-io）と `tokio` を `Cargo.toml` へ追加する
- [x] 1.2 `run_inspect_*` / `run_read_*` の出力部分を、JSON文字列を返す純粋な関数へ抽出する（CLI振る舞いは不変、stdoutへの print はCLIエントリに集約する）
- [x] 1.3 既存テスト（xlsx / docx / pptx）が全件成功することを確認する

## 2. MCP サーバー実装

- [x] 2.1 `officedump mcp` サブコマンドと `src/mcp.rs` を追加し、stdio トランスポートでサーバーを起動する
- [x] 2.2 `inspect` ツールを実装する（file 引数、構造概要JSONをツール結果として返す）
- [x] 2.3 `read` ツールを実装する（file / sheet / range / para / slide / out / stdout 引数、既定は manifest 相当、`stdout` 指定時は全量）
- [x] 2.4 形式別引数の排他チェックを `AppError::Usage` 経由でツールエラーとして返す
- [x] 2.5 `AppError` の kind / message を含むツール実行エラー報告を実装する（サーバーは継続する）

## 3. 検証とドキュメント

- [x] 3.1 MCP プロトコルの統合テストを追加する（initialize、tools/list、inspect 呼び出し、read 呼び出し、形式不一致引数、破損ファイルのエラー）
- [x] 3.2 README.md と docs/agent-integration.md に MCP サーバーの利用方法を追記する
- [x] 3.3 `cargo fmt --check`、`cargo test`、`cargo clippy -- -D warnings`、`openspec validate add-mcp-server --strict` を実行する
