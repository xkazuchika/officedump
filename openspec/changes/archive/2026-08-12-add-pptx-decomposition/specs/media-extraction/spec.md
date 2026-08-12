## ADDED Requirements

### Requirement: pptx 画像の位置情報の保持

ツールは pptx 内の画像について、スライド索引、図形の zOrder、geometry（x/y/cx/cy の EMU 生値）を JSON に保持しなければならない（SHALL）。ツールは画像の位置・サイズ・描画順を解釈して並べ替えや再配置を行ってはならない（SHALL NOT）。

#### Scenario: geometry を持つスライド画像の分解

- **WHEN** 特定の位置とサイズを持つ画像を含むスライドを分解する
- **THEN** メディア項目にスライド索引、zOrder、x/y/cx/cy の EMU 生値が含まれる
