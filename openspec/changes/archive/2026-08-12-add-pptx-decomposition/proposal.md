# Proposal: add-pptx-decomposition

## Why

PowerPoint（pptx）は、文字・図形・画像が空間的に配置されるため、既存コンバータでは読み順や図の位置が特に崩れやすい。officedump の第三形式として、意味的な読み順を決めずに、スライド内の描画順・座標・サイズを忠実に JSON へ保持する。

## What Changes

- pptx を JSON 中間表現（IR）へ分解する機能を追加する：スライド順、基本図形、テキストボックス、表、画像
- 各図形にスライド内の描画順（z-order）と geometry（x/y/cx/cy の EMU 生値）を保持する。ツールは読み順を推定しない
- `inspect` にスライド数とタイトルプレースホルダーの一覧を追加する
- `read` に `--slide N:M` を追加し、必要なスライドだけを分解できるようにする
- `ppt/media/` の画像を既存の `media/` 出力へ抽出し、スライド番号・図形の描画順・geometry をアンカーとして保持する
- 既存のファイル中心出力契約を pptx に拡張する。manifest summary は slides/shapes/media 件数を返す
- 拡張子ディスパッチを `.pptx` へ拡張する

非目標（Non-goals）:

- 発表者ノート、コメント、アニメーション、画面切替、埋め込み OLE オブジェクト
- 読み順・重要度・タイトルの意味的推測（座標・z-order・プレースホルダー種別を LLM に渡す）
- Markdown 等への変換・整形（LLM の仕事）
- 旧バイナリ形式（.ppt）、MCP サーバー化

## Capabilities

### New Capabilities

- `pptx-decomposition`: pptx をスライド・図形・テキスト・表・画像の JSON 中間表現へ分解し、描画順と geometry を保持する能力。スライド概要、`--slide` による部分読み出し、未知要素の生 XML 保持を含む

### Modified Capabilities

- `media-extraction`: pptx の画像について、スライド番号・図形の描画順・geometry を位置情報として保持する要件を追加する
- `file-output-mode`: pptx の既定 manifest に slides/shapes/media 件数を返す要件を追加する

## Impact

- **コード**: `OfficeFormat::Pptx`、pptx parser、スライド IR、`--slide` CLI オプションを追加
- **共通基盤**: `ppt/media/` を既存の出力ルートへ接続。manifest summary を拡張
- **既存形式**: xlsx / docx の出力・挙動を変更しない
- **テスト**: 最小 pptx フィクスチャで図形、表、画像、geometry、部分読み出し、未知要素、manifest を検証
