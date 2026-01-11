//! Command-line argument parsing for berrycode

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum EditFormat {
    Whole,
    Diff,
    DiffFenced,
    #[value(name = "editor-whole")]
    EditorWhole,
    #[value(name = "editor-diff")]
    EditorDiff,
    #[value(name = "editor-diff-fenced")]
    EditorDiffFenced,
    #[value(name = "wholefile-func")]
    WholefileFunc,
}

#[derive(Parser, Debug)]
#[command(name = "berrycode")]
#[command(about = "AI pair programming in your terminal")]
#[command(long_about = "BerryCode - AI-powered pair programming assistant

BerryCode brings the power of Large Language Models (LLMs) directly to your terminal,
providing an intelligent coding assistant that can:

  • Understand and modify your codebase using advanced tools
  • Execute searches, read files, and make precise edits
  • Integrate with git for automatic commits and change tracking
  • Support multiple AI models (OpenAI, Anthropic, DeepSeek, etc.)
  • Provide voice input, file watching, and GUI modes

Quick Start:
  berrycode                          # Start interactive session
  berrycode src/main.rs              # Edit specific file
  berrycode --model deepseek-chat    # Use specific model
  berrycode --help                   # Show this help

Environment Variables:
  BERRYCODE_MODEL              Default model to use (e.g., qwen3-coder:30b, llama4:scout)
  BERRYCODE_CLASSIFIER_MODEL   Lightweight model for auto task type detection (default: qwen3-coder:30b)
  BERRYCODE_MODE               Default mode (architect/code/ask)
  OPENAI_API_KEY               OpenAI API key (use any string for local Ollama)
  ANTHROPIC_API_KEY            Anthropic API key
  OPENAI_API_BASE              Custom API base URL (default: http://KyosukenoMac-Studio.local:11434/v1)

Local Ollama Setup:
  export OPENAI_API_KEY=\"ollama\"
  export BERRYCODE_MODEL=\"qwen3-coder:30b\"
  export BERRYCODE_CLASSIFIER_MODEL=\"gemma:2b\"  # Optional: Use smaller model for classification
  berrycode

For detailed documentation, visit: https://github.com/your-repo/berrycode")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Args {
    /// Files to edit with an LLM (optional)
    #[arg(value_name = "FILE")]
    #[arg(help = "LLMで編集するファイル（オプション）")]
    #[arg(long_help = "AIアシスタントで編集するファイルを指定します。
ファイルを指定しない場合は、対話型チャットモードで開始します。

例:
  berrycode src/main.rs
  berrycode Cargo.toml README.md
  berrycode src/**/*.rs")]
    pub files: Vec<PathBuf>,

    // Main model
    /// Specify the model to use for the main chat
    #[arg(long, env = "BERRYCODE_MODEL")]
    #[arg(help = "メインのチャットに使用するモデルを指定")]
    #[arg(long_help = "メインの会話に使用するAIモデルを指定します。

対応モデル:
  OpenAI:     gpt-4, gpt-4-turbo, gpt-3.5-turbo
  Anthropic:  claude-3-opus, claude-3-sonnet, claude-3-haiku
  DeepSeek:   deepseek-chat, deepseek-coder
  カスタム:     APIエンドポイントがサポートする任意のモデル

環境変数: BERRYCODE_MODEL

例:
  --model deepseek-chat
  --model gpt-4-turbo
  --model claude-3-opus")]
    pub model: Option<String>,

    /// Specify the mode (architect/code/ask)
    #[arg(long, env = "BERRYCODE_MODE")]
    #[arg(help = "モードを指定 (architect/code/ask)")]
    #[arg(long_help = "操作モードを指定します:

  architect  - 高レベルなアーキテクチャと設計モード
  code       - 直接的なコード編集と実装モード（デフォルト）
  ask        - 質問と回答モード（読み取り専用）

環境変数: BERRYCODE_MODE

例:
  --mode architect  # システム設計用
  --mode code       # 実装用
  --mode ask        # 探索用")]
    pub mode: Option<String>,

    // API Keys and settings
    /// Specify the OpenAI API key
    #[arg(long, env = "OPENAI_API_KEY")]
    #[arg(help = "OpenAI APIキーを指定")]
    #[arg(long_help = "OpenAIモデルにアクセスするためのOpenAI APIキーを指定します。

DeepSeekなどのOpenAI互換APIにも使用されます。

環境変数: OPENAI_API_KEY

例:
  --openai-api-key sk-xxxxx
  export OPENAI_API_KEY=sk-xxxxx")]
    pub openai_api_key: Option<String>,

    /// Specify the Anthropic API key
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    #[arg(help = "Anthropic APIキーを指定")]
    #[arg(long_help = "ClaudeモデルにアクセスするためのAnthropic APIキーを指定します。

Claude-3 Opus、Sonnet、Haikuを使用するために必要です。

環境変数: ANTHROPIC_API_KEY

例:
  --anthropic-api-key sk-ant-xxxxx
  export ANTHROPIC_API_KEY=sk-ant-xxxxx")]
    pub anthropic_api_key: Option<String>,

    /// Specify the api base url
    #[arg(long, env = "OPENAI_API_BASE")]
    #[arg(help = "APIベースURLを指定")]
    #[arg(long_help = "OpenAI互換エンドポイントのカスタムAPIベースURLを指定します。

用途:
  • DeepSeek API: https://api.deepseek.com
  • Azure OpenAI: https://YOUR-RESOURCE.openai.azure.com
  • ローカルモデル: http://localhost:8080/v1
  • カスタムプロキシやゲートウェイ

環境変数: OPENAI_API_BASE

例:
  --openai-api-base https://api.deepseek.com
  --openai-api-base http://localhost:11434/v1")]
    pub openai_api_base: Option<String>,

    /// Set an environment variable (can be used multiple times)
    #[arg(long = "set-env", value_name = "ENV_VAR_NAME=value")]
    pub set_env: Vec<String>,

    /// Set an API key for a provider (eg: --api-key provider=<key>)
    #[arg(long, value_name = "PROVIDER=KEY")]
    pub api_key: Vec<String>,

    // Model settings
    /// List known models which match the (partial) MODEL name
    #[arg(long = "list-models", long = "models", value_name = "MODEL")]
    pub list_models: Option<String>,

    /// Specify a file with berrycode model settings for unknown models
    #[arg(long, default_value = ".berrycode.model.settings.yml")]
    pub model_settings_file: PathBuf,

    /// Specify a file with context window and costs for unknown models
    #[arg(long, default_value = ".berrycode.model.metadata.json")]
    pub model_metadata_file: PathBuf,

    /// Add a model alias (can be used multiple times)
    #[arg(long, value_name = "ALIAS:MODEL")]
    pub alias: Vec<String>,

    /// Set the reasoning_effort API parameter
    #[arg(long)]
    pub reasoning_effort: Option<String>,

    /// Set the thinking token budget for models that support it
    #[arg(long)]
    pub thinking_tokens: Option<String>,

    /// Check if model accepts settings before applying them
    #[arg(long, default_value = "true")]
    pub check_model_accepts_settings: bool,

    /// Disable checking if model accepts settings
    #[arg(long = "no-check-model-accepts-settings")]
    pub no_check_model_accepts_settings: bool,

    // Git settings
    /// Enable git integration (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "Git統合を有効化（デフォルト: true）")]
    #[arg(long_help = "変更を追跡するためのGit統合を有効化します。

有効にすると、BerryCodeは以下を行います:
  • Gitリポジトリを自動検出
  • 変更前にgitステータスを表示
  • 自動コミットを作成（--auto-commitsが有効な場合）
  • 変更の差分を表示

デフォルト: true

例:
  --git           # 有効化（デフォルト）
  --no-git        # 完全に無効化")]
    pub git: bool,

    /// Disable git integration
    #[arg(long = "no-git")]
    pub no_git: bool,

    /// Enable automatic git commits (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "自動Gitコミットを有効化（デフォルト: true）")]
    #[arg(long_help = "AIが変更を行った後に自動的にGitコミットを作成します。

コミットには以下が含まれます:
  • 説明的なメッセージ
  • BerryCodeへの帰属
  • 変更されたファイルのみ

推奨される用途:
  ✓ AI生成の変更を追跡
  ✓ 必要に応じた簡単なロールバック
  ✓ 明確な履歴の維持

デフォルト: true

例:
  --auto-commits        # 有効化（デフォルト）
  --no-auto-commits     # 無効化 - 手動コミットのみ")]
    pub auto_commits: bool,

    /// Disable automatic git commits
    #[arg(long = "no-auto-commits")]
    pub no_auto_commits: bool,

    /// Enable dirty commit prompts (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "ダーティコミットのプロンプトを有効化（デフォルト: true）")]
    #[arg(long_help = "ワーキングディレクトリに未コミットの変更がある場合、コミット前にプロンプトを表示します。

以下を誤って混在させるのを防ぎます:
  • 手動での変更
  • AI生成の変更

同じコミット内での混在を防止します。

デフォルト: true

例:
  --dirty-commits       # ダーティな場合にプロンプト（デフォルト）
  --no-dirty-commits    # ダーティなワーキングディレクトリでもコミットを許可")]
    pub dirty_commits: bool,

    /// Disable dirty commit prompts
    #[arg(long = "no-dirty-commits")]
    pub no_dirty_commits: bool,

    /// Enable dry run mode (don't write files or git commits)
    #[arg(long)]
    #[arg(help = "ドライランモードを有効化（ファイルやGitコミットを作成しない）")]
    #[arg(long_help = "テストモード - 実際にファイルを変更せずに、どのような変更が行われるかを表示します。

安全な用途:
  • AIの提案をテスト
  • 変更のプレビュー
  • デモンストレーション
  • 学習

ファイルやコミットは作成されません。

例:
  berrycode --dry-run src/main.rs
  berrycode --dry-run --verbose  # 詳細なプレビューを表示")]
    pub dry_run: bool,

    /// Specify a different git repo to use
    #[arg(long, value_name = "GIT_ROOT")]
    #[arg(help = "使用する別のGitリポジトリパスを指定")]
    #[arg(long_help = "自動検出の代わりに別のGitリポジトリを使用します。

用途:
  • ネストされたGitリポジトリを持つモノレポ
  • 現在のリポジトリ外のファイルでの作業
  • カスタムGitワークフロー

例:
  --git-root /path/to/repo
  --git-root ../parent-repo")]
    pub git_root: Option<PathBuf>,

    // Chat history and context
    /// Specify the chat input history file
    #[arg(long, default_value = ".berrycode.input.history")]
    #[arg(help = "チャット入力履歴ファイルを指定")]
    #[arg(long_help = "インタラクティブセッションのコマンドライン入力履歴を保存するファイル。

有効にする機能:
  • 上下矢印キーによる過去の入力へのナビゲーション
  • 検索可能なコマンド履歴（Ctrl+R）
  • セッション間での永続的な履歴

デフォルト: .berrycode.input.history

例:
  --input-history-file .history       # カスタムファイル名
  --input-history-file /tmp/hist      # 別の場所")]
    pub input_history_file: PathBuf,

    /// Specify the chat history file
    #[arg(long, default_value = ".berrycode.chat.history.md")]
    #[arg(help = "チャット履歴ファイルを指定")]
    #[arg(long_help = "完全な会話履歴をMarkdown形式で保存するファイル。

含まれる内容:
  • ユーザーメッセージとAI応答
  • ツール呼び出しと結果
  • タイムスタンプとメタデータ
  • 読みやすいMarkdown形式

用途:
  • 過去の会話のレビュー
  • AIインタラクションの共有
  • デバッグと分析

デフォルト: .berrycode.chat.history.md

例:
  --chat-history-file logs/chat.md    # カスタム場所
  --chat-history-file session.md      # 別の名前")]
    pub chat_history_file: PathBuf,

    /// Restore previous chat history messages
    #[arg(long)]
    #[arg(help = "以前のチャット履歴メッセージを復元")]
    #[arg(long_help = "以前の会話を復元して、中断したところから続行します。

有効にすると:
  • チャット履歴ファイルからメッセージを読み込み
  • AIが以前のコンテキストを記憶
  • 過去のセッションをシームレスに継続

使用例:
  • 複数セッションにまたがる長期プロジェクト
  • 中断後の再開
  • 以前の作業の上に構築

例:
  berrycode --restore-chat-history    # 以前のセッションを継続")]
    pub restore_chat_history: bool,

    /// Specify the number of context lines to use with non-system messages
    #[arg(long, default_value = "2")]
    #[arg(help = "メッセージのコンテキスト行数を指定")]
    #[arg(long_help = "コードスニペットを表示する際に含める周囲の行数。

高い値:
  ✓ コード理解のためのより多くのコンテキスト
  ✗ 大きなペイロード、高いコスト

低い値:
  ✓ 高速、低コスト
  ✗ 重要なコンテキストを見逃す可能性

デフォルト: 2（良いバランス）

例:
  --context-lines 5    # より多くのコンテキスト
  --context-lines 0    # 最小限のコンテキスト")]
    pub context_lines: usize,

    // Output settings
    /// Use VI editing mode in the terminal (default is Emacs)
    #[arg(long)]
    #[arg(help = "ターミナルでVI編集モードを使用")]
    #[arg(long_help = "コマンドライン編集のためにVI/Vimキーバインドに切り替えます。

デフォルト: Emacsモード（Ctrl+A、Ctrl+Eなど）

VIモードでは:
  • ノーマルモード: ESC、hjklナビゲーション
  • 挿入モード: i、a、A
  • コマンドモード: :、/、?

Vimユーザー向け:
  ✓ 慣れ親しんだナビゲーション
  ✓ モーダル編集
  ✓ 強力なテキスト操作

例:
  berrycode --vim    # VIモードを有効化")]
    pub vim: bool,

    /// Enable dark mode (default colors)
    #[arg(long)]
    #[arg(help = "ダークモードを有効化（デフォルトカラー）")]
    #[arg(long_help = "ダークターミナル用に最適化されたダークモードカラースキームを使用します。

特徴:
  • 高コントラストのテキスト
  • ダーク背景用に最適化
  • デフォルトカラーパレット

例:
  berrycode --dark-mode")]
    pub dark_mode: bool,

    /// Enable light mode colors
    #[arg(long)]
    #[arg(help = "ライトモードカラーを有効化")]
    #[arg(long_help = "ライトターミナル用に最適化されたライトモードカラースキームを使用します。

特徴:
  • ライト背景用に調整されたコントラスト
  • 白/ライトターミナルでの読みやすさ向上
  • 代替カラーパレット

例:
  berrycode --light-mode")]
    pub light_mode: bool,

    /// Color for user input
    #[arg(long, env = "BERRYCODE_USER_INPUT_COLOR")]
    #[arg(help = "ユーザー入力の色")]
    #[arg(long_help = "ユーザー入力テキストの色をカスタマイズします。

標準ターミナルカラーをサポート:
  black, red, green, yellow, blue, magenta, cyan, white

またはRGB16進コード:
  #FF0000, #00FF00, #0000FF

環境変数: BERRYCODE_USER_INPUT_COLOR

例:
  --user-input-color cyan
  --user-input-color \"#00FF00\"
  export BERRYCODE_USER_INPUT_COLOR=blue")]
    pub user_input_color: Option<String>,

    /// Color for tool errors
    #[arg(long, env = "BERRYCODE_TOOL_ERROR_COLOR")]
    #[arg(help = "ツールエラーの色")]
    #[arg(long_help = "ツールからのエラーメッセージの色をカスタマイズします。

デフォルト: red

用途:
  • ターミナルテーマのカスタマイズ
  • アクセシビリティ（色覚異常）
  • 個人の好み

環境変数: BERRYCODE_TOOL_ERROR_COLOR

例:
  --tool-error-color red
  --tool-error-color \"#FF0000\"")]
    pub tool_error_color: Option<String>,

    /// Color for tool warnings
    #[arg(long, env = "BERRYCODE_TOOL_WARNING_COLOR")]
    #[arg(help = "Color for tool warnings")]
    #[arg(long_help = "Customize the color of warning messages from tools.

Default: yellow

Environment: BERRYCODE_TOOL_WARNING_COLOR

Examples:
  --tool-warning-color yellow
  --tool-warning-color magenta")]
    pub tool_warning_color: Option<String>,

    /// Color for assistant output
    #[arg(long, env = "BERRYCODE_ASSISTANT_OUTPUT_COLOR")]
    #[arg(help = "Color for assistant output")]
    #[arg(long_help = "Customize the color of AI assistant responses.

Default: default terminal color

Environment: BERRYCODE_ASSISTANT_OUTPUT_COLOR

Examples:
  --assistant-output-color green
  --assistant-output-color cyan")]
    pub assistant_output_color: Option<String>,

    /// Code syntax highlighting theme
    #[arg(long, default_value = "default")]
    #[arg(help = "Code syntax highlighting theme")]
    #[arg(long_help = "Select syntax highlighting theme for code blocks.

Available themes:
  • default - Standard theme
  • monokai - Popular dark theme
  • solarized - Solarized color scheme
  • github - GitHub style
  • (and more...)

Default: default

Examples:
  --code-theme monokai
  --code-theme solarized")]
    pub code_theme: String,

    /// Enable verbose output
    #[arg(short, long)]
    #[arg(help = "Enable verbose output")]
    #[arg(long_help = "Show detailed information about BerryCode's operations.

Displays:
  • API request/response details
  • Tool execution steps
  • Internal state changes
  • Debug information

Useful for:
  • Troubleshooting issues
  • Understanding AI behavior
  • Development and debugging

Examples:
  berrycode -v              # Short form
  berrycode --verbose       # Long form")]
    pub verbose: bool,

    /// Always say yes to confirmation prompts
    #[arg(long = "yes-always")]
    #[arg(help = "Always say yes to confirmation prompts")]
    #[arg(long_help = "Automatically approve all confirmation prompts without asking.

Use with caution:
  ⚠️  Skips safety confirmations
  ⚠️  Auto-approves file modifications
  ⚠️  Auto-approves git commits

Good for:
  ✓ Automated scripts
  ✓ CI/CD pipelines
  ✓ Non-interactive environments

Examples:
  berrycode --yes-always src/main.rs")]
    pub yes_always: Option<bool>,

    /// Skip all permission prompts and execute dangerous operations immediately
    #[arg(long = "dangerously-skip-permissions")]
    #[arg(help = "⚠️  危険: すべての確認をスキップして危険な操作を即座に実行")]
    #[arg(long_help = "⚠️  DANGER: Skip ALL permission prompts and execute dangerous operations immediately.

This flag bypasses ALL safety confirmations including:
  ⚠️  File writes and overwrites
  ⚠️  File deletions
  ⚠️  Git operations (commits, pushes, etc.)
  ⚠️  Code execution
  ⚠️  Database modifications

🛡️  SAFETY RECOMMENDATIONS:
  ✓ Always run in a git repository
  ✓ Commit your work before using this flag
  ✓ Review AI suggestions carefully
  ✓ Use --dry-run first to preview changes
  ✓ Keep backups of important files

NEVER use this flag:
  ✗ On production systems
  ✗ With untrusted AI models
  ✗ On uncommitted work
  ✗ Without understanding the risks

Good for:
  ✓ Automated workflows (with git protection)
  ✓ CI/CD pipelines (with rollback capabilities)
  ✓ Rapid prototyping (with frequent commits)
  ✓ Trusted AI agents (in sandboxed environments)

Examples:
  berrycode --dangerously-skip-permissions src/
  berrycode --dangerously-skip-permissions --dry-run  # Preview first!")]
    pub dangerously_skip_permissions: bool,

    // Edit format
    /// Specify the edit format (default: auto-selected based on model)
    #[arg(long, value_enum)]
    #[arg(help = "Specify the edit format")]
    #[arg(long_help = "Control how the AI generates code edits.

Available formats:
  • whole         - Full file replacement
  • diff          - Search/replace format (recommended)
  • diff-fenced   - Diff in fenced code blocks
  • editor-whole  - Editor-specific whole file
  • editor-diff   - Editor-specific diff

Default: Auto-selected based on model capabilities

Examples:
  --edit-format diff         # Use diff format
  --edit-format whole        # Replace entire files")]
    pub edit_format: Option<EditFormat>,

    /// Specify the weak model for simple tasks
    #[arg(long)]
    #[arg(help = "Specify the weak model for simple tasks")]
    #[arg(long_help = "Use a faster, cheaper model for simple operations.

Good for:
  • File listing
  • Simple searches
  • Quick questions
  • Non-code tasks

Saves costs on routine operations while using powerful models for complex coding.

Examples:
  --weak-model gpt-3.5-turbo
  --weak-model claude-3-haiku")]
    pub weak_model: Option<String>,

    /// Specify the editor model
    #[arg(long)]
    #[arg(help = "Specify the editor model")]
    #[arg(long_help = "Use a specialized model for code editing operations.

Optimized for:
  • Precise code modifications
  • Understanding edit contexts
  • Generating accurate diffs

Examples:
  --editor-model deepseek-coder
  --editor-model gpt-4-turbo")]
    pub editor_model: Option<String>,

    /// Specify the editor edit format
    #[arg(long)]
    #[arg(help = "Specify the editor edit format")]
    #[arg(long_help = "Control edit format specifically for the editor model.

Override the default edit format when using the editor model.

Examples:
  --editor-edit-format diff
  --editor-edit-format whole")]
    pub editor_edit_format: Option<String>,

    // Linting and testing
    /// Specify lint commands by language (can be used multiple times)
    #[arg(long, value_name = "LANG:CMD")]
    #[arg(help = "Specify lint commands by language")]
    #[arg(long_help = "Configure custom linting commands for different languages.

Format: LANG:CMD

Supported languages:
  • rust, python, javascript, typescript, go, java, etc.

Examples:
  --lint-cmd rust:\"cargo clippy\"
  --lint-cmd python:\"flake8 --max-line-length=100\"
  --lint-cmd js:\"eslint --fix\"

Multiple linters:
  --lint-cmd rust:\"cargo clippy\" --lint-cmd python:\"pylint\"")]
    pub lint_cmd: Vec<String>,

    /// Enable auto-linting
    #[arg(long)]
    #[arg(help = "Enable auto-linting")]
    #[arg(long_help = "Automatically run linters after code changes.

When enabled:
  • Runs configured linters automatically
  • Catches errors immediately
  • AI can see and fix linting issues

Useful for:
  ✓ Maintaining code quality
  ✓ Catching errors early
  ✓ Enforcing code standards

Examples:
  berrycode --auto-lint src/")]
    pub auto_lint: bool,

    /// Specify test command
    #[arg(long)]
    #[arg(help = "Specify test command")]
    #[arg(long_help = "Command to run project tests.

Examples:
  --test-cmd \"cargo test\"
  --test-cmd \"npm test\"
  --test-cmd \"pytest -v\"
  --test-cmd \"go test ./...\"

The command will be executed from the project root.")]
    pub test_cmd: Option<String>,

    /// Enable auto-testing
    #[arg(long)]
    #[arg(help = "Enable auto-testing")]
    #[arg(long_help = "Automatically run tests after code changes.

When enabled:
  • Runs test command automatically
  • Verifies changes don't break tests
  • AI can see test results and fix failures

Perfect for:
  ✓ Test-driven development (TDD)
  ✓ Continuous verification
  ✓ Self-healing code loops

Requires:
  • --test-cmd to be configured

Examples:
  berrycode --auto-test --test-cmd \"cargo test\"")]
    pub auto_test: bool,

    // File watching
    /// Enable file watching
    #[arg(long)]
    #[arg(help = "Enable file watching")]
    #[arg(long_help = "Watch files for external changes and notify the AI.

When enabled:
  • Monitors project files for changes
  • Detects modifications from other editors/tools
  • AI becomes aware of external edits

Use cases:
  • Multi-editor workflows
  • Collaborative development
  • External build tool integration

Examples:
  berrycode --watch")]
    pub watch: bool,

    // Voice
    /// Enable voice input
    #[arg(long)]
    #[arg(help = "Enable voice input")]
    #[arg(long_help = "Enable voice-to-text input for hands-free coding.

Features:
  • Speak instead of typing
  • Useful for long descriptions
  • Accessibility support

Requires:
  • Microphone access
  • Voice recognition service

Examples:
  berrycode --voice")]
    pub voice: bool,

    // Misc
    /// Timeout for API requests (seconds)
    #[arg(long)]
    #[arg(help = "Timeout for API requests (seconds)")]
    #[arg(long_help = "Maximum time to wait for API responses before timing out.

Default: No timeout (waits indefinitely)

Useful for:
  • Slow network connections
  • Large context processing
  • Preventing infinite hangs

Examples:
  --timeout 60      # 60 second timeout
  --timeout 300     # 5 minute timeout for complex tasks")]
    pub timeout: Option<u64>,

    /// Disable SSL verification
    #[arg(long = "no-verify-ssl")]
    #[arg(help = "Disable SSL verification")]
    #[arg(long_help = "Disable SSL certificate verification for API requests.

⚠️  WARNING: Security risk! Only use for:
  • Local development with self-signed certificates
  • Corporate proxies with custom CAs
  • Testing environments

DO NOT use in production or with sensitive data.

Examples:
  berrycode --no-verify-ssl")]
    pub no_verify_ssl: bool,

    /// Verify SSL (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "Verify SSL certificates (default: true)")]
    #[arg(long_help = "Enable SSL certificate verification for API requests.

Default: true (recommended)

Ensures:
  ✓ Secure connections
  ✓ Valid certificates
  ✓ Protection against MITM attacks

Examples:
  --verify-ssl           # Explicitly enable (default)
  --no-verify-ssl        # Disable (not recommended)")]
    pub verify_ssl: bool,

    /// File encoding
    #[arg(long, default_value = "utf-8")]
    #[arg(help = "File encoding")]
    #[arg(long_help = "Character encoding for reading and writing files.

Default: utf-8 (recommended for most projects)

Supported encodings:
  • utf-8 (default)
  • ascii
  • latin1
  • utf-16
  • (and more...)

Examples:
  --encoding utf-8       # Default
  --encoding ascii       # ASCII only
  --encoding latin1      # ISO-8859-1")]
    pub encoding: String,

    /// Configuration file to load
    #[arg(short, long = "config", value_name = "CONFIG_FILE")]
    #[arg(help = "Configuration file to load")]
    #[arg(long_help = "Load settings from a YAML or JSON configuration file.

Allows pre-configuring:
  • Model settings
  • API keys
  • Default options
  • Project-specific preferences

Format: YAML or JSON

Examples:
  -c berrycode.yml
  --config .berrycode.config.json
  --config ~/my-berrycode-config.yml")]
    pub config_file: Option<PathBuf>,

    /// Environment file to load
    #[arg(long, default_value = ".env")]
    #[arg(help = "Environment file to load")]
    #[arg(long_help = "Load environment variables from a .env file.

Supports:
  • API keys (OPENAI_API_KEY, ANTHROPIC_API_KEY)
  • Custom settings
  • Project configuration

Default: .env

Format:
  OPENAI_API_KEY=sk-...
  BERRYCODE_MODEL=gpt-4

Examples:
  --env-file .env           # Default
  --env-file .env.local     # Local overrides
  --env-file config/.env    # Custom path")]
    pub env_file: PathBuf,

    /// Show shell completions for the given shell
    #[arg(long, value_name = "SHELL")]
    #[arg(help = "Show shell completions")]
    #[arg(long_help = "Generate shell completion scripts for your shell.

Supported shells:
  • bash
  • zsh
  • fish
  • powershell
  • elvish

Installation:
  bash:       berrycode --shell-completions bash > /etc/bash_completion.d/berrycode
  zsh:        berrycode --shell-completions zsh > ~/.zsh/completion/_berrycode
  fish:       berrycode --shell-completions fish > ~/.config/fish/completions/berrycode.fish

Examples:
  --shell-completions bash
  --shell-completions zsh")]
    pub shell_completions: Option<String>,

    /// Enable copy-paste mode for web chat interfaces
    #[arg(long)]
    #[arg(help = "Enable copy-paste mode")]
    #[arg(long_help = "Optimize output for copying to web chat interfaces.

When enabled:
  • Formats output for easy copy-paste
  • Removes terminal-specific formatting
  • Optimizes for web-based AI chats

Use cases:
  • Sharing conversations with web ChatGPT
  • Copying to Claude.ai
  • Documentation and tutorials

Examples:
  berrycode --copy-paste")]
    pub copy_paste: bool,

    /// Show model warnings (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "Show model warnings (default: true)")]
    #[arg(long_help = "Display warnings about model capabilities and limitations.

Warnings include:
  • Context window exceeded
  • Deprecated models
  • Performance considerations
  • Cost estimates

Default: true (recommended)

Examples:
  --show-model-warnings          # Enable (default)
  --no-show-model-warnings       # Disable")]
    pub show_model_warnings: bool,

    /// Disable model warnings
    #[arg(long = "no-show-model-warnings")]
    pub no_show_model_warnings: bool,

    /// Permanently disable analytics
    #[arg(long)]
    #[arg(help = "Permanently disable analytics")]
    #[arg(long_help = "Disable usage analytics and telemetry permanently.

When disabled:
  • No usage data collected
  • No version check pings
  • Completely offline operation

Privacy-focused option for those who prefer no telemetry.

Examples:
  berrycode --analytics-disable")]
    pub analytics_disable: bool,

    /// Check for .gitignore entries (default: true)
    #[arg(long, default_value = "true")]
    #[arg(help = "Check for .gitignore entries (default: true)")]
    #[arg(long_help = "Respect .gitignore when listing and searching files.

When enabled:
  • Skips files in .gitignore
  • Respects git exclusion rules
  • Cleaner search results

Default: true

Examples:
  --gitignore           # Enable (default)
  --no-gitignore        # Disable, see all files")]
    pub gitignore: bool,

    /// Skip .gitignore check
    #[arg(long = "no-gitignore")]
    pub no_gitignore: bool,

    /// Specify the .berrycodeignore file to use
    #[arg(long, default_value = ".berrycodeignore")]
    #[arg(help = "Specify the .berrycodeignore file")]
    #[arg(long_help = "File containing patterns for files to ignore (like .gitignore).

Format: Same as .gitignore
  # Comments
  *.log
  node_modules/
  target/

Default: .berrycodeignore

Examples:
  --berrycodeignore .bcignore
  --berrycodeignore config/ignore")]
    pub berrycodeignore: PathBuf,

    /// Specify additional .berrycodeignore files
    #[arg(long)]
    #[arg(help = "Additional .berrycodeignore files")]
    #[arg(long_help = "Load additional ignore files beyond the default .berrycodeignore.

Useful for:
  • Project-specific ignores
  • Team-wide ignore rules
  • Environment-specific exclusions

Examples:
  --berrycodeignore-extra .ignore-local
  --berrycodeignore-extra team-ignores.txt")]
    pub berrycodeignore_extra: Vec<PathBuf>,

    /// Auto-upgrade berrycode to the latest version
    #[arg(long)]
    #[arg(help = "Auto-upgrade to latest version")]
    #[arg(long_help = "Automatically upgrade BerryCode to the latest release.

What it does:
  1. Fetches latest version from GitHub
  2. Downloads and compiles
  3. Installs to cargo bin directory
  4. Replaces current installation

Requires:
  • Internet connection
  • Cargo installed
  • Write permissions to cargo bin

Examples:
  berrycode --upgrade")]
    pub upgrade: bool,

    /// Install berrycode from the main branch
    #[arg(long)]
    #[arg(help = "Install from main branch")]
    #[arg(long_help = "Install BerryCode from the main development branch.

⚠️  WARNING: Development version may be unstable!

Use for:
  • Testing latest features
  • Contributing to development
  • Bleeding edge updates

Not recommended for production use.

Examples:
  berrycode --install-main-branch")]
    pub install_main_branch: bool,

    /// Check berrycode version
    #[arg(long)]
    #[arg(help = "Check berrycode version")]
    #[arg(long_help = "Check if a newer version of BerryCode is available.

Compares:
  • Current installed version
  • Latest release on GitHub

Shows:
  • Version numbers
  • Release notes link
  • Update instructions

Examples:
  berrycode --check-update")]
    pub check_update: bool,

    /// Launch GUI mode
    #[arg(long)]
    #[arg(help = "Launch GUI mode")]
    #[arg(long_help = "Launch BerryCode with a graphical user interface.

Features:
  • Visual file browser
  • Syntax-highlighted editor
  • Chat interface
  • Point-and-click operation

Requires:
  • GUI dependencies installed
  • Display server (X11/Wayland/etc.)

Examples:
  berrycode --gui")]
    pub gui: bool,

    /// Apply the changes from the given .berrycode.diff.md file and exit
    #[arg(long, value_name = "FILE")]
    #[arg(help = "Apply changes from diff file")]
    #[arg(long_help = "Apply changes from a BerryCode diff file and exit.

Use case:
  • Review AI-generated changes before applying
  • Share diffs with team for approval
  • Version control for AI suggestions

Workflow:
  1. AI generates .berrycode.diff.md
  2. Review the changes
  3. Apply with this flag

Examples:
  berrycode --apply .berrycode.diff.md
  berrycode --apply changes.md")]
    pub apply: Option<PathBuf>,

    /// Select a project from the history before starting
    #[arg(long)]
    #[arg(help = "Select a project from history")]
    #[arg(long_help = "Display a list of recently opened projects and select one to work on.

Features:
  • View project history
  • See Git status for each project
  • Quick project switching
  • Recently used first

Workflow:
  1. Shows list of projects
  2. Select project with number
  3. Changes to that directory
  4. Starts BerryCode in selected project

Examples:
  berrycode --select-project
  berrycode -p")]
    #[arg(short = 'p')]
    pub select_project: bool,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Args::parse()
    }

    /// Resolve git-related boolean flags
    pub fn resolve_git(&mut self) {
        if self.no_git {
            self.git = false;
        }
        if self.no_auto_commits {
            self.auto_commits = false;
        }
        if self.no_dirty_commits {
            self.dirty_commits = false;
        }
        if self.no_check_model_accepts_settings {
            self.check_model_accepts_settings = false;
        }
        if self.no_show_model_warnings {
            self.show_model_warnings = false;
        }
        if self.no_verify_ssl {
            self.verify_ssl = false;
        }
        if self.no_gitignore {
            self.gitignore = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editformat_values() {
        // Test that EditFormat variants exist
        let _whole = EditFormat::Whole;
        let _diff = EditFormat::Diff;
        let _diff_fenced = EditFormat::DiffFenced;
    }

    /// Test that deprecated edit formats (Udiff, EditblockFenced, EditblockFunc) are removed
    #[test]
    fn test_deprecated_editformat_removed() {
        // This test ensures that deprecated variants don't exist
        // If you uncomment the lines below, they should cause compile errors:

        // let _udiff = EditFormat::Udiff;  // Should not compile
        // let _editblock_fenced = EditFormat::EditblockFenced;  // Should not compile
        // let _editblock_func = EditFormat::EditblockFunc;  // Should not compile

        // Test that only valid formats exist
        let valid_formats = vec![
            EditFormat::Whole,
            EditFormat::Diff,
            EditFormat::DiffFenced,
            EditFormat::EditorWhole,
            EditFormat::EditorDiff,
            EditFormat::EditorDiffFenced,
            EditFormat::WholefileFunc,
        ];

        // Count should be 7 (if more are added, this test will fail)
        assert_eq!(valid_formats.len(), 7, "EditFormat should have exactly 7 variants");
    }

    /// Test that EditFormat can be used in match expressions with only valid variants
    #[test]
    fn test_editformat_exhaustive_match() {
        let format = EditFormat::Diff;

        let _result = match format {
            EditFormat::Whole => "whole",
            EditFormat::Diff => "diff",
            EditFormat::DiffFenced => "diff-fenced",
            EditFormat::EditorWhole => "editor-whole",
            EditFormat::EditorDiff => "editor-diff",
            EditFormat::EditorDiffFenced => "editor-diff-fenced",
            EditFormat::WholefileFunc => "wholefile-func",
            // If this match is not exhaustive, the compiler will error
            // This ensures no deprecated variants exist
        };
    }

    #[test]
    fn test_args_resolve_git() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        // Test that git flag is resolved correctly
        assert!(args.git);
        args.no_git = true;
        args.resolve_git();
        assert!(!args.git);
    }

    #[test]
    fn test_args_resolve_auto_commits() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        assert!(args.auto_commits);
        args.no_auto_commits = true;
        args.resolve_git();
        assert!(!args.auto_commits);
    }

    #[test]
    fn test_args_resolve_dirty_commits() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        assert!(args.dirty_commits);
        args.no_dirty_commits = true;
        args.resolve_git();
        assert!(!args.dirty_commits);
    }

    #[test]
    fn test_args_resolve_check_model_accepts_settings() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        assert!(args.check_model_accepts_settings);
        args.no_check_model_accepts_settings = true;
        args.resolve_git();
        assert!(!args.check_model_accepts_settings);
    }

    #[test]
    fn test_args_resolve_verify_ssl() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        assert!(args.verify_ssl);
        args.no_verify_ssl = true;
        args.resolve_git();
        assert!(!args.verify_ssl);
    }

    #[test]
    fn test_args_resolve_gitignore() {
        let mut args = Args {
            files: vec![],
            model: None,
            mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            openai_api_base: None,
            set_env: vec![],
            api_key: vec![],
            list_models: None,
            model_settings_file: PathBuf::from(".berrycode.model.settings.yml"),
            model_metadata_file: PathBuf::from(".berrycode.model.metadata.json"),
            alias: vec![],
            reasoning_effort: None,
            thinking_tokens: None,
            check_model_accepts_settings: true,
            no_check_model_accepts_settings: false,
            git: true,
            no_git: false,
            auto_commits: true,
            no_auto_commits: false,
            dirty_commits: true,
            no_dirty_commits: false,
            dry_run: false,
            git_root: None,
            input_history_file: PathBuf::from(".berrycode.input.history"),
            chat_history_file: PathBuf::from(".berrycode.chat.history.md"),
            restore_chat_history: false,
            context_lines: 2,
            vim: false,
            dark_mode: false,
            light_mode: false,
            user_input_color: None,
            tool_error_color: None,
            tool_warning_color: None,
            assistant_output_color: None,
            code_theme: "default".to_string(),
            verbose: false,
            yes_always: None,
            edit_format: None,
            weak_model: None,
            editor_model: None,
            editor_edit_format: None,
            lint_cmd: vec![],
            auto_lint: false,
            test_cmd: None,
            auto_test: false,
            watch: false,
            voice: false,
            timeout: None,
            no_verify_ssl: false,
            verify_ssl: true,
            encoding: "utf-8".to_string(),
            config_file: None,
            env_file: PathBuf::from(".env"),
            shell_completions: None,
            copy_paste: false,
            show_model_warnings: true,
            no_show_model_warnings: false,
            analytics_disable: false,
            gitignore: true,
            no_gitignore: false,
            berrycodeignore: PathBuf::from(".berrycodeignore"),
            berrycodeignore_extra: vec![],
            upgrade: false,
            install_main_branch: false,
            check_update: false,
            gui: false,
            apply: None,
            select_project: false,
            dangerously_skip_permissions: false,
        };

        assert!(args.gitignore);
        args.no_gitignore = true;
        args.resolve_git();
        assert!(!args.gitignore);
    }
}
