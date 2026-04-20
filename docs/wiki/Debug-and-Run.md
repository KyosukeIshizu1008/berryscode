# Debug & Run / デバッグ・実行

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

### Debugger Panel (Bottom)

VS Code-style debug UI.

**Toolbar**: Continue / Pause / Stop / Step Over / Step Into / Step Out

| Tab | Content |
|-----|---------|
| **Variables** | Local variable tree (expandable) |
| **Watch** | Watch expression editor + evaluation results |
| **Call Stack** | Call stack list (click to jump to frame) |
| **Debug Console** | Debug console I/O |

**Features**: Breakpoint management (set/clear/conditional), gutter indicators, stopped-location highlighting, thread selection, DAP integration.

### Run Panel (Bottom)

Execute and monitor `cargo run`.

- Debug / Release mode toggle
- Real-time output streaming
- Line severity classification (Info / Warning / Error)
- Process lifecycle management
- Game window capture integration

### Tool Panel (Bottom Dock)

Resizable bottom panel with 4 tabs:

- **Console** — stderr/stdout display, auto-scroll, severity colors
- **Timeline** — Visual keyframe timeline, animation editing
- **Dopesheet** — Per-property keyframe editing, curve editor
- **Profiler** — Frame time graph, per-system timing, memory usage, FPS counter

### Build System

- One-click "Run Bevy Project" button in header
- Debug/Release toggle
- Build progress indicator
- Auto `cargo check` on file save
- Diagnostics reflected in editor

---

<a name="japanese"></a>

## 日本語

## デバッガーパネル (底部)

VS Code スタイルのデバッグ UI。

### ツールバー
- Continue / Pause / Stop
- Step Over / Step Into / Step Out

### 4タブ

| タブ | 内容 |
|------|------|
| **Variables** | ローカル変数ツリー（展開可能） |
| **Watch** | ウォッチ式の追加・評価結果表示 |
| **Call Stack** | コールスタック一覧（クリックでフレームジャンプ） |
| **Debug Console** | デバッグコンソール入出力 |

### 機能
- ブレークポイント管理（設定/解除/条件付き）
- ガターのブレークポイントインジケーター
- 停止位置のエディタハイライト
- スレッド選択（マルチスレッド対応）
- DAP (Debug Adapter Protocol) 統合

---

## 実行パネル (底部)

`cargo run` の実行と出力表示。

- Debug / Release モード切替
- 出力のリアルタイムストリーミング
- 行ごとの重要度分類:
  - Info (通常出力)
  - Warning (`warning:` を含む行)
  - Error (`error:` を含む行)
- プロセスライフサイクル管理
- ゲームウィンドウキャプチャ連携

---

## ツールパネル (底部ドック)

リサイズ可能な底部パネル。4タブ構成:

### Console
- stderr/stdout のテキスト表示
- 自動スクロール
- クリアボタン
- 重要度カラー表示

### Timeline
- キーフレームの視覚的タイムライン
- アニメーション編集

### Dopesheet
- プロパティごとのキーフレーム編集
- カーブエディタ

### Profiler
- フレームタイムグラフ
- システムごとのタイミング
- メモリ使用量
- FPS カウンター

---

## ビルドシステム

### ヘッダーからのワンクリック実行
- 「Run Bevy Project」ボタン
- Debug/Release トグル
- ビルド進捗インジケーター

### 保存時 Cargo Check
- ファイル保存時に自動で `cargo check` 実行
- 診断結果をエディタに反映

### ビルド設定
- ターゲット選択
- ビルドフラグ設定
- 出力ディレクトリ指定
