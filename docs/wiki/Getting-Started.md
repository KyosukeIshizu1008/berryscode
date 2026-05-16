# Getting Started / はじめかた

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

This page is the shortest path from "I downloaded BerryCode" to "I can open, edit, run, and inspect a Bevy project."

### Install BerryCode

Use a pre-built release when possible:

- macOS: download the latest `.dmg` from GitHub Releases
- Linux: use the `.tar.gz`, Snap, Flatpak, Homebrew, or Cargo
- Windows: use the `.zip`, winget, or Cargo

Build from source:

```bash
git clone https://github.com/KyosukeIshizu1008/berryscode
cd berryscode
cargo run --bin berrycode
```

For local development on BerryCode itself, prefer:

```bash
cargo check -p berrycode
cargo fmt --all
```

### First Launch

1. Open BerryCode.
2. Open a Bevy project folder.
3. Use Explorer (`Ctrl+1`) to browse files.
4. Open Rust source files in the central editor.
5. Press the run button in the header to launch the project.

The app has two side areas:

- Left activity bar: Explorer, Search, Git, Terminal, Settings, and Bevy tools
- Right panel: AI Chat
- Bottom dock: Problems, Output, Timeline, Dopesheet, Profiler

### Recommended Bevy Workflow

1. Create or open a Bevy project.
2. Use **Bevy Templates** to generate components, resources, systems, events, and plugins.
3. Use the **Scene Editor** to author `.scn.ron` / `.bscene` scenes visually.
4. Use **Run** to launch the game from inside the IDE.
5. Use **ECS Inspector** to connect to a running app and inspect entities/resources.
6. Use **Debug & Run** tools when the game fails to build, panics, or needs profiling.

### Common Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+1` | Explorer |
| `Ctrl+2` | Search |
| `Ctrl+3` | Git |
| `Ctrl+4` | Terminal |
| `Ctrl+5` | Settings |
| `Ctrl+6` | ECS Inspector |
| `Ctrl+9` | Scene Editor |
| `Cmd+S` | Save |
| `Cmd+Space` | Completion |
| `F12` / `Cmd+Click` | Go to definition |

See [Keyboard Shortcuts](Keyboard-Shortcuts) for the complete list.

### Files BerryCode Writes

- `~/.berrycode/` — local IDE settings, snippets, caches
- `<project>/build_settings.ron` — build and packaging settings
- `<project>/mobile.toml` — cargo-mobile2 project metadata

### Troubleshooting

#### BerryCode says it is already running

On macOS BerryCode uses a lockfile at:

```text
~/Library/Caches/berrycode.lock
```

If the app crashed and no BerryCode window is open, remove that file and launch again.

#### Rust diagnostics look stale

Use `cargo check -p berrycode` or the Run/Problems panels as the source of truth. Rust-analyzer diagnostics can lag briefly after large edits.

#### Mobile toolchain is missing

Open [Mobile & DevOps](Mobile-and-DevOps), then run **Mobile Toolchain → Doctor** inside BerryCode. The Doctor report explains what is missing and which next step is actionable.

---

<a name="japanese"></a>

## 日本語

このページは「BerryCode を入れた」状態から、「Bevy プロジェクトを開く・編集する・実行する・調べる」までの最短ルートです。

### インストール

基本はビルド済みリリースを使うのがおすすめです。

- macOS: GitHub Releases から最新の `.dmg`
- Linux: `.tar.gz` / Snap / Flatpak / Homebrew / Cargo
- Windows: `.zip` / winget / Cargo

ソースから起動する場合:

```bash
git clone https://github.com/KyosukeIshizu1008/berryscode
cd berryscode
cargo run --bin berrycode
```

BerryCode 本体を開発するときは、普段の確認はこれで回します。

```bash
cargo check -p berrycode
cargo fmt --all
```

### 初回起動

1. BerryCode を開く
2. Bevy プロジェクトフォルダを開く
3. Explorer (`Ctrl+1`) でファイルを見る
4. 中央エディタで Rust ファイルを開く
5. ヘッダーの Run ボタンでプロジェクトを起動する

主な領域は次の通りです。

- 左アクティビティバー: Explorer、Search、Git、Terminal、Settings、Bevy 専用ツール
- 右パネル: AI Chat
- 下部ドック: Problems、Output、Timeline、Dopesheet、Profiler

### おすすめの Bevy ワークフロー

1. Bevy プロジェクトを作成または開く
2. **Bevy Templates** で Component / Resource / System / Event / Plugin を生成
3. **Scene Editor** で `.scn.ron` / `.bscene` をビジュアル編集
4. **Run** で IDE 内からゲームを起動
5. **ECS Inspector** で実行中アプリの Entity / Resource を調べる
6. ビルド失敗・panic・負荷調査には **Debug & Run** を使う

### よく使うショートカット

| ショートカット | 操作 |
|----------------|------|
| `Ctrl+1` | Explorer |
| `Ctrl+2` | Search |
| `Ctrl+3` | Git |
| `Ctrl+4` | Terminal |
| `Ctrl+5` | Settings |
| `Ctrl+6` | ECS Inspector |
| `Ctrl+9` | Scene Editor |
| `Cmd+S` | 保存 |
| `Cmd+Space` | 補完 |
| `F12` / `Cmd+Click` | 定義へ移動 |

完全な一覧は [Keyboard Shortcuts](Keyboard-Shortcuts) を参照してください。

### BerryCode が作るファイル

- `~/.berrycode/` — IDE 設定、スニペット、キャッシュ
- `<project>/build_settings.ron` — ビルド・パッケージング設定
- `<project>/mobile.toml` — cargo-mobile2 のプロジェクト設定

### トラブルシュート

#### すでに起動中と言われる

macOS では次の lockfile を使います。

```text
~/Library/Caches/berrycode.lock
```

クラッシュ後に BerryCode のウィンドウが残っていない場合は、このファイルを消してから起動し直してください。

#### Rust 診断が古く見える

`cargo check -p berrycode`、または Run / Problems パネルの結果を信頼してください。大きな編集直後は rust-analyzer の診断が少し遅れることがあります。

#### モバイル環境が足りない

[Mobile & DevOps](Mobile-and-DevOps) を読み、BerryCode 内の **Mobile Toolchain → Doctor** を実行してください。足りないものと次にやることが表示されます。
