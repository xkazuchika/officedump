# Design: add-docx-decomposition

## Context

第二段の形式対応。動機とスコープは proposal.md を参照。docx も「zip + XML（WordprocessingML）」であり、xlsx で確立したパイプライン（zip 層・XML パース・IR・escape hatch・メディア抽出）を再利用する。差異はデータモデルにある：xlsx が表形式（行列）であるのに対し、docx は流し込みの文書構造（段落・表・セクション）である。

## Goals / Non-Goals

**Goals:**

- docx →「セクション×ブロックの JSON IR + `media/` 子フォルダ」の分解を確立する
- 見出しアウトライン付き inspect と、`--para N:M` によるブロック範囲の部分読み出し
- 既存 xlsx 挙動を一切壊さない（拡張子ディスパッチで共存）

**Non-Goals:**

- 脚注・尾注・コメント・変更履歴、リスト番号の解決、Markdown 変換、MCP 化

## Decisions

### D1: ブロックモデル — sections × blocks、1始まりの索引

- 出力は `sections: [{type, blocks: []}]`。type は `body` / `header` / `footer`（＋後述の変種）
- ブロック種別は `paragraph` / `table`。各ブロックに文書順の1始まりの `index` を付与し、メディアアンカー・部分読み出し・見出しアウトラインの共通キーにする
- 段落はランの配列を保持し、結合しない（書式の境界が失われるため）。表のセル内容も入れ子のブロックで表現する
- **代替案**: フラットな要素列（却下 — セクション構造が見えなくなる）、DOM を丸ごと JSON 化（却下 — トークン爆発）

### D2: ヘッダー/フッターは独立セクションとして保持

- `sectPr` の `headerReference` / `footerReference`（種別 default/first/even）を解決し、参照先パートを `header-default` / `footer-even` のような種別付きセクションとして分解する
- 本文ブロックと混ぜない（順序の曖昧さとトークン浪費の回避）
- **代替案**: 破棄（却下 — 「属性は全保持」の原則違反）

### D3: 見出しの判定は outlineLvl で行う

- `styles.xml` で段落スタイル定義に `<w:outlineLvl>` を持つものを「見出しスタイル」とみなす（組み込みの Heading スタイルはこの属性を持つ）
- 属性ベースの決定論的な判定であり、解釈には該当しない。判定結果は inspect のアウトラインにのみ使い、本文 IR のスタイル属性自体は生値を保持する

### D4: ハイパーリンクは URL 解決、フィールドは原文保持

- ハイパーリンク: リレーション ID を `document.xml.rels` で解決して対象 URL を保持。`w:anchor`（文書内リンク）は属性のまま保持
- フィールド（`fldSimple` / `instrText`）: 命令テキストを生テキストのまま保持し、評価しない（数式キャッシュ値の方針と同じ）

### D5: drawing アンカーは配置種別＋ブロック/ラン基準

- `wp:inline` → `placement: inline`、`wp:anchor` → `placement: floating`
- アンカーは `{section, block, run}` の索引で示す（xlsx の sheet/セル座標に相当する位置情報）
- `extent`（cx/cy）は EMU 生値。floating は `posH` / `posV`（relativeFrom と off/align を生値で保持）
- blip の `r:embed` は drawing rels で解決し、既存のメディア抽出・整合性ルール（media-extraction capability）をそのまま適用する

### D6: CLI は拡張子ディスパッチ＋`--para`

- `.xlsx` / `.docx` を拡張子で振り分け、未知の拡張子は `unsupported_format` 種別の機械可読エラー
- `--para N:M` は本文セクションのブロック範囲にのみ適用する（ヘッダー/フッターは少量のため常に含める）
- 範囲の区切り文字は xlsx の `--range` と統一してコロン（`1:10`）を使う

### D7: escape hatch の適用範囲

- 本文（`w:body`）の未処理の直接子要素 → ドキュメントレベルの `unhandledElements`（生 XML、上限超過で truncated フラグ — xlsx と同一挙動）
- 段落・セル内部の未処理要素 → 該当ブロックの `unhandled` に保持（ブロック粒度で文脈が分かるようにする）

## Risks / Trade-offs

- [WordprocessingML の要素種別が非常に多く、初回カバレッジが限定的] → escape hatch をブロック粒度まで広げて「落とさない」を保証。MVP は段落/ラン/表/ハイパーリンク/フィールド/drawing に集中する
- [ヘッダー/フッターの参照変種（default 以外の first/even、複数 sectPr）] → 参照種別をそのままセクション種別に反映し、未処理の参照はパート名ベースで保持する
- [見出し判定が outlineLvl に依存する] → カスタム見出しスタイルで outlineLvl が欠ける場合はアウトラインに出ないが、本文 IR のスタイル属性は保持されるため LLM 側で復元可能。許容する

## Open Questions

（なし — 範囲（本文+ヘッダー/フッター+表+メディア）、inspect（アウトライン付き）、部分読み出し（`--para N:M`）は提案時に確定済み）
