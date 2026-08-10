# Proposal: add-officedump-mvp

## Why

LLMエージェントが Office ファイルを扱う既存手段（pandoc、markitdown 等）は「抽出」と「解釈」を決定論的に一括で行うため、図の位置ずれ・図形や文字の順序崩壊といった精度問題が発生する。エージェントが信頼して使える「忠実な分解器」が存在しないため、これを Rust 製 CLI として新規に作る。

## What Changes

- Rust 製 CLI `officedump` を新規作成する。MVP の対象形式は **xlsx のみ**（docx / pptx は後続フェーズ）
- xlsx を JSON 中間表現（IR）に分解して標準出力へ出す。設計原則は「構造は正規化、属性は全保持、判断はしない」
- 画像などのメディアは子フォルダへファイルとして抽出し、JSON 側からパスで参照できるようにする
- `inspect`（構造概要の取得）と範囲指定による部分読み出しを提供し、エージェントがトークンを節約しながら段階的に読めるようにする
- 変換先の整形（Markdown 化など）はツールの責務とせず、LLM が行う前提とする

非目標（Non-goals）:

- Markdown 等への意味的な変換・整形（LLM の仕事とする）
- 数式の評価（式とキャッシュ値を併記するに留める）
- docx / pptx 対応、MCP サーバー化（いずれも後続の change で扱う）
- 旧バイナリ形式（.xls / .doc / .ppt）の対応

## Capabilities

### New Capabilities

- `xlsx-decomposition`: xlsx ワークブックを、構造を正規化しつつ属性を欠落なく保持した JSON 中間表現へ分解する能力。シート構成の概要取得、セル値・型・数式（キャッシュ値併記）・結合セルの保持、範囲指定の部分読み出しを含む
- `media-extraction`: Office ファイル内のメディア（画像等）を子フォルダへ抽出し、JSON 側からのパス参照と位置情報の保持を行う能力

### Modified Capabilities

（なし — 新規プロジェクトのため既存 capability への変更なし）

## Impact

- **新規コード**: `officeapp/officedump/` 配下に Rust プロジェクトを新設（独立 git リポジトリとして管理）
- **依存クレート（想定）**: `zip`, `quick-xml`, `serde` / `serde_json`, `clap`
- **配布**: 単一バイナリ（GitHub Releases を想定）。GitHub Copilot 等の LLM エージェントがシェル経由で呼び出す
- **外部システムへの影響**: なし
