# Agent Integration Guide

`officedump` を GitHub Copilot などの LLM エージェントから使うための正本資料です。エージェント固有の `SKILL.md`、Copilot instructions、MCP ツール説明を作るときは、この資料を短く具体的に要約してください。ここにない独自の利用契約を Skill 側で作らないでください。

## 目的

Office ファイルの内容を LLM が理解できる JSON へ分解します。`officedump` は Markdown 化や要約を行いません。構造・書式属性・メディア位置を保持し、意味づけと整形はエージェントが行います。

大きなファイルの全内容をツールstdoutから受け取ると、エージェントのコンテキストを過剰に消費します。そのため、`read` は既定で JSON をファイルへ書き出し、stdoutには小さい manifest だけを返します。

## 必須ワークフロー

Office ファイルを読む必要があるときは、次の順で実行します。

1. `inspect` で構造だけを確認する
2. 必要な範囲を決める
3. 範囲を絞った `read` を実行する
4. stdout の manifest から `content` の絶対パスを取得する
5. `content.json` の必要部分だけを読む
6. 必要なら、取得した構造化情報をMarkdown、要約、回答などへ整形する

## コマンド契約

### inspect

`inspect` は常に小さい構造概要 JSON をstdoutへ出します。ファイル出力はしません。

```sh
officedump inspect report.xlsx
officedump inspect report.docx
officedump inspect deck.pptx
```

### read

`read` は既定で出力ルートに次を生成します。

```text
<output-root>/
├── content.json  # 分解JSON全量
└── media/        # 抽出した画像等。メディアなしでも存在する
```

stdoutはmanifest JSONだけです。

```json
{
  "file": "report.docx",
  "format": "docx",
  "content": "/absolute/path/report.officedump/content.json",
  "mediaDir": "/absolute/path/report.officedump/media",
  "summary": {
    "sections": 3,
    "blocks": 120,
    "media": 4
  }
}
```

- `content` と `mediaDir` は絶対パスです
- `summary` は xlsx では `sheets` / `cells` / `media`、docx では `sections` / `blocks` / `media`、pptx では `slides` / `shapes` / `media` を返します
- `--out <dir>` を指定すると、そのディレクトリが出力ルートになります
- `--out` を省略すると、カレントディレクトリの `<入力stem>.officedump/` が出力ルートになります
- 同じ出力ルートで再実行すると `content.json` は上書きされます。複数結果を残す必要がある場合は、呼び出しごとに固有の `--out` を指定してください

## 形式別の範囲指定

### xlsx

`inspect` のシート一覧・寸法を見てから、`--sheet` と `--range` を指定します。

```sh
officedump inspect report.xlsx
officedump read report.xlsx --sheet "売上" --range A1:F30
```

`--range` は以下を使えます。

- `A1:F50`: セル範囲
- `A:C`: 列範囲
- `1:30`: 行範囲

### docx

`inspect` の見出しアウトラインを見てから、本文ブロック範囲を `--para` で指定します。

```sh
officedump inspect report.docx
officedump read report.docx --para 20:45
```

- `--para N:M` は本文ブロックだけを絞ります
- ヘッダー/フッターは常に content.json に保持されます

### pptx

`inspect` でスライド数とタイトルプレースホルダーを見てから、対象スライドを `--slide` で指定します。

```sh
officedump inspect deck.pptx
officedump read deck.pptx --slide 5:12
```

- `--slide N:M` は1始まりのスライド範囲を絞ります
- 図形の `zOrder`、EMU生値の `geometry`、テキストラン、表、画像アンカーを保持します
- 読み順や図形・スライドの意味は推測しません

## `--stdout` を使う場合

`--stdout` は分解 JSON 全量をstdoutへ出し、`content.json` を作りません。

```sh
officedump read small.xlsx --stdout
```

エージェントは原則として使わないでください。例外は次だけです。

- 対象範囲が小さく、全量をコンテキストへ入れても安全な場合
- Unixパイプで即時にJSONを別ツールへ渡す場合

メディアは `--stdout` 使用時も `<output-root>/media/` に抽出されます。

## エラー契約

失敗時は非ゼロ終了コードです。標準エラーには次の形のJSONが出ます。

```json
{
  "error": {
    "kind": "unsupported_format",
    "message": "必須パートが見つかりません: ppt/presentation.xml"
  }
}
```

エージェントは `kind` を見て次の行動を判断してください。

- `unsupported_format`: xlsx / docx / pptx 以外の形式。別手段を使うか、対応changeを提案する
- `invalid_xlsx` / `invalid_docx` / `invalid_pptx`: ファイル破損または対応外の構造。利用者へ報告する
- `missing_part`: 不完全なOOXMLパッケージ。利用者へ報告する
- `invalid_range`: 範囲指定を見直し、`inspect` 結果から再試行する
- `sheet_not_found`: シート名を `inspect` の結果で確認する
- `output_error` / `io_error`: 出力先の権限・パス・容量を確認する

## MCP サーバー

シェル実行なしでエージェントから利用する場合は、`officedump mcp` が stdio トランスポートの MCP サーバーを起動します。CLI と同じ `inspect` / `read` の2ツールを公開し、コマンド契約・形式別の範囲指定・エラー契約も CLI と同一です。

クライアント設定の例:

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

ツールと引数:

- `inspect`: `file`（必須）。構造概要 JSON をテキストで返します
- `read`: `file`（必須）、`sheet` / `range` / `para` / `slide` / `out` / `stdout`（任意）。既定では manifest JSON をテキストで返し、`stdout: true` のときだけ分解 JSON 全量を返します

ツール実行の失敗は MCP のツール実行エラー（`isError: true`）として報告され、本文は CLI と同じ `kind` / `message` を持つ JSON です。エラー後もサーバーは後続のツール呼び出しを受け付けます。

## Skill 作成時の方針

Skillはこの資料の利用手順を要約し、対象エージェントが実行できるコマンド形式に合わせます。最低限、次を含めてください。

1. Officeファイルを読むときは必ず `inspect` から始める
2. xlsx は `--sheet` / `--range`、docx は `--para`、pptx は `--slide` で範囲を絞る
3. `read` stdoutはmanifestであり、`content` のファイルを読むこと
4. `--stdout` は小さい出力かパイプ用途に限定する
5. stderrのJSONエラーを解釈して再試行または利用者への報告を行う

SkillへIR全体の仕様を複製しないでください。CLI契約の変更に追従するため、この資料とREADMEへのリンクを残してください。
