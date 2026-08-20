## MODIFIED Requirements

### Requirement: 本文ブロックの忠実な分解

ツールは本文を文書順のブロック配列に分解し、各ブロックに1始まりの索引を付与しなければならない（SHALL）。段落はスタイル識別子とリスト属性（numId/ilvl）を保持しなければならない（SHALL）。段落プロパティ（`pPr`）の配置（`jc`）、インデント（`ind`：左・右・最初の行・ぶら下げ）、段落間隔（`spacing`：前・後・行間）が存在する場合は構造化して保持しなければならない（SHALL）。`pPr` の未知の子要素は生 XML で保持しなければならない（SHALL）。ランはテキストと書式属性を個別に保持しなければならない（SHALL）。ラン書式プロパティ（`rPr`）として、太字・斜体・下線・取り消し線に加えて、サイズ（`sz`）、色（`color`）、フォント（`rFonts`）、縦位置（`vertAlign`）、字間（`spacing`）、カーニング（`kern`）、位置（`position`）が存在する場合は保持しなければならない（SHALL）。`rPr` の未知の子要素は生 XML で保持しなければならない（SHALL）。ツールはランの結合や Markdown などへの整形を行ってはならない（SHALL NOT）。

#### Scenario: 混在書式の段落の分解

- **WHEN** 「太字のテキスト」と「通常のテキスト」の2ランからなる段落を含む docx を分解する
- **THEN** 当該段落ブロックに2つのランが個別に保持され、それぞれのテキストと書式属性が出力される

#### Scenario: ラン書式プロパティの保持

- **WHEN** `sz="24"`（12pt）、`color val="FF0000"`、`rFonts ascii="Arial"`、`vertAlign val="superscript"` を持つランを含む docx を分解する
- **THEN** 出力 JSON の当該ランにサイズ・色・フォント・縦位置が保持される

#### Scenario: 段落プロパティの保持

- **WHEN** `jc val="center"`、`ind left="720" firstLine="360"`、`spacing before="120" after="60" line="360"` を持つ段落を含む docx を分解する
- **THEN** 出力 JSON の当該段落に配置・インデント・間隔情報が構造化されて保持される

### Requirement: 表の忠実な分解

ツールは表についてグリッド構成・行・セルを保持し、セル結合（gridSpan/vMerge）の属性を保持しなければならない（SHALL）。表プロパティ（`tblPr`）を生 XML で保持しなければならない（SHALL）。行プロパティとして、行の高さ（`trHeight`）、分割禁止（`cantSplit`）、ヘッダー行繰り返し（`tblHeader`）が存在する場合は保持しなければならない（SHALL）。セルプロパティとして、セル幅（`tcW`）、塗りつぶし（`shd`）、セルマージン（`tcMar`）、垂直配置（`vAlign`）、折り返し禁止（`noWrap`）、セル罫線（`tcBorders`）が存在する場合は保持しなければならない（SHALL）。セルの内容は入れ子のブロックとして表現しなければならない（SHALL）。

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

## ADDED Requirements

### Requirement: セクションプロパティの保持

ツールは各セクションのプロパティ（`sectPr`）を生 XML で保持しなければならない（SHALL）。セクションプロパティにはページサイズ（`pgSz`）、ページマージン（`pgMar`）、列構成（`cols`）、セクション種別（`type`）等が含まれる。ツールはセクションプロパティを解釈・適用してはならない（SHALL NOT。生 XML を保持するに留める）。

#### Scenario: ページサイズとマージンの保持

- **WHEN** `pgSz w="11906" h="16838"` と `pgMar top="1440" bottom="1440" left="1800" right="1800"` を持つセクションを含む docx を分解する
- **THEN** 出力 JSON の当該セクションにセクションプロパティが生 XML で保持される
