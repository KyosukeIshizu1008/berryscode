# 🚨 根本的なアーキテクチャ制約の発見

## 📊 診断結果のサマリー

### ✅ 確認できたこと
1. **WGPU は完全に動作**している
   - `✅ WGPU frame rendered successfully (561 lines)`
   - テキストは正しく描画されている（`x=0.0, y=0.0` 等）
   - 赤い背景も描画されている（内部的には）

2. **エディタ領域のレイアウトは正常**
   - 赤と黄色の枠線が表示されている
   - Dioxus UI は正常に動作

3. **座標オフセットの実装も完了**
   - `offset: (0, 0)` が正しく渡されている

### ❌ 決定的な問題

**背景が真っ黒** = **WGPUレイヤーが画面に全く表示されていない**

---

## 🎯 根本原因：Dioxus Desktop のアーキテクチャ制約

### Dioxus Desktop（tao + wry）の仕組み

```
┌─────────────────────────────────────┐
│ OS Window                           │
│  ┌───────────────────────────────┐ │
│  │ WebView（フルスクリーン）     │ │
│  │ ┌───────────────────────────┐ │ │
│  │ │ HTML/CSS/JavaScript       │ │ │
│  │ │ (Dioxus UI)               │ │ │
│  │ └───────────────────────────┘ │ │
│  └───────────────────────────────┘ │
│                                     │
│  WGPU Surface (画面に表示されない)  │
│  ← WebViewに完全に覆われている      │
└─────────────────────────────────────┘
```

### 問題の本質

1. **Dioxus Desktop は WebView ベース**
   - ウィンドウ全体が WebView で覆われる
   - WebView は不透明なレイヤーとして動作

2. **`.with_transparent(true)` の限界**
   - これはウィンドウ自体を透明にする（壁紙が見える）
   - しかし、**WebView レイヤーは不透明のまま**

3. **WGPU が描画する場所**
   - WGPU は「ウィンドウの native surface」に描画
   - でも、その上に WebView が覆い被さっている
   - **結果：WGPU の出力は完全に隠れる**

---

## 🔍 試したこと

### 1. ウィンドウの透明化
```rust
.with_transparent(true)
```
**結果**: WebView は透明にならない

### 2. CSS での透明化
```css
html, body {
    background: transparent !important;
}
* {
    background-color: transparent !important;
}
```
**結果**: WebView 自体は透明にならない

### 3. ウィンドウ枠の削除
```rust
.with_decorations(false)
```
**結果**: テスト中（おそらく効果なし）

---

## 🎓 技術的な理解

### WebView の本質

**WebView（wry/tao）は**:
- macOS: `WKWebView`
- Windows: `WebView2` (Edge Chromium)
- Linux: `WebKitGTK`

これらは全て**独立したレンダリングエンジン**であり、ウィンドウ全体を**専有**します。

**WebView と native rendering の共存**は、標準的なユースケースではありません。

---

## 💡 解決策の選択肢

### Option A: Canvas API を使う（推奨）

**概要**: WebView 内の `<canvas>` 要素を使ってテキストをレンダリング

**メリット**:
- Dioxus Desktop をそのまま使える
- WebView との統合が簡単
- クロスプラットフォーム

**デメリット**:
- JavaScript/WASM が必要
- Canvas API の制約（フォントレンダリング品質）
- パフォーマンスが WGPU より劣る可能性

**実装**:
```rust
// VirtualEditorPanel 内で <canvas> を使う
rsx! {
    canvas {
        id: "editor-canvas",
        width: "100%",
        height: "100%",
        // JavaScript で Canvas 2D API を使う
    }
}
```

---

### Option B: 完全に WGPU に移行

**概要**: Dioxus Desktop を使わず、pure WGPU アプリにする

**メリット**:
- 最高のパフォーマンス
- 完全なコントロール
- GPU アクセラレーション

**デメリット**:
- UI（ファイルツリー、パネル等）も WGPU で実装する必要がある
- 開発コストが非常に高い
- クロスプラットフォームの UI ライブラリが必要（egui 等）

**実装**:
```rust
// main.rs を完全に書き直し
use winit::event_loop::EventLoop;
use wgpu::Surface;

fn main() {
    let event_loop = EventLoop::new();
    // WGPU ウィンドウを直接作成
}
```

---

### Option C: エディタを別ウィンドウに分離

**概要**: UI（Dioxus）とエディタ（WGPU）を別々のウィンドウで開く

**メリット**:
- 両方の技術を活かせる
- 実装がシンプル

**デメリット**:
- UX が悪い（2つのウィンドウを管理）
- ウィンドウ間通信が必要

---

### Option D: WebView 内で WGPU を使う（実験的）

**概要**: WebView 内で WebGPU API を使う

**メリット**:
- ブラウザの WebGPU サポートを利用
- Dioxus Desktop をそのまま使える

**デメリット**:
- WebGPU は実験的（ブラウザサポートが限定的）
- WASM 経由でのアクセスが必要
- 複雑

---

### Option E: レイアウトを変更（暫定策）

**概要**: エディタ部分を HTML/CSS でレンダリング（現在の ContentEditable 方式に戻す）

**メリット**:
- すぐに動作する
- 既存のコードを活かせる

**デメリット**:
- パフォーマンスが劣る
- ContentEditable の制約

---

## 🎯 推奨アプローチ

### 短期（すぐに動作させる）

**Option E: HTML/CSS レンダリング**

`CLAUDE.md` の記録によると、以前は ContentEditable で動作していました：

> **2025-12-31**: Complete migration from ContentEditable to Canvas
> - Removed all `contenteditable` dependencies
> - Implemented 100% Rust event handling

**提案**: 一旦、**Canvas API**（WebView 内の 2D Canvas）を使う方向に戻す。

---

### 中期（パフォーマンス改善）

**Option A: Canvas API + OffscreenCanvas**

WebView 内の Canvas を使いつつ、オフスクリーンレンダリングで最適化：

```rust
// JavaScript側
const canvas = document.getElementById('editor');
const offscreen = canvas.transferControlToOffscreen();
const worker = new Worker('editor-worker.js');
worker.postMessage({ canvas: offscreen }, [offscreen]);
```

---

### 長期（究極のパフォーマンス）

**Option B: 完全 WGPU 化**

- UI フレームワーク: `egui` または `iced`
- テキストレンダリング: `glyphon`（既に実装済み）
- ファイルツリー: カスタム実装

**参考**: VS Code の Electron → ネイティブ移行の例

---

## 📊 技術的な深掘り

### なぜ WebView と WGPU は共存できないのか？

#### 1. レンダリングパイプラインの違い

**WebView**:
```
HTML/CSS → レイアウトエンジン → 合成 → ディスプレイ
```

**WGPU**:
```
Rust コード → GPU コマンド → Surface → ディスプレイ
```

これらは**別々のパイプライン**であり、同じウィンドウで**レイヤー合成**することを想定していません。

#### 2. ウィンドウの所有権

**WebView を含むウィンドウ**:
- WebView が「ウィンドウ全体」を所有
- WebView のレンダリングが最優先
- Native surface は「背景」扱い

**WGPU ウィンドウ**:
- WGPU が「ウィンドウ全体」を所有
- Surface に直接描画
- WebView を追加する標準的な方法がない

#### 3. macOS 固有の制約

macOS の `WKWebView` は：
- **不透明なビュー**として動作
- 背後のコンテンツを表示する設定がない（標準では）
- `CALayer` レベルでの合成が必要（高度な API）

---

## 🔧 実験的な解決策（上級者向け）

### WebView の一部を透明にする（未検証）

**理論**:
WebView の特定の領域だけを透明にして、そこに WGPU を表示

**macOS での実装（仮）**:
```rust
// wry の内部にアクセス（非公式）
use cocoa::appkit::NSView;
use objc::runtime::Object;

unsafe {
    let webview: *mut Object = /* get WKWebView pointer */;
    let _: () = msg_send![webview, setOpaque: NO];
    let _: () = msg_send![webview, setBackgroundColor: NSColor::clearColor()];
}
```

**問題**:
- 非公式 API
- プラットフォーム依存
- wry の内部構造に依存

---

## ✅ 結論と次のステップ

### 現状

**Dioxus Desktop + WGPU のハイブリッド構成は、現状では実現不可能**

理由：
- WebView が WGPU レイヤーを完全に覆う
- 標準的なアーキテクチャでは共存できない

### 推奨される次のステップ

#### ステップ 1: WebView Canvas に切り替え（短期）

`src/core/canvas_renderer.rs` を復活させ、WebView 内の Canvas 2D API を使用。

**期待される結果**:
- すぐに動作する
- パフォーマンスは許容範囲

#### ステップ 2: WebGPU を検討（中期）

WebView 内で WebGPU API を使い、GPU アクセラレーションを活用。

**期待される結果**:
- WGPU に近いパフォーマンス
- Dioxus Desktop をそのまま使える

#### ステップ 3: 完全ネイティブ化を検討（長期）

`egui` や `iced` を使って、完全な native アプリに移行。

**期待される結果**:
- 最高のパフォーマンス
- 完全なコントロール

---

## 📝 学んだこと

### 1. アーキテクチャの選択は重要

**WebView ベースのフレームワーク**（Dioxus Desktop, Tauri, Electron）は：
- UI には最適
- エディタのような高性能レンダリングには不向き

### 2. レイヤー合成の理解

**ネイティブレンダリング**（WGPU, Metal, Vulkan）と **WebView** は、別々のレンダリングシステムであり、簡単には統合できない。

### 3. プロトタイピングの価値

今回の実装により、以下が証明されました：
- WGPU レンダリングは完全に動作する
- 座標系の計算も正確
- 問題はレイヤー合成のみ

---

## 🎯 次のアクション

**ユーザーに確認**:

1. **Canvas API に戻す**（すぐに動作する）
   - ContentEditable ではなく、Canvas 2D API を使用
   - パフォーマンスは中程度

2. **WGPU を諦めない**（実験を続ける）
   - WebView の透明化を試す（非公式 API）
   - 別ウィンドウの検討

3. **完全ネイティブ化**（大規模な書き直し）
   - `egui` や `iced` に移行
   - 最高のパフォーマンス

---

**作成日時**: 2026-01-15 15:22
**ステータス**: 根本的な制約を発見、代替案を検討中
