# Go-to-Definition ジャンプ機能修正完了

**日付**: 2026-01-16
**ステータス**: ✅ 実装完了・テスト準備OK

---

## 🐛 問題

ログには「✅ Found definition at line XX」と表示されるが、実際にはエディタのカーソルが定義位置にジャンプしない。

### 原因

`tab.cursor_line` と `tab.cursor_col` を更新しても、egui の `TextEdit` ウィジェットは自動的にこの変更を反映しない。egui は即時モードUIなので、内部状態の変更を明示的にウィジェットに伝える必要がある。

---

## ✅ 修正内容

### 1. EditorTabに新規フィールド追加

```rust
pub struct EditorTab {
    // ... 既存フィールド ...
    pub pending_cursor_jump: Option<(usize, usize)>,  // NEW: (line, col) for programmatic cursor movement
}
```

### 2. エディタ描画時にカーソル位置を設定

**ファイル**: `src/egui_app.rs` (行1233-1254)

```rust
// pending_cursor_jumpがある場合、文字オフセットを計算
let cursor_range_to_set = if let Some((jump_line, jump_col)) = tab.pending_cursor_jump {
    let char_offset = {
        let mut offset = 0;
        for (line_idx, line) in text.lines().enumerate() {
            if line_idx == jump_line {
                offset += jump_col.min(line.len());
                break;
            }
            offset += line.len() + 1; // +1 for newline
        }
        offset
    };

    tracing::info!("📍 Jumping to line {} col {} (char offset: {})", jump_line, jump_col, char_offset);
    Some(egui::text::CCursorRange::one(egui::text::CCursor::new(char_offset)))
} else {
    None
};
```

### 3. TextEdit state に cursor_range を適用

**ファイル**: `src/egui_app.rs` (行1319-1326)

```rust
// Manually set cursor if we have a pending jump
if let Some(cursor_range) = cursor_range_to_set {
    let response_id = output.response.id;
    let mut state = output.state.clone();
    state.cursor.set_char_range(Some(cursor_range));
    state.store(ui.ctx(), response_id);
}
```

### 4. ナビゲーション関数を更新

以下の関数で `pending_cursor_jump` を設定するように変更:

- **`navigate_to_location()`** (行2278-2284)
- **`fallback_goto_definition()`** (行2090-2096)
- **`search_definition_in_project()`** (行2185-2191)

**修正例**:
```rust
// 定義が見つかったら、pending_cursor_jumpを設定
if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
    tab.cursor_line = line_idx;
    tab.cursor_col = 0;
    tab.pending_cursor_jump = Some((line_idx, 0));
    tracing::info!("⏭️ Scheduled cursor jump to line {}", line_idx);
}
```

### 5. ジャンプ後のクリーンアップ

**ファイル**: `src/egui_app.rs` (行1331-1335)

```rust
// Clear pending cursor jump after rendering
if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
    if tab.pending_cursor_jump.is_some() {
        tab.pending_cursor_jump = None;
    }
}
```

---

## 🎯 動作フロー

1. **ユーザーがCmd+Click**: `handle_go_to_definition()` 呼び出し
2. **定義を検索**: LSPまたは正規表現で検索
3. **定義が見つかる**: `tab.pending_cursor_jump = Some((line, col))` を設定
4. **次のフレーム**:
   - `pending_cursor_jump` を検出
   - 行・列から文字オフセットを計算
   - `CCursorRange` を作成
   - TextEdit の `state.cursor` に適用
   - `state.store()` で永続化
5. **クリーンアップ**: `pending_cursor_jump = None` でリセット

---

## 📝 テスト手順

### 前提条件
BerryCode が起動していること

### テストケース

1. **ローカルファイル内のジャンプ**:
   - `test_stdlib.rs` を開く
   - 9行目の `test_function();` の `test_function` の上で **Cmd+Click**
   - → 3行目の `fn test_function()` にジャンプすること

2. **F12キーでのジャンプ**:
   - 関数呼び出しにカーソルを置く
   - **F12キー** を押す
   - → 定義にジャンプすること

3. **プロジェクト横断ジャンプ**:
   - 別ファイルで定義されている構造体・関数でCmd+Click
   - → 該当ファイルが開き、定義位置にジャンプすること

4. **標準ライブラリジャンプ**:
   - `Vec::new()` の上でCmd+Click
   - → rustup toolchain内のファイルが読み取り専用で開くこと
   - → 定義位置にジャンプすること

5. **複数定義の処理**:
   - trait実装が複数あるシンボルでCmd+Click
   - → 選択UIが表示されること
   - → 選択した定義にジャンプすること

### 期待されるログ

```
🖱️ Cmd+Click detected via interact()
📍 Cursor position: 1176
🔍 Triggering go-to-definition at position 1176
🔍 Looking for definition of: 'test_function'
📝 LSP unavailable, using local regex search
✅ Found definition at line 3: fn test_function() {
⏭️ Scheduled cursor jump to line 3
📍 Jumping to line 3 col 0 (char offset: XXX)
```

---

## 🔧 修正したファイル

1. **`src/egui_app.rs`**:
   - EditorTab構造体 (行14-22)
   - EditorTab::new() (行25-35)
   - エディタ描画ロジック (行1233-1335)
   - navigate_to_location() (行2278-2284)
   - fallback_goto_definition() (行2090-2096)
   - search_definition_in_project() (行2185-2191)

---

## 🎯 完了条件

- ✅ EditorTabに`pending_cursor_jump`フィールド追加
- ✅ エディタ描画時に文字オフセット計算
- ✅ TextEdit stateにcursor_range設定
- ✅ 全ナビゲーション関数を更新
- ✅ ビルド成功
- ⏳ **手動テスト実行中**

---

**次のステップ**: ユーザーにCmd+Clickとジャンプ機能をテストしてもらい、ログとUIの動作を確認する。
