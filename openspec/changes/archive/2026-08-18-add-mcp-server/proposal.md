## Why

officedump は現在シェル経由での利用のみを想定しており、エージェントはコマンド実行権限のある環境でしか利用できない。MCP（Model Context Protocol）サーバーを提供することで、シェルなしに GitHub Copilot 等の MCP クライアントから直接 inspect / read を呼べるようになり、 利用経路が広がり、エージェント統合の手間（Skill 記述・権限設定）が減る。

## What Changes

- `officedump mcp` サブコマンドを追加し、stdio トランスポートの MCP サーバーを起動する
- MCP ツールとして既存の `inspect`（構造概要）と `read`（分解読み出し）を公開する
  - ツール引数は既存CLIオプション（`--sheet` / `--range` / `--para` / `--slide` / `--out`）に対応させる
  - `read` は既定で content.json / media/ をファイルへ書き出し、manifest 情報をツール結果として返す（CLIと同じ契約）
- MCP プロトコル処理には公式 Rust SDK（rmcp）を利用する
- 既存の CLI コマンド・出力契約は変更しない（**破壊的変更なし**）

## Capabilities

### New Capabilities

- `mcp-server`: `officedump mcp` サブコマンドによる stdio MCP サーバー。inspect / read ツールの公開、ツール結果の契約、形式別引数のバリデーション、JSONエラー報告を扱う

### Modified Capabilities

（なし。既存CLIの振る舞いは変更しない）

## Impact

- 依存関係: `rmcp`（公式 Rust MCP SDK）とその依存クレートを追加
- コード: `src/main.rs`（サブコマンド追加）、新規 `src/mcp.rs`（サーバー実装）。既存の inspect/read 処理は再利用し、重実装しない
- ドキュメント: README.md、docs/agent-integration.md に MCP 利用方法を追記
- 影響なし: 既存CLI出力、content.json / manifest 契約、xlsx/docx/pptx の各仕様
