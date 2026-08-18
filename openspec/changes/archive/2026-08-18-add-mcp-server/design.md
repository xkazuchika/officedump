## Context

現在の officedump は `inspect` / `read` の2サブコマンドを持つCLIで、処理本体は `run_inspect_*` / `run_read_*` 関数（src/main.rs）が担い、stdoutへの `println!` と終了コードで結果を返している。エラーは `AppError` に集約され、`report` が stderr へ JSON を書き出す。MCP化では、この処理本体をサーバーから再利用しつつ、結果を「MCPツール結果」として返す必要がある。現状の関数は戻り値なしで直接stdoutへ書くため、そのままでは再利用できない。

## Goals / Non-Goals

**Goals:**

- `officedump mcp` で stdio MCP サーバーが動く（rmcp 利用）
- `inspect` / `read` の2ツールを公開し、CLIと同じJSON契約をツール結果で返す
- 既存のCLI振る舞い・出力契約を一切変更しない
- 既存の解析コード（xlsx/docx/pptxパーサ）を変更しない

**Non-Goals:**

- HTTP / SSE トランスポート（stdio のみ）
- MCP リソース・プロンプト・サンプリング等、ツール以外の機能
- 認証・マルチクライアントセッション管理（ローカル単一クライアント想定）

## Decisions

### D1: 処理本体の戻り値化（リファクタリング）

`run_inspect_*` / `run_read_*` を「JSON値（または文字列）を返す純粋な関数」へ抽出し、CLI側はそれを `println!` するだけ、MCP側はそれをツール結果へ包むだけ、という二層構造にする。`AppError` はそのまま伝播させる。

- 代替案: MCP側でCLIをサブプロセス起動 — 二重プロセス起動のコストと、stdout/stderrの再パースが必要になり契約が二重管理になるため不採用
- 代替案: MCP側に処理を複製 — 同期コストが恒久的に発生するため不採用

### D2: rmcp（公式 Rust SDK）を採用

プロトコルバージョン交渉・JSON-RPCフレーミング・初期化シーケンスをSDKに任せる。依存は `rmcp`（feature: server, transport-io）のみ追加する。

- 代替案: 自前で JSON-RPC を実装 — 依存は増えないがプロトコル詳細の追従保守が自前になるため不採用

### D3: ツール引数のスキーマ

ツール引数は既存CLIオプションを1対1で写す:

- `inspect`: `file`（必須）
- `read`: `file`（必須）、`sheet` / `range` / `para` / `slide` / `out` / `stdout`（すべて任意、CLIと同じ排他規則）

引数スキーマは inputSchema として公開し、形式別の排他チェック（`--para` は docx 専用 等）は既存の `run_read` ディスパッチと同じ `AppError::Usage` 経由でツールエラーとして返す。

### D4: ツール結果の表現

- `inspect`: 構造概要JSONをそのままテキストで返す
- `read` 既定: manifest相当（content / mediaDir 絶対パス、summary）をテキストで返す。巨大な content.json はツール結果に含めない（CLIのfile-first設計と同じ理由）
- `read` + `stdout`: 分解JSON全量をツール結果として返す（CLIの `--stdout` と同じ例外条件）
- エラー: `AppError` の kind / message を含むテキストをツール実行エラー（isError）として返す

### D5: バイナリ形態

サブコマンド `officedump mcp` として同一バイナリに含める。エントリは `src/main.rs` の `Command::Mcp` から `src/mcp.rs` の `run_mcp()` を呼ぶ。

## Risks / Trade-offs

- [rmcp の依存ツリーが大きくビルド時間が伸びる] → feature を server + transport-io に絞り、不要な機能（transport-child-process 等）を入れない
- [rmcp の API 変更・破壊的更新] → `src/mcp.rs` にSDK接触を局所化し、バージョンを固定する
- [stdout の混入がMCPフレーミングを壊す] → 抽出した処理関数が直接 `println!` しない設計にする（D1）。MCPモードでは解析処理からのprintを排除
- [大ファイル読み出しでツール結果が巨大化] → 既定はfile-firstでmanifestのみ返す設計が既存仕様どおり効く

## Migration Plan

新規サブコマンド追加のみで、既存機能への影響なし。ロールバックは `officedump mcp` を使わなければ完了（revert してもCLI契約は不変）。
