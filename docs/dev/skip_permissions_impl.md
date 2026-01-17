# `--dangerously-skip-permissions` フラグの実装

## 概要

Claude Code の `--dangerously-skip-permissions` フラグに触発された機能を BerryCode に実装しました。このフラグにより、ユーザーの確認プロンプトをスキップして危険な操作（ファイル書き換え、削除、Git操作など）を即座に実行できるようになります。

## ⚠️ 警告

**このフラグは非常に危険です。以下の点に十分注意してください:**

- ファイルの上書き・削除が確認なしで実行されます
- Git コミット、プッシュが自動的に実行されます
- コード実行が即座に行われます
- データベース変更が確認なしで適用されます

## 実装詳細

### 1. CLI 引数の定義 (`src-tauri/src/berrycode/args.rs`)

```rust
/// Skip all permission prompts and execute dangerous operations immediately
#[arg(long = "dangerously-skip-permissions")]
#[arg(help = "⚠️  危険: すべての確認をスキップして危険な操作を即座に実行")]
pub dangerously_skip_permissions: bool,
```

**特徴:**
- デフォルト値: `false` (安全モード)
- 明示的な有効化が必要
- 日英両方のヘルプメッセージ

### 2. バックエンドステートでの保持 (`src-tauri/src/berrycode_commands.rs`)

```rust
pub struct BerryCodeState {
    /// Project root directory
    pub project_root: Mutex<Option<PathBuf>>,

    /// Skip all permission prompts (DANGEROUS!)
    pub dangerously_skip_permissions: bool,
}

impl BerryCodeState {
    pub fn new(project_root: Option<PathBuf>, dangerously_skip_permissions: bool) -> Self {
        Self {
            project_root: Mutex::new(project_root),
            dangerously_skip_permissions,
        }
    }

    pub fn should_skip_permissions(&self) -> bool {
        self.dangerously_skip_permissions
    }
}
```

**設計上のポイント:**
- シンプルな boolean フラグ
- `should_skip_permissions()` メソッドで読み取り専用アクセス
- デフォルトコンストラクタでは `false` (安全)

### 3. コマンド実装での利用

```rust
#[tauri::command]
pub async fn berrycode_execute_command(
    command: String,
    state: State<'_, BerryCodeState>,
) -> Result<String, String> {
    // Check for dangerous operations
    let is_dangerous = command.starts_with("/delete")
        || command.starts_with("/write")
        || command.starts_with("/commit")
        || command.starts_with("/push");

    if is_dangerous && !state.should_skip_permissions() {
        return Err(format!(
            "Permission denied for dangerous command: {}. \
             Use --dangerously-skip-permissions to execute without confirmation, \
             or review the changes in the UI first.",
            command
        ));
    }

    // If permissions are skipped, log warning
    if state.should_skip_permissions() {
        eprintln!(
            "[BerryCode] ⚠️  DANGEROUS MODE: Executing '{}' without permission check",
            command
        );
    }

    // ... 実際の処理 ...
}
```

**実装パターン:**
1. 危険な操作かどうかを判定
2. フラグが無効 && 危険な操作 → エラーを返す
3. フラグが有効 → 警告ログを出して実行

## 使用方法

### 基本的な使用例

```bash
# 通常モード (確認あり)
berrycode src/main.rs

# 危険モード (確認なし)
berrycode --dangerously-skip-permissions src/main.rs
```

### 推奨ワークフロー

#### ステップ1: ドライランで確認

```bash
# 何が起こるかをプレビュー
berrycode --dry-run --dangerously-skip-permissions src/
```

#### ステップ2: Gitで保護

```bash
# 現在の作業をコミット
git add .
git commit -m "Before AI changes"

# 危険モードで実行
berrycode --dangerously-skip-permissions src/

# 問題があればロールバック
git reset --hard HEAD
```

#### ステップ3: バックアップと実行

```bash
# バックアップ作成
tar -czf backup-$(date +%Y%m%d-%H%M%S).tar.gz .

# 安心して実行
berrycode --dangerously-skip-permissions src/
```

## 安全性のベストプラクティス

### ✅ すべきこと

1. **Git リポジトリで実行**
   ```bash
   git init  # まだリポジトリでない場合
   git add .
   git commit -m "Initial commit before AI"
   ```

2. **変更前にコミット**
   ```bash
   git status  # 未コミットの変更を確認
   git commit -am "Before AI session"
   ```

3. **ドライランで確認**
   ```bash
   berrycode --dry-run --dangerously-skip-permissions
   ```

4. **段階的に実行**
   ```bash
   # まず1ファイルだけで試す
   berrycode --dangerously-skip-permissions src/main.rs

   # 問題なければ全体に適用
   berrycode --dangerously-skip-permissions src/
   ```

5. **ログを監視**
   ```bash
   berrycode --dangerously-skip-permissions --verbose 2>&1 | tee berrycode.log
   ```

### ❌ してはいけないこと

1. **本番環境で使用**
   ```bash
   # ❌ 絶対にやらない
   berrycode --dangerously-skip-permissions /var/www/production/
   ```

2. **未コミットの重要な変更がある状態で実行**
   ```bash
   git status  # Uncommitted changes detected
   # ❌ ここで危険モードを使わない
   ```

3. **信頼できないモデルで使用**
   ```bash
   # ❌ 不明なエンドポイントで使用しない
   berrycode --model unknown-model --dangerously-skip-permissions
   ```

4. **バックアップなしで使用**
   ```bash
   # ❌ バックアップ手段がない状態で使用しない
   ```

## エラーメッセージ例

### フラグなしで危険な操作を試みた場合

```
Error: Permission denied for dangerous command: /delete src/old_code.rs.
Use --dangerously-skip-permissions to execute without confirmation,
or review the changes in the UI first.
```

### フラグありで実行した場合

```
[BerryCode] ⚠️  DANGEROUS MODE: Executing '/delete src/old_code.rs' without permission check
Successfully deleted: src/old_code.rs
```

## テストケース

### ユニットテスト

```rust
#[test]
fn test_berrycode_state_default() {
    let state = BerryCodeState::default();
    assert!(!state.should_skip_permissions());
}

#[test]
fn test_berrycode_state_with_skip_permissions() {
    let state = BerryCodeState::new(None, true);
    assert!(state.should_skip_permissions());
}

#[tokio::test]
async fn test_execute_command_blocks_dangerous_operations() {
    let state = BerryCodeState::default();

    let result = berrycode_execute_command(
        "/delete some_file".to_string(),
        State::from(&state),
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Permission denied"));
}

#[tokio::test]
async fn test_execute_command_allows_with_flag() {
    let state = BerryCodeState::new(None, true);

    let result = berrycode_execute_command(
        "/delete some_file".to_string(),
        State::from(&state),
    ).await;

    assert!(result.is_ok());
}
```

## 今後の拡張案

### 1. ホワイトリスト機能

特定の操作のみ許可するモード:

```bash
berrycode --dangerously-skip-permissions \
  --allowed-operations write,commit \
  --deny-operations delete,push
```

### 2. 監査ログ

すべての危険な操作をログファイルに記録:

```rust
pub struct BerryCodeState {
    pub dangerously_skip_permissions: bool,
    pub audit_log_path: Option<PathBuf>,  // 新規追加
}
```

### 3. タイムアウト機能

一定時間後に自動的に安全モードに戻る:

```bash
berrycode --dangerously-skip-permissions \
  --permission-timeout 3600  # 1時間後に無効化
```

### 4. インタラクティブ確認

重要な操作だけ確認を求める:

```rust
let is_critical = command.starts_with("/push") || command.starts_with("/rm -rf");

if is_critical && !state.should_skip_permissions() {
    // UIで確認ダイアログを表示
}
```

## セキュリティ考慮事項

### 脅威モデル

1. **誤操作**: AIが誤ったファイルを削除
   - 対策: Git による保護、ドライラン

2. **悪意のある入力**: 攻撃者がプロンプトインジェクション
   - 対策: 入力検証、サンドボックス実行

3. **設定ミス**: 本番環境で誤って有効化
   - 対策: 環境変数チェック、明示的な警告

### 推奨セキュリティ設定

```bash
# .berrycode.config.yml
security:
  # 本番環境で無効化
  disable_dangerous_flags_in_production: true

  # 監査ログ必須
  require_audit_log: true

  # Git 必須
  require_git_repository: true

  # 実行前確認
  confirm_before_destructive_ops: true
```

## FAQ

### Q: `--yes-always` との違いは？

**A:**
- `--yes-always`: 通常の確認プロンプトをスキップ (中程度の危険)
- `--dangerously-skip-permissions`: **すべての**安全チェックをバイパス (非常に危険)

### Q: どんな時に使うべき？

**A:**
- CI/CD パイプライン (Git で完全に管理されている環境)
- 自動テストフレームワーク
- 信頼できる AI モデルでの高速プロトタイピング

### Q: 絶対に使ってはいけない場面は？

**A:**
- 本番環境
- バックアップがない環境
- 未コミットの重要な変更がある状態
- 信頼できないAIモデル

## 変更履歴

- **2026-01-06**: 初期実装完了
  - CLI引数追加 (`--dangerously-skip-permissions`)
  - `BerryCodeState` にフラグ保持機能追加
  - `berrycode_execute_command` での例示実装
  - ユニットテスト4個追加
  - 包括的なドキュメント作成

## 関連ファイル

- `src-tauri/src/berrycode/args.rs` (547-581行): CLI引数定義
- `src-tauri/src/berrycode_commands.rs` (14-46行): ステート管理
- `src-tauri/src/berrycode_commands.rs` (305-335行): 実装例
- `src-tauri/src/berrycode_commands.rs` (469-509行): ユニットテスト

## 参考資料

- Claude Code: `--dangerously-skip-permissions` フラグ
- Rust Clap: Command-line argument parsing
- Tauri State Management: Global state in Tauri apps
