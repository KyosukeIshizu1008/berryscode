# Go-to-Definition テストガイド

**日付**: 2026-01-16 23:10
**目的**: LSP による標準ライブラリジャンプのテスト

---

## 🎯 テスト目的

BerryCode で HashMap などの標準ライブラリ型をクリックして、rust-analyzer 経由で標準ライブラリのソースコードにジャンプできることを確認します。

---

## ✅ 実装済み機能

1. **LSP Service** (berry-api-server)
   - ✅ LSPService gRPC 実装完了
   - ✅ rust-analyzer プロセス起動・管理
   - ✅ goto_definition RPC 実装

2. **LSP Client** (BerryCode)
   - ✅ LSP 接続・初期化
   - ✅ goto_definition リクエスト送信
   - ✅ レスポンス処理（単一定義、複数定義、フォールバック）
   - ✅ 標準ライブラリ検出（`/.rustup/` パス）
   - ✅ 読み取り専用設定
   - ✅ スクロール（画面中央に表示）
   - ✅ デバッグログ強化

---

## 🚀 テスト環境

### 起動状態

- **berry-api-server**: 起動中（PID確認: `ps aux | grep berry-api-server`）
- **BerryCode**: 起動中（PID確認: `ps aux | grep berrycode-egui`）
- **LSP 接続**: ✅ 成功（ログに「🟢 LSP connection established」）

### ログファイル

- **BerryCode**: `/tmp/berrycode_goto_test.log`
- **berry-api-server**: `/tmp/berry_api_server.log`

---

## 📋 テスト手順

### テスト 1: 標準ライブラリへのジャンプ（HashMap）

**ファイル**: `src/test_goto_hashmap.rs`

```rust
use std::collections::HashMap;

fn main() {
    // Test HashMap go-to-definition
    let mut map: HashMap<String, i32> = HashMap::new();
    map.insert("test".to_string(), 42);

    println!("Map: {:?}", map);
}
```

**手順**:

1. BerryCode を開く
2. ファイルツリーから `src/test_goto_hashmap.rs` を開く
3. 5行目の `HashMap` の上で **Cmd+Click** (Mac) または **Ctrl+Click** (Windows)
4. または、カーソルを `HashMap` の上に置いて **F12** を押す

**期待される動作**:

✅ **成功した場合**:
- 標準ライブラリのファイルが開く（例: `~/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/collections/hash/map.rs`）
- カーソルが `HashMap` struct の定義にジャンプ
- ファイルが画面中央にスクロールされる
- ステータスバーに「📖 READ-ONLY」と表示される
- エディタで編集しようとしても変更できない

❌ **失敗した場合**:
- 何も起こらない
- エラーメッセージが表示される
- ローカルファイル内で検索してしまう

**ログ確認**:

```bash
tail -f /tmp/berrycode_goto_test.log
```

期待されるログ:
```
🔍 Looking for definition of: 'HashMap'
🚀 Requesting LSP goto_definition
   File: /Users/.../berrycode/src/test_goto_hashmap.rs
   Position: line=4, column=16
📍 LSP returned 1 locations
   Location 1: file:///.rustup/toolchains/.../hash/map.rs
📍 Navigating to location:
   File: /.rustup/toolchains/.../hash/map.rs
   Line: XXX, Column: YYY
📖 Detected standard library file
📖 Opened as read-only (stdlib)
⏭️ Scheduled cursor jump to line XXX col YYY
✅ Jumped to map.rs
```

---

### テスト 2: 同一ファイル内のジャンプ

**ファイル**: 任意の Rust ファイル（例: `src/egui_app.rs`）

**手順**:

1. `src/egui_app.rs` を開く
2. 関数呼び出し（例: `handle_go_to_definition`）の上で **Cmd+Click**
3. その関数の定義にジャンプすることを確認

**期待される動作**:

✅ 定義の行にジャンプし、画面中央にスクロールされる

---

### テスト 3: 複数定義の選択UI

**ファイル**: trait を実装している複数の型がある場合

**手順**:

1. trait 名の上で **Cmd+Click**
2. 複数の実装がある場合

**期待される動作**:

✅ **「📋 Choose Definition」ウィンドウが表示される**:
- 定義のリストが表示される
- ファイル名・行番号が表示される
- クリックするとその定義にジャンプ
- Cancel ボタンで閉じられる

---

### テスト 4: LSP フォールバック（LSP が使えない場合）

**手順**:

1. berry-api-server を停止: `pkill -f berry-api-server`
2. BerryCode で何かの定義にジャンプを試みる

**期待される動作**:

✅ **正規表現検索にフォールバック**:
- ステータスバーに「LSP unavailable, using local regex search」
- ローカルファイル内で正規表現検索
- 見つからない場合はプロジェクト全体を検索

---

### テスト 5: 読み取り専用ファイルの編集防止

**手順**:

1. HashMap の定義にジャンプ（テスト 1 参照）
2. 標準ライブラリファイルが開いたら、テキストを編集しようとする

**期待される動作**:

✅ **編集不可**:
- キーボード入力が無視される
- ステータスバーに「📖 READ-ONLY」表示
- エディタの上部に「⚠️ This file is read-only (standard library source)」警告

---

## 🐛 トラブルシューティング

### 問題 1: ジャンプしない

**症状**: Cmd+Click しても何も起こらない

**確認事項**:
1. LSP が接続されているか: ステータスバーに「🟢 LSP: Connected」
2. rust-analyzer が起動しているか: `ps aux | grep rust-analyzer`
3. ログを確認: `/tmp/berrycode_goto_test.log`

**解決策**:
- berry-api-server を再起動: `cd berry_api && cargo run --bin berry-api-server`
- BerryCode を再起動

---

### 問題 2: ファイルが開くがカーソルがおかしい

**症状**: ファイルは開くが、定義の位置にカーソルがない

**確認事項**:
- ログで「⏭️ Scheduled cursor jump to line X col Y」を確認
- スクロール位置を確認

**既知の問題**:
- スクロールは `egui::Align::Center` で中央に配置されるが、完璧ではない場合がある

---

### 問題 3: LSP が初期化に失敗

**症状**: ログに「❌ LSP initialization failed」

**解決策**:
1. berry-api-server が起動しているか確認
2. berry-api-server のログを確認: `/tmp/berry_api_server.log`
3. rust-analyzer がインストールされているか確認: `which rust-analyzer`

---

## 📊 期待される結果

### 全テスト成功の場合

- ✅ HashMap → 標準ライブラリにジャンプ
- ✅ 標準ライブラリファイルは読み取り専用
- ✅ 同一ファイル内のジャンプも動作
- ✅ 複数定義の選択UIが表示される
- ✅ LSP が使えない場合はフォールバック
- ✅ スクロールが画面中央に表示される

### ログの確認

**成功した場合のログ例**:

```
🚀 Requesting LSP goto_definition
   File: /Users/kyosukeishizu/oracleberry/berrycode/src/test_goto_hashmap.rs
   Position: line=4, column=16
📍 LSP returned 1 locations
   Location 1: file:///.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/collections/hash/map.rs
🔍 Received 1 definition locations
📍 Navigating to location:
   File: /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/collections/hash/map.rs
   Line: 147, Column: 11
📖 Detected standard library file
📄 Opening file: /.rustup/toolchains/...
✅ File loaded in new tab: ...
📖 Opened as read-only (stdlib)
⏭️ Scheduled cursor jump to line 147 col 11
📍 Jumping to line 147 col 11 (char offset: 5678, y: 2871.5)
📜 Scrolling to rect at y=2871.5
✅ Jumped to map.rs
```

---

## 🎯 次のステップ

テストが全て成功したら、以下の機能も実装可能です：

1. **コード補完** (Ctrl+Space)
   - LSP completions の実装

2. **ホバー情報** (マウスオーバー)
   - LSP hover の実装

3. **参照検索** (すべての使用箇所を検索)
   - LSP find_references の実装

4. **診断情報** (エラー・警告の表示)
   - LSP diagnostics の実装

---

**最終更新**: 2026-01-16 23:10
**テスト実行者**: ユーザー
**環境**: macOS / Windows / Linux
