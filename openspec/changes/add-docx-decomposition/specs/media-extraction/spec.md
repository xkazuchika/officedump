## ADDED Requirements

### Requirement: docx drawing の位置情報の保持

ツールは docx 内の drawing について、アンカー（どのセクションのどのブロック索引・どのラン）と配置の種別（インライン／フローティング）、サイズ情報を JSON に保持しなければならない（SHALL）。フローティング配置では位置属性（posH/posV）と extent（EMU 生値）を保持しなければならない（SHALL）。配置の意味づけ・並べ替え・再配置を行ってはならない（SHALL NOT）。

#### Scenario: インライン画像の分解

- **WHEN** インライン配置の画像を含む docx を分解する
- **THEN** メディア項目に placement=inline、extent（EMU 生値）、ブロック/ランのアンカーが含まれる

#### Scenario: フローティング画像の分解

- **WHEN** フローティング（anchor）配置の画像を含む docx を分解する
- **THEN** メディア項目に placement=floating、posH/posV、extent（EMU 生値）が含まれる
