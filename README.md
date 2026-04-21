# BerryCode - The IDE Built for Bevy

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/KyosukeIshizu1008)](https://github.com/sponsors/KyosukeIshizu1008)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-ff5e5b?logo=ko-fi)](https://ko-fi.com/berrycode)

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

**The first IDE purpose-built for the Bevy game engine.**

BerryCode is not a general-purpose editor with Bevy plugins bolted on — it's an IDE designed from the ground up for Bevy development. Built entirely in Rust with Bevy + bevy_egui + WGPU, it understands Bevy's ECS architecture, scene format, and development workflow natively.

> **Why not just use VS Code?**
> VS Code treats Bevy as "just another Rust project." BerryCode treats Bevy as a first-class game engine — with a built-in Scene Editor, ECS Inspector, System Graph, and more. No extensions needed.

### Screenshots

<p align="center">
  <img src="docs/demo/check_01.png" width="80%" alt="BerryCode — Bevy IDE with 3D rendering">
</p>

### What Makes BerryCode Different

| Feature | VS Code + Extensions | BerryCode |
|---------|---------------------|-----------|
| Scene editing | Text-only `.scn.ron` | Visual 3D viewport with gizmos |
| ECS inspection | None | Live entity/component/resource browser |
| System ordering | None | Visual system dependency graph |
| Bevy events | `println!` debugging | Real-time event monitor |
| Play in editor | Switch to terminal | Embedded game view with live capture |
| Bevy templates | Manually type boilerplate | One-click Component/System/Plugin generation |
| Plugin discovery | Search crates.io manually | Built-in Bevy plugin browser |
| Built with | Electron (web tech) | Bevy + WGPU (same stack as your game) |

### Bevy-Native Tools

These tools understand Bevy's architecture — they're not generic wrappers.

#### Scene Editor (Unity-class)
- 3D viewport with translate/rotate/scale gizmos (`W`/`E`/`R`)
- Entity hierarchy with drag & drop reparenting
- Inspector with type-aware component editors (Vec3, Color, Handle, etc.)
- Prefab system — create, instantiate, override
- Multi-scene tabs with independent undo/redo
- Export to `.scn.ron` (Bevy native) or `.bscene` (binary)

#### ECS Inspector
- Connect to a running Bevy app via BRP (Bevy Remote Protocol)
- Browse entities, components, and resources in real-time
- Filter and search by component type
- Auto-refresh with connection status indicator

#### System Graph
- Visualize system execution order and dependencies
- Identify bottlenecks and ordering issues
- Understand schedule topology at a glance

#### Event Monitor
- Real-time log of all Bevy events
- Filter by event type
- Inspect event payloads

#### Query Visualizer
- See which entities match a given query
- Performance metrics per query
- Optimization hints

#### State Editor
- View and manage Bevy `States` enum variants
- Manually trigger state transitions for testing

#### Bevy Templates
- Generate `Component`, `Resource`, `System`, `Plugin`, `Event`, `State` boilerplate
- Dynamic field/parameter input
- Insert directly at cursor position

#### Plugin Browser
- Search crates.io for Bevy-compatible plugins
- View metadata (version, downloads, description)
- One-click add to `Cargo.toml`

#### Animation System
- Timeline editor with keyframe scrubbing
- Dopesheet for per-property keyframe editing
- Animator editor with clip selection and blend controls

#### Additional Scene Tools
- Visual Scripting (node-based, Blueprint-style)
- Shader Graph editor with live preview
- Material preview with PBR properties
- Terrain editor, Skeleton/Rig editor, Navmesh generator
- Physics simulator, Particle preview

### Also a Full-Featured Code Editor

BerryCode isn't just Bevy tools — it's a complete Rust IDE.

- **LSP** — completions, hover, go-to-definition, references, diagnostics, format, rename, code actions, inlay hints, macro expansion
- **Syntax highlighting** — Rust, Python, JavaScript, C/C++, TOML, Markdown (Tree-sitter + Syntect)
- **Vim mode** — full modal editing (Normal, Insert, Visual, Command, Replace) with operators, text objects, registers, marks, dot repeat
- **Terminal** — iTerm2-class PTY emulator (VT100/xterm, ANSI 256 colors, 10K scrollback, multi-tab)
- **Git** — 6-tab panel (Status, History, Branches, Remotes, Tags, Stash) with commit graph and diff viewer
- **Search** — project-wide regex search with parallel execution (Rayon)
- **Debugger** — variables, call stack, watch expressions, breakpoints (DAP)
- **AI Chat** — integrated LLM assistant via gRPC
- **Minimap, code folding, snippets, image/3D model preview, test runner**

### Quick Start

```bash
# Run BerryCode
cargo run --bin berrycode

# With AI features
cd berry_api && cargo run  # Terminal 1
cargo run --bin berrycode  # Terminal 2

# Release build
cargo build --release --bin berrycode
```

**Prerequisites**: Rust 1.75+ | Linux: `libx11-dev libasound2-dev libudev-dev libpipewire-0.3-dev`

### Architecture

BerryCode runs on the same technology stack as your Bevy game:

| Layer | Technology |
|-------|-----------|
| Engine | **Bevy 0.15** |
| Rendering | **WGPU** (Metal / Vulkan / DX12) |
| UI | bevy_egui + egui 0.30 |
| Text Buffer | Ropey (rope-based) |
| Syntax | Tree-sitter + Syntect |
| Terminal | portable-pty + VTE |
| Git | libgit2 |
| Search | Rayon + regex |
| LSP | lsp-types (native) |
| AI | gRPC (tonic + prost) |
| 3D Assets | gltf, tobj, image |
| Window Capture | xcap |

### Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS | Metal | Supported |
| Linux | Vulkan / OpenGL | Supported |
| Windows | DirectX 12 | Supported |

---

<a name="japanese"></a>

## 日本語

**Bevy ゲームエンジン専用に作られた、初めての IDE。**

BerryCode は汎用エディタに Bevy プラグインを後付けしたものではありません。Bevy の ECS アーキテクチャ、シーンフォーマット、開発ワークフローをネイティブに理解する、Bevy 開発のためにゼロから設計された IDE です。Rust + Bevy + bevy_egui + WGPU で構築 — あなたのゲームと同じ技術スタック。

> **VS Code じゃダメなの？**
> VS Code は Bevy を「ただの Rust プロジェクト」として扱います。BerryCode は Bevy をファーストクラスのゲームエンジンとして扱います — シーンエディタ、ECS インスペクター、システムグラフ等が組み込み済み。拡張機能は不要です。

### スクリーンショット

<p align="center">
  <img src="docs/demo/check_01.png" width="80%" alt="BerryCode — Bevy IDE + 3Dレンダリング">
</p>

### BerryCode が他と違う点

| 機能 | VS Code + 拡張機能 | BerryCode |
|------|-------------------|-----------|
| シーン編集 | テキストで `.scn.ron` | ギズモ付き3Dビューポート |
| ECS 監視 | なし | ライブ エンティティ/コンポーネント/リソース ブラウザ |
| システム順序 | なし | ビジュアルシステム依存グラフ |
| Bevy イベント | `println!` デバッグ | リアルタイムイベントモニター |
| エディタ内プレイ | ターミナルに切替 | ゲームウィンドウ埋め込みキャプチャ |
| Bevy テンプレート | 手動でボイラープレート入力 | ワンクリック Component/System/Plugin 生成 |
| プラグイン検索 | crates.io を手動検索 | 組み込み Bevy プラグインブラウザ |
| 構築技術 | Electron (Web技術) | Bevy + WGPU (ゲームと同じスタック) |

### Bevy ネイティブツール

Bevy のアーキテクチャを理解した専用ツール群。

#### シーンエディタ (Unity クラス)
- 移動/回転/スケールギズモ付き3Dビューポート (`W`/`E`/`R`)
- ドラッグ&ドロップによる親子関係変更が可能なエンティティヒエラルキー
- 型対応コンポーネントエディタ付きインスペクター (Vec3, Color, Handle 等)
- プレハブシステム — 作成、インスタンス化、オーバーライド
- 独立した Undo/Redo 付きマルチシーンタブ
- `.scn.ron` (Bevy ネイティブ) / `.bscene` (バイナリ) エクスポート

#### ECS インスペクター
- BRP (Bevy Remote Protocol) 経由で実行中の Bevy アプリに接続
- エンティティ、コンポーネント、リソースをリアルタイムに閲覧
- コンポーネント型でフィルター・検索
- 自動リフレッシュ + 接続ステータスインジケーター

#### システムグラフ
- システム実行順序と依存関係を可視化
- ボトルネックと順序問題の特定

#### イベントモニター
- 全 Bevy イベントのリアルタイムログ
- イベント型でフィルタリング

#### クエリビジュアライザー
- 指定クエリにマッチするエンティティの確認
- クエリごとのパフォーマンスメトリクス

#### ステートエディタ
- Bevy `States` enum の表示・管理
- テスト用の手動ステート遷移

#### Bevy テンプレート
- `Component`, `Resource`, `System`, `Plugin`, `Event`, `State` のボイラープレート生成
- カーソル位置に直接挿入

#### プラグインブラウザ
- crates.io から Bevy 対応プラグインを検索
- ワンクリックで `Cargo.toml` に追加

#### アニメーションシステム
- キーフレーム付きタイムラインエディタ
- プロパティごとのドープシート
- クリップ選択・ブレンド付きアニメーターエディタ

#### その他のシーンツール
- ビジュアルスクリプト (ノードベース、Blueprint スタイル)
- ライブプレビュー付きシェーダーグラフエディタ
- PBR プロパティ付きマテリアルプレビュー
- テレインエディタ、スケルトン/リグエディタ、Navmesh ジェネレーター
- 物理シミュレーター、パーティクルプレビュー

### フル機能のコードエディタでもある

Bevy ツールだけではなく、完全な Rust IDE。

- **LSP** — 補完、ホバー、定義ジャンプ、参照検索、診断、フォーマット、リネーム、コードアクション、インレイヒント、マクロ展開
- **シンタックスハイライト** — Rust, Python, JavaScript, C/C++, TOML, Markdown (Tree-sitter + Syntect)
- **Vim モード** — フルモーダル編集 (Normal, Insert, Visual, Command, Replace) + オペレータ、テキストオブジェクト、レジスタ、マーク、ドットリピート
- **ターミナル** — iTerm2 クラス PTY エミュレータ (VT100/xterm, ANSI 256色, 10K スクロールバック, マルチタブ)
- **Git** — 6タブパネル (Status, History, Branches, Remotes, Tags, Stash) + コミットグラフ、差分ビューアー
- **検索** — プロジェクト全体の正規表現検索 (Rayon 並列)
- **デバッガー** — 変数、コールスタック、ウォッチ、ブレークポイント (DAP)
- **AI チャット** — gRPC 経由の統合 LLM アシスタント
- **ミニマップ、コード折りたたみ、スニペット、画像/3Dモデルプレビュー、テストランナー**

### クイックスタート

```bash
# BerryCode 起動
cargo run --bin berrycode

# AI 機能付き
cd berry_api && cargo run  # ターミナル1
cargo run --bin berrycode  # ターミナル2

# リリースビルド
cargo build --release --bin berrycode
```

**前提条件**: Rust 1.75+ | Linux: `libx11-dev libasound2-dev libudev-dev libpipewire-0.3-dev`

### アーキテクチャ

BerryCode はあなたの Bevy ゲームと同じ技術スタックで動きます:

| レイヤー | 技術 |
|---------|------|
| エンジン | **Bevy 0.15** |
| レンダリング | **WGPU** (Metal / Vulkan / DX12) |
| UI | bevy_egui + egui 0.30 |
| テキストバッファ | Ropey (ロープ構造) |
| シンタックス | Tree-sitter + Syntect |
| ターミナル | portable-pty + VTE |
| Git | libgit2 |
| 検索 | Rayon + regex |
| LSP | lsp-types (ネイティブ) |
| AI | gRPC (tonic + prost) |
| 3D アセット | gltf, tobj, image |
| ウィンドウキャプチャ | xcap |

### プラットフォーム対応

| プラットフォーム | バックエンド | ステータス |
|----------------|------------|-----------|
| macOS | Metal | 対応済み |
| Linux | Vulkan / OpenGL | 対応済み |
| Windows | DirectX 12 | 対応済み |

---

## License

MIT
