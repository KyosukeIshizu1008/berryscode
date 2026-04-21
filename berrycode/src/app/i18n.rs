//! Internationalization (i18n) module
//!
//! Provides a `t()` macro-like function to translate UI strings
//! based on the current UiLanguage setting.

use super::types::UiLanguage;

/// Translate a UI string based on the current language.
/// English is the key, Japanese is the translation.
pub fn t(lang: UiLanguage, en: &'static str) -> &'static str {
    match lang {
        UiLanguage::English => en,
        UiLanguage::Japanese => translate_ja(en),
    }
}

fn translate_ja(en: &'static str) -> &'static str {
    match en {
        // ── Header / Top Bar ──
        "Close Project" => "プロジェクトを閉じる",
        "+ New Bevy Project" => "+ 新規 Bevy プロジェクト",
        "Run" => "実行",
        "Stop" => "停止",
        "Debug" => "デバッグ",
        "Release" => "リリース",
        "Play in Editor" => "エディタ内プレイ",
        "Build Settings" => "ビルド設定",

        // ── Editor ──
        "BerryCode Editor" => "BerryCode エディタ",
        "Select a file from the file tree" => "ファイルツリーからファイルを選択してください",
        "Project:" => "プロジェクト:",
        "Diagnostics:" => "診断:",
        "Show Completions (Ctrl+Space)" => "補完を表示 (Ctrl+Space)",

        // ── File Tree ──
        "New File" => "新規ファイル",
        "New Folder" => "新規フォルダ",
        "File name:" => "ファイル名:",
        "Folder name:" => "フォルダ名:",
        "New name:" => "新しい名前:",
        "Create" => "作成",
        "Cancel" => "キャンセル",
        "Rename" => "名前変更",
        "Rename..." => "名前変更...",
        "Delete" => "削除",
        "Copy Path" => "パスをコピー",
        "Copy" => "コピー",
        "New File Here..." => "ここに新規ファイル...",
        "New Folder Here..." => "ここに新規フォルダ...",
        "Loading..." => "読み込み中...",
        "Error:" => "エラー:",

        // ── Status Bar ──
        "File count:" => "ファイル数:",
        "Language:" => "言語:",
        "Format (Cmd+Shift+F)" => "整形 (Cmd+Shift+F)",
        "READ-ONLY" => "読み取り専用",
        "Plain Text" => "プレーンテキスト",

        // ── Search ──
        "Search" => "検索",
        "Search..." => "検索...",
        "Replace" => "置換",
        "Replace All" => "すべて置換",
        "Case Sensitive" => "大文字/小文字を区別",
        "Regex" => "正規表現",
        "Go" => "実行",

        // ── Git Panel ──
        "Git" => "Git",
        "Refresh" => "更新",
        "Branch:" => "ブランチ:",
        "Message:" => "メッセージ:",
        "Commit" => "コミット",
        "Stage All" => "すべてステージ",
        "No changes" => "変更なし",
        "Load More" => "さらに読み込む",
        "Author:" => "作者:",
        "No commits. Click Refresh to load." => "コミットがありません。更新をクリックして読み込んでください。",
        "Commit Details" => "コミット詳細",
        "Select a commit to view details" => "コミットを選択して詳細を表示",
        "Status" => "ステータス",
        "History" => "履歴",
        "Branches" => "ブランチ",
        "Remotes" => "リモート",
        "Tags" => "タグ",
        "Stash" => "スタッシュ",

        // ── Terminal ──
        "Terminal" => "ターミナル",

        // ── Settings ──
        "Settings" => "設定",
        "Appearance" => "外観",
        "Editor > Color Scheme" => "エディタ > カラースキーム",
        "Keybindings" => "キーバインド",
        "Language" => "言語設定",
        "UI Language" => "表示言語",
        "Plugins" => "プラグイン",
        "GitHub Review" => "GitHub レビュー",
        "Other Plugins" => "その他のプラグイン",
        "Coming soon..." => "近日公開...",
        "Window theme, font settings, etc." => "ウィンドウテーマ、フォント設定など",
        "Pull request review features." => "プルリクエストレビュー機能。",
        "Additional plugin configurations." => "追加プラグイン設定。",
        "Color Scheme: Darcula (Customized)" => "カラースキーム: Darcula (カスタマイズ)",
        "Customize syntax highlighting colors:" => "シンタックスハイライトの色をカスタマイズ:",
        "Keyword (fn, let, match)" => "キーワード (fn, let, match)",
        "Function / Macro" => "関数 / マクロ",
        "Type (struct, enum)" => "型 (struct, enum)",
        "String" => "文字列",
        "Number" => "数値",
        "Comment" => "コメント",
        "Macro (println!)" => "マクロ (println!)",
        "Attribute (#[derive])" => "アトリビュート (#[derive])",
        "Constant (STATIC)" => "定数 (STATIC)",
        "Lifetime ('a, 'static)" => "ライフタイム ('a, 'static)",
        "Live Preview:" => "ライブプレビュー:",
        "Reset to Darcula Defaults" => "Darcula のデフォルトに戻す",

        // ── AI Chat ──
        "AI Chat" => "AI チャット",
        "+ New" => "+ 新規",
        "Chat" => "チャット",
        "Auto" => "自動",
        "Ask anything..." => "何でも聞いてください...",
        "Ask about image..." => "画像について質問...",
        "Explain the design" => "設計を教えて",
        "Fix compile errors" => "コンパイルエラーを直して",
        "Commit changes" => "変更をコミットして",
        "Security check" => "セキュリティチェック",
        "Images can be attached via drag & drop" => "画像はドラッグ&ドロップで添付できます",
        "gRPC · berry-api-server" => "gRPC · berry-api-server",
        "thinking..." => "考え中...",
        "AI Chat - Use right panel instead." => "AI チャット - 右パネルを使用してください。",

        // ── Debugger ──
        "Continue (F5)" => "続行 (F5)",
        "Pause" => "一時停止",
        "Step Over (F10)" => "ステップオーバー (F10)",
        "Step Into (F11)" => "ステップイン (F11)",
        "Step Out (Shift+F11)" => "ステップアウト (Shift+F11)",
        "Restart (Ctrl+Shift+F5)" => "再起動 (Ctrl+Shift+F5)",
        "Stop (Shift+F5)" => "停止 (Shift+F5)",
        "Variables" => "変数",
        "Watch" => "ウォッチ",
        "Call Stack" => "コールスタック",
        "Debug Console" => "デバッグコンソール",
        "Name" => "名前",
        "Value" => "値",
        "Type" => "型",
        "Add expression..." => "式を追加...",

        // ── Scene Editor ──
        "Inspector" => "インスペクター",
        "No entity selected" => "エンティティが選択されていません",
        "Entity not found" => "エンティティが見つかりません",
        "Revert to Prefab" => "プレハブに戻す",
        "Apply to Prefab" => "プレハブに適用",
        "Unpack Prefab" => "プレハブを展開",
        "ID:" => "ID:",
        "Name:" => "名前:",
        "Position" => "位置",
        "Rotation" => "回転",
        "Scale" => "スケール",
        "World Transform (read-only)" => "ワールドトランスフォーム (読み取り専用)",
        "World Pos:" => "ワールド座標:",
        "World Rot:" => "ワールド回転:",
        "World Scale:" => "ワールドスケール:",
        "Components" => "コンポーネント",
        "Copy component" => "コンポーネントをコピー",
        "Add Component" => "コンポーネントを追加",
        "Size:" => "サイズ:",
        "Color:" => "色:",
        "Metallic:" => "メタリック:",
        "Roughness:" => "ラフネス:",
        "Emissive:" => "エミッシブ:",
        "Texture:" => "テクスチャ:",
        "Normal Map:" => "ノーマルマップ:",
        "image path" => "画像パス",
        "normal map path" => "ノーマルマップパス",

        // ── Scene Hierarchy ──
        "Profiler" => "プロファイラー",
        "Timeline" => "タイムライン",
        "Dopesheet" => "ドープシート",
        "Systems" => "システム",
        "Events" => "イベント",
        "Queries" => "クエリ",
        "States" => "ステート",
        "Export .scn.ron" => ".scn.ron にエクスポート",
        "Filter:" => "フィルター:",
        "Duplicate" => "複製",
        "Add Child (Empty)" => "子を追加 (空)",
        "Save as Prefab..." => "プレハブとして保存...",

        // ── Game View ──
        "Play" => "再生",
        "Close" => "閉じる",
        "Game View" => "ゲームビュー",
        "Game not running. Click Play to start." => "ゲームが起動していません。再生をクリックして開始。",
        "Waiting for game window..." => "ゲームウィンドウを待機中...",
        "Game not running." => "ゲームが起動していません。",
        "Click Play to run your Bevy project" => "再生をクリックしてBevyプロジェクトを実行",

        // ── Asset Browser ──
        "Asset Browser" => "アセットブラウザ",
        "Root:" => "ルート:",
        "No assets directory found. Create an 'assets/' folder in your project root." => "アセットディレクトリが見つかりません。プロジェクトルートに 'assets/' フォルダを作成してください。",
        "No assets match the current filter." => "現在のフィルターに一致するアセットがありません。",
        "Process" => "処理",
        "All" => "すべて",
        "Images" => "画像",
        "Models" => "モデル",
        "Audio" => "オーディオ",
        "Scenes" => "シーン",
        "Shaders" => "シェーダー",

        // ── ECS Inspector ──
        "ECS Inspector" => "ECS インスペクター",
        "Entities" => "エンティティ",
        "Resources" => "リソース",
        "Connect" => "接続",
        "Disconnect" => "切断",
        "Auto-refresh" => "自動更新",
        "Endpoint:" => "エンドポイント:",

        // ── Bevy Templates ──
        "Bevy Templates" => "Bevy テンプレート",
        "Component" => "コンポーネント",
        "Resource" => "リソース",
        "System" => "システム",
        "Plugin" => "プラグイン",
        "Startup System" => "スタートアップシステム",
        "Event" => "イベント",
        "State" => "ステート",
        "Insert" => "挿入",
        "Preview" => "プレビュー",
        "Field name" => "フィールド名",
        "Field type" => "フィールド型",

        // ── Visual Script / Shader Graph ──
        "Save" => "保存",
        "Add Node" => "ノードを追加",
        "On Start" => "開始時",
        "On Update" => "更新時",
        "Branch" => "分岐",
        "Print" => "出力",

        // ── Run Panel ──
        "Console" => "コンソール",
        "Clear" => "クリア",

        // ── Dock / Tool Panel ──
        "Time" => "時間",
        "Data" => "データ",
        "Scan Code" => "コードをスキャン",
        "Clear All" => "すべてクリア",

        // ── New Project Dialog ──
        "Project Name:" => "プロジェクト名:",
        "Location:" => "場所:",
        "Empty 2D" => "空の2D",
        "Empty 3D" => "空の3D",
        "3D Walker" => "3D ウォーカー",

        // ── Misc ──
        "Explorer" => "エクスプローラー",
        "Scene Editor" => "シーンエディタ",
        "Reveal in Finder" => "Finderで表示",
        "Open in Terminal" => "ターミナルで開く",

        // ── Fallback: return English ──
        _ => en,
    }
}
