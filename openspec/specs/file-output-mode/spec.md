# file-output-mode Specification

## Purpose
LLM エージェントが大きな Office ファイルを扱う際、分解 JSON 全量をコンテキストへ流し込まずに済むよう、`read` の既定出力をファイル中心にする能力。標準出力には次の操作を判断できる小さい manifest だけを返し、必要な内容はファイルから段階的に読めるようにする。
## Requirements
### Requirement: 既定のファイル中心 read 出力

ツールは `read` を `--stdout` なしで実行した場合、分解 JSON 全量を出力ルートの `content.json` に書き出さなければならない（SHALL）。標準出力には分解 JSON 全量を含めず、manifest JSON だけを書き出さなければならない（MUST NOT）。manifest は入力ファイル名、形式、`content.json` の絶対パス、`media/` ディレクトリの絶対パス、形式別の件数要約を含まなければならない（SHALL）。

#### Scenario: docx の既定出力

- **WHEN** 画像を含む docx に対して `read report.docx --out result` を実行する
- **THEN** `result/content.json` と `result/media/` が生成され、標準出力は content の絶対パス・media ディレクトリの絶対パス・sections/blocks/media 件数を含む manifest JSON のみになる

#### Scenario: xlsx の既定出力

- **WHEN** xlsx に対して `read report.xlsx --out result` を実行する
- **THEN** `result/content.json` と `result/media/` が生成され、標準出力は content の絶対パス・media ディレクトリの絶対パス・sheets/cells/media 件数を含む manifest JSON のみになる

### Requirement: 既定出力先と明示出力先

ツールは `--out <dir>` が指定された場合、そのディレクトリを `content.json` と `media/` の共通出力ルートとして使用しなければならない（SHALL）。`--out` が省略された場合、入力ファイルの stem に `.officedump` を付加したディレクトリをカレントディレクトリに作成しなければならない（SHALL）。出力ルートと `media/` ディレクトリは、メディアが存在しない場合も作成しなければならない（SHALL）。

#### Scenario: 既定出力先の使用

- **WHEN** カレントディレクトリで `read report.docx` を実行する
- **THEN** `report.officedump/content.json` と `report.officedump/media/` が生成される

### Requirement: 明示的な標準出力モード

ツールは `read` に `--stdout` が指定された場合、分解 JSON 全量を標準出力へ書き出さなければならない（SHALL）。この場合、manifest を標準出力へ書き出してはならず（MUST NOT）、`content.json` を生成してはならない（MUST NOT）。メディア抽出は既定出力先または `--out` 指定先の `media/` に引き続き行わなければならない（SHALL）。

#### Scenario: パイプ向けの全量標準出力

- **WHEN** `read report.xlsx --stdout --out result` を実行する
- **THEN** 標準出力には xlsx の分解 JSON 全量が出力され、`result/content.json` は生成されず、画像があれば `result/media/` に抽出される

### Requirement: 書き出し完了後の manifest 通知

ツールは、`content.json` とメディアの書き出しが完了するまで manifest を標準出力へ書き出してはならない（MUST NOT）。生成された `content.json` は妥当な JSON 文書でなければならない（SHALL）。

#### Scenario: 生成直後の content 読み出し

- **WHEN** 既定モードの `read` が成功して manifest を返す
- **THEN** manifest の content パスにあるファイルを直後に JSON として読み取れる

