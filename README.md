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

### read — JSON 中間表現を出力

```sh
$ officedump read report.xlsx --sheet 売上 --range A1:F30
{
  "file": "report.xlsx",
  "format": "xlsx",
  "sheets": [
    {
      "name": "売上",
      "mergedCells": ["A1:C1"],
      "cells": [
        { "ref": "B11", "type": "n", "value": 550, "formula": "SUM(A2:A11)" }
      ],
      "unhandledElements": []
    }
  ],
  "media": [
    {
      "ref": "media/image1.png",
      "anchor": { "sheet": "売上", "anchorType": "twoCellAnchor",
                  "placement": "floating", "from": { "col": 1, "row": 3, ... } }
    }
  ]
}
```

- 画像などのメディアは `<出力先>/media/` に元バイナリのまま抽出され、JSON からはパスで参照します
- 出力先は `--out <dir>` で指定。省略時は `<ファイル名>.officedump/`
- `--range` は `A1:F50`（範囲）、`A:C`（列）、`1:30`（行）を指定できます
- エラーは非ゼロ終了コード + 標準エラーへの JSON で報告します（エージェントが機械処理可能）

## 対応状況

| 形式 | 状態 |
|---|---|
| xlsx | ✅ MVP 対応済み |
| docx | 未対応（予定） |
| pptx | 未対応（予定） |
| MCP サーバー化 | 未対応（予定） |

旧バイナリ形式（.xls / .doc / .ppt）は対象外です。

## 開発

```sh
cargo build
cargo test
```

このリポジトリは spec-driven development で開発されています。仕様・設計・タスクは [`openspec/`](openspec/) を参照してください（成果物は日本語で記述するルールです）。

## ライセンス

[MIT](LICENSE)
