# Sidebar Panels / サイドバーパネル

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

The left sidebar consists of 10 panels switchable via `Ctrl+1` through `Ctrl+9`. Panels 6-10 are **Bevy-native tools** that don't exist in any other IDE.

---

### Bevy-Native Panels

### 6. ECS Inspector `Ctrl+6`

Real-time ECS state monitoring for running Bevy apps. **No other IDE has this.**

- Connect to Bevy app via gRPC (BRP — Bevy Remote Protocol)
- **Entities tab**: Entity list + component details
- **Resources tab**: Resource type list + values
- Filter/search by component type
- Auto-refresh
- Connection status indicator (green/red)

### 7. Bevy Templates `Ctrl+7`

One-click Bevy boilerplate generation. **Stop typing `#[derive(Component)]` by hand.**

| Template | Generated Code |
|----------|---------------|
| Component | Struct with fields + Component derive |
| Resource | Struct with fields + Resource derive |
| System | System function with parameters |
| Plugin | Plugin trait implementation |
| Startup System | Startup system function |
| Event | Event struct |
| State | Enum + States derive |

- Dynamic field name/type input
- Insert at cursor position in current file
- Preview before insertion

### 8. Asset Browser `Ctrl+8`

Browse and manage project assets — textures, models, audio, scenes, shaders.

- Root directory input + recursive scan
- Type filter: All / Images / Models / Audio / Scenes / Shaders
- List/Grid view toggle
- Image thumbnail preview
- Double-click to import/use
- Drag & drop to editor/scene

### 9. Scene Editor (Hierarchy) `Ctrl+9`

Unity-style scene hierarchy panel. See [Scene Editor](Scene-Editor) for the full 3D viewport documentation.

- Scene tabs (multi-scene editing, dirty marker `*`)
- Entity tree (expand/collapse)
- Drag & drop reparenting
- Right-click menu: Create, Delete, Duplicate, Rename
- Inline rename (double-click)
- Toolbar: New / Save / Tools dropdown
- Play Mode banner (editing disabled during playback)

---

### General IDE Panels

### 1. Explorer `Ctrl+1`

Hierarchical file browser for the project.

- Color-coded file type icons
- New File / New Folder action buttons
- Right-click context menu (New, Delete, Rename, Reveal in Finder)
- Git status indicators (modified / staged)
- Drag & drop 3D assets (GLB/OBJ/STL/PLY) onto Scene View
- Favorites / bookmarks

### 2. Search `Ctrl+2`

Project-wide text search.

- Regex support
- Case-sensitive option
- Results list (file path + line number + preview)
- Click to jump to line
- Find & Replace (`Cmd+H`)
- Parallel search via Rayon

### 3. Git `Ctrl+3`

SourceTree-style 6-tab Git panel.

| Tab | Features |
|-----|----------|
| **Status** | Changed files list, stage/unstage, commit message input |
| **History** | Commit graph, author/message filter, pagination |
| **Branches** | Local/remote branches, merge, create new branch |
| **Remotes** | Remote list, add/edit/remove |
| **Tags** | Tag list, annotated tag creation |
| **Stash** | Stash list, pop/apply |

### 4. Terminal `Ctrl+4`

iTerm2-class PTY terminal emulator.

- Multiple tabs (click `+` to add)
- Full VT100/xterm escape sequence support
- ANSI 256 colors
- Mouse selection + clipboard copy
- 10,000-line scrollback buffer
- Cursor blink animation
- Alternate screen buffer (vim/less support)
- Right-click menu (Copy/Paste/Clear/Close)

### 5. Settings `Ctrl+5`

RustRover-style IDE settings panel.

- **Color Scheme**: Live syntax color customization (Keyword, Function, Type, String, Number, Comment, Attribute)
- **Keybindings**: Custom keyboard shortcuts
- **Appearance**: Theme settings (Coming soon)
- **Plugins**: Plugin management (Coming soon)

---

<a name="japanese"></a>

## 日本語

左サイドバーは `Ctrl+1`〜`Ctrl+9` で切り替え可能な10パネルで構成されています。パネル 6-10 は**他のIDEには存在しない Bevy 専用ツール**です。

---

### Bevy 専用パネル

### 6. ECS Inspector `Ctrl+6`

実行中の Bevy アプリの ECS 状態をリアルタイム監視。**他のIDEにこの機能はありません。**

- BRP (Bevy Remote Protocol) 経由で gRPC 接続
- **Entities タブ**: エンティティ一覧 + コンポーネント詳細
- **Resources タブ**: リソース型一覧 + 値表示
- コンポーネント型でフィルター・検索
- 自動リフレッシュ + 接続ステータスインジケーター（緑/赤）

### 7. Bevy Templates `Ctrl+7`

ワンクリックの Bevy ボイラープレート生成。**`#[derive(Component)]` を手で打つのはもう終わり。**

| テンプレート | 生成内容 |
|-------------|---------|
| Component | フィールド付き struct + Component derive |
| Resource | フィールド付き struct + Resource derive |
| System | パラメータ付きシステム関数 |
| Plugin | Plugin trait 実装 |
| Startup System | スタートアップシステム |
| Event | イベント struct |
| State | enum + States derive |

- フィールド名と型を動的入力
- 現在のファイルのカーソル位置に挿入
- プレビュー表示

### 8. Asset Browser `Ctrl+8`

プロジェクトアセットの閲覧・管理 — テクスチャ、モデル、オーディオ、シーン、シェーダー。

- ルートディレクトリ指定 + 再帰スキャン
- タイプフィルター: All / Images / Models / Audio / Scenes / Shaders
- リスト表示/グリッド表示の切替
- 画像サムネイルプレビュー
- ダブルクリックでインポート/使用
- ドラッグ&ドロップでエディタ/シーンに追加

### 9. Scene Editor (シーンヒエラルキー) `Ctrl+9`

Unity スタイルのシーン階層パネル。3Dビューポートの詳細は [シーンエディタ](Scene-Editor) を参照。

- シーンタブ（複数シーン同時編集、変更マーク `*`）
- エンティティツリー（展開/折りたたみ）
- ドラッグ&ドロップで親子関係変更
- 右クリックメニュー: 新規作成, 削除, 複製, リネーム
- インラインリネーム（ダブルクリック）
- ツールバー: New / Save / Tools ドロップダウン
- Play Mode バナー（再生中は編集無効）

---

### 汎用 IDE パネル

### 1. Explorer (ファイルツリー) `Ctrl+1`

プロジェクトのファイルを階層表示するブラウザ。

- ファイル種別ごとのカラーアイコン
- 新規ファイル/フォルダ作成ボタン
- 右クリックコンテキストメニュー（新規, 削除, リネーム, Finderで開く）
- Git ステータスインジケーター（変更/ステージ済み）
- 3Dアセット（GLB/OBJ/STL/PLY）のシーンビューへのドラッグ&ドロップ
- お気に入り/ブックマーク

### 2. Search (検索) `Ctrl+2`

プロジェクト全体のテキスト検索。

- 正規表現対応
- 大文字/小文字区別オプション
- 結果一覧（ファイルパス + 行番号 + プレビュー）
- クリックで該当行にジャンプ
- 置換機能 (`Cmd+H`)
- Rayon による並列検索

### 3. Git `Ctrl+3`

SourceTree スタイルの6タブ Git パネル。

| タブ | 機能 |
|------|------|
| **Status** | 変更ファイル一覧、ステージ/アンステージ、コミットメッセージ入力 |
| **History** | コミットグラフ、著者/メッセージフィルター、ページネーション |
| **Branches** | ローカル/リモートブランチ、マージ、新規ブランチ作成 |
| **Remotes** | リモート一覧、追加/編集/削除 |
| **Tags** | タグ一覧、注釈付きタグ作成 |
| **Stash** | スタッシュ一覧、pop/apply |

### 4. Terminal (ターミナル) `Ctrl+4`

iTerm2 クラスの PTY ターミナルエミュレータ。

- 複数タブ対応（`+` で新規追加）
- VT100/xterm エスケープシーケンス完全対応
- ANSI 256 カラー
- マウス選択 + クリップボードコピー
- スクロールバック 10,000 行
- カーソルブリンクアニメーション
- 代替スクリーンバッファ（vim/less 対応）
- 右クリックメニュー（コピー/ペースト/クリア/閉じる）

### 5. Settings (設定) `Ctrl+5`

RustRover スタイルの IDE 設定パネル。

- **Color Scheme**: シンタックスカラーのライブカスタマイズ (Keyword, Function, Type, String, Number, Comment, Attribute)
- **Keybindings**: キーバインドのカスタマイズ
- **Appearance**: テーマ設定（Coming soon）
- **Plugins**: プラグイン管理（Coming soon）

### 6. ECS Inspector `Ctrl+6`

実行中の Bevy アプリの ECS 状態をリアルタイム監視。

- gRPC 経由で Bevy アプリに接続
- **Entities タブ**: エンティティ一覧 + コンポーネント詳細
- **Resources タブ**: リソース型一覧 + 値表示
- フィルター/検索
- 自動リフレッシュ
- 接続ステータスインジケーター（緑/赤）

### 7. Bevy Templates `Ctrl+7`

Bevy ボイラープレートのコード生成。

| テンプレート | 生成内容 |
|-------------|---------|
| Component | フィールド付き struct + Component derive |
| Resource | フィールド付き struct + Resource derive |
| System | パラメータ付きシステム関数 |
| Plugin | Plugin trait 実装 |
| Startup System | スタートアップシステム |
| Event | イベント struct |
| State | enum + States derive |

- フィールド名と型を動的入力
- 現在のファイルのカーソル位置に挿入
- プレビュー表示

### 8. Asset Browser `Ctrl+8`

プロジェクトアセットの閲覧・管理。

- ルートディレクトリ指定 + 再帰スキャン
- タイプフィルター: All / Images / Models / Audio / Scenes / Shaders
- リスト表示/グリッド表示の切替
- 画像サムネイルプレビュー
- ファイルサイズ・解像度表示
- ダブルクリックでインポート/使用
- ドラッグ&ドロップでエディタ/シーンに追加

### 9. Scene Editor (シーンヒエラルキー) `Ctrl+9`

Unity スタイルのシーン階層パネル。

- シーンタブ（複数シーン同時編集、変更マーク `*`）
- エンティティツリー（展開/折りたたみ）
- ドラッグ&ドロップで親子関係変更
- 右クリックメニュー: 新規作成, 削除, 複製, リネーム
- インラインリネーム（ダブルクリック）
- ツールバー: New / Save / Tools ドロップダウン
- Play Mode バナー（再生中は編集無効）

