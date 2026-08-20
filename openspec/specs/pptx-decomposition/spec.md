# pptx-decomposition Specification

## Purpose

pptx プレゼンテーションを、スライド内の図形・テキスト・表・画像を含む JSON 中間表現（IR）へ分解する能力。スライド順、描画順、geometry の生値を保持し、既存コンバータで起きやすい文字・図形・画像の順序崩壊を、意味的な読み順を決めずに回避する。

## Requirements

### Requirement: プレゼンテーション構造の概要取得

ツールは pptx を読み、スライド順のスライド数と、タイトルプレースホルダーを持つスライドの索引・タイトルテキストを JSON で返さなければならない（SHALL）。概要にタイトル以外の図形テキストを含めてはならない（MUST NOT）。

#### Scenario: タイトルを持つ複数スライドの概要取得

- **WHEN** タイトル付きスライド3枚を持つ pptx の構造概要を要求する
- **THEN** スライド数と、各スライドの索引・タイトルテキストがスライド順で返される

### Requirement: 図形の描画順と geometry の保持

ツールはスライドの図形ツリーに現れる基本図形・画像・表を JSON 配列へ分解し、図形ツリー内の順序を1始まりの zOrder として保持しなければならない（SHALL）。各図形は存在する場合、geometry の x/y/cx/cy を EMU 生値で保持しなければならない（SHALL）。`xfrm` 要素の回転（`rot`）、水平反転（`flipH`）、垂直反転（`flipV`）属性が存在する場合は保持しなければならない（SHALL）。プリセット図形種別（`prstGeom` の `prst` 属性: rect/roundRect/ellipse 等）が存在する場合は保持しなければならない（SHALL）。`spPr` 要素の生 XML を保持しなければならない（SHALL）。プレースホルダー（`ph`）の `type`・`idx`・`sz`・`orient` 属性が存在する場合は構造化して保持しなければならない（SHALL）。ツールは zOrder や geometry から意味的な読み順を推定・並べ替えしてはならない（SHALL NOT）。

#### Scenario: 空間的に配置された図形の分解

- **WHEN** 描画順が異なり、異なる位置とサイズを持つテキストボックス2つを含むスライドを分解する
- **THEN** 各図形に元の zOrder と x/y/cx/cy が保持され、配列順や geometry に基づく並べ替えは行われない

#### Scenario: 回転と反転の保持

- **WHEN** `rot="2700000"`（45度）、`flipH="1"`、`flipV="1"` 属性を持つ図形を含むスライドを分解する
- **THEN** 出力 JSON の当該図形の geometry に回転・水平反転・垂直反転が保持される

#### Scenario: プリセット図形種別の保持

- **WHEN** `prstGeom prst="roundRect"` を持つ図形を含むスライドを分解する
- **THEN** 出力 JSON の当該図形にプリセット図形種別 `roundRect` が保持される

#### Scenario: プレースホルダー属性の保持

- **WHEN** `type="title"`・`idx="0"`・`sz="half"` を持つプレースホルダー図形を含むスライドを分解する
- **THEN** 出力 JSON の当該図形にプレースホルダー種別・インデックス・サイズが構造化されて保持される

### Requirement: テキスト図形の忠実な分解

ツールはテキストを持つ図形について、テキスト段落とランを個別に保持しなければならない（SHALL）。ランのテキストと書式属性（太字・斜体・下線・取り消し線）に加えて、サイズ（`sz`）、色（`color`）、フォント名（`typeface`）が存在する場合は保持しなければならない（SHALL）。段落プロパティとして、配置（`algn`）、レベル（`lvl`）、箇条書き（`buChar`/`buAutoNum`）、左マージン（`marL`）、インデント（`indent`）、行間（`lnSpc`）、段落間隔（`spcBef`/`spcAft`）が存在する場合は保持しなければならない（SHALL）。テキストボディプロパティ（`bodyPr`）の生 XML を保持しなければならない（SHALL）。ツールはランの結合や Markdown 等への整形を行ってはならない（SHALL NOT）。タイトルプレースホルダーの種別は図形属性として保持しなければならない（SHALL）。

#### Scenario: 混在書式のタイトル図形の分解

- **WHEN** 太字ランと通常ランを含むタイトルプレースホルダー図形を持つスライドを分解する
- **THEN** 図形にタイトルプレースホルダー種別が保持され、2つのランのテキストと書式属性が個別に出力される

#### Scenario: テキストランの書式プロパティ保持

- **WHEN** `sz="2400"`（24pt）、色 `srgbClr val="FF0000"`、`typeface="Arial"` を持つランを含むスライドを分解する
- **THEN** 出力 JSON の当該ランにサイズ・色・フォント名が保持される

#### Scenario: 段落プロパティの保持

- **WHEN** `algn="ctr"`・`lvl="1"`・`marL="457200"`・`indent="228600"` と `buChar char="•"` を持つ段落を含むスライドを分解する
- **THEN** 出力 JSON の当該段落に配置・レベル・マージン・インデント・箇条書き文字が保持される

#### Scenario: テキストボディプロパティの保持

- **WHEN** `vert="vert"`・`anchor="ctr"`・`wrap="square"` を持つ `bodyPr` を含む図形を含むスライドを分解する
- **THEN** 出力 JSON の当該テキストフレームに `bodyPr` が生 XML で保持される

### Requirement: 表図形の忠実な分解

ツールは表を持つ graphicFrame 図形について、列幅・行・セルを保持しなければならない（SHALL）。各行の高さ（`h`）が存在する場合は保持しなければならない（SHALL）。各セルのテキスト段落とランを保持しなければならない（SHALL）。セル結合として、`gridSpan`・`rowSpan`・`hMerge`・`vMerge` が存在する場合は保持しなければならない（SHALL）。`graphicFrame` の `graphicData/@uri` を見て、表 (`.../table`) 以外（チャート等）を `type: "table"` と誤判定してはならない（MUST NOT）。

#### Scenario: 表を含むスライドの分解

- **WHEN** 2列2行の表を含むスライドを分解する
- **THEN** 当該図形に列幅、2行のセル構造、および各セルのテキストが含まれる

#### Scenario: チャート graphicFrame の分解

- **WHEN** 表ではなくチャートを含む `graphicFrame` を持つスライドを分解する
- **THEN** 当該図形の `type` は "chart" または "graphicFrame" であり、誤って `table` にならない

#### Scenario: セル結合を持つ表の分解

- **WHEN** `gridSpan="2"` と `rowSpan="2"` を持つセルを含む表を含むスライドを分解する
- **THEN** 出力 JSON の当該セルに gridSpan と rowSpan が保持される

#### Scenario: 行の高さの保持

- **WHEN** `h="457200"` を持つ行を含む表を含むスライドを分解する
- **THEN** 出力 JSON の当該行に行の高さが保持される

### Requirement: スライド範囲による部分読み出し

ツールはスライド範囲（`N:M` 形式）を指定して、その範囲内のスライドだけを JSON として返せなければならない（SHALL）。範囲外のスライドを出力に含めてはならない（MUST NOT）。スライド索引はプレゼンテーション全体での1始まりの索引を保持しなければならない（SHALL）。

#### Scenario: 長いプレゼンテーションの部分読み出し

- **WHEN** 20枚の pptx に対し `--slide 5:8` を指定して読み出す
- **THEN** 索引5〜8の4スライドだけが出力され、各スライドの索引は5〜8のまま保持される

### Requirement: 未知図形要素の保持

ツールはスライド図形ツリー内の未処理要素を生 XML のまま保持しなければならず（SHALL）、破棄してはならない（MUST NOT）。上限超過時は切り詰めフラグを付与する。

#### Scenario: 未知図形要素を含むスライドの分解

- **WHEN** ツール未処理の図形ツリー要素を含む pptx を分解する
- **THEN** 当該スライドの unhandledElements に要素名と生 XML が保持される

### Requirement: 機械可読な出力とエラー報告

ツールは `read` を `--stdout` なしで実行した場合、分解結果を出力ルートの `content.json` に JSON として書き出し、標準出力には slides/shapes/media 件数を持つ manifest JSON を書き出さなければならない（SHALL）。`--stdout` 指定時は分解結果 JSON 全量を標準出力へ書き出さなければならない（SHALL）。失敗時は非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON を標準エラーに出力しなければならない（SHALL）。

#### Scenario: 破損 pptx の処理

- **WHEN** 破損した（zip として開けない）pptx ファイルを指定して分解を試みる
- **THEN** 非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON が標準エラーに出力される
