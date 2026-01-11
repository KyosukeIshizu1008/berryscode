# BerryCode Auto-Testing Loop - Self-Healing Code Verification

## 概要

BerryCode の自動テスト機能により、AIがコード変更を行った後、即座に `cargo check` や `cargo test` を実行して問題を検出できます。これにより、**自己修復ループ** (Self-Healing Loop) が実現でき、AIが自身のミスを自動的に修正できます。

## 🎯 実装完了機能

### 1. `berrycode_cargo_check()` - コンパイルチェック

**ファイル**: `src-tauri/src/berrycode_commands.rs` (lines 471-555)

**機能**:
- `cargo check --message-format=json` を実行
- JSON出力をパースして構造化されたエラー/警告を抽出
- ファイル名、行番号、列番号、エラーコードを含む詳細情報を返却
- 実行時間を計測

**返り値**: `TestResult`
```rust
pub struct TestResult {
    pub success: bool,              // コンパイル成功/失敗
    pub output: String,             // cargo の生出力 (JSON)
    pub errors: Vec<CompilationError>,      // エラーのリスト
    pub warnings: Vec<CompilationWarning>,  // 警告のリスト
    pub duration_ms: u64,           // 実行時間 (ミリ秒)
}

pub struct CompilationError {
    pub file: String,               // ファイルパス
    pub line: u32,                  // 行番号
    pub column: u32,                // 列番号
    pub message: String,            // エラーメッセージ
    pub code: Option<String>,       // エラーコード (e.g., "E0425")
}
```

### 2. `berrycode_cargo_test()` - テスト実行

**ファイル**: `src-tauri/src/berrycode_commands.rs` (lines 557-600)

**機能**:
- `cargo test -- --nocapture` を実行
- テスト成功/失敗を判定
- stdout と stderr を結合して返却
- 実行時間を計測

**返り値**: `TestResult`
```rust
pub struct TestResult {
    pub success: bool,              // テスト成功/失敗
    pub output: String,             // テスト結果の全出力
    pub errors: Vec<CompilationError>,      // (空)
    pub warnings: Vec<CompilationWarning>,  // (空)
    pub duration_ms: u64,           // 実行時間 (ミリ秒)
}
```

## 📋 使用例

### 基本的な使い方

#### バックエンド (Rust)

```rust
use crate::berrycode_commands::berrycode_cargo_check;

// コンパイルチェックを実行
let result = berrycode_cargo_check(state).await?;

if result.success {
    println!("✅ Compilation successful! ({} warnings)", result.warnings.len());
} else {
    println!("❌ Compilation failed! ({} errors)", result.errors.len());

    for error in &result.errors {
        println!("  {}:{}:{} - {}",
            error.file, error.line, error.column, error.message);
    }
}
```

#### フロントエンド (Leptos/WASM)

```rust
use crate::tauri_bindings_berrycode::berrycode_cargo_check;

let result = berrycode_cargo_check().await?;

if !result.success {
    // AIにエラー情報を渡して修正を依頼
    let error_context = result.errors.iter()
        .map(|e| format!("{}:{} - {}", e.file, e.line, e.message))
        .collect::<Vec<_>>()
        .join("\n");

    let fix_prompt = format!(
        "以下のコンパイルエラーを修正してください:\n{}",
        error_context
    );

    // AIに送信...
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
        eprintln!("[Self-Healing] Attempt {}/{}", attempt, max_attempts);

        // ステップ1: コードを適用
        apply_code_changes(&code).await?;

        // ステップ2: cargo check を実行
        let check_result = berrycode_cargo_check(state.clone()).await?;

        if check_result.success {
            eprintln!("[Self-Healing] ✅ Success after {} attempts!", attempt);
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

        eprintln!("[Self-Healing] ❌ Errors found:\n{}", error_summary);

        // ステップ4: AIに修正を依頼
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

### パターン1: コード生成 → 検証 → 修正

```rust
// 1. AIにコード生成を依頼
let generated_code = ai_generate_code(prompt).await?;

// 2. ファイルに書き込み
write_file("src/new_feature.rs", &generated_code)?;

// 3. コンパイルチェック
let check = berrycode_cargo_check(state).await?;

if !check.success {
    // 4. エラーがあれば修正を依頼
    let errors = format_errors(&check.errors);
    let fixed_code = ai_fix_code(&generated_code, &errors).await?;

    // 5. 修正版を適用
    write_file("src/new_feature.rs", &fixed_code)?;

    // 6. 再チェック
    let recheck = berrycode_cargo_check(state).await?;
    assert!(recheck.success, "Still has errors!");
}

// 7. テストも実行
let test = berrycode_cargo_test(state).await?;
assert!(test.success, "Tests failed!");
```

### パターン2: CI/CD統合

```bash
#!/bin/bash
# BerryCode AI を使った自動リファクタリング + 検証

set -e

echo "🤖 Step 1: AI Refactoring"
berrycode --dangerously-skip-permissions \
  "Refactor src/legacy.rs to use async/await"

echo "🔍 Step 2: Compile Check"
if ! cargo check; then
  echo "❌ Compilation failed, asking AI to fix..."

  # AIに修正を依頼 (疑似コード)
  berrycode --dangerously-skip-permissions \
    "Fix compilation errors shown in cargo check"

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

### `cargo test` 実行

```rust
TestResult {
    success: true,
    output: "
running 45 tests
test buffer::tests::test_insert ... ok
test buffer::tests::test_delete ... ok
...
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
    ",
    errors: [],
    warnings: [],
    duration_ms: 5432,
}
```

**ログ出力**:
```
[BerryCode] 🧪 Running cargo test...
[BerryCode] ✅ cargo test passed (5432ms)
```

## 🎯 AIプロンプトへの活用

### エラー情報をコンテキストに追加

```javascript
// フロントエンドでの使用例
const context = await berrycode_get_context();
const checkResult = await berrycode_cargo_check();

let aiPrompt = `
プロジェクト: ${context.project_root}
ファイル数: ${context.files.length}
Gitブランチ: ${context.git_status?.branch}

現在のコンパイル状態:
`;

if (checkResult.success) {
  aiPrompt += `✅ コンパイル成功 (${checkResult.warnings.length} 警告)`;
} else {
  aiPrompt += `❌ コンパイルエラー:\n`;
  checkResult.errors.forEach(err => {
    aiPrompt += `  - ${err.file}:${err.line} - ${err.message}\n`;
  });
}

aiPrompt += "\n\nタスク: 上記のエラーを修正してください。";

// AIに送信...
```

### 段階的な修正戦略

```javascript
async function incrementalFix(prompt) {
  // 1. 初回コード生成
  let code = await aiGenerate(prompt);
  let attempts = 0;
  const maxAttempts = 3;

  while (attempts < maxAttempts) {
    attempts++;

    // 2. コンパイルチェック
    const result = await berrycode_cargo_check();

    if (result.success) {
      console.log(`✅ 成功 (試行回数: ${attempts})`);
      return code;
    }

    // 3. エラーの優先度付け
    const criticalErrors = result.errors.filter(e =>
      e.code && ["E0425", "E0308", "E0277"].includes(e.code)
    );

    // 4. 重要なエラーから修正
    const fixPrompt = `
以下の重要なコンパイルエラーを修正してください:
${criticalErrors.map(e => `${e.file}:${e.line} - ${e.message}`).join('\n')}

現在のコード:
\`\`\`rust
${code}
\`\`\`
    `;

    code = await aiGenerate(fixPrompt);
  }

  throw new Error(`${maxAttempts}回試行しても修正できませんでした`);
}
```

## 🚀 将来の拡張案

### 1. インクリメンタルチェック

ファイル変更を監視して、自動的に `cargo check` を実行:

```rust
pub async fn watch_and_check(
    file_path: String,
) -> Result<TestResult, String> {
    // ファイル変更を検出したら自動実行
}
```

### 2. テスト範囲の最適化

変更されたファイルに関連するテストのみを実行:

```rust
pub async fn berrycode_cargo_test_affected(
    changed_files: Vec<String>,
) -> Result<TestResult, String> {
    // 影響を受けるテストのみを実行
}
```

### 3. パフォーマンス分析

コンパイル時間とテスト時間を可視化:

```rust
pub struct PerformanceReport {
    pub compile_time_ms: u64,
    pub test_time_ms: u64,
    pub slowest_tests: Vec<(String, u64)>,
}
```

### 4. エラーの優先度付け

クリティカルなエラーを優先的に修正:

```rust
pub fn prioritize_errors(errors: &[CompilationError]) -> Vec<CompilationError> {
    // E0425 (undefined) > E0308 (type mismatch) > 警告
}
```

## 📝 デバッグTips

### ログの確認

```bash
# Tauri開発モードで実行
cargo tauri dev

# コンソールで以下のように表示される:
[BerryCode] 🔍 Running cargo check...
[BerryCode] ❌ cargo check failed (2 errors, 1 warnings)
```

### DevToolsでの手動テスト

```javascript
// ブラウザのDevToolsコンソールで:
const result = await window.__TAURI__.core.invoke('berrycode_cargo_check');
console.log(result);

// 出力:
{
  success: false,
  errors: [
    { file: "src/main.rs", line: 42, column: 5, message: "..." }
  ],
  warnings: [],
  duration_ms: 1234
}
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

---

## 📊 テスト実行例

### 実行コマンド

```bash
cd src-tauri
cargo tauri dev

# DevToolsコンソールで:
```

```javascript
// テスト1: cargo check
const check = await window.__TAURI__.core.invoke('berrycode_cargo_check');
console.log('Cargo Check:', check.success ? '✅' : '❌', check);

// テスト2: cargo test
const test = await window.__TAURI__.core.invoke('berrycode_cargo_test');
console.log('Cargo Test:', test.success ? '✅' : '❌', test);

// テスト3: エラー詳細表示
if (!check.success) {
  check.errors.forEach(err => {
    console.error(`${err.file}:${err.line}:${err.column} - ${err.message}`);
  });
}
```

### 期待される出力

```
[BerryCode] 🔍 Running cargo check...
[BerryCode] ✅ cargo check passed (38 warnings)
Cargo Check: ✅ {success: true, errors: [], warnings: [...38 warnings], duration_ms: 2341}

[BerryCode] 🧪 Running cargo test...
[BerryCode] ✅ cargo test passed (4532ms)
Cargo Test: ✅ {success: true, output: "...", duration_ms: 4532}
```

---

**実装完了！** 🎉

BerryCode は自己修復能力を獲得しました。AIが生成したコードに問題があっても、自動的に検出・修正できます。
