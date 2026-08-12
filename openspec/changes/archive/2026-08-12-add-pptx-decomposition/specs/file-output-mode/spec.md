## ADDED Requirements

### Requirement: pptx manifest の要約

ツールは pptx の `read` を既定モードで実行した場合、manifest の summary に slides、shapes、media の件数を含めなければならない（SHALL）。content と mediaDir は既存のファイル中心出力契約どおり絶対パスで返さなければならない（SHALL）。

#### Scenario: pptx 既定出力の manifest

- **WHEN** 図形と画像を持つ pptx を `--stdout` なしで読み出す
- **THEN** manifest の summary に slides、shapes、media 件数があり、content と mediaDir は絶対パスで返される
