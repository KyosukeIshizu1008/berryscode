# Mobile & DevOps / モバイル・DevOps

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

BerryCode v0.8 focuses on removing the toolchain friction around mobile Bevy development. The same part of the IDE also includes practical infrastructure panels such as Docker.

### Mobile Toolchain Panel

Open the Mobile Toolchain window from the header. It checks:

- Xcode version, SDKs, iOS / visionOS simulators, codesign identities
- Android SDK root, platforms, build tools, NDK, `adb`, connected devices
- Rust mobile targets: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-linux-android`, `aarch64-apple-visionos`
- cargo-mobile2 availability
- whether the current project has `mobile.toml`

Use **Refresh** when you install new SDKs, targets, or devices while BerryCode is open.

### Mobile Doctor

The **Doctor** button generates a copyable diagnostic report. Use it when a mobile run fails or a teammate needs to understand your local setup.

The report includes:

- detected Xcode / Android / Rust target state
- missing SDKs or tools
- missing `mobile.toml`
- suggested next steps

Only command rows are meant to be pasted into a terminal. Explanatory rows are shown as setup hints.

### One-Click Mobile Run

BerryCode wraps `cargo-mobile2` for the common mobile loop:

1. Probe `cargo mobile --version`.
2. Offer to install cargo-mobile2 if missing.
3. Initialize the project if `mobile.toml` does not exist.
4. Run on iOS Simulator or Android.
5. Stream logs into the integrated log panel.

Supported one-click actions:

- **Install cargo-mobile2**
- **Initialize for Mobile**
- **Run on iOS Sim**
- **Run on Android**
- **Stop**

### Manual Run on Device / Simulator

For pre-built artifacts, the run section can launch:

- iOS Simulator `.app`
- Android `.apk`
- iOS device bundles when the target path is available

Pick a target, choose an artifact, then press **Run**. Logs are classified by severity so panics, warnings, and platform messages are easier to scan.

### Required External Tools

| Platform | Required |
|----------|----------|
| iOS Simulator | macOS, Xcode, iOS simulator runtime, `aarch64-apple-ios-sim` |
| iOS Device | macOS, Xcode, Apple developer signing, `aarch64-apple-ios` |
| Android | Android Studio or command-line SDK, NDK, platform-tools / `adb`, `aarch64-linux-android` |
| visionOS | Xcode visionOS SDK, `aarch64-apple-visionos` |

Common target install:

```bash
rustup target add aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android
```

### Build Settings

Mobile packaging uses values from **File → Build Settings**:

- iOS Bundle ID
- Apple Team ID
- Android package name
- Android keystore path and key alias
- Play Console service account JSON

These settings are saved to `<project>/build_settings.ron`.

### Docker Panel

The Docker panel gives BerryCode a lightweight Docker Desktop-style view backed by the local `docker` CLI.

| Tab | Shows |
|-----|-------|
| Containers | container name, image, status, ports, actions |
| Images | repository, tag, size, created time |
| Volumes | volume name, driver, mountpoint |

Container actions:

- Start
- Stop
- Restart
- Remove
- Reload logs

If Docker is missing, BerryCode shows an install hint and a link to Docker Desktop.

### Docker Troubleshooting

#### Docker CLI not found

Install Docker Desktop, then restart BerryCode or press **Re-check** in the Docker panel.

BerryCode checks common macOS locations such as:

- `/Applications/Docker.app/Contents/Resources/bin/docker`
- `/usr/local/bin/docker`
- `/opt/homebrew/bin/docker`

#### Docker daemon not responding

Open Docker Desktop and wait until it reports that the engine is running, then press **Refresh**.

#### Logs are empty

Select a container first. Logs are loaded for the selected container and can be reloaded manually.

---

<a name="japanese"></a>

## 日本語

BerryCode v0.8 は、Bevy モバイル開発の「ツールチェーン地獄」を IDE 側で吸収することに重点を置いています。同じ流れで Docker などの実用的なインフラ系パネルも扱います。

### Mobile Toolchain パネル

ヘッダーから Mobile Toolchain ウィンドウを開きます。ここでは次を確認できます。

- Xcode バージョン、SDK、iOS / visionOS Simulator、codesign identity
- Android SDK root、platform、build tools、NDK、`adb`、接続デバイス
- Rust モバイルターゲット: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-linux-android`, `aarch64-apple-visionos`
- cargo-mobile2 の有無
- 現在のプロジェクトに `mobile.toml` があるか

BerryCode 起動中に SDK、ターゲット、デバイスを追加した場合は **Refresh** を押してください。

### Mobile Doctor

**Doctor** ボタンは、コピー可能な診断レポートを生成します。モバイル実行が失敗した時や、チームメイトにローカル環境を共有したい時に使います。

レポートには次が含まれます。

- Xcode / Android / Rust target の検出状況
- 足りない SDK やツール
- `mobile.toml` の有無
- 次にやること

ターミナルに貼る前提なのは Command 行だけです。説明文はセットアップヒントとして表示されます。

### ワンクリック Mobile Run

BerryCode は `cargo-mobile2` をラップして、よくあるモバイル実行手順を IDE 内で完結させます。

1. `cargo mobile --version` を確認
2. cargo-mobile2 がなければインストールを提案
3. `mobile.toml` がなければプロジェクトを初期化
4. iOS Simulator または Android で実行
5. ログを IDE 内のログパネルへ流す

対応アクション:

- **Install cargo-mobile2**
- **Initialize for Mobile**
- **Run on iOS Sim**
- **Run on Android**
- **Stop**

### デバイス / シミュレータへの手動実行

ビルド済み成果物を指定して実行できます。

- iOS Simulator の `.app`
- Android の `.apk`
- 対象パスがある場合の iOS device bundle

ターゲットを選び、成果物を選び、**Run** を押します。ログは重要度ごとに色分けされるため、panic、warning、プラットフォームログを追いやすくなります。

### 必要な外部ツール

| プラットフォーム | 必要なもの |
|------------------|------------|
| iOS Simulator | macOS、Xcode、iOS simulator runtime、`aarch64-apple-ios-sim` |
| iOS Device | macOS、Xcode、Apple developer signing、`aarch64-apple-ios` |
| Android | Android Studio または command-line SDK、NDK、platform-tools / `adb`、`aarch64-linux-android` |
| visionOS | Xcode visionOS SDK、`aarch64-apple-visionos` |

よく使う target 追加:

```bash
rustup target add aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android
```

### Build Settings

モバイルパッケージングでは **File → Build Settings** の値を使います。

- iOS Bundle ID
- Apple Team ID
- Android package name
- Android keystore path / key alias
- Play Console service account JSON

これらは `<project>/build_settings.ron` に保存されます。

### Docker パネル

Docker パネルは、ローカルの `docker` CLI を使う軽量な Docker Desktop 風ビューです。

| タブ | 表示内容 |
|------|----------|
| Containers | container name、image、status、ports、actions |
| Images | repository、tag、size、created time |
| Volumes | volume name、driver、mountpoint |

コンテナ操作:

- Start
- Stop
- Restart
- Remove
- Reload logs

Docker が見つからない場合は、インストールヒントと Docker Desktop へのリンクを表示します。

### Docker トラブルシュート

#### Docker CLI が見つからない

Docker Desktop をインストールし、BerryCode を再起動するか Docker パネルの **Re-check** を押してください。

BerryCode は macOS の代表的な場所も確認します。

- `/Applications/Docker.app/Contents/Resources/bin/docker`
- `/usr/local/bin/docker`
- `/opt/homebrew/bin/docker`

#### Docker daemon が応答しない

Docker Desktop を開き、engine が起動するまで待ってから **Refresh** を押してください。

#### ログが空

先にコンテナを選択してください。選択中コンテナのログが読み込まれ、手動で Reload できます。
