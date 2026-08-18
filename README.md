# officedump

Office ファイル（xlsx / docx / pptx）を**解釈せず、忠実に JSON へ分解する** CLI ツールです。

LLM エージェント（GitHub Copilot など）がシェル経由で呼び出し、構造化された JSON を得ることを想定しています。Markdown への整形・意味づけはツールではなく LLM が行います。

## なぜ？

pandoc や markitdown などの既存コンバータは「抽出」と「解釈」を決定論的に一括で行うため、図の位置ずれ・図形や文字の順序崩壊といった精度問題が起きます。

officedump は逆に、ツール側では一切の判断をしません：

- **構造は正規化する**（名前空間ノイズは除く）
- **属性は全保持する**（書式・数式・結合セル・画像の座標など）
- **判断はしない**（日付変換、数式評価、読み順の決定、Markdown 化は行わない）

情報を一滴も落とさず分解し、意味の解釈は LLM の仕事に委ねます。

## インストール

```sh
cargo install --path .
```

## 使い方

### inspect — 構造の概要だけを取得（トークン節約）

```sh
$ officedump inspect report.xlsx
{
  "file": "report.xlsx",
  "format": "xlsx",
  "sheets": [
    { "name": "売上", "rows": 1200, "cols": 8 },
    { "name": "経費", "rows": 340, "cols": 5 }
  ]
}
```

### read — JSON 中間表現をファイルへ出力

```sh
$ officedump read report.xlsx --sheet 売上 --range A1:F30
{
  "file": "report.xlsx",
  "format": "xlsx",
  "content": "/absolute/path/report.officedump/content.json",
  "mediaDir": "/absolute/path/report.officedump/media",
  "summary": { "sheets": 1, "cells": 180, "media": 2 }
}
```

- 分解 JSON 全量は `<出力先>/content.json` に書き出され、stdout は小さい manifest だけを返します
- 画像などのメディアは `<出力先>/media/` に元バイナリのまま抽出され、content.json からはパスで参照します
- 出力先は `--out <dir>` で指定。省略時は `<ファイル名のstem>.officedump/`
- `--range` は `A1:F50`（範囲）、`A:C`（列）、`1:30`（行）を指定できます
- エラーは非ゼロ終了コード + 標準エラーへの JSON で報告します（エージェントが機械処理可能）

小さいファイルをパイプ処理したい場合だけ、全 JSON をstdoutへ出します。

```sh
officedump read report.xlsx --sheet 売上 --range A1:F30 --stdout
```

### docx の段階的な読み出し

```sh
$ officedump inspect report.docx
{
  "format": "docx",
  "sections": [{ "type": "body", "blocks": 120 }],
  "outline": [{ "index": 1, "level": 0, "style": "heading 1", "text": "概要" }]
}

$ officedump read report.docx --para 1:20
```

- docx は本文の段落/ラン/表、ヘッダー/フッター、ハイパーリンク、フィールドを構造化します
- `--para N:M` は本文ブロックの範囲だけを出力します。ヘッダー/フッターは常に保持します
- docx の画像はインライン/フローティングの配置、ブロック/ランのアンカー、EMU 座標を保持します

### pptx の段階的な読み出し

```sh
$ officedump inspect deck.pptx
{
  "format": "pptx",
  "slides": 24,
  "titles": [{ "index": 1, "title": "四半期レビュー" }]
}

$ officedump read deck.pptx --slide 1:5
```

- pptx は `presentation.xml` のスライド順に、基本図形・テキスト段落/ラン・表・画像を構造化します
- 図形ツリー順は `zOrder`、位置とサイズは `geometry` の EMU 生値（`x` / `y` / `cx` / `cy`）で保持します
- `--slide N:M` で対象スライドを絞れます。読み順やスライド内容の意味づけは行いません

### MCP サーバー — シェルなしでエージェントから直接利用

```sh
officedump mcp
```

- stdio トランスポートの MCP サーバーを起動し、`inspect` / `read` を MCP ツールとして公開します
- ツール引数は CLI オプションに対応します（`sheet` / `range` / `para` / `slide` / `out` / `stdout`）
- `read` は CLI と同じ契約で動きます。既定では content.json / media/ を書き出して manifest を返し、`stdout: true` のときだけ分解 JSON 全量をツール結果として返します
- エラーは CLI と同じ `kind` / `message` を含む JSON をツール実行エラーとして返します（サーバーは継続します）

MCP クライアントの設定例:

```json
{
  "mcpServers": {
    "officedump": {
      "command": "officedump",
      "args": ["mcp"]
    }
  }
}
```

## LLM エージェント連携

Office ファイルを扱うエージェントは、まず `inspect` で構造を確認してから、範囲を絞った `read` を実行してください。`read` のstdout manifestから content.json のパスを受け取り、必要な情報だけを読めます。

詳細な手順、エラー契約、将来の Skill 作成方針は [docs/agent-integration.md](docs/agent-integration.md) を参照してください。

## 対応状況

| 形式 | 状態 |
|---|---|
| xlsx | ✅ MVP 対応済み |
| docx | ✅ MVP 対応済み |
| pptx | ✅ MVP 対応済み |
| MCP サーバー化 | ✅ 対応済み（`officedump mcp`、stdio） |

旧バイナリ形式（.xls / .doc / .ppt）は対象外です。

## 開発

```sh
cargo build
cargo test
```

このリポジトリは spec-driven development で開発されています。仕様・設計・タスクは [`openspec/`](openspec/) を参照してください（成果物は日本語で記述するルールです）。

## ライセンス

[MIT](LICENSE)
