# Design: add-file-output-mode

## Context

現在の `read` は xlsx / docx どちらも分解 JSON 全量を標準出力へ書き出す。これはシェルのパイプでは便利だが、LLM エージェントのツール呼び出しでは巨大な出力がそのままコンテキストを消費する。既存の `inspect`、`--range`、`--para` は段階的開示を意図しているため、`read` の既定出力もその意図に合わせる。

## Goals / Non-Goals

**Goals:**

- `read` の既定 stdout を小さい manifest に限定し、分解 JSON をファイルへ出す
- xlsx / docx で同一の出力ディレクトリ契約を使う
- `--stdout` で従来のパイプ用途を明示的に維持する
- manifest を受けたエージェントが content を必要な範囲だけ読めるようにする
- 将来のエージェント固有 Skill を作るための正本ドキュメントをリポジトリ内に用意する

**Non-Goals:**

- JSON の自動チャンク化、トークン数推定、ページネーション
- content.json の履歴管理や複数 read 結果の自動保持
- `inspect` の出力方式変更
- MCP サーバー化

## Decisions

### D1: `read` はファイル中心、`inspect` は stdout 中心

- `read` の既定動作: `<out>/content.json` に全 IR を書き出し、stdout は manifest だけ
- `inspect` は構造概要が十分小さいため現行どおり stdout JSON を維持
- **代替案**: stdout / ファイルを両方常時出す（却下 — LLM のコンテキスト消費を解決しない）

### D2: output root は content と media の共通ルート

- `--out <dir>` は `<dir>/content.json` と `<dir>/media/` を生成する
- 未指定時は入力 stem を基にした `<stem>.officedump/` をカレントディレクトリに作る
- 同じ出力先で複数回 read すると `content.json` は上書きされる。複数結果を残す利用者/エージェントは `--out` に固有ディレクトリを指定する
- **代替案**: selector を含むファイル名の自動生成（却下 — 命名規則・衝突回避・パスの複雑さを先に導入しない）

### D3: `--stdout` は互換用の明示的 escape hatch

- `--stdout` 指定時は content.json を書かず、全 IR を stdout へ出す
- 画像参照の整合性を保つため、media は通常どおり output root の `media/` へ抽出する
- **代替案**: `--stdout` 時に media を base64 埋め込み（却下 — 出力とトークンをさらに肥大化させる）

### D4: manifest は形式別 summary を持つ

- 共通フィールド: `file`, `format`, `content`, `mediaDir`
- xlsx summary: `sheets`, `cells`, `media`
- docx summary: `sections`, `blocks`, `media`
- content / mediaDir は成功後に存在する絶対パスとして返す。エージェントが作業ディレクトリを推測せずに直接読める

### D5: 出力処理を共通化する

- xlsx / docx の parser は IR 値を構築し、共通の出力ヘルパーが JSON serialize・content 書き込み・manifest 生成を担う
- content は一時ファイルへ書いた後に rename し、stdout manifest は完了後だけ出す
- media ディレクトリは常に事前作成する。メディアなしの場合も manifest の参照先を安定させる

### D6: エージェント連携資料を Skill の正本にする

- `docs/agent-integration.md` に、Office ファイルを扱う際の `inspect → read → manifest → content.json` の手順、xlsx/docx の絞り込み方法、`--stdout` を使う条件、機械可読エラーの扱いを記載する
- これは人間と Skill 作成者の共通資料とする。Copilot など固有の `SKILL.md` は、この資料を短く具体的に要約して作る
- **代替案**: 各 Skill だけに利用手順を書く（却下 — エージェントごとに仕様説明が重複し、CLI 契約の変更時にずれやすい）

## Risks / Trade-offs

- [既存の `read | jq` 等が壊れる] → **BREAKING** として README / help に明記し、`--stdout` を提供する
- [連続した部分読み出しが content.json を上書きする] → manifest の `content` パスで直後に読む用途を想定。保持が必要なら `--out` を明示する
- [ファイル出力失敗時に media だけ残る] → manifest は出さない。content は一時ファイル経由で書き、完了済み JSON を指すことを保証する
