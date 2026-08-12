# Tasks: add-file-output-mode

## 1. 共通出力基盤

- [x] 1.1 content / media の共通出力ルート解決を実装する（`--out` または `<stem>.officedump/`、media なしでも `media/` を生成）
- [x] 1.2 IR を一時ファイル経由で `<out>/content.json` へ原子的に書き出す共通ヘルパーを実装する
- [x] 1.3 content・mediaDir の絶対パスと形式別 summary を持つ manifest IR を追加する

## 2. read 出力モード

- [x] 2.1 `read` に `--stdout` を追加し、未指定時は content ファイル＋manifest stdout、指定時は全 IR stdout に切り替える
- [x] 2.2 xlsx read を共通出力ヘルパーへ接続し、sheets/cells/media summary を返す
- [x] 2.3 docx read を共通出力ヘルパーへ接続し、sections/blocks/media summary を返す
- [x] 2.4 manifest は content と media の書き出し成功後にのみ stdout へ出すようにする

## 3. ドキュメントとテスト

- [x] 3.1 README、CLI help、`docs/agent-integration.md` を、既定のファイル中心出力・`--stdout`・部分読み出しのエージェント向けワークフローに更新する。agent-integration.md は将来の Skill 作成の正本とする
- [x] 3.2 xlsx / docx の既存統合テストを更新し、既定 manifest、content.json、media ディレクトリ、`--stdout` の全シナリオを追加する
- [x] 3.3 `cargo fmt --check`、`cargo test`、`cargo clippy -- -D warnings`、`openspec validate add-file-output-mode --strict` を実行する
