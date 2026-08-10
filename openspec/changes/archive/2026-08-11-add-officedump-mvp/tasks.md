# Tasks: add-officedump-mvp

## 1. プロジェクト基盤

- [x] 1.1 `officedump/` を git init し、cargo プロジェクトを初期化する（依存: `clap`, `zip`, `quick-xml`, `serde`, `serde_json`）
- [x] 1.2 CLI 骨格を作る: `inspect` / `read` サブコマンドと `--out` / `--sheet` / `--range` オプションのパース（`--out` 省略時は `<ファイル名>.officedump/`）
- [x] 1.3 エラー出力を統一する: 非ゼロ終了コード + 標準エラーへ JSON（種別・メッセージ）

## 2. xlsx 分解コア

- [x] 2.1 zip 展開レイヤーを実装する（`workbook.xml` / `worksheets/*.xml` / `sharedStrings.xml` / `styles.xml` の取得）
- [x] 2.2 共有文字列テーブルとスタイル表を解決する
- [x] 2.3 ワークシートをストリーミングパースし、セル IR（参照・値・型・書式識別子・数式＋キャッシュ値）を構築する
- [x] 2.4 結合セル範囲を IR に保持する
- [x] 2.5 未知要素を生 XML のまま保持する escape hatch を IR に用意する（情報を落とさない）

## 3. メディア抽出

- [x] 3.1 `xl/media/` 内のファイルを `<out>/media/` へ元バイナリのまま抽出する
- [x] 3.2 drawing XML からアンカー情報・配置種別（インライン／フローティング）・座標・サイズを IR 化する
- [x] 3.3 JSON のメディア参照と抽出ファイルの整合性を担保する（参照切れ・孤立ファイルを出さない）

## 4. CLI 出力

- [x] 4.1 `inspect` を実装する（シート名・寸法。`dimension` 属性を優先し、欠落・不正時は走査で確定）
- [x] 4.2 `read` を実装する（`--sheet` / `--range` フィルタ、JSON IR を標準出力へ）

## 5. テスト・検証

- [x] 5.1 テスト用 xlsx フィクスチャを用意する（数式セル／日付書式セル／結合セル／画像2点／フローティング画像／破損ファイル）
- [x] 5.2 specs の全シナリオに対応する統合テストを書く
- [x] 5.3 `openspec validate add-officedump-mvp --strict` が通ることを確認する
