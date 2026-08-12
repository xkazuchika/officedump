# Proposal: add-file-output-mode

## Why

`read` は現在、分解した JSON 全量を標準出力へ書き出す。LLM エージェントが大きな xlsx / docx を呼び出すと、その全量がコンテキストへ流れ込み、トークンを過剰に消費する。エージェントがまず小さい結果を受け取り、必要な部分だけファイルから読む段階的開示を、CLI の既定動作として確立する。

## What Changes

- `read` の既定出力をファイル中心へ変更する。分解 JSON は `<出力先>/content.json` に書き出し、標準出力には小さい manifest JSON のみを返す
- manifest に content のパス、media ディレクトリのパス、形式別の件数要約を含める
- `--stdout` を追加し、分解 JSON 全量を標準出力へ明示的に出せるようにする
- `--out <dir>` を content と `media/` の共通出力ルートとして扱う。省略時は既存どおり `<ファイル名のstem>.officedump/` を使う
- `inspect` は既存どおり、常に小さい構造概要 JSON を標準出力へ書き出す
- README、CLI ヘルプ、`docs/agent-integration.md` を、エージェント向けのファイル中心ワークフローへ更新する

**BREAKING**: `read` の既定標準出力が分解 JSON 全量から manifest へ変わる。パイプや既存スクリプトで全 JSON を読む場合は `--stdout` を明示する必要がある。

## Capabilities

### New Capabilities

- `file-output-mode`: `read` のファイル中心出力、標準出力 manifest、`--stdout` による明示的な全量出力、出力先選択の共通契約

### Modified Capabilities

- `xlsx-decomposition`: 機械可読な出力要件を、既定 manifest / `--stdout` 全量出力の契約へ変更する
- `docx-decomposition`: 機械可読な出力要件を、既定 manifest / `--stdout` 全量出力の契約へ変更する

## Impact

- **CLI**: `read` に `--stdout` を追加し、既定 stdout の互換性を変更
- **出力ファイル**: xlsx / docx ともに `content.json` と既存の `media/` を同一出力ルートへ生成
- **エージェント連携**: Copilot 等は manifest を受けてから content の必要部分だけを読む
- **テスト/ドキュメント**: 既存の read 統合テストと README の出力例を更新。`docs/agent-integration.md` を、将来の Copilot 等の Skill を作るための正本として追加
