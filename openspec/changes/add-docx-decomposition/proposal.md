# Proposal: add-docx-decomposition

## Why

対応形式の第二段として Word（docx）に対応する。pandoc/markitdown の精度問題（図のずれ、順序崩壊）は Word でも同様であり、officedump の原則「構造は正規化、属性は全保持、判断はしない」をそのまま適用する。xlsx で確立した土台（zip 層、未知要素の escape hatch、メディア抽出）はそのまま流用できる。

## What Changes

- docx を JSON 中間表現（IR）へ分解する機能を追加する：本文ブロック（段落/ラン/表）、ヘッダー/フッター、ハイパーリンク、フィールド
- `inspect` に見出しアウトライン（見出しスタイルの段落の索引とテキスト）を追加する
- `read` にブロック範囲指定 `--para N-M` を追加する（xlsx の `--range` に相当）
- メディア抽出を docx の drawing（inline/anchor）へ拡張する：配置種別、extent、posH/posV を保持
- ファイル拡張子による形式ディスパッチ（`.xlsx` / `.docx`。未知の拡張子は機械可読エラー）

非目標（Non-goals）:

- 脚注・尾注・コメント・変更履歴（構造的に別扱い。後続 change）
- リスト番号の解決（abstractNum の計算は行わず、numId/ilvl を属性として保持）
- Markdown 等への変換・整形（LLM の仕事）
- pptx 対応、MCP サーバー化（いずれも後続 change）

## Capabilities

### New Capabilities

- `docx-decomposition`: docx 文書を JSON 中間表現へ分解する能力。段落/ラン/表のブロック化と索引付け、ヘッダー/フッターの保持、ハイパーリンク・フィールドの解決と保持、見出しアウトライン付き構造概要、ブロック範囲の部分読み出し、未知要素の生 XML 保持を含む

### Modified Capabilities

- `media-extraction`: メディア位置情報の保持を docx の drawing へ拡張する。ブロック基準のアンカー（どの段落・どのラン）、配置種別（インライン／フローティング）、extent（EMU）、posH/posV の保持要件を ADDED で追加する（既存要件の振る舞いは変更しない）

## Impact

- **コード**: zip アクセス層を xlsx 専用から汎用へ分離。docx 用モジュールと IR 型を追加。media モジュールを拡張
- **CLI**: `--para` オプション追加、拡張子ディスパッチ導入。既存 xlsx 挙動への影響なし
- **配布・依存**: 変更なし（同一バイナリが両形式に対応）
