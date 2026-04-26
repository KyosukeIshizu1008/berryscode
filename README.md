# BerryCode - The IDE Built for Bevy

[![CI](https://github.com/KyosukeIshizu1008/berryscode/actions/workflows/tests.yml/badge.svg)](https://github.com/KyosukeIshizu1008/berryscode/actions/workflows/tests.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/berrycode)](https://crates.io/crates/berrycode)
[![Downloads](https://img.shields.io/github/downloads/KyosukeIshizu1008/berryscode/total)](https://github.com/KyosukeIshizu1008/berryscode/releases)
[![Discord](https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white)](https://discord.gg/tBYX3EZW)

[![GitHub Sponsors](https://img.shields.io/github/sponsors/KyosukeIshizu1008?logo=github&label=GitHub%20Sponsors)](https://github.com/sponsors/KyosukeIshizu1008)
[![Open Collective](https://img.shields.io/badge/Open%20Collective-Support-7FADF2?logo=opencollective)](https://opencollective.com/berrycode)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-ff5e5b?logo=ko-fi)](https://ko-fi.com/berrycode)
[![Liberapay](https://img.shields.io/badge/Liberapay-Support-f6c915?logo=liberapay)](https://liberapay.com/berrycode/)
[![IssueHunt](https://img.shields.io/badge/IssueHunt-Bounties-00CC99)](https://issuehunt.io/r/KyosukeIshizu1008/berryscode)

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

**The first IDE purpose-built for the Bevy game engine.**

BerryCode is not a general-purpose editor with Bevy plugins bolted on — it's an IDE designed from the ground up for Bevy development. Built entirely in Rust with Bevy + bevy_egui + WGPU, it understands Bevy's ECS architecture, scene format, and development workflow natively.

> **Why not just use VS Code?**
> VS Code treats Bevy as "just another Rust project." BerryCode treats Bevy as a first-class game engine — with a built-in Scene Editor, ECS Inspector, System Graph, and more. No extensions needed.

### Demo

<p align="center">
  <video src="https://github.com/KyosukeIshizu1008/berryscode/raw/main/docs/demo/demo.mp4" width="80%" autoplay loop muted playsinline>
    <img src="docs/demo/demo.gif" width="80%" alt="BerryCode Demo">
  </video>
</p>

### Screenshots

| Scene Editor | Game Runtime |
|:---:|:---:|
| ![Scene Editor](docs/screenshots/scene_editor_fox.png) | ![Game Runtime](docs/screenshots/game_runtime.png) |

| ECS Inspector | Git Integration |
|:---:|:---:|
| ![ECS Inspector](docs/screenshots/ecs_inspector.png) | ![Git Panel](docs/screenshots/git_panel.png) |

| Code Editor + AI Chat |
|:---:|
| ![Code Editor](docs/screenshots/code_editor.png) |

### What Makes BerryCode Different

| Feature | VS Code + Extensions | BerryCode |
|---------|---------------------|-----------|
| Scene editing | Text-only `.scn.ron` | Visual 3D viewport with gizmos |
| ECS inspection | None | Live entity/component/resource browser |
| System ordering | None | Visual system dependency graph |
| Bevy events | `println!` debugging | Real-time event monitor |
| Play in editor | Switch to terminal | Run with integrated console output |
| Bevy templates | Manually type boilerplate | One-click Component/System/Plugin generation |
| Plugin discovery | Search crates.io manually | Built-in Bevy plugin browser |
| Built with | Electron (web tech) | Bevy + WGPU (same stack as your game) |

### Bevy-Native Tools

These tools understand Bevy's architecture — they're not generic wrappers.

#### Scene Editor (Unity-class)
- 3D viewport with translate/rotate/scale gizmos (`W`/`E`/`R`)
- VS Code-style panel headers and compact toolbar layout
- Entity hierarchy with file-tree-style rendering (Codicon icons, full-row selection, indent guides)
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

### Install

#### Pre-built binaries (recommended)

Grab the latest release artifact for your platform from the
[Releases page](https://github.com/KyosukeIshizu1008/berryscode/releases/latest):

| Platform | Artifact |
|----------|----------|
| macOS (Apple Silicon + Intel) | `berrycode-macos-universal.tar.gz` |
| Linux (x86_64) | `berrycode-linux-x86_64.tar.gz` |
| Windows (x86_64) | `berrycode-windows-x86_64.zip` |

Releases are signed with [Sigstore](https://www.sigstore.dev/).

#### Package managers

```bash
# macOS / Linux — Homebrew
brew install berrycode

# Windows — winget
winget install KyosukeIshizu1008.BerryCode

# Linux — Snap
sudo snap install berrycode

# Linux — Flatpak
flatpak install flathub dev.berrycode.BerryCode

# Cargo (any platform with Rust 1.75+)
cargo install berrycode
```

#### Build from source

```bash
git clone https://github.com/KyosukeIshizu1008/berryscode
cd berryscode
cargo run --bin berrycode               # debug
cargo build --release --bin berrycode   # release

# AI features (optional)
cd berry_api && cargo run               # terminal 1
cargo run --bin berrycode               # terminal 2
```

**Prerequisites**: Rust 1.75+ | Linux: `libx11-dev libasound2-dev libudev-dev libpipewire-0.3-dev`

### Roadmap

BerryCode is in active development. The next milestones in priority order:

#### v0.4 — Editor polish (target: Q3 2026)
- [ ] GPU PBR preview for GLB/GLTF models ([#1](https://github.com/KyosukeIshizu1008/berryscode/issues/1))
- [ ] In-progress IME preedit display in source code editor (currently terminal only)
- [ ] LSP: completion details (signature help, parameter hints)
- [ ] Scene Editor: prefab nested overrides
- [ ] Settings UI for keybindings and theme

#### v0.5 — Bevy depth (target: Q4 2026)
- [ ] System Graph: drag-to-reorder + visual scheduling
- [ ] Animation: blend tree node graph editor
- [ ] Shader Graph: live-recompile preview
- [ ] Hot reload for `.bscene` and shader assets
- [ ] Plugin Browser: install with one click + auto-update

#### v0.6 — Collaboration (target: 2027 H1)
- [ ] Multi-cursor + collaborative editing (CRDT-based)
- [ ] Profiler integration (Bevy + Tracy)
- [ ] Asset import pipeline (FBX, OBJ, custom converters)
- [ ] Visual scripting → Rust codegen

#### v0.7 — Architecture → Game pipeline (target: 2027 H2)

> _Bring your buildings to life._ Treat the artifacts that architects, BIM
> engineers, and archviz studios already produce as first-class Bevy
> assets — no Blender / Unreal detour required.

- [ ] **Native CAD importers**: DWG / DXF / IFC (BIM) / STEP / IGES / SketchUp `.skp`, plus a Revit → IFC bridge
- [ ] **Auto-prep for real-time rendering**: Z-up → Y-up, mm → m, LOD synthesis, UV unwrap, layer-name → PBR material inference (`Wall` → masonry, `Glass` → transmissive, `Floor` → tiled, …)
- [ ] **Walkable scene scaffolding**: collision auto-generation, first-person walkthrough template, day/night lighting presets, door / elevator interaction defaults
- [ ] **Targets**: architecture firms shipping client demos, real-estate viz, BIM-driven digital twins, Quest / Vision Pro VR walkthroughs

Positioning: an open-source Bevy-based alternative to Twinmotion / Enscape / Datasmith.

#### v0.8 — Ship to phones from one IDE (target: 2028 H1)

> _Bevy mobile development without the toolchain hell._ Replace the
> cargo-mobile + Xcode + Android Studio + SDK juggling with a single
> integrated workflow.

- [ ] **Toolchain setup**: one-click iOS / Android targets, auto-detect missing SDK / NDK / simulators, manage signing certs & keystores from the IDE
- [ ] **Deploy & run**: one-click deploy to device or simulator, unified log / crashtrace console, **WiFi hot reload** for asset edits
- [ ] **Mobile-aware editor**: visual touch-input editor (virtual joysticks, tap zones, gestures), safe-area / notch / orientation-aware layouts, mobile UI templates, auto texture compression (ASTC / ETC2), mobile LOD presets
- [ ] **Performance**: integrated GPU profiler (Metal frame capture / RenderDoc Android), frame-budget visualisation, battery-cost estimator, lifecycle (Background / Foreground / Lock) test harness
- [ ] **Ship**: IPA / AAB build & signing inside the IDE, App Store Connect / Play Console upload helper, TestFlight & internal-test QR generator
- [ ] **VR/AR bonus**: Vision Pro / Quest builds reuse the v0.7 walkable scenes — pipeline becomes "CAD → walkthrough → headset" in one tool

#### Long term
- [ ] WASM build for in-browser editing
- [ ] Cloud sync for workspaces

See [open issues](https://github.com/KyosukeIshizu1008/berryscode/issues) for the current backlog and
[Discussions](https://github.com/KyosukeIshizu1008/berryscode/discussions) to suggest new directions.

### Community

Join us on [Discord](https://discord.gg/tBYX3EZW) for questions, feedback, and discussion.

### Architecture

BerryCode runs on the same technology stack as your Bevy game:

| Layer | Technology |
|-------|-----------|
| Engine | **Bevy 0.18** |
| Rendering | **WGPU** (Metal / Vulkan / DX12) |
| UI | bevy_egui 0.39 + egui 0.33 |
| Text Buffer | Ropey (rope-based) |
| Syntax | Tree-sitter + Syntect |
| Terminal | portable-pty + VTE |
| Git | libgit2 |
| Search | Rayon + regex |
| LSP | lsp-types (native) |
| AI | gRPC (tonic + prost) |
| 3D Assets | gltf, tobj, image |

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

### デモ

<p align="center">
  <video src="https://github.com/KyosukeIshizu1008/berryscode/raw/main/docs/demo/demo.mp4" width="80%" autoplay loop muted playsinline>
    <img src="docs/demo/demo.gif" width="80%" alt="BerryCode デモ">
  </video>
</p>

### スクリーンショット

| シーンエディタ | ゲーム実行 |
|:---:|:---:|
| ![シーンエディタ](docs/screenshots/scene_editor_fox.png) | ![ゲーム実行](docs/screenshots/game_runtime.png) |

| ECS インスペクター | Git 統合 |
|:---:|:---:|
| ![ECS インスペクター](docs/screenshots/ecs_inspector.png) | ![Git パネル](docs/screenshots/git_panel.png) |

| コードエディタ + AI チャット |
|:---:|
| ![コードエディタ](docs/screenshots/code_editor.png) |

### BerryCode が他と違う点

| 機能 | VS Code + 拡張機能 | BerryCode |
|------|-------------------|-----------|
| シーン編集 | テキストで `.scn.ron` | ギズモ付き3Dビューポート |
| ECS 監視 | なし | ライブ エンティティ/コンポーネント/リソース ブラウザ |
| システム順序 | なし | ビジュアルシステム依存グラフ |
| Bevy イベント | `println!` デバッグ | リアルタイムイベントモニター |
| エディタ内プレイ | ターミナルに切替 | 統合コンソール出力付きで実行 |
| Bevy テンプレート | 手動でボイラープレート入力 | ワンクリック Component/System/Plugin 生成 |
| プラグイン検索 | crates.io を手動検索 | 組み込み Bevy プラグインブラウザ |
| 構築技術 | Electron (Web技術) | Bevy + WGPU (ゲームと同じスタック) |

### Bevy ネイティブツール

Bevy のアーキテクチャを理解した専用ツール群。

#### シーンエディタ (Unity クラス)
- 移動/回転/スケールギズモ付き3Dビューポート (`W`/`E`/`R`)
- VS Code 風パネルヘッダーとコンパクトなツールバーレイアウト
- ファイルツリー風のエンティティヒエラルキー (Codicon アイコン、フル幅選択、インデントガイド)
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

### インストール

#### ビルド済みバイナリ（推奨）

[Releases ページ](https://github.com/KyosukeIshizu1008/berryscode/releases/latest)から
プラットフォーム別にダウンロード:

| プラットフォーム | アーティファクト |
|------------------|------------------|
| macOS (Apple Silicon + Intel) | `berrycode-macos-universal.tar.gz` |
| Linux (x86_64) | `berrycode-linux-x86_64.tar.gz` |
| Windows (x86_64) | `berrycode-windows-x86_64.zip` |

リリースは [Sigstore](https://www.sigstore.dev/) で署名されています。

#### パッケージマネージャー

```bash
# macOS / Linux — Homebrew
brew install berrycode

# Windows — winget
winget install KyosukeIshizu1008.BerryCode

# Linux — Snap
sudo snap install berrycode

# Linux — Flatpak
flatpak install flathub dev.berrycode.BerryCode

# Cargo (Rust 1.75+ があればどのプラットフォームでも)
cargo install berrycode
```

#### ソースからビルド

```bash
git clone https://github.com/KyosukeIshizu1008/berryscode
cd berryscode
cargo run --bin berrycode               # デバッグビルド
cargo build --release --bin berrycode   # リリースビルド

# AI 機能 (オプション)
cd berry_api && cargo run               # ターミナル1
cargo run --bin berrycode               # ターミナル2
```

**前提条件**: Rust 1.75+ | Linux: `libx11-dev libasound2-dev libudev-dev libpipewire-0.3-dev`

### ロードマップ

BerryCode は活発に開発中です。優先順位順の今後のマイルストーン:

#### v0.4 — エディタの磨き込み (目標: 2026 Q3)
- [ ] GLB/GLTF モデルの GPU PBR プレビュー ([#1](https://github.com/KyosukeIshizu1008/berryscode/issues/1))
- [ ] ソースコードエディタでの IME preedit 表示（現状ターミナルのみ）
- [ ] LSP: 補完詳細（シグネチャヘルプ、パラメータヒント）
- [ ] シーンエディタ: プレハブのネストオーバーライド
- [ ] キーバインド・テーマの設定 UI

#### v0.5 — Bevy 深耕 (目標: 2026 Q4)
- [ ] システムグラフ: ドラッグで順序変更 + 視覚的スケジューリング
- [ ] アニメーション: ブレンドツリーノードグラフエディタ
- [ ] シェーダーグラフ: ライブ再コンパイルプレビュー
- [ ] `.bscene` とシェーダーアセットのホットリロード
- [ ] プラグインブラウザ: ワンクリックインストール + 自動アップデート

#### v0.6 — コラボレーション (目標: 2027 H1)
- [ ] マルチカーソル + 共同編集 (CRDT ベース)
- [ ] プロファイラ統合 (Bevy + Tracy)
- [ ] アセットインポートパイプライン (FBX, OBJ, カスタムコンバータ)
- [ ] ビジュアルスクリプト → Rust コード生成

#### v0.7 — 建築 → ゲーム パイプライン (目標: 2027 H2)

> _建物に命を吹き込む。_ 建築家・BIM エンジニア・archviz スタジオが既に
> 作っているデータをそのまま Bevy のアセットとして扱う —
> Blender や Unreal を経由する必要なし。

- [ ] **CAD ネイティブインポート**: DWG / DXF / IFC (BIM) / STEP / IGES / SketchUp `.skp`、Revit → IFC ブリッジ
- [ ] **リアルタイムレンダリング向け自動最適化**: Z-up → Y-up、mm → m、LOD 合成、UV アンラップ、レイヤー名から PBR マテリアル自動推定 (`Wall` → 石造、`Glass` → 透過、`Floor` → タイル、…)
- [ ] **ウォークスルーテンプレート**: コリジョン自動生成、ファーストパーソン視点、昼夜ライティングプリセット、ドア・エレベーターのインタラクション defaults
- [ ] **ターゲット**: クライアント向けデモを作る建築事務所、不動産 viz、BIM 駆動のデジタルツイン、Quest / Vision Pro 向け VR ウォークスルー

ポジショニング: Twinmotion / Enscape / Datasmith のオープンソース Bevy 版。

#### v0.8 — モバイル開発を1つの IDE で完結 (目標: 2028 H1)

> _Bevy のモバイル開発をツールチェーン地獄から解放。_
> cargo-mobile + Xcode + Android Studio + 各種 SDK の往復を、
> 1つの統合ワークフローに置き換える。

- [ ] **ツールチェーンセットアップ**: iOS / Android ターゲットのワンクリック追加、不足 SDK / NDK / シミュレータの自動検知、署名証明書 & keystore を IDE 内管理
- [ ] **デプロイ・実行**: 実機・シミュレータへのワンクリックデプロイ、ログ・クラッシュトレース統合コンソール、**アセット変更の WiFi ホットリロード**
- [ ] **モバイル対応エディタ**: タッチ入力ビジュアルエディタ (仮想ジョイスティック、タップゾーン、ジェスチャー)、セーフエリア / ノッチ / 縦横回転対応レイアウト、モバイル UI テンプレート、テクスチャ自動圧縮 (ASTC / ETC2)、モバイル LOD プリセット
- [ ] **パフォーマンス**: GPU プロファイラ統合 (Metal frame capture / RenderDoc Android)、フレーム予算可視化、バッテリー消費見積もり、ライフサイクル (Background / Foreground / Lock) テストハーネス
- [ ] **公開**: IPA / AAB ビルド・署名を IDE 内で完結、App Store Connect / Play Console アップロード補助、TestFlight / 内部テスト用 QR 生成
- [ ] **VR/AR ボーナス**: v0.7 の walkable シーンをそのまま Vision Pro / Quest ビルドに流用 — 「CAD → ウォークスルー → ヘッドセット」が 1 ツールで完結

#### 長期
- [ ] ブラウザ内編集用 WASM ビルド
- [ ] ワークスペースのクラウド同期

現在のバックログは [open issues](https://github.com/KyosukeIshizu1008/berryscode/issues)、
新規アイデアは [Discussions](https://github.com/KyosukeIshizu1008/berryscode/discussions) を参照。

### コミュニティ

[Discord](https://discord.gg/tBYX3EZW) で質問・フィードバック・議論ができます。

### アーキテクチャ

BerryCode はあなたの Bevy ゲームと同じ技術スタックで動きます:

| レイヤー | 技術 |
|---------|------|
| エンジン | **Bevy 0.18** |
| レンダリング | **WGPU** (Metal / Vulkan / DX12) |
| UI | bevy_egui 0.39 + egui 0.33 |
| テキストバッファ | Ropey (ロープ構造) |
| シンタックス | Tree-sitter + Syntect |
| ターミナル | portable-pty + VTE |
| Git | libgit2 |
| 検索 | Rayon + regex |
| LSP | lsp-types (ネイティブ) |
| AI | gRPC (tonic + prost) |
| 3D アセット | gltf, tobj, image |

### プラットフォーム対応

| プラットフォーム | バックエンド | ステータス |
|----------------|------------|-----------|
| macOS | Metal | 対応済み |
| Linux | Vulkan / OpenGL | 対応済み |
| Windows | DirectX 12 | 対応済み |

---

## License

MIT
