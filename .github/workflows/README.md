# GitHub Actions CI/CD

このディレクトリには、プッシュ時に自動実行されるテストワークフローが含まれています。

## ワークフロー: `tests.yml`

### トリガー
- `main` または `develop` ブランチへのプッシュ
- `main` または `develop` ブランチへのプルリクエスト

### ジョブ

#### 1. Unit Tests (単体テスト)
- **実行内容**: `cargo test --lib`
- **対象**: 80個のユニットテスト
- **所要時間**: 約1-2分

#### 2. WASM Integration Tests (WASM結合テスト)
- **実行内容**: `wasm-pack test --headless --firefox`
- **対象**: 16個のWASMテストスイート (約230個のテスト)
- **環境**: Firefox + geckodriver
- **所要時間**: 約3-5分

#### 3. E2E Tests (エンドツーエンドテスト)
- **実行内容**: `./run_e2e_tests.sh`
- **対象**:
  - Syntax HTML Rendering
  - Rendering Accuracy
  - Codicon Font Loading
  - Database Panel E2E
  - Terminal Panel E2E
- **環境**: Xvfb (仮想ディスプレイ) + Firefox + Tauri
- **所要時間**: 約5-10分

#### 4. Lint and Format (リントとフォーマット)
- **実行内容**:
  - `cargo fmt --all -- --check` (フォーマットチェック)
  - `cargo clippy --all-targets --all-features -- -D warnings` (静的解析)
- **所要時間**: 約2-3分

### 合計所要時間
並列実行により、約10-15分で全てのテストが完了します。

## ローカルでの実行

### 全てのテストを実行
```bash
# 単体テスト
cargo test --lib

# WASM結合テスト
wasm-pack test --headless --firefox

# E2Eテスト
./run_e2e_tests.sh

# リント
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
```

### 単一のテストスイートを実行
```bash
# 特定のE2Eテスト
cargo test --test codicon_font_loading_test -- --ignored

# 特定のWASMテスト
wasm-pack test --headless --firefox --test canvas_rendering_test
```

## CI環境の要件

### Ubuntu Runner
- Rust 1.70+
- wasm32-unknown-unknown target
- Firefox + geckodriver
- Xvfb (仮想ディスプレイ)
- Tauri dependencies:
  - libwebkit2gtk-4.1-dev
  - libayatana-appindicator3-dev
  - librsvg2-dev

### キャッシュ
以下がキャッシュされ、ビルド時間を短縮:
- `~/.cargo/registry` (Cargoパッケージレジストリ)
- `~/.cargo/git` (Cargoパッケージindex)
- `target/` (ビルド成果物)

## トラブルシューティング

### E2Eテストが失敗する場合
1. ログを確認: GitHub Actions → 該当ワークフロー → "E2E Tests" ジョブ
2. アーティファクトをダウンロード: `/tmp/test_*.log`
3. ローカルで再現: `./run_e2e_tests.sh`

### WASM テストが失敗する場合
1. geckodriverのバージョンを確認
2. Firefoxのバージョンを確認
3. ローカルで再現: `wasm-pack test --headless --firefox`

### タイムアウトする場合
`tests.yml` の `timeout-minutes` を調整 (デフォルト: 30分)

## ステータスバッジ

リポジトリのREADMEに追加:
```markdown
[![Tests](https://github.com/YOUR_USERNAME/YOUR_REPO/workflows/Tests/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions)
```
