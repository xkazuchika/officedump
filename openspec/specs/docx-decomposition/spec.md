# docx-decomposition Specification

## Purpose
docx 文書を、構造を正規化しつつ属性を欠落なく保持した JSON 中間表現（IR）へ分解する能力。段落・表・ヘッダー/フッターをブロック単位で索引化し、後段の LLM が意味づけ・整形を行うための情報損失のない入力を提供することを目的とする。
## Requirements
### Requirement: 文書構造の概要取得

ツールは docx を読み、セクション（本文・ヘッダー/フッター）ごとのブロック数と、見出しアウトライン（見出しスタイルを持つ段落の索引・スタイル・テキスト）を JSON で返さなければならない（SHALL）。概要には見出し以外の本文テキストを含めてはならない（MUST NOT）。

#### Scenario: 見出しを含む文書の概要取得

- **WHEN** 見出し段落3つ（Heading1×1、Heading2×2）と本文50ブロックを持つ docx の構造概要を要求する
- **THEN** セクション数・ブロック数と、見出し3件の索引・スタイル・テキストからなるアウトラインが返され、見出し以外の本文テキストは含まれない

### Requirement: 本文ブロックの忠実な分解

ツールは本文を文書順のブロック配列に分解し、各ブロックに1始まりの索引を付与しなければならない（SHALL）。段落はスタイル識別子とリスト属性（numId/ilvl）を保持しなければならない（SHALL）。`ilvl` が省略された場合は `0` とみなして numId を保持しなければならない（SHALL）。段落プロパティ（`pPr`）の配置（`jc`）、インデント（`ind`：左・右・最初の行・ぶら下げ）、段落間隔（`spacing`：前・後・行間）が存在する場合は構造化して保持しなければならない（SHALL）。`pPr` の未知の子要素は生 XML で保持しなければならない（SHALL）。ランはテキストと書式属性を個別に保持しなければならない（SHALL）。ラン書式プロパティ（`rPr`）として、太字・斜体・下線・取り消し線に加えて、サイズ（`sz`）、色（`color`）、フォント（`rFonts`）、縦位置（`vertAlign`）、字間（`spacing`）、カーニング（`kern`）、位置（`position`）が存在する場合は保持しなければならない（SHALL）。`rPr` の未知の子要素は生 XML で保持しなければならない（SHALL）。ツールはランの結合や Markdown などへの整形を行ってはならない（SHALL NOT）。

#### Scenario: 混在書式の段落の分解

- **WHEN** 「太字のテキスト」と「通常のテキスト」の2ランからなる段落を含む docx を分解する
- **THEN** 当該段落ブロックに2つのランが個別に保持され、それぞれのテキストと書式属性が出力される

#### Scenario: ラン書式プロパティの保持

- **WHEN** `sz="24"`（12pt）、`color val="FF0000"`、`rFonts ascii="Arial"`、`vertAlign val="superscript"` を持つランを含む docx を分解する
- **THEN** 出力 JSON の当該ランにサイズ・色・フォント・縦位置が保持される

#### Scenario: 段落プロパティの保持

- **WHEN** `jc val="center"`、`ind left="720" firstLine="360"`、`spacing before="120" after="60" line="360"` を持つ段落を含む docx を分解する
- **THEN** 出力 JSON の当該段落に配置・インデント・間隔情報が構造化されて保持される

#### Scenario: ilvl 省略時のリスト属性保持

- **WHEN** `<w:numPr><w:numId w:val="1"/></w:numPr>` のみを持つ段落を含む docx を分解する
- **THEN** 出力 JSON の当該段落に `num.numId=1`、`num.ilvl=0` が保持される

### Requirement: 表の忠実な分解

ツールは表についてグリッド構成・行・セルを保持し、セル結合（gridSpan/vMerge）の属性を保持しなければならない（SHALL）。表プロパティ（`tblPr`）を生 XML で保持しなければならない（SHALL）。行プロパティとして、行の高さ（`trHeight`）、分割禁止（`cantSplit`）、ヘッダー行繰り返し（`tblHeader`）が存在する場合は保持しなければならない（SHALL）。セルプロパティとして、セル幅（`tcW`）、塗りつぶし（`shd`）、セルマージン（`tcMar`）、垂直配置（`vAlign`）、折り返し禁止（`noWrap`）、セル罫線（`tcBorders`）が存在する場合は保持しなければならない（SHALL）。空要素形式の `tcMar` や `tcBorders`（`<w:tcMar/>` 等）によって、後続の `tcPr` 属性（`vAlign` 等）が失われてはならない（MUST NOT）。セルの内容は入れ子のブロックとして表現しなければならない（SHALL）。

#### Scenario: セル結合を持つ表の分解

- **WHEN** 1行目のセルが結合（gridSpan=2）された2×2の表を含む docx を分解する
- **THEN** 出力 JSON にグリッド構成、gridSpan 属性、およびセル内容の入れ子ブロックが含まれる

#### Scenario: 表プロパティの保持

- **WHEN** `tblW w="5000" type="dxa"` と `tblBorders` を持つ表を含む docx を分解する
- **THEN** 出力 JSON の当該表に表プロパティが生 XML で保持される

#### Scenario: 行プロパティの保持

- **WHEN** `trHeight val="720" hRule="atLeast"` と `tblHeader` を持つ行を含む docx を分解する
- **THEN** 出力 JSON の当該行に行の高さとヘッダー行フラグが保持される

#### Scenario: セルプロパティの保持

- **WHEN** `shd fill="FFFF00"` と `vAlign val="center"` と `tcMar` を持つセルを含む docx を分解する
- **THEN** 出力 JSON の当該セルに塗りつぶし・垂直配置・セルマージンが保持される

#### Scenario: 空の tcMar の後続属性保持

- **WHEN** `<w:tcMar/>` の直後に `<w:vAlign val="center"/>` を持つセルを含む docx を分解する
- **THEN** 出力 JSON の当該セルに `vAlign: "center"` が保持される

### Requirement: ヘッダー/フッターの保持

ツールは文書が参照するヘッダー/フッターを分解し、独立したセクション（header/footer 種別付き）として保持しなければならない（SHALL）。それらのブロックを本文のブロック配列に混ぜてはならない（MUST NOT）。ヘッダー/フッター内の画像等のリレーションは、当該ヘッダー/フッター part の `.rels` を使って解決しなければならない（SHALL）。本文の `document.xml.rels` で誤解決してはならない（MUST NOT）。

#### Scenario: ヘッダーを持つ文書の分解

- **WHEN** ヘッダー部分を参照する docx を分解する
- **THEN** 出力 JSON に header セクションが含まれ、そのブロックは本文セクションに含まれない

#### Scenario: ヘッダー画像のリレーション解決

- **WHEN** ヘッダー内に画像を含み、かつ `header1.xml.rels` に画像リレーションが定義されている docx を分解する
- **THEN** 出力 JSON のメディア項目に `section: "header-default"` のアンカーが含まれ、本文画像と誤って入れ替わらない

### Requirement: ハイパーリンクとフィールドの解決

ツールはハイパーリンクのリレーション ID を解決し、対象 URL を保持しなければならない（SHALL）。ハイパーリンクの `history`・`tooltip`・`tgtFrame` 属性が存在する場合は保持しなければならない（SHALL）。フィールド（ページ番号等）の命令テキストは原文のまま保持し、ツールは評価を行ってはならない（SHALL NOT）。フィールドの `fldLock`・`dirty` 属性が存在する場合は保持しなければならない（SHALL）。

#### Scenario: ハイパーリンクとページ番号フィールドの分解

- **WHEN** 外部リンクのハイパーリンクとページ番号フィールドを含む docx を分解する
- **THEN** ハイパーリンクに対象 URL が保持され、フィールドに命令テキスト（PAGE 等）が原文のまま保持される

#### Scenario: ハイパーリンクの追加属性保持

- **WHEN** `tooltip="ヒント"` と `tgtFrame="_blank"` を持つハイパーリンクを含む docx を分解する
- **THEN** 出力 JSON の当該ハイパーリンクにツールチップとターゲットフレームが保持される

#### Scenario: フィールドの属性保持

- **WHEN** `dirty="1"` と `fldLock="1"` を持つフィールドを含む docx を分解する
- **THEN** 出力 JSON の当該フィールドに dirty と fldLock フラグが保持される

### Requirement: ブロック範囲による部分読み出し

ツールはブロック範囲（`N:M` 形式）を指定して、本文セクションのうち当該範囲内のブロックだけを JSON で返せなければならない（SHALL）。範囲外の本文ブロックを出力に含めてはならない（MUST NOT）。ヘッダー/フッターセクションは範囲指定の影響を受けない。範囲外の本文ブロックに紐づく drawing メディアアンカーも出力に含めてはならない（MUST NOT）。

#### Scenario: 長大な文書の部分読み出し

- **WHEN** 本文100ブロックの docx に対し範囲 1:10 を指定して読み出す
- **THEN** 本文は索引1〜10のブロックのみが返され、11以降は含まれない

#### Scenario: 範囲外ブロックの drawing アンカー削除

- **WHEN** 画像を含む第2本文ブロックを持つ docx に対し範囲 1:1 を指定して読み出す
- **THEN** 第2ブロックの drawing に対するメディアアンカーは出力に含まれない

### Requirement: 未知要素の保持

ツールは本文内の未処理の要素を生 XML のまま保持しなければならず（SHALL）、破棄してはならない（MUST NOT）。上限超過時は切り詰めフラグを付与する。

#### Scenario: 未知要素を含む文書の分解

- **WHEN** 本文がツール未処理の要素を含む docx を分解する
- **THEN** 出力 JSON の unhandledElements に要素名と生 XML が保持される

### Requirement: 機械可読な出力とエラー報告

ツールは `read` を `--stdout` なしで実行した場合、分解結果を出力ルートの `content.json` に JSON として書き出し、標準出力には content と media ディレクトリのパスおよび sections/blocks/media 件数を持つ manifest JSON を書き出さなければならない（SHALL）。ツールは `--stdout` が指定された場合、分解結果 JSON 全量を標準出力に書き出さなければならない（SHALL）。失敗時は非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON を標準エラーに出力しなければならない（SHALL）。未対応の拡張子を持つファイルには機械可読なエラーを返さなければならない（SHALL）。

#### Scenario: 未対応拡張子の処理

- **WHEN** .pptx ファイルを指定して分解を試みる
- **THEN** 非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON が標準エラーに出力される

#### Scenario: docx のファイル中心出力

- **WHEN** docx を `--stdout` なしで読み出す
- **THEN** 分解 JSON は `content.json` に書き出され、標準出力には sections/blocks/media 件数を含む manifest JSON のみが出力される

### Requirement: セクションプロパティの保持

ツールは各セクションのプロパティ（`sectPr`）を生 XML で保持しなければならない（SHALL）。セクションプロパティにはページサイズ（`pgSz`）、ページマージン（`pgMar`）、列構成（`cols`）、セクション種別（`type`）等が含まれる。ツールはセクションプロパティを解釈・適用してはならない（SHALL NOT。生 XML を保持するに留める）。

#### Scenario: ページサイズとマージンの保持

- **WHEN** `pgSz w="11906" h="16838"` と `pgMar top="1440" bottom="1440" left="1800" right="1800"` を持つセクションを含む docx を分解する
- **THEN** 出力 JSON の当該セクションにセクションプロパティが生 XML で保持される

