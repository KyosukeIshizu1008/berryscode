# AI & Collaboration / AI・コラボレーション

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

### AI Chat (Right Panel)

Integrated AI assistant connected to LLM via berry-api gRPC server.

- Header: connection status (green/red dot) + "New Chat" button
- Message area: user messages (blue bg) / AI responses (dark bg)
- Input area: text field + send button
- Image attachment via drag & drop (PNG/JPG/GIF/WebP/BMP)
- Conversation history retention
- Code-related questions, generation, explanation

### Live Collaboration

VS Code Live Share-style real-time co-editing.

- WebSocket relay server architecture
- Full-doc sync (CRDT planned)
- Remote cursor display (colored caret + name label)
- Shared document editing
- Session sharing link
- In-session chat
- States: Inactive → Hosting/Joining → Connected

### Remote Development

SSH-based remote editing (VS Code Remote style).

- SSH tunnel + berrycode-server
- JSON-RPC protocol
- Remote: file operations, LSP proxy, PTY proxy
- Remote file browser with file cache
- Messages: `fs/read`, `fs/write`, `lsp/*`, `pty/*`

### Plugin System

- `manifest.json` per plugin
- Activation events: `onLanguage`, `onCommand`
- Contributing commands & keybindings
- Shell command execution
- Plugin manager UI with enable/disable toggle
- Future: WASM-based plugins

---

<a name="japanese"></a>

## 日本語

## AI Chat (右パネル)

統合 AI アシスタント。berry-api サーバー経由で LLM に接続。

### UI
- ヘッダー: 接続ステータス（緑/赤ドット）+ 「New Chat」ボタン
- メッセージエリア: ユーザーメッセージ（青背景）/ AI 応答（暗い背景）
- 入力エリア: テキスト入力 + 送信ボタン
- 画像添付プレビュー

### 機能
- ドラッグ&ドロップで画像添付（PNG/JPG/GIF/WebP/BMP）
- gRPC 接続による AI サービス通信
- 会話履歴の保持
- New Chat で履歴クリア
- コード関連の質問・生成・説明

---

## Live Collaboration (リアルタイム共同編集)

VS Code Live Share スタイルのリアルタイム共同編集。

### アーキテクチャ
- WebSocket リレーサーバー
- 全文同期（将来的に CRDT）
- オペレーショナルトランスフォーム

### 機能
- リモートカーソル表示（色付きキャレット + 名前ラベル）
- 共有ドキュメント編集
- セッション共有リンク
- 色分けされたコラボレーター一覧
- セッション内チャット

### セッション状態
- Inactive → Hosting / Joining → Connected
- エラーハンドリング

---

## Remote Development (リモート開発)

VS Code Remote スタイルの SSH ベースリモート編集。

### アーキテクチャ
- SSH トンネル + berrycode-server
- JSON-RPC プロトコル
- リモート側: ファイル操作, LSP プロキシ, PTY プロキシ

### 機能
- リモートホスト/ユーザー/パス設定
- リモートファイルブラウザ
- ファイルキャッシュ（再取得回避）
- SSH プロセス管理

### メッセージ
- `fs/read`, `fs/write` — ファイル操作
- `lsp/*` — LSP プロキシ
- `pty/*` — ターミナルプロキシ

---

## プラグインシステム

### 構造
- `manifest.json` — プラグイン定義
- アクティベーションイベント: `onLanguage`, `onCommand`
- コマンド & キーバインドの追加
- シェルコマンド実行

### 管理
- プラグインマネージャー UI
- 有効/無効トグル
- マーケットプレイスリスト

### 将来
- WASM ベースプラグイン対応予定
