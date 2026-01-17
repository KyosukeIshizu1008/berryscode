# ファイル表示バグのデバッグ手順

## 🎯 問題
ファイルを開いたとき、ソースコードが表示されない問題が発生しています。

## ✅ 追加したデバッグ用ログ

ファイル選択から WGPU レンダリングまでの完全なフローを追跡するため、以下のログを追加しました。

### 1. ファイル開くフロー (virtual_editor.rs)

```
🔍 DEBUG: Effect triggered, current_file=Some("path/to/file.rs")
🔍 DEBUG: Opening file: path/to/file.rs
📂 Creating new tab for: path/to/file.rs, content length: 12345
📂 New tab created at index 0, render trigger updated: 0 -> 1
```

### 2. レンダリング準備フロー (virtual_editor.rs)

```
🔄 WGPU render use_effect triggered (value: 1)
🎨 Preparing to render tab 0 (file: path/to/file.rs, lines: 350)
⚠️ Tab buffer is empty! file: path/to/file.rs, content length: 0  ← 空の場合のみ
🎨 WGPU render command sent with syntax highlighting (trigger: 1, lines: 350)
```

### 3. WGPU レンダリングフロー (wgpu_integration.rs)

```
🎨 WGPU thread received RenderText command: 350 lines, cursor: (0, 0), visible: true
✅ WGPU frame rendered successfully (350 lines)
```

## 🔧 実行方法

```bash
cd berrycode

# フルログを有効にして実行（ファイルに保存）
RUST_LOG=info cargo run --bin berrycode 2>&1 | tee debug_$(date +%Y%m%d_%H%M%S).log
```

実行後、ファイルツリーでファイルをクリックして、ログを確認してください。

## 🔍 ログ分析方法

### 正常な場合（ファイルが表示される）

上記のログが順番に全て出力されます：
1. `🔍 DEBUG: Effect triggered` → ファイル選択を検出
2. `📂 Creating new tab` → タブ作成成功
3. `🔄 WGPU render use_effect triggered` → レンダリングトリガー
4. `🎨 Preparing to render tab` → シンタックスハイライト準備
5. `🎨 WGPU thread received` → WGPU スレッドがコマンド受信
6. `✅ WGPU frame rendered successfully` → レンダリング成功

### 異常パターンと原因

| 症状 | 出力されるログ | 原因 | 対策 |
|------|---------------|------|------|
| ファイルツリーをクリックしても何も起きない | ログなし | イベントハンドラ未動作 | file_tree.rs の onclick を確認 |
| タブが作られない | 1 のみ | Signal 更新失敗 | selected_file シグナルの接続を確認 |
| レンダリングが始まらない | 1-2 のみ | render_trigger 未更新 | L940/L949 のシグナル書き込みを確認 |
| バッファが空 | 1-3 + ⚠️ 警告 | コンテンツが Buffer に入っていない | EditorTab::new() を確認 |
| WGPU が受信しない | 1-4 のみ | チャネル通信失敗 | WGPU_SENDER 初期化を確認 |
| レンダリングエラー | 1-5 + ❌ エラー | GPU/サーフェスエラー | エラーメッセージ詳細を確認 |
| 全て成功だが画面に何も出ない | 1-6 全て | Z-index/透明度の問題 | 下記「画面表示の確認」参照 |

## 🎨 画面表示の確認

ログで全て成功しているのに画面に何も表示されない場合：

### チェック項目

1. **WGPULayer の CSS 設定**
   - `pointer-events: none` が設定されているか？
   - z-index が正しいか？

2. **エディタ領域の可視化**
   - 一時的に `border: 2px solid red` を追加して領域を確認
   - 背景が透明になっているか確認

3. **ウィンドウサイズ**
   - ログの `📐 Window size: WIDTHxHEIGHT` を確認
   - 実際のウィンドウサイズと一致しているか？

4. **DPR (Device Pixel Ratio)**
   - ログの `📐 Device Pixel Ratio (DPR): X.XX` を確認
   - Retina ディスプレイの場合、2.0 になっているか？

## 📝 ログ検索コマンド

```bash
# ファイルを開こうとした全ての試行を検索
grep "📂 Creating new tab" debug_*.log

# WGPU レンダリングコマンドを全て検索
grep "🎨 WGPU thread received" debug_*.log

# エラー・警告を全て検索
grep -E "(❌|⚠️)" debug_*.log

# 特定のファイルのフローを追跡
grep "/path/to/your/file.rs" debug_*.log
```

## 🚀 既知の正常動作

過去のログから、システムは正常に動作可能であることが確認されています：
- 399 行、1917 行のファイルのレンダリング成功実績あり
- WGPU 初期化成功: `✅ WGPU background layer initialized successfully`
- 初期テストフレーム描画成功: `✅ Initial frame rendered`

これは、バグが以下の可能性を示唆しています：
- **間欠的** (タイミング関連？)
- **特定のファイルや条件に依存**
- **レンダリングインフラではなく、ファイル開くフローに問題**

## 🔧 コード変更内容

### 1. wgpu_integration.rs (L161-189)

**変更前:**
```rust
tracing::debug!("📝 Rendering {} lines ...", lines.len());
if let Err(e) = renderer.lock().render_frame(...) {
    tracing::error!("❌ WGPU render error: {}", e);
}
```

**変更後:**
```rust
tracing::info!("🎨 WGPU thread received RenderText command: {} lines, cursor: ({}, {}), visible: {}",
    lines.len(), cursor_line, cursor_col, cursor_visible);

let lines_count = lines.len();
match renderer.lock().render_frame(...) {
    Ok(_) => {
        tracing::info!("✅ WGPU frame rendered successfully ({} lines)", lines_count);
    }
    Err(e) => {
        tracing::error!("❌ WGPU render error: {}", e);
    }
}
```

**目的:** WGPU スレッドがコマンドを受信したこと、およびレンダリングの成功/失敗を明示的にログに記録。

### 2. virtual_editor.rs (L775-784)

**追加:**
```rust
let total_lines = tab.buffer.len_lines();
tracing::info!("🎨 Preparing to render tab {} (file: {}, lines: {})",
    active_idx, tab.file_path, total_lines);

// ⭐ Safety check: Ensure buffer has content
if total_lines == 0 {
    tracing::warn!("⚠️ Tab buffer is empty! file: {}, content length: {}",
        tab.file_path, tab.buffer.len_chars());
    // Don't try to render empty buffer
    return;
}
```

**目的:** バッファが空の場合を検出し、早期リターンで無駄なレンダリング処理を回避。

## 📚 参考ドキュメント

- `DEBUG_FILE_DISPLAY.md` - 詳細なデバッグシナリオ解説（英語）
- `DEBUGGING_CHANGES_SUMMARY.md` - 全変更内容の詳細（英語）

## 🔄 変更を元に戻す方法

もしこれらの変更で問題が発生した場合：

```bash
git checkout HEAD -- src/core/wgpu_integration.rs src/core/virtual_editor.rs
```

---

## ❓ 次のステップ

1. **上記コマンドでアプリを実行**
2. **ファイルツリーで複数のファイルをクリック**
3. **ログを保存**
4. **ログを分析して、どのパターンに該当するか確認**
5. **結果を報告**

ログ出力があれば、正確な原因特定が可能になります！
