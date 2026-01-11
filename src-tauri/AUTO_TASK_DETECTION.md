# 🤖 AI自動タスク判定機能

## 概要

BerryCodeは、ユーザーのメッセージを自動的に分析し、最適なAIモデルを選択する機能を搭載しています。

## 🎯 動作フロー

```
ユーザーメッセージ
    ↓
[軽量AIモデルで分類]
    ↓
タスクタイプを判定 (design/implementation/review/test/debug)
    ↓
最適なモデルを自動選択
    ↓
llama4:scout (設計) または qwen3-coder:30b (実装・デバッグ) で実行
```

## 📊 タスクタイプと使用モデル

| タスクタイプ | 判定基準 | 使用モデル | 用途 |
|-------------|---------|-----------|------|
| **design** | アーキテクチャ設計、システム設計、技術選定 | `llama4:scout` (67GB) | 高度な推論が必要な設計判断 |
| **implementation** | コード実装、機能追加、リファクタリング | `qwen3-coder:30b` (30GB) | プログラミング特化 |
| **review** | コードレビュー、改善提案、ベストプラクティス | `llama4:scout` (67GB) | 人間味のあるレビュー |
| **test** | テストコード作成、テスト戦略 | `qwen3-coder:30b` (30GB) | テスト実装 |
| **debug** | バグ修正、エラー解決、動作不良の原因調査 | `qwen3-coder:30b` (30GB) | 高速なデバッグ |

## ⚙️ 設定方法

### 基本設定

```bash
export OPENAI_API_KEY="ollama"  # ローカルOllama用ダミーキー
export BERRYCODE_MODEL="qwen3-coder:30b"  # デフォルトモデル
```

### クラシファイアモデルのカスタマイズ

より軽量なモデルで分類を高速化できます：

```bash
export BERRYCODE_CLASSIFIER_MODEL="gemma:2b"  # 2GBの軽量モデル
```

**推奨クラシファイアモデル:**
- `gemma:2b` (2GB) - 最速、十分な精度
- `phi:2.7b` (2.7GB) - バランス型
- `qwen3-coder:30b` (30GB) - デフォルト、最高精度

## 🚀 使用例

### Web API経由

```json
{
  "type": "user_message",
  "content": "ユーザー認証のアーキテクチャを設計してください"
}
```

→ 自動的に `design` と判定され、`llama4:scout` が使用されます

```json
{
  "type": "user_message",
  "content": "この関数のバグを修正して"
}
```

→ 自動的に `debug` と判定され、`qwen3-coder:30b` が使用されます

### 手動指定も可能

自動判定をスキップして明示的にタスクタイプを指定できます：

```json
{
  "type": "user_message",
  "content": "この関数をリファクタリングして",
  "task_type": "implementation"  // 手動指定
}
```

## 🔍 判定ロジック

分類AIに渡されるプロンプト：

```
あなたはタスク分類の専門家です。ユーザーのメッセージを分析し、
以下の5つのカテゴリのうち最も適切なものを1単語で答えてください：

- design: アーキテクチャ設計、システム設計、技術選定、設計方針の相談
- implementation: コードの実装、機能追加、リファクタリング、コード記述
- review: コードレビュー、改善提案、ベストプラクティスの確認
- test: テストコード作成、テスト戦略、テストケースの設計
- debug: バグ修正、エラー解決、動作不良の原因調査

必ず上記5つの単語のいずれか1つだけを返してください。
```

## 📈 パフォーマンス

| クラシファイアモデル | 分類速度 | メモリ使用量 | 精度 |
|-------------------|---------|------------|------|
| `gemma:2b` | ~200ms | 2GB | 85% |
| `phi:2.7b` | ~300ms | 2.7GB | 90% |
| `qwen3-coder:30b` | ~500ms | 30GB | 95% |

## 🛠️ トラブルシューティング

### 分類が失敗する場合

ログを確認：
```bash
tail -f ~/.berrycode/logs/berrycode.log
```

失敗時は自動的に `implementation` にフォールバックされます。

### 手動指定したい場合

環境変数で自動判定を無効化（将来実装予定）：
```bash
export BERRYCODE_AUTO_DETECT_TASK=false
```

## 🎓 実装詳細

- **ファイル**: `src-tauri/src/berrycode/web/infrastructure/websocket.rs`
- **関数**: `detect_task_type_with_ai()`
- **設定API**: `src-tauri/src/berrycode/web/api/settings/model_settings_api.rs`

---

**Note**: この機能はWeb API（WebSocket）でのみ動作します。CLI版では未実装です。
