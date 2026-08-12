# Design: add-pptx-decomposition

## Context

pptx は xlsx / docx と同じく zip + XML（PresentationML）だが、意味的な文書順ではなくスライド上の空間配置が中心である。既存の file-output-mode により、pptx も最初から content.json + manifest のエージェント向け出力契約を使う。動機とスコープは proposal.md を参照。

## Goals / Non-Goals

**Goals:**

- スライド順、図形ツリー順（zOrder）、geometry を保持する JSON IR
- 基本図形、テキストボックス、表、画像への対応
- `inspect` のタイトル一覧と `--slide N:M` の段階的開示
- ppt/media の抽出と、画像位置情報の保持

**Non-Goals:**

- 発表者ノート、コメント、アニメーション、画面切替、埋め込みオブジェクト
- グループ図形の変換行列解決、スマートアート、チャートの意味的データ化
- 読み順・重要度・タイトルの意味的推測

## Decisions

### D1: スライド順は presentation.xml の sldIdLst で決定する

- `ppt/presentation.xml` の `p:sldIdLst` にある `r:id` を presentation rels で解決し、スライド順と1始まりの索引を確定する
- zip 内の slideN.xml のファイル名順は使わない
- **代替案**: ファイル名順（却下 — presentation の実際の順序と異なり得る）

### D2: 図形ツリーの XML 順を zOrder として保持する

- `p:spTree` 直下の `p:sp`（図形）、`p:pic`（画像）、`p:graphicFrame`（表）を出現順に走査し、1始まりの zOrder を付与する
- 各図形の `p:xfrm/a:off`（x/y）と `a:ext`（cx/cy）を EMU 生値で出力する
- XML 順は描画重なり順として保持するだけで、ツールは座標から読み順を作らない

### D3: テキストは図形内の段落・ランを分離して保持する

- `p:txBody/a:p/a:r` を段落・ランとして出力し、太字・斜体・下線などの run property を保持する
- `p:nvPr/p:ph` が `title` / `ctrTitle` の場合、プレースホルダー種別を保持し、inspect のタイトル候補として使う
- **代替案**: 図形内のテキストを連結（却下 — 書式境界とテキストの空間的所属が失われる）

### D4: 表は graphicFrame 内の DrawingML table を構造化する

- `p:graphicFrame/a:graphic/a:graphicData/a:tbl` の gridCol、tr、tc を保持する
- セル内テキストは図形テキストと同じ段落・ランモデルを再利用する
- chart、smartArt 等の他の graphicData は生 XML の escape hatch へ送る

### D5: 画像は slide rels で解決し、スライド/図形の geometry をアンカーにする

- `p:pic/a:blip r:embed` を slide rels で `ppt/media/*` へ解決し、共通 media 抽出経路を使う
- media anchor は `{slide, zOrder, x, y, cx, cy}` を持つ。配置の意味は推定しない

### D6: CLI は拡張子ディスパッチと `--slide` を追加する

- `.pptx` を OfficeFormat に追加し、`inspect` / `read` を pptx parser へ振り分ける
- `--slide N:M` は pptx 専用。`--sheet` / `--range` / `--para` との混用は usage_error
- `--stdout`、`--out`、manifest は file-output-mode の共通処理をそのまま利用し、summary だけ slides/shapes/media に拡張する

### D7: 未対応の図形ツリー要素はスライド粒度で生 XML を保持する

- 基本図形・画像・表以外の `spTree` 直下要素は slide の `unhandledElements` に生 XML として保持する
- raw XML が上限を超える場合は既存の truncated 契約を使う

## Risks / Trade-offs

- [図形XML順と人間の読み順が一致しない] → zOrder と geometry の両方を保持し、LLM に判断を委ねる
- [グループ図形の座標は親変換を加味しないと絶対位置にならない] → MVP では group shape を escape hatch へ送る。変換行列解決は後続change
- [プレースホルダーを使わない独自デザインではタイトルを取得できない] → inspect はタイトル候補がないスライドを空タイトルとして示す。図形テキストの推測はしない
- [チャート/SmartArtの情報が失われる] → 生 XML を保持する。構造化したデータ抽出は別change
