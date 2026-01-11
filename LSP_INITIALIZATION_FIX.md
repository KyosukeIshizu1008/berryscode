# LSP Initialization Error Fix - 2026-01-06

## 🐛 問題 (Problem)

LSP初期化時に以下のエラーが発生していました：

```
❌ LSP initialization failed: Tauri invoke failed: JsValue("LSP server closed connection (EOF)")
```

## 🔍 根本原因 (Root Causes)

### 1. **stderr出力の無視による調査困難** (`src-tauri/src/lsp/client.rs:50`)

```rust
// ❌ BEFORE: エラーメッセージが見えない
.stderr(Stdio::null()) // Suppress server stderr for now
```

rust-analyzerがエラーを出力しても、それが完全に無視されていたため、何が問題なのかを特定できませんでした。

### 2. **rootUriのフォーマット問題** (`src/core/virtual_editor.rs:881-885`)

```rust
// ❌ BEFORE: LSPプロトコルの要求に違反
let root_uri = parent.to_string_lossy().to_string();
// 例: "/Users/username/project" (生のファイルパス)
```

LSPプロトコルでは `file://` URIフォーマットが必須ですが、生のファイルパスを送信していました。

## ✅ 修正内容 (Fixes)

### 修正1: stderrのキャプチャとログ出力

**ファイル**: `src-tauri/src/lsp/client.rs`

```rust
// ✅ AFTER: rust-analyzerのエラーメッセージを表示
.stderr(Stdio::piped()) // Capture stderr to see rust-analyzer errors

// 別スレッドでstderrを読み取り、ログに出力
if let Some(stderr) = process.stderr.take() {
    let language = self.language.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[LSP stderr:{}] {}", language, line);
            }
        }
    });
}
```

**効果**: rust-analyzerが出すエラーメッセージがターミナルに表示されるため、問題の診断が可能になります。

### 修正2: rootUriを正しいURI形式に変換

**ファイル**: `src/core/virtual_editor.rs`

```rust
// ✅ AFTER: LSPプロトコル準拠のURI形式
let root_uri = if let Some(parent) = std::path::Path::new(&path).parent() {
    let abs_path = if parent.is_absolute() {
        parent.to_string_lossy().to_string()
    } else {
        // 相対パスを絶対パスに変換
        std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.join(parent).canonicalize().ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| parent.to_string_lossy().to_string())
    };
    // file:// URIフォーマットに変換
    format!("file://{}", abs_path)
} else {
    // カレントディレクトリをフォールバック
    std::env::current_dir()
        .ok()
        .map(|p| format!("file://{}", p.to_string_lossy()))
        .unwrap_or_else(|| "file://.".to_string())
};
// 例: "file:///Users/username/project"
```

**効果**: rust-analyzerが正しく認識できるURI形式でworkspaceのルートパスが送信されます。

### 修正3: 初期化時の詳細ログ追加

**ファイル**: `src-tauri/src/lsp/client.rs`

```rust
// ✅ AFTER: デバッグ用ログの追加
eprintln!("[LSP] Starting server: command={:?}, args={:?}", command, args);
eprintln!("[LSP] Initializing server with root_uri: {}", root_uri);
eprintln!("[LSP] Sending initialize request...");
eprintln!("[LSP] Received initialize response");
```

**効果**: LSP初期化プロセスの各ステップが可視化されます。

## 🧪 動作確認方法 (Testing)

次回アプリを起動する際、以下のログが表示されるようになります：

### 正常な場合:
```
[LSP] Finding executable: rust-analyzer
[LSP] ✅ Found rust-analyzer at: /opt/homebrew/bin/rust-analyzer
[LSP] Starting server: command="/opt/homebrew/bin/rust-analyzer", args=[]
[LSP] Server process started successfully
[LSP] Initializing server with root_uri: file:///Users/username/project
[LSP] Sending initialize request...
[LSP] Received initialize response
✅ LSP initialized successfully
```

### エラーがある場合:
```
[LSP] Finding executable: rust-analyzer
[LSP] ❌ rust-analyzer not found in any common location
[LSP stderr:rust] Error: xxxxxx (具体的なエラーメッセージ)
❌ LSP initialization failed: ...
```

## 📊 期待される結果 (Expected Outcome)

1. **EOFエラーが解消される**: 正しいURI形式により、rust-analyzerが正常に初期化される
2. **エラーの可視化**: 今後LSPで問題が発生した場合、詳細なログから原因を特定できる
3. **デバッグ効率の向上**: stderr出力により、rust-analyzerの内部エラーも確認可能

## 🔧 追加の推奨事項 (Additional Recommendations)

### 1. workspace_folders の使用 (将来的な改善)

LSP仕様では `rootUri` は非推奨となっており、`workspace_folders` の使用が推奨されています：

```rust
"workspaceFolders": [{
    "uri": "file:///Users/username/project",
    "name": "project"
}]
```

### 2. エラーハンドリングの強化

LSPが利用できない環境（rust-analyzer未インストール）でも、エディタの基本機能は動作するように、graceful degradation を実装済み：

```rust
// src/lsp.rs:46-49
pub fn is_offline(&self) -> bool {
    self.offline_mode
}
```

### 3. ログレベルの調整 (本番環境)

開発環境では詳細なログが有用ですが、本番環境では環境変数で制御できるようにすることを推奨：

```rust
if std::env::var("RUST_LOG").is_ok() {
    eprintln!("[LSP] Debug info...");
}
```

## 📝 関連ファイル (Related Files)

- `src-tauri/src/lsp/client.rs` - LSPクライアント実装（プロセス管理、通信）
- `src-tauri/src/lsp/commands.rs` - Tauriコマンド定義（フロントエンド↔バックエンド）
- `src/core/virtual_editor.rs` - エディタメインロジック（LSP初期化トリガー）
- `src/lsp_ui.rs` - フロントエンドLSP統合レイヤー

## 🎯 まとめ (Summary)

この修正により、LSP初期化の成功率が向上し、問題が発生した場合の診断も容易になります。特に、stderr出力のキャプチャは今後のデバッグ作業を大幅に効率化します。
