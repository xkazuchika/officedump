## MODIFIED Requirements

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

## ADDED Requirements

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
