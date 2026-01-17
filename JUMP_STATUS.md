# Go-to-Definition 実装状況レポート

**日付**: 2026-01-16 22:30
**ステータス**: 🟡 部分的に動作（同一ファイル内は完璧、別ファイルは課題あり）

---

## ✅ 動作している機能

### 1. 同一ファイル内のジャンプ
- ✅ Cmd+Click で定義にジャンプ
- ✅ F12 キーでジャンプ
- ✅ カーソル位置が正確に設定される
- ✅ スクロールが実行される（`📜 Scrolling to rect` ログ確認済み）

**ログ例**:
```
⏭️ Scheduled cursor jump to line 39
📍 Jumping to line 39 col 0 (char offset: 1355, y: 760.5)
📜 Scrolling to rect at y=760.5
```

---

## 🟡 部分的に動作（別ファイルへのジャンプ）

### 問題 1: 検索パターンが機能しない

**症状**:
- プロジェクト横断検索が実行される
- しかし **「Found definition in」** が表示されない
- 結果として別ファイルが開かない

**ログ例**:
```
🔍 Searching in project for 'TextBuffer'
⚠️ Definition not found for 'TextBuffer'  ← 実際は buffer.rs に存在するのに見つからない
```

**試した修正**:
1. ✅ `(pub\s+)?struct` パターン → 失敗
2. ✅ `pub struct` と `struct` を別々に検索 → **現在のバージョン**

**次の確認ポイント**:
- `native::search::search_in_files()` の実装を確認
- 正規表現エンジンの違いを調査
- 単純な文字列検索でテスト

### 問題 2: スクロール位置が微妙

**症状**:
- ジャンプは成功する
- スクロールも実行される（ログに表示される）
- しかし **定義が画面の見やすい位置に表示されない**

**現在の実装**:
```rust
// ScrollArea 内で ui.scroll_to_rect() を呼んでいる
ui.scroll_to_rect(cursor_rect, Some(egui::Align::Center));
```

**問題点**:
- ScrollArea 内の `ui.scroll_to_rect()` は効果が限定的
- egui の TextEdit は自動スクロール機能があるが、動いていない可能性

**次の修正案**:
1. ScrollArea の `scroll_to_cursor` オプションを調査
2. TextEdit の自動スクロール機能を有効化
3. ScrollArea の状態を直接操作

---

## 🔧 実装されているコード

### EditorTab 構造体
```rust
pub struct EditorTab {
    pub file_path: String,
    pub buffer: TextBuffer,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub is_dirty: bool,
    pub is_readonly: bool,
    pub pending_cursor_jump: Option<(usize, usize)>,  // NEW
}
```

### ジャンプフロー
1. **Cmd+Click 検出** → `handle_go_to_definition()`
2. **ローカル検索** → `fallback_goto_definition()`
3. **プロジェクト検索** → `search_definition_in_project()`
4. **ファイルオープン** → `open_file_from_path()`
5. **カーソル設定** → `pending_cursor_jump = Some((line, col))`
6. **次のフレーム**:
   - 文字オフセット計算
   - `CCursorRange` 作成
   - `state.cursor.set_char_range()`
   - `ui.scroll_to_rect()`

---

## 📝 テスト結果

### 同一ファイル内
| テスト | 結果 | 備考 |
|--------|------|------|
| Cmd+Click | ✅ | 完璧に動作 |
| F12 キー | ✅ | 完璧に動作 |
| スクロール | ✅ | 定義が表示される |
| カーソル位置 | ✅ | 正確 |

### 別ファイル
| テスト | 結果 | 備考 |
|--------|------|------|
| 検索実行 | ✅ | ログに表示される |
| 定義発見 | ❌ | "Definition not found" |
| ファイルオープン | ❌ | 見つからないので開かない |
| ジャンプ | - | ファイルが開かないのでN/A |

---

## 🎯 次のステップ

### 優先度 高: 検索パターン修正

1. **`native::search::search_in_files()` を調査**:
   - 正規表現がどう処理されているか確認
   - case_sensitive フラグの挙動確認

2. **シンプルなパターンでテスト**:
   ```rust
   "pub struct TextBuffer"  // 単純な文字列マッチ
   ```

3. **デバッグログ追加**:
   - 検索実行時のパターンをログ出力
   - マッチした結果数をログ出力

### 優先度 中: スクロール改善

1. **egui ドキュメント調査**:
   - ScrollArea の `scroll_to_cursor` オプション
   - TextEdit の自動スクロール機能

2. **別のアプローチ**:
   - ScrollArea の ID を使って状態を直接操作
   - `ctx.scroll_to_rect()` を ScrollArea の外で呼ぶ

---

## 📊 修正ファイル

- `src/egui_app.rs`:
  - EditorTab 構造体（行14-22）
  - エディタ描画（行1233-1370）
  - `handle_go_to_definition()`
  - `fallback_goto_definition()`
  - `search_definition_in_project()` ← **最新修正箇所**
  - `navigate_to_location()`

---

## 🚀 使用方法（現時点）

### 動作するケース
```bash
# 同一ファイル内の定義へジャンプ
1. Rustファイルを開く
2. 関数名の上で Cmd+Click または F12
3. → 定義にジャンプ ✅
```

### 動作しないケース
```bash
# 別ファイルの定義へジャンプ
1. egui_app.rs で TextBuffer を見つける
2. Cmd+Click
3. → "Definition not found" ❌
```

---

**最終更新**: 2026-01-16 22:30
**ログファイル**: `/tmp/berrycode_jump_test5.log`
