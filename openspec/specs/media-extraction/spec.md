# media-extraction Specification

## Purpose
Office ファイル内のメディア（画像等）をファイルシステム上の子フォルダへ抽出し、JSON 側からの参照と位置情報を保持する能力。既存コンバータで頻発する「図のずれ」を、データを線形テキストへ無理に流し込まないことで原理的に排除する。
## Requirements
### Requirement: メディアの子フォルダへの抽出

ツールはファイル内のメディアを、出力先の子フォルダ（`media/`）へ元のバイナリのまま抽出しなければならない（SHALL）。JSON 中間表現にメディアのバイナリを埋め込んではならず（MUST NOT）、抽出先への相対パスで参照しなければならない（SHALL）。メディアパスは正規化され、`media/` 出力先の外にファイルを書き出してはならない（MUST NOT）。`..` による逸脱や空・カレント成分は解決または拒否しなければならない（SHALL）。サブディレクトリに存在するメディアはその構造を保持して basename 衝突を避けなければならない（SHALL）。

#### Scenario: 画像を含む xlsx の分解

- **WHEN** 画像2点を含む xlsx を分解する
- **THEN** 子フォルダに2つの画像ファイルが生成され、JSON にはそれらへの相対パスが記録される

#### Scenario: 悪意あるメディアパスの拒否

- **WHEN** `word/media/..` のようなメディアエントリを含む Office ファイルを分解する
- **THEN** エラー終了し、出力ルートの外にファイルを書き出さない

#### Scenario: サブディレクトリ内メディアの保持

- **WHEN** `word/media/sub/image.png` を含む docx を分解する
- **THEN** `media/sub/image.png` として抽出され、他の `image.png` との basename 衝突を避ける

### Requirement: メディアの位置情報の保持

ツールは各メディアについて、アンカー情報（どのシート・どの位置に紐づくか）と、配置の種別（インライン／フローティング）を JSON に保持しなければならない（SHALL）。座標・サイズが存在する場合はそれらも保持しなければならない（SHALL）。配置の意味づけ・並べ替え・再配置を行ってはならない（SHALL NOT）。

#### Scenario: フローティング画像の分解

- **WHEN** セル範囲にまたがるフローティング画像を含む xlsx を分解する
- **THEN** JSON の当該メディア項目に、アンカー情報と座標・サイズが含まれる

### Requirement: 抽出物と参照の整合性

JSON から参照されるメディアのパスはすべて実在しなければならない（SHALL）。また、抽出されたメディアファイルはすべて JSON から参照されなければならない（SHALL。参照切れ・孤立ファイルを出してはならない）。

#### Scenario: 整合性の検証

- **WHEN** メディアを含む xlsx の分解が完了する
- **THEN** JSON 内の全メディア参照が実在ファイルを指し、かつ JSON から参照されない抽出ファイルが存在しない

### Requirement: docx drawing の位置情報の保持

ツールは docx 内の drawing について、アンカー（どのセクションのどのブロック索引・どのラン）と配置の種別（インライン／フローティング）、サイズ情報を JSON に保持しなければならない（SHALL）。フローティング配置では位置属性（posH/posV）と extent（EMU 生値）を保持しなければならない（SHALL）。配置の意味づけ・並べ替え・再配置を行ってはならない（SHALL NOT）。

#### Scenario: インライン画像の分解

- **WHEN** インライン配置の画像を含む docx を分解する
- **THEN** メディア項目に placement=inline、extent（EMU 生値）、ブロック/ランのアンカーが含まれる

#### Scenario: フローティング画像の分解

- **WHEN** フローティング（anchor）配置の画像を含む docx を分解する
- **THEN** メディア項目に placement=floating、posH/posV、extent（EMU 生値）が含まれる

### Requirement: pptx 画像の位置情報の保持

ツールは pptx 内の画像について、スライド索引、図形の zOrder、geometry（x/y/cx/cy の EMU 生値）を JSON に保持しなければならない（SHALL）。ツールは画像の位置・サイズ・描画順を解釈して並べ替えや再配置を行ってはならない（SHALL NOT）。

#### Scenario: geometry を持つスライド画像の分解

- **WHEN** 特定の位置とサイズを持つ画像を含むスライドを分解する
- **THEN** メディア項目にスライド索引、zOrder、x/y/cx/cy の EMU 生値が含まれる
