## MODIFIED Requirements

### Requirement: 機械可読な出力とエラー報告

ツールは `read` を `--stdout` なしで実行した場合、分解結果を出力ルートの `content.json` に JSON として書き出し、標準出力には content と media ディレクトリのパスおよび sheets/cells/media 件数を持つ manifest JSON を書き出さなければならない（SHALL）。ツールは `--stdout` が指定された場合、分解結果 JSON 全量を標準出力に書き出さなければならない（SHALL）。失敗時は非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON を標準エラーに出力しなければならない（SHALL）。

#### Scenario: 破損ファイルの処理

- **WHEN** 破損した（zip として開けない）xlsx ファイルを指定して分解を試みる
- **THEN** 非ゼロの終了コードで終了し、エラー種別とメッセージを含む JSON が標準エラーに出力される

#### Scenario: xlsx のファイル中心出力

- **WHEN** xlsx を `--stdout` なしで読み出す
- **THEN** 分解 JSON は `content.json` に書き出され、標準出力には sheets/cells/media 件数を含む manifest JSON のみが出力される
