# Go-to-Definition テスト手順

## 準備
1. BerryCodeを起動: `RUST_LOG=info cargo run --bin berrycode-egui`
2. `test_stdlib.rs` を開く

## テスト1: ローカル関数ジャンプ（LSPなしで動作するはず）
1. 9行目の `test_function();` の上にカーソルを置く
2. `test_function` の文字の上で **Cmd+Click** (Mac) または **Ctrl+Click** (Windows/Linux)
3. **期待結果**: 3行目の `fn test_function() {` にジャンプ

### ログで確認すべき内容:
```
🖱️ Cmd+Click detected in editor
📍 Cursor position: XXX
🔍 Triggering go-to-definition at position XXX
🔍 Looking for definition of: 'test_function'
📝 LSP unavailable, using local regex search
✅ Found definition at line 3
```

## テスト2: F12キーでジャンプ
1. 9行目の `test_function();` の上にカーソルを置く
2. **F12キーを押す**
3. **期待結果**: 3行目にジャンプ

## もしジャンプしない場合の確認事項:
1. ターミナルに `🖱️ Cmd+Click detected in editor` が表示されるか？
   - **表示されない** → Cmd+Clickイベントが検出されていない
   - **表示される** → イベントは検出されている、別の問題

2. `📝 LSP unavailable, using local regex search` が表示されるか？
   - **表示される** → fallbackは動作している
   - **表示されない** → LSPが中途半端に接続されている可能性

3. `✅ Found definition` が表示されるか？
   - **表示されない** → 正規表現がマッチしていない
