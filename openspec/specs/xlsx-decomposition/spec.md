# xlsx-decomposition Specification

## Purpose
xlsx ワークブックを、構造を正規化しつつ属性を欠落なく保持した JSON 中間表現（IR）へ分解する能力。後段の LLM が意味づけ・整形（Markdown 化等）を行うための、情報損失のない入力を提供することを目的とする。
## Requirements
### Requirement: ワークブック構造の概要取得

ツールは xlsx ファイルを読み、シート名と各シートの寸法（行数・列数）を含む構造概要を JSON で返さなければならない（SHALL）。この概要にはセルデータ本体を含めてはならない（MUST NOT）。

#### Scenario: 複数シートの概要取得

- **WHEN** 3つのシート（売上: 1200行×8列、経費: 340行×5列、空シート）を持つ xlsx の構造概要を要求する
- **THEN** 各シートの名前・行数・列数が JSON で返され、セルデータは含まれない

### Requirement: セルデータの忠実な分解

ツールは各セルについて、セル参照（A1形式）、値、型、表示書式の識別情報を保持しなければならない（SHALL）。数式を持つセルは、数式文字列とキャッシュされた評価値の両方を保持しなければならない（SHALL）。数式要素が属性を持つ場合、数式種別（`t`: normal/array/shared/dataTable）、配列・共有数式の対象範囲（`ref`）、共有数式グループインデックス（`si`）、その他の数式属性（`aca`/`bx`/`ca`）を保持しなければならない（SHALL）。共有文字列参照（`t="s"`）のセルは、共有文字列テーブルの対応エントリがリッチテキストランを持つ場合、連結テキストに加えてランごとの書式情報とテキストを構造化して保持しなければならない（SHALL）。インライン文字列（`t="inlineStr"`）のセルも同様に、リッチテキストランを持つ場合はランごとの書式情報を保持しなければならない（SHALL）。ツールは数式を評価してはならない（SHALL NOT）。また、日付・時刻のシリアル値を ISO 文字列等へ変換してはならない（SHALL NOT。生値と書式識別情報を保持するに留める）。

#### Scenario: 数式セルの分解

- **WHEN** 数式 `=SUM(A1:A10)` とキャッシュ値 `550` を持つセルを含む xlsx を分解する
- **THEN** 出力 JSON の当該セルに、数式文字列とキャッシュ値 `550` の両方が含まれる

#### Scenario: 日付書式セルの分解

- **WHEN** 日付書式が適用されたシリアル値のセルを分解する
- **THEN** 生のシリアル値と書式識別情報が保持され、日付文字列への変換は行われない

#### Scenario: 配列数式のメタデータ保持

- **WHEN** `t="array"` と `ref="B2:D5"` 属性を持つ配列数式を含む xlsx を分解する
- **THEN** 出力 JSON の当該セルの数式情報に、数式種別 `array` と対象範囲 `B2:D5` が含まれる

#### Scenario: 共有数式のメタデータ保持

- **WHEN** `t="shared"` と `si="0"` 属性を持つ共有数式を含む xlsx を分解する
- **THEN** 出力 JSON の当該セルの数式情報に、数式種別 `shared` とグループインデックス `0` が含まれる

#### Scenario: 共有文字列のリッチテキスト保持

- **WHEN** 共有文字列テーブルのエントリが、太字のランと斜体のランを持つリッチテキストを含む xlsx を分解する
- **THEN** 当該文字列を参照するセルの出力 JSON に、連結テキストと共にランごとの書式（太字・斜体）とテキストを含む構造化データが含まれる

#### Scenario: インライン文字列のリッチテキスト保持

- **WHEN** インライン文字列セル（`t="inlineStr"`）の `<is>` 要素がリッチテキストランを持つ xlsx を分解する
- **THEN** 出力 JSON の当該セルに、連結テキストと共にランごとの書式とテキストを含む構造化データが含まれる

### Requirement: 結合セルの保持

ツールは結合セルの範囲を、開始・終了セルを特定できる形式で JSON に保持しなければならない（SHALL）。`mergeCell` 要素は空要素形式（`<mergeCell .../>`）のみならず、Start/End 形式（`<mergeCell ...></mergeCell>`）でも読み取らなければならない（SHALL）。

#### Scenario: 結合セルを含むシートの分解

- **WHEN** セル範囲 A1:C1 が結合されたシートを含む xlsx を分解する
- **THEN** 出力 JSON に結合範囲 A1:C1 を示す情報が含まれる

#### Scenario: Start/End 形式の結合セルの分解

- **WHEN** `<mergeCell ref="A1:C1"></mergeCell>` 形式で結合セルが定義された xlsx を分解する
- **THEN** 出力 JSON に結合範囲 A1:C1 が含まれる

### Requirement: 範囲指定による部分読み出し

ツールはシートとセル範囲を指定して、その部分だけを JSON として返せなければならない（SHALL）。指定範囲外のセルデータを出力に含めてはならない（MUST NOT）。

#### Scenario: 巨大シートの部分読み出し

- **WHEN** 1200行あるシートに対し 1〜30行目の範囲を指定して読み出す
- **THEN** 指定範囲内のセルのみが返され、31行目以降のデータは含まれない

#### Scenario: 逆転した範囲の正規化

- **WHEN** 範囲 `B1:A1` を指定して読み出す
- **THEN** A1 と B1 のセルが返され、空結果にならない

### Requirement: 画像を含まない drawing の処理

ツールは、画像等のメディアを含まない drawing（`drawing.xml.rels` が存在しない）があっても、読み出しを継続しなければならない（SHALL）。そのような drawing に対して `.rels` ファイルが必須である旨のエラーを返してはならない（MUST NOT）。

#### Scenario: 画像なし drawing の分解

- **WHEN** `<drawing>` 要素を持つが `drawing.xml.rels` を持たない xlsx を分解する
- **THEN** エラーなく読み出され、`media` は空として返される

### Requirement: 機械可読な出力とエラー報告

ツールは `read` を `--stdout` なしで実行した場合、分解結果を出力ルートの `content.json` に JSON として書き出し、標準出力には content と media ディレクトリのパスおよび sheets/cells/media 件数を持つ manifest JSON を書き出さなければならない（SHALL）。ツールは `--stdout` が指定された場合、分解結果 JSON 全量を標準出力に書き出さなければならない（SHALL）。失敗時は非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON を標準エラーに出力しなければならない（SHALL）。

#### Scenario: 破損ファイルの処理

- **WHEN** 破損した（zip として開けない）xlsx ファイルを指定して分解を試みる
- **THEN** 非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON が標準エラーに出力される

#### Scenario: xlsx のファイル中心出力

- **WHEN** xlsx を `--stdout` なしで読み出す
- **THEN** 分解 JSON は `content.json` に書き出され、標準出力には sheets/cells/media 件数を含む manifest JSON のみが出力される

### Requirement: 行属性の保持

ツールは各行について、行番号に加えて、非表示状態（`hidden`）、高さ（`ht`）、カスタム高さフラグ（`customHeight`）、アウトラインレベル（`outlineLevel`）、折りたたみ状態（`collapsed`）、列範囲（`spans`）、カスタム書式フラグ（`customFormat`）、スタイルインデックス（`s`）、太線設定（`thickTop`/`thickBottom`）が存在する場合にそれらを保持しなければならない（SHALL）。これらの属性が省略された場合は既定値と解釈し、出力に含めなくてもよい（MAY）。

#### Scenario: 非表示行の保持

- **WHEN** `hidden="1"` と `ht="0"` 属性を持つ行を含む xlsx を分解する
- **THEN** 出力 JSON の当該行の属性に、非表示状態と高さが保持される

#### Scenario: アウトラインレベルの保持

- **WHEN** `outlineLevel="2"` と `collapsed="1"` 属性を持つ行を含む xlsx を分解する
- **THEN** 出力 JSON の当該行の属性に、アウトラインレベルと折りたたみ状態が保持される

#### Scenario: 通常行の属性省略

- **WHEN** 行番号以外の属性を持たない行を含む xlsx を分解する
- **THEN** 出力 JSON の当該行には行番号のみが保持され、省略された属性は含まれない

### Requirement: スタイル情報の完全な保持

ツールは `styles.xml` の各構成要素を読み、IR に含めなければならない（SHALL）。`cellXfs` の各 `<xf>` 要素について、`numFmtId`・`fontId`・`fillId`・`borderId`・`xfId`・`applyNumberFormat`・`applyFont`・`applyFill`・`applyBorder`・`applyAlignment`・`applyProtection`・`quotePrefix` 属性が存在する場合は保持しなければならない（SHALL）。`<xf>` の子要素 `<alignment>`（水平・垂直配置、折り返し、インデント、回転等）と `<protection>`（ロック・非表示）が存在する場合は構造化して保持しなければならない（SHALL）。`fonts`・`fills`・`borders`・`cellStyleXfs`・`cellStyles`（名前付きスタイル）の各要素を読み、IR に含めなければならない（SHALL）。ただし、ツールはスタイルを解決・適用してはならない（SHALL NOT。スタイル定義の生値を保持するに留める）。

#### Scenario: フォント・配置を含むスタイルの保持

- **WHEN** `fontId="2"`・`applyFont="1"`・`applyAlignment="1"` 属性と `<alignment horizontal="center" vertical="top" wrapText="1"/>` 子要素を持つ `<xf>` を含む xlsx を分解する
- **THEN** 出力 JSON のスタイル情報に、フォントインデックス・配置属性・配置子要素が保持される

#### Scenario: 名前付きスタイルの保持

- **WHEN** `cellStyles` に `name="見出し 1"`・`xfId="0"` を持つ `<cellStyle>` 要素を含む xlsx を分解する
- **THEN** 出力 JSON のスタイル情報に、名前付きスタイルの名前と対応する `xfId` が含まれる

#### Scenario: セルのスタイルインデックス解決

- **WHEN** `s="3"` 属性を持つセルを含む xlsx を分解する
- **THEN** 出力 JSON の当該セルのスタイル情報に、`cellXfs` インデックス `3` に対応する全 `xf` 属性と子要素が含まれる

### Requirement: 拡張書式コードの読み取り

ツールは `numFmt` 要素の `formatCode16` 属性（MS-XLSX 拡張）が存在する場合、`formatCode` より優先して保持しなければならない（SHALL）。`formatCode16` が存在しない場合は従来どおり `formatCode` を保持する（SHALL）。

#### Scenario: formatCode16 の優先

- **WHEN** `formatCode="yyyy/m/d"` と `formatCode16="yyyy/m/d;[$-ja-JP,0x01104011]"` の両方を持つ `<numFmt>` を含む xlsx を分解する
- **THEN** 出力 JSON の当該書式の `formatCode` に `formatCode16` の値が保持される

#### Scenario: formatCode16 不在時の従来動作

- **WHEN** `formatCode` 属性のみを持ち `formatCode16` 属性を持たない `<numFmt>` を含む xlsx を分解する
- **THEN** 出力 JSON の当該書式の `formatCode` に従来どおり `formatCode` の値が保持される

