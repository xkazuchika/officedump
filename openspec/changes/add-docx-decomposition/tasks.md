# Tasks: add-docx-decomposition

## 1. 共通層のリファクタリング

- [x] 1.1 zip アクセス層を xlsx.rs から汎用パッケージモジュールへ切り出す（`XlsxPackage` → 形式共通の `OfficePackage`）
- [x] 1.2 IR を拡張する: docx のブロック型（Section / Paragraph / Run / Table）、MediaAnchor の docx 対応（ブロック/ラン基準アンカー、posH/posV）
- [x] 1.3 CLI を拡張する: `--para N:M` オプション、拡張子ディスパッチ（`.xlsx` / `.docx`、未知の拡張子は `unsupported_format` エラー）

## 2. docx 分解コア

- [x] 2.1 `document.xml` をパースする: 本文の段落とラン（テキスト＋書式属性、numId/ilvl、1始まりのブロック索引）
- [x] 2.2 `styles.xml` を解決する（styleId→name）し、`outlineLvl` で見出しスタイルを検出する
- [x] 2.3 表を分解する（グリッド・行・セル、gridSpan/vMerge 属性、セル内容の入れ子ブロック）
- [x] 2.4 ヘッダー/フッターを分解する（headerReference/footerReference の解決、種別付きセクション化）
- [x] 2.5 ハイパーリンクを解決する（r:id→URL、w:anchor 保持）し、フィールドの instrText を原文保持する
- [x] 2.6 未知要素の escape hatch を実装する（本文直接子はドキュメントレベル、段落・セル内はブロックレベル）

## 3. メディアと CLI コマンド

- [x] 3.1 docx drawing をパースする（wp:inline→inline / wp:anchor→floating、extent・posH/posV、ブロック/ランアンカー）
- [x] 3.2 `word/media/` の抽出を既存の抽出経路に接続する（整合性ルールは共通）
- [x] 3.3 `inspect` を実装する（セクション/ブロック数＋見出しアウトライン）
- [x] 3.4 `read` を実装する（`--para N:M` は本文セクションのみフィルタ、ヘッダー/フッターは常に含める）

## 4. テスト・検証

- [x] 4.1 docx フィクスチャを用意する（見出し・混在書式ラン・結合セル表・ヘッダー・ハイパーリンク・フィールド・インライン/フローティング画像・未知要素・未知拡張子）
- [x] 4.2 specs の全シナリオに対応する統合テストを書く
- [x] 4.3 `openspec validate add-docx-decomposition --strict` が通ることを確認する
