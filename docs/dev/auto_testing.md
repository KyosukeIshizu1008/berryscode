# BerryCode Auto-Testing Loop - Self-Healing Code Verification (egui Native)

## 概要

BerryCode の自動テスト機能により、AIがコード変更を行った後、即座に `cargo check` や `cargo test` を実行して問題を検出できます。これにより、**自己修復ループ** (Self-Healing Loop) が実現でき、AIが自身のミスを自動的に修正できます。

**重要**: BerryCode は **100% Rust Native Desktop** アプリケーションです。Tauri/JavaScript は使用せず、全ての操作は Rust の `native::*` モジュールを通じて実行されます。

## 🎯 実装済み機能

### 1. Cargo Check/Test の実行

**アーキテクチャ**: egui Native Desktop + `tokio::process::Command`

**機能**:
- `cargo check --message-format=json` を実行
- `cargo test -- --nocapture` を実行
- JSON出力をパースして構造化されたエラー/警告を抽出
- ファイル名、行番号、列番号、エラーコードを含む詳細情報を返却
- 実行時間を計測

**データ構造**:
```rust
pub struct TestResult {
    pub success: bool,              // コンパイル成功/失敗
    pub output: String,             // cargo の生出力 (JSON)
    pub errors: Vec<CompilationError>,
    pub warnings: Vec<CompilationWarning>,
    pub duration_ms: u64,
}

pub struct CompilationError {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub code: Option<String>,       // エラーコード (e.g., "E0425")
}
```

## 📋 使用例

### 基本的な使い方 (Pure Rust)

```rust
use tokio::process::Command;
use std::time::Instant;

async fn run_cargo_check() -> anyhow::Result<TestResult> {
    let start = Instant::now();

    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .output()
        .await?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let json_output = String::from_utf8_lossy(&output.stdout);

    // JSON パース処理
    let errors = parse_cargo_json(&json_output)?;

    Ok(TestResult {
        success: output.status.success(),
        output: json_output.to_string(),
        errors,
        warnings: vec![],
        duration_ms,
    })
}
```

### egui アプリでの使用

```rust
pub struct TestPanel {
    test_result: Option<TestResult>,
    is_running: bool,
}

impl TestPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Test Panel");

        if ui.button("Run Tests").clicked() {
            self.is_running = true;
            // Spawn async task to run tests
            // (実際にはtokioランタイムと統合が必要)
        }

        if self.is_running {
            ui.label("Running tests...");
        } else if let Some(result) = &self.test_result {
            if result.success {
                ui.colored_label(egui::Color32::GREEN, "✅ All tests passed!");
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("❌ {} errors found", result.errors.len())
                );

                for error in &result.errors {
                    ui.label(format!("{}:{} - {}", error.file, error.line, error.message));
                }
            }
        }
    }
}
```

### 自己修復ループの実装例

```rust
pub async fn self_healing_loop(
    initial_code: String,
    max_attempts: usize,
) -> Result<String, String> {
    let mut code = initial_code;
    let mut attempt = 0;

    while attempt < max_attempts {
        attempt += 1;
        tracing::info!("[Self-Healing] Attempt {}/{}", attempt, max_attempts);

        // ステップ1: コードを適用
        apply_code_changes(&code).await?;

        // ステップ2: cargo check を実行
        let check_result = run_cargo_check().await
            .map_err(|e| e.to_string())?;

        if check_result.success {
            tracing::info!("[Self-Healing] ✅ Success after {} attempts!", attempt);
            return Ok(code);
        }

        // ステップ3: エラー情報を収集
        let error_summary = check_result.errors.iter()
            .map(|e| format!("{}:{}:{} - {} ({})",
                e.file, e.line, e.column, e.message,
                e.code.as_deref().unwrap_or("unknown")
            ))
            .collect::<Vec<_>>()
            .join("\n");

        tracing::error!("[Self-Healing] ❌ Errors found:\n{}", error_summary);

        // ステップ4: AIに修正を依頼 (native::grpc 経由)
        let fix_prompt = format!(
            "以下のコンパイルエラーがあります。コードを修正してください:\n\n{}",
            error_summary
        );

        code = ask_ai_to_fix(&fix_prompt).await?;
    }

    Err(format!("Failed to fix errors after {} attempts", max_attempts))
}
```

## 🔄 ワークフロー統合

### パターン1: コード生成 → 検証 → 修正 (100% Rust)

```rust
use crate::native::grpc;
use tokio::fs;

async fn generate_and_verify(prompt: String) -> anyhow::Result<()> {
    // 1. AIにコード生成を依頼 (native::grpc 経由)
    let session_id = grpc::start_session("./".to_string(), true).await?;
    let mut stream = grpc::chat_stream(session_id, prompt).await?;

    let mut generated_code = String::new();
    while let Some(chunk) = stream.recv().await {
        generated_code.push_str(&chunk);
    }

    // 2. ファイルに書き込み (native::fs 経由)
    fs::write("src/new_feature.rs", &generated_code).await?;

    // 3. コンパイルチェック
    let check = run_cargo_check().await?;

    if !check.success {
        // 4. エラーがあれば修正を依頼
        let errors = format_errors(&check.errors);
        let fix_prompt = format!("修正してください:\n{}", errors);

        // 5. 修正版を取得
        let mut fix_stream = grpc::chat_stream(session_id.clone(), fix_prompt).await?;
        let mut fixed_code = String::new();
        while let Some(chunk) = fix_stream.recv().await {
            fixed_code.push_str(&chunk);
        }

        // 6. 修正版を適用
        fs::write("src/new_feature.rs", &fixed_code).await?;

        // 7. 再チェック
        let recheck = run_cargo_check().await?;
        if !recheck.success {
            anyhow::bail!("Still has errors!");
        }
    }

    // 8. テストも実行
    let test = run_cargo_test().await?;
    if !test.success {
        anyhow::bail!("Tests failed!");
    }

    Ok(())
}
```

### パターン2: CI/CD統合

```bash
#!/bin/bash
# BerryCode AI を使った自動リファクタリング + 検証
# 注意: JavaScriptは使用しません - 全てRustで実装

set -e

echo "🤖 Step 1: AI Refactoring"
# BerryCode CLIまたはRust経由でAI呼び出し
cargo run --bin berrycode -- refactor src/legacy.rs

echo "🔍 Step 2: Compile Check"
if ! cargo check; then
  echo "❌ Compilation failed, asking AI to fix..."

  # AIに修正を依頼 (Rust経由)
  cargo run --bin berrycode -- fix-errors

  # 再チェック
  cargo check || exit 1
fi

echo "🧪 Step 3: Run Tests"
cargo test || {
  echo "❌ Tests failed, reverting changes..."
  git reset --hard HEAD
  exit 1
}

echo "✅ Success! Committing changes..."
git commit -am "AI refactoring: async/await migration"
```

## 📊 出力例

### `cargo check` 成功時

```rust
TestResult {
    success: true,
    output: "{\"reason\":\"compiler-artifact\",...}\n{\"reason\":\"build-finished\",...}",
    errors: [],
    warnings: [
        CompilationWarning {
            file: "src/buffer.rs",
            line: 42,
            column: 9,
            message: "unused variable: `old_text`",
        }
    ],
    duration_ms: 1234,
}
```

**ログ出力**:
```
[BerryCode] 🔍 Running cargo check...
[BerryCode] ✅ cargo check passed (1 warnings)
```

### `cargo check` 失敗時

```rust
TestResult {
    success: false,
    output: "...",
    errors: [
        CompilationError {
            file: "src/syntax.rs",
            line: 127,
            column: 18,
            message: "mismatched types\nexpected `&str`, found `String`",
            code: Some("E0308"),
        },
        CompilationError {
            file: "src/main.rs",
            line: 89,
            column: 5,
            message: "cannot find function `init_logger` in this scope",
            code: Some("E0425"),
        },
    ],
    warnings: [],
    duration_ms: 987,
}
```

**ログ出力**:
```
[BerryCode] 🔍 Running cargo check...
[BerryCode] ❌ cargo check failed (2 errors, 0 warnings)
```

## 🎯 AIプロンプトへの活用

### エラー情報をコンテキストに追加 (Pure Rust)

```rust
use crate::native::{fs, git};

async fn build_ai_context() -> String {
    // プロジェクト情報を収集
    let project_root = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let git_branch = git::get_current_branch(&project_root)
        .unwrap_or_else(|_| "unknown".to_string());

    // コンパイルチェック実行
    let check_result = run_cargo_check().await.unwrap();

    let mut prompt = format!(
        "プロジェクト: {}\nGitブランチ: {}\n\n現在のコンパイル状態:\n",
        project_root, git_branch
    );

    if check_result.success {
        prompt.push_str(&format!("✅ コンパイル成功 ({} 警告)", check_result.warnings.len()));
    } else {
        prompt.push_str("❌ コンパイルエラー:\n");
        for err in &check_result.errors {
            prompt.push_str(&format!("  - {}:{} - {}\n", err.file, err.line, err.message));
        }
    }

    prompt.push_str("\n\nタスク: 上記のエラーを修正してください。");
    prompt
}
```

### 段階的な修正戦略 (Pure Rust)

```rust
async fn incremental_fix(initial_prompt: String) -> anyhow::Result<String> {
    let session_id = grpc::start_session("./".to_string(), true).await?;

    // 1. 初回コード生成
    let mut stream = grpc::chat_stream(session_id.clone(), initial_prompt).await?;
    let mut code = String::new();
    while let Some(chunk) = stream.recv().await {
        code.push_str(&chunk);
    }

    let mut attempts = 0;
    const MAX_ATTEMPTS: usize = 3;

    while attempts < MAX_ATTEMPTS {
        attempts += 1;

        // 2. コンパイルチェック
        let result = run_cargo_check().await?;

        if result.success {
            tracing::info!("✅ 成功 (試行回数: {})", attempts);
            return Ok(code);
        }

        // 3. エラーの優先度付け
        let critical_errors: Vec<_> = result.errors.iter()
            .filter(|e| {
                e.code.as_ref()
                    .map(|c| ["E0425", "E0308", "E0277"].contains(&c.as_str()))
                    .unwrap_or(false)
            })
            .collect();

        // 4. 重要なエラーから修正
        let fix_prompt = format!(
            "以下の重要なコンパイルエラーを修正してください:\n{}\n\n現在のコード:\n```rust\n{}\n```",
            critical_errors.iter()
                .map(|e| format!("{}:{} - {}", e.file, e.line, e.message))
                .collect::<Vec<_>>()
                .join("\n"),
            code
        );

        let mut fix_stream = grpc::chat_stream(session_id.clone(), fix_prompt).await?;
        code.clear();
        while let Some(chunk) = fix_stream.recv().await {
            code.push_str(&chunk);
        }
    }

    anyhow::bail!("{}回試行しても修正できませんでした", MAX_ATTEMPTS)
}
```

## 🚀 将来の拡張案

### 1. インクリメンタルチェック (native::watcher 統合)

ファイル変更を監視して、自動的に `cargo check` を実行:

```rust
use crate::native::watcher::{FileWatcher, FileEvent};

pub async fn watch_and_check(repo_path: &str) -> anyhow::Result<()> {
    let mut watcher = FileWatcher::new()?;
    watcher.watch(repo_path)?;

    loop {
        match watcher.try_recv() {
            Some(FileEvent::Modified(path)) => {
                if path.ends_with(".rs") {
                    tracing::info!("🔍 File changed: {:?}, running check...", path);
                    let _ = run_cargo_check().await;
                }
            }
            _ => {}
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
```

### 2. テスト範囲の最適化

変更されたファイルに関連するテストのみを実行:

```rust
pub async fn run_tests_for_files(
    changed_files: Vec<String>,
) -> anyhow::Result<TestResult> {
    // 影響を受けるテストのみを実行
    let test_filter = changed_files.iter()
        .filter_map(|f| f.strip_suffix(".rs"))
        .map(|f| f.replace("/", "::"))
        .collect::<Vec<_>>()
        .join("|");

    let output = Command::new("cargo")
        .arg("test")
        .arg(&test_filter)
        .output()
        .await?;

    // ...
}
```

## 📝 デバッグTips

### ログの確認

```bash
# 環境変数でログレベルを設定
RUST_LOG=debug cargo run

# 出力例:
[BerryCode] 🔍 Running cargo check...
[BerryCode] ❌ cargo check failed (2 errors, 1 warnings)
```

### JSON出力の確認

```bash
# 直接 cargo check を実行して JSON を確認
cargo check --message-format=json | jq '.message.message'
```

## 🎓 参考資料

- [Cargo Output Format](https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages)
- [Rust Error Codes](https://doc.rust-lang.org/error-index.html)
- [Self-Healing Systems](https://en.wikipedia.org/wiki/Self-healing_system)
- [egui](https://github.com/emilk/egui)
- [Tokio Process](https://docs.rs/tokio/latest/tokio/process/)

---

## 📊 テスト実行例

### 実行コマンド

```bash
# egui Native Desktop アプリとして実行
cargo run --bin berrycode-egui

# または個別テスト
cargo test --lib
```

### コンソールでのテスト (Pure Rust)

```rust
#[tokio::test]
async fn test_cargo_check_integration() {
    let result = run_cargo_check().await.unwrap();

    if result.success {
        println!("✅ Cargo Check: Success");
        println!("Warnings: {}", result.warnings.len());
    } else {
        println!("❌ Cargo Check: Failed");
        for error in &result.errors {
            eprintln!("{}:{}:{} - {}",
                error.file, error.line, error.column, error.message);
        }
    }
}
```

### 期待される出力

```
[BerryCode] 🔍 Running cargo check...
[BerryCode] ✅ cargo check passed (38 warnings)

[BerryCode] 🧪 Running cargo test...
[BerryCode] ✅ cargo test passed (4532ms)
```

---

**実装方針** 🎉

BerryCode は **100% Rust + egui Native** で自己修復能力を実現します：
- ❌ JavaScript/Tauri は使用しない
- ✅ `native::*` モジュール経由で全操作
- ✅ `tokio::process::Command` で cargo 実行
- ✅ `native::grpc` で AI 統合
- ✅ egui Immediate Mode UI で構築
