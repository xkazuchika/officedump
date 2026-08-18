# mcp-server Specification

## Purpose

`officedump mcp` サブコマンドが提供する stdio トランスポートの MCP サーバー能力。シェル実行なしに MCP クライアントから inspect / read を直接呼び出せるようにし、CLI と同じ出力契約をツール結果として返す。

## Requirements

### Requirement: stdio MCP サーバーの起動

ツールは `officedump mcp` サブコマンドを実行した場合、stdio トランスポートで MCP サーバーとして起動し、MCP クライアントからの接続を受け付けなければならない（SHALL）。サーバーは標準入出力以外のポートを開いてはならない（MUST NOT）。

#### Scenario: サーバー起動と初期化

- **WHEN** MCP クライアントが `officedump mcp` プロセスを起動し、initialize リクエストを送る
- **THEN** サーバーは対応するバージョンの initialize 応答を返し、ツール一覧を要求できる状態になる

### Requirement: inspect ツールの公開

サーバーは `inspect` ツールを公開し、ファイルパス1つを引数として受け取った場合、CLI の `inspect` と同じ構造概要 JSON をツール結果として返さなければならない（SHALL）。

#### Scenario: xlsx の構造概要の取得

- **WHEN** MCP クライアントが `inspect` ツールに xlsx ファイルパスを指定して呼び出す
- **THEN** CLI `inspect` と同じ sheets の構造概要 JSON がツール結果として返される

### Requirement: read ツールの公開

サーバーは `read` ツールを公開し、ファイルパスと形式別の範囲引数（xlsx は sheet / range、docx は para、pptx は slide）と出力先（out）を受け取らなければならない（SHALL）。既定では CLI `read` と同じく content.json と media/ を出力ルートへ書き出し、manifest と同等の情報（content と mediaDir の絶対パス、形式別件数要約）をツール結果として返さなければならない（SHALL）。`stdout` オプション指定時は分解 JSON 全量をツール結果として返し、content.json を生成してはならない（MUST NOT）。

#### Scenario: docx の既定読み出し

- **WHEN** MCP クライアントが `read` ツールに docx ファイルパスと para 範囲を指定して呼び出す
- **THEN** 出力ルートに content.json と media/ が生成され、ツール結果に content と mediaDir の絶対パスおよび件数要約が含まれる

#### Scenario: pptx の部分読み出し

- **WHEN** MCP クライアントが `read` ツールに pptx ファイルパスと slide 範囲 `5:8` を指定して呼び出す
- **THEN** 索引5〜8のスライドだけを含む分解結果が出力され、範囲外スライドは含まれない

### Requirement: 形式別引数のバリデーション

サーバーは形式専用の引数に対して CLI と同じ制約を適用しなければならない（SHALL）。対象形式に無効な引数の組み合わせ（例: pptx への sheet 指定）をエラーとして報告しなければならない（SHALL）。

#### Scenario: 形式不一致引数の拒否

- **WHEN** MCP クライアントが pptx ファイルに対して sheet 引数を指定して `read` ツールを呼び出す
- **THEN** ツールはエラーを結果として返し、ファイル処理は行われない

### Requirement: ツールエラーの機械可読な報告

サーバーはツール実行の失敗を、CLI のエラー契約と同じエラー種別（kind）とメッセージを含む形で MCP のツール実行エラーとして報告しなければならない（SHALL）。サーバープロセスを異常終了させてはならない（MUST NOT）。

#### Scenario: 破損ファイルの処理

- **WHEN** MCP クライアントが `read` ツールに破損した（zip として開けない）ファイルを指定して呼び出す
- **THEN** ツール実行エラーとして invalid 形式のエラー種別とメッセージが返り、サーバーは後続のツール呼び出しを受け付け続ける
