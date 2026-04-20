# Code Editor / コードエディタ

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

Multi-tab code editor displayed in the central area. VS Code-class features.

### Core Features

- Multi-tab editing (file icons + close buttons)
- Line number gutter
- Syntax highlighting (Tree-sitter + Syntect) — Rust, Python, JavaScript, C, C++, TOML, Markdown
- Selection highlight
- Cursor position display
- Code folding (brace matching)

### LSP Integration

| Feature | Shortcut | Description |
|---------|----------|-------------|
| Completions | `Cmd+Space` | Auto-complete with snippet support |
| Hover | Mouse over | Type info & documentation |
| Go to Definition | `F12` / `Cmd+Click` | Jump to symbol definition |
| Find References | `Shift+F12` | Show all usages |
| Diagnostics | Auto | Real-time error/warning display |
| Format | `Cmd+Shift+F` | Format with rustfmt |
| Rename | Dialog | Workspace-wide symbol rename |
| Code Actions | Lightbulb | Quick fixes & refactoring |
| Inlay Hints | Auto | Inline type annotations & parameter names |
| Macro Expand | `Ctrl+Shift+M` | Show expanded Rust macro |

### Vim Mode

Full modal editing. Modes: Normal, Insert, Visual, Command, Replace.

- **Operators**: `d`(delete), `c`(change), `y`(yank), `>`(indent), `<`(dedent), `~`(case toggle)
- **Text objects**: `iw`, `aw`, `i"`, `a"`, `i(`, `a(`, `i{`, `a{`
- **Dot repeat** (`.`), **registers** (`"a`-`"z`), **marks** (`ma`, `'a`), **count prefix** (`5dw`)

### Minimap

60px code overview on right side. Comments in green, functions/keywords in blue, viewport indicator.

### Snippets

- LSP snippet syntax: `$1`, `${2:placeholder}`, `${3|choice1,choice2|}`
- Tab/Shift+Tab navigation between stops
- Custom snippets: `~/.berrycode/snippets/*.json` (VS Code compatible)
- Built-in Rust snippet library

### Cargo.toml Completion

- Fuzzy crate name search via crates.io API
- Version list fetching
- Crate info (name, version, description, downloads)

### Peek Definition

- Float below cursor line with file/line header
- Preview first 10 lines of definition
- `Esc` to close

### Code Folding

- Click fold markers in gutter
- Placeholder: `// ... (N lines)`
- Brace-matching auto-detection
- Per-tab fold state persistence

### Image Preview

- Formats: PNG, JPG, GIF, WebP, BMP, ICO, SVG
- Fit-to-view scaling, SVG via resvg

### 3D Model Preview

- Formats: GLB/GLTF, OBJ, STL, PLY
- Mesh/vertex/triangle/material/animation counts
- Gaussian Splatting support (PLY)
- GPU-rendered preview (512x512 off-screen, orbit camera)

### Test Runner

- Scan `#[test]` / `#[tokio::test]` from .rs files
- Run individual tests (`cargo test --exact`)
- Inline pass/fail results
- Test explorer panel

---

<a name="japanese"></a>

## 日本語

中央エリアに表示されるマルチタブコードエディタ。VS Code クラスの機能を備えています。

## 基本機能

- マルチタブ編集（ファイルアイコン + 閉じるボタン）
- 行番号ガター
- シンタックスハイライト（Tree-sitter + Syntect）
  - Rust, Python, JavaScript, C, C++, TOML, Markdown
- 選択ハイライト
- カーソル位置表示
- コード折りたたみ（ブレースマッチング）

---

## LSP 統合

| 機能 | ショートカット | 説明 |
|------|-------------|------|
| 補完 | `Cmd+Space` | スニペット対応の自動補完 |
| ホバー | マウスオーバー | 型情報・ドキュメント表示 |
| 定義に移動 | `F12` / `Cmd+Click` | シンボルの定義元にジャンプ |
| 参照検索 | `Shift+F12` | シンボルの全使用箇所を表示 |
| 診断 | 自動 | リアルタイムエラー/警告表示 |
| フォーマット | `Cmd+Shift+F` | rustfmt で整形 |
| リネーム | ダイアログ | ワークスペース全体のシンボルリネーム |
| コードアクション | 電球アイコン | クイックフィックス・リファクタリング |
| インレイヒント | 自動 | 型注釈・パラメータ名のインライン表示 |
| マクロ展開 | `Ctrl+Shift+M` | Rust マクロの展開表示 |

---

## Vim Mode

完全なモーダル編集。モード:

| モード | 説明 |
|--------|------|
| **Normal** | `hjkl` 移動, `w/b/e` ワード, `0/$` 行頭/末, `gg/G` ファイル先頭/末, `f/F/t/T` 文字検索 |
| **Insert** | テキスト入力, `Esc` で Normal に戻る |
| **Visual** | `v` 文字選択, `V` 行選択, `Ctrl+V` 矩形選択 |
| **Command** | `:w` 保存, `:q` 閉じる, `:wq` 保存して閉じる, `:<n>` 行移動 |
| **Replace** | `r` + 文字で1文字置換 |

オペレータ: `d`(削除), `c`(変更), `y`(コピー), `>`(インデント), `<`(デデント), `~`(大文字小文字)

テキストオブジェクト: `iw`, `aw`, `i"`, `a"`, `i(`, `a(`, `i{`, `a{`

ドットリピート(`.`), レジスタ(`"a`-`"z`), マーク(`ma`, `'a`), カウント前置(`5dw`)

---

## ミニマップ

エディタ右側の60pxコード概要表示。

- コメント: 緑
- 関数/構造体/キーワード: 青
- 通常コード: 薄グレー
- ビューポートインジケーター（半透明ハイライト）
- ファイルサイズに応じた自動スケーリング

---

## スニペット

LSP スニペット構文対応のテンプレート展開。

- `$1`, `${2:placeholder}`, `${3|choice1,choice2|}` 対応
- `Tab` / `Shift+Tab` でタブストップ間を移動
- カスタムスニペット: `~/.berrycode/snippets/*.json`（VS Code 互換フォーマット）
- 組み込み Rust スニペットライブラリ
- テンプレート変数: `$TM_FILENAME`, `$TM_LINE_NUMBER` 等

---

## Cargo.toml 補完

- crates.io API を利用したクレート名の fuzzy 検索
- バージョン一覧の取得・補完
- クレート情報（名前, バージョン, 説明, ダウンロード数）
- 非同期 HTTP リクエスト

---

## Peek Definition (定義プレビュー)

- カーソル下の行にフロート表示
- ファイルパス + 行番号ヘッダー
- 定義の先頭 10 行をプレビュー
- `Esc` で閉じる

---

## コード折りたたみ

- ガターの折りたたみマーカーをクリック
- プレースホルダー: `// ... (N lines)`
- ブレースマッチングによる自動検出
- タブごとに折りたたみ状態を保持

---

## 画像プレビュー

エディタタブ内で画像ファイルを表示。

- 対応フォーマット: PNG, JPG, GIF, WebP, BMP, ICO, SVG
- ビューにフィット表示
- SVG は resvg でレンダリング
- 画像サイズ・ファイルサイズ表示

---

## 3D モデルプレビュー

エディタタブ内で 3D モデルのメタデータ + ワイヤーフレーム表示。

- 対応フォーマット: GLB/GLTF, OBJ, STL, PLY
- メッシュ数, 頂点/三角形数, マテリアル数, アニメーション数
- Gaussian Splatting 対応（PLY）
- GPU レンダリングプレビュー（512x512 オフスクリーン, オービットカメラ）

---

## テストランナー

- `#[test]` / `#[tokio::test]` を .rs ファイルからスキャン
- 個別テスト実行（`cargo test --exact`）
- 結果インライン表示（pass/fail）
- テストエクスプローラーパネル
