# BerryCode - Bevy-Powered Native IDE for Rust & Game Development

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/KyosukeIshizu1008)](https://github.com/sponsors/KyosukeIshizu1008)

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

A fully-featured native IDE built on **Bevy + bevy_egui + WGPU**, designed for Rust and Bevy game development. Zero web technologies — 100% Rust from rendering to logic.

### Screenshots

<p align="center">
  <img src="docs/demo/01_startup.png" width="80%" alt="BerryCode Editor">
</p>

### Features

#### Code Editor
- Syntax highlighting (Rust, Python, JavaScript, C/C++) via Tree-sitter + Syntect
- LSP integration (completions, hover, go-to-definition, references, diagnostics)
- Code actions, inlay hints, rename refactoring
- Vim mode
- Minimap
- Multi-tab editing with image/3D model preview

#### Project & Files
- File explorer with drag & drop
- Project-wide search & replace (regex, parallel via Rayon)
- Git integration (status, diff viewer, branches, stash, commit graph)

#### Terminal
- iTerm2-class PTY terminal emulator (VT100/xterm)
- ANSI color rendering
- Multiple terminal sessions

#### Bevy Game Engine Tools
- **Scene Editor** — Unity-class 3D viewport with gizmos, hierarchy, inspector
- **ECS Inspector** — entities, components, resources
- **Asset Browser** — textures, models, audio preview
- **Game View** — play-in-editor with live window capture
- **System Graph** — visualize Bevy system ordering
- **Event Monitor** — real-time Bevy event log
- **Query Visualizer** — inspect ECS queries
- **State Editor** — manage Bevy states
- **Animation** — timeline, dopesheet, animator editor
- **Visual Scripting** & **Shader Graph** editors
- **Bevy Templates** — quick project scaffolding
- **Plugin Browser** — search crates.io for Bevy plugins

#### Developer Tools
- Debugger panel (variables, call stack, watch, breakpoints)
- Run/Build panel with console output
- Dockable tool panel (Console / Timeline / Dopesheet / Profiler)
- AI Chat assistant (via berry-api gRPC server)
- Live collaboration
- Remote development
- Custom snippet system
- Plugin system

### Quick Start

#### Prerequisites

- Rust toolchain (stable 1.75+)
- On Linux: `libx11-dev`, `libasound2-dev`, `libudev-dev`

#### Run

```bash
cargo run --bin berrycode
```

#### Run with AI Features

```bash
# Terminal 1: Start API server
cd berry_api && cargo run

# Terminal 2: Start BerryCode
cargo run --bin berrycode
```

#### Demo Mode (Feature Showcase + Screenshots + Video)

```bash
BERRYCODE_DEMO=1 cargo run --bin berrycode
# Outputs: docs/demo/*.png + docs/demo/demo.mp4
```

#### Build Release

```bash
cargo build --release --bin berrycode
```

### Architecture

| Layer | Technology |
|-------|-----------|
| Window & Rendering | Bevy 0.15 + WGPU (Metal / Vulkan / DX12) |
| UI Framework | bevy_egui + egui 0.30 |
| Text Buffer | Ropey (rope-based) |
| Syntax | Tree-sitter + Syntect |
| Terminal | portable-pty + VTE parser |
| Git | libgit2 (via git2 crate) |
| Search | Rayon (parallel) + regex |
| LSP | lsp-types (native client) |
| AI Backend | gRPC (tonic + prost) |
| 3D Assets | gltf, tobj (OBJ), image |
| Window Capture | xcap |

### Project Structure

```
berrycode/
├── src/
│   ├── bin/berrycode-egui.rs   # Entry point
│   ├── app/                     # UI modules
│   │   ├── editor.rs           # Code editor
│   │   ├── terminal_emulator.rs # PTY terminal
│   │   ├── git.rs              # Git panel
│   │   ├── scene_editor/       # 3D scene editor
│   │   ├── debugger.rs         # Debugger
│   │   ├── ai_chat.rs          # AI assistant
│   │   ├── demo_capture.rs     # Feature showcase capture
│   │   └── ...                 # 50+ feature modules
│   ├── native/                  # Platform abstraction
│   ├── bevy_plugin.rs          # Bevy plugin integration
│   └── lib.rs
berry_api/                       # AI backend server (gRPC)
```

### Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS | Metal | Supported |
| Linux | Vulkan / OpenGL | Supported |
| Windows | DirectX 12 | Supported |

---

<a name="japanese"></a>

## 日本語

**Bevy + bevy_egui + WGPU** で構築された、Rust とBevyゲーム開発のためのフル機能ネイティブ IDE。Web 技術ゼロ — レンダリングからロジックまで 100% Rust。

### スクリーンショット

<p align="center">
  <img src="docs/demo/01_startup.png" width="80%" alt="BerryCode エディタ">
</p>

### 機能

#### コードエディタ
- シンタックスハイライト（Rust, Python, JavaScript, C/C++）— Tree-sitter + Syntect
- LSP 統合（補完、ホバー、定義ジャンプ、参照検索、診断）
- コードアクション、インレイヒント、リネームリファクタリング
- Vim モード
- ミニマップ
- マルチタブ編集（画像/3Dモデルプレビュー対応）

#### プロジェクト・ファイル
- ドラッグ&ドロップ対応ファイルエクスプローラー
- プロジェクト全体の検索・置換（正規表現、Rayon 並列処理）
- Git 統合（ステータス、差分ビューアー、ブランチ、スタッシュ、コミットグラフ）

#### ターミナル
- iTerm2 クラスの PTY ターミナルエミュレータ（VT100/xterm）
- ANSI カラーレンダリング
- 複数ターミナルセッション

#### Bevy ゲームエンジンツール
- **シーンエディタ** — Unity クラスの3Dビューポート（ギズモ、ヒエラルキー、インスペクター）
- **ECS インスペクター** — エンティティ、コンポーネント、リソース
- **アセットブラウザ** — テクスチャ、モデル、オーディオプレビュー
- **ゲームビュー** — エディタ内プレイ（ライブウィンドウキャプチャ）
- **システムグラフ** — Bevy システム順序の可視化
- **イベントモニター** — リアルタイム Bevy イベントログ
- **クエリビジュアライザー** — ECS クエリの検証
- **ステートエディタ** — Bevy ステート管理
- **アニメーション** — タイムライン、ドープシート、アニメーターエディタ
- **ビジュアルスクリプト** & **シェーダーグラフ** エディタ
- **Bevy テンプレート** — プロジェクト雛形の生成
- **プラグインブラウザ** — crates.io から Bevy プラグイン検索

#### 開発ツール
- デバッガーパネル（変数、コールスタック、ウォッチ、ブレークポイント）
- 実行/ビルドパネル + コンソール出力
- ドッカブルツールパネル（Console / Timeline / Dopesheet / Profiler）
- AI チャットアシスタント（berry-api gRPC サーバー経由）
- ライブコラボレーション
- リモート開発
- カスタムスニペットシステム
- プラグインシステム

### クイックスタート

#### 前提条件

- Rust ツールチェイン（stable 1.75+）
- Linux の場合: `libx11-dev`, `libasound2-dev`, `libudev-dev`

#### 実行

```bash
cargo run --bin berrycode
```

#### AI 機能付きで実行

```bash
# ターミナル1: API サーバー起動
cd berry_api && cargo run

# ターミナル2: BerryCode 起動
cargo run --bin berrycode
```

#### デモモード（機能紹介 + スクリーンショット + 動画）

```bash
BERRYCODE_DEMO=1 cargo run --bin berrycode
# 出力: docs/demo/*.png + docs/demo/demo.mp4
```

#### リリースビルド

```bash
cargo build --release --bin berrycode
```

### アーキテクチャ

| レイヤー | 技術 |
|---------|------|
| ウィンドウ & レンダリング | Bevy 0.15 + WGPU (Metal / Vulkan / DX12) |
| UI フレームワーク | bevy_egui + egui 0.30 |
| テキストバッファ | Ropey (ロープ構造) |
| シンタックス | Tree-sitter + Syntect |
| ターミナル | portable-pty + VTE パーサー |
| Git | libgit2 (git2 クレート経由) |
| 検索 | Rayon (並列) + regex |
| LSP | lsp-types (ネイティブクライアント) |
| AI バックエンド | gRPC (tonic + prost) |
| 3D アセット | gltf, tobj (OBJ), image |
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
