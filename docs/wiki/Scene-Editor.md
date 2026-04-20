# Scene Editor / シーンエディタ

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

**Unity-class scene editing, built specifically for Bevy.** Edit `.scn.ron` files visually instead of as text. Inspect ECS components with type-aware editors. Preview your scene with the same renderer your game uses.

### 3D Viewport (Scene View)

| Action | Input |
|--------|-------|
| Orbit | LMB drag on empty space |
| Zoom | Scroll wheel |
| Select entity | Click (Ray vs AABB picking) |
| Gizmo | Drag axis/plane |
| Switch gizmo | `W`(Move) / `E`(Rotate) / `R`(Scale) |

- Yellow wireframe AABB on selected entity
- Transform gizmo (Red=X, Green=Y, Blue=Z)
- Grid floor, lighting preview
- Quad-view (Perspective / Front / Right / Top)
- Orthographic/Perspective toggle

### Gizmo

- **Move (W)**: 3 axis arrows + XY/YZ/XZ plane handles
- **Rotate (E)**: Rotation arcs per axis
- **Scale (R)**: Scale boxes per axis
- Axis locking, snap support, screen-space projection

### Inspector (Right Panel)

Edit selected entity properties with type-aware field editors:

- Float: drag slider | Int: number field | Bool: checkbox
- String: text input | Vec3/Vec4/Quat: multi-field | Color: color picker
- Texture: file picker + preview | Enum: dropdown | Audio: play button
- Add/Remove component buttons

### Animation

- **Timeline**: Visual keyframe editing, scrubber, loop, speed control
- **Dopesheet**: Per-property keyframes, add/delete/move, interpolation types
- **Animator**: Clip selection, playback controls, blend settings

### Visual Scripting & Shader Graph

- Node-based scripting (Blueprint-style) with node palette and connections
- Visual shader editor with real-time preview and code output

### Bevy-Specific Tools

- **System Graph** — Visualize system dependencies and execution order
- **Event Monitor** — Real-time Bevy event log with filter
- **Query Visualizer** — Show query matching and entity results
- **State Editor** — Manage Bevy states, manual transitions
- **Plugin Browser** — Search crates.io for Bevy plugins

### Scene Management

- Formats: `.bscene` (binary), `.scn.ron` (Bevy RON), GLTF/GLB export
- Multi-scene tabs with independent undo/redo
- Prefab system (create, instantiate, override)
- Play Mode (Play/Pause/Stop, speed control, live inspection)

### Additional Tools

- Terrain editor (brush tools, texture painting, heightmap)
- Skeleton/Rig editor (bone hierarchy, IK chains, weight painting)
- Navmesh generator (path visualization, agent parameters)
- Physics simulator (gravity, collision visualization)
- Particle preview (emitter visualization, lifetime editor)
- Material preview (PBR properties, real-time preview sphere)

---

<a name="japanese"></a>

## 日本語

**Bevy 専用の Unity クラスのシーン編集環境。** `.scn.ron` ファイルをテキストではなくビジュアルに編集。ECS コンポーネントを型対応エディタで検査。ゲームと同じレンダラーでシーンをプレビュー。

---

## 3D ビューポート (Scene View)

中央に表示される 3D エディタビューポート。

### 操作

| 操作 | 入力 |
|------|------|
| オービット回転 | 左ドラッグ（空白部分） |
| ズーム | スクロールホイール |
| エンティティ選択 | クリック (Ray vs AABB ピッキング) |
| ギズモ操作 | 軸/平面をドラッグ |
| ギズモモード切替 | `W`(移動) / `E`(回転) / `R`(スケール) |

### 表示

- 選択エンティティの黄色ワイヤーフレーム AABB
- トランスフォームギズモ（赤=X, 緑=Y, 青=Z）
- グリッドフロア
- ライティングプレビュー
- クアッドビュー（Perspective / Front / Right / Top）
- 正投影/透視投影の切替

---

## ギズモ (Transform Handle)

### 移動モード (W)
- 3軸の矢印ハンドル
- XY/YZ/XZ 平面ハンドル

### 回転モード (E)
- 各軸の回転アーク

### スケールモード (R)
- 各軸のスケールボックス

### 共通機能
- 軸ロック
- スナップ対応
- スクリーンスペース投影
- 透視/正射影両対応

---

## インスペクター (右パネル)

選択エンティティのプロパティ編集。

### コンポーネントエディタ

| 型 | UI |
|----|-----|
| Float | ドラッグスライダー |
| Int | 数値入力 |
| Bool | チェックボックス |
| String | テキスト入力 |
| Vec3/Vec4/Quat | 3-4 フィールド入力 |
| Color | カラーピッカー |
| Texture | ファイルピッカー + プレビュー |
| Enum | ドロップダウン |
| Audio | 再生ボタン付きプレビュー |

- コンポーネント追加ボタン（検索付きドロップダウン）
- コンポーネント削除（X アイコン）
- コンポーネントコピー/ペースト

---

## アニメーションシステム

### タイムライン
- キーフレームの視覚的編集
- 再生ヘッドのスクラブ
- ループ切替
- 速度制御

### ドープシート
- プロパティごとのキーフレーム表示
- キーフレームの追加/削除/移動
- 補間タイプ設定（リニア/ベジェ等）

### アニメーターエディタ
- アニメーションクリップ選択
- 再生コントロール（Play/Pause/Stop）
- ブレンド設定

---

## ビジュアルスクリプト

ノードベースのスクリプトエディタ（Blueprint スタイル）。

- ノードグラフキャンバス
- ノードパレット/ライブラリ
- 入力/出力ポート間の接続
- フロー制御ノード

---

## シェーダーグラフ

ノードベースのビジュアルシェーダーエディタ。

- ビルトインシェーダーノードライブラリ
- リアルタイムマテリアルプレビュー
- シェーダーコード出力表示
- PBR マテリアルプロパティ編集

---

## マテリアルプレビュー

- マテリアルプロパティ（Color, Metallic, Roughness 等）
- ライティング付きリアルタイムプレビュー球
- テクスチャプレビュー

---

## プレハブシステム

- エンティティからプレハブ作成
- シーンへのインスタンス化
- プレハブオーバーライド
- 更新追跡

---

## シーン管理

### 保存フォーマット
- `.bscene` — BerryCode バイナリフォーマット
- `.scn.ron` — Bevy RON フォーマット
- GLTF/GLB エクスポート

### マルチシーン
- タブによる複数シーン同時編集
- シーンごとの独立した Undo/Redo 履歴
- 変更マーカー（`*`）

### Undo/Redo
- コマンドパターンによる操作履歴
- 全操作が取り消し可能

---

## Bevy 固有ツール

### System Graph
- Bevy システムの依存関係を可視化
- 実行順序の確認
- ボトルネックの特定

### Event Monitor
- Bevy イベントのリアルタイムログ
- フィルタリング
- イベント詳細表示

### Query Visualizer
- ECS クエリのマッチ結果表示
- パフォーマンスメトリクス
- 最適化ヒント

### State Editor
- Bevy States の一覧表示
- 手動ステート遷移（テスト用）
- 現在のステートインジケーター

### Plugin Browser
- crates.io から Bevy プラグインを検索
- プラグイン情報表示
- ワンクリックで Cargo.toml に追加

---

## Play Mode

- Play/Pause/Stop ボタン
- 速度倍率調整
- ライブエンティティ監視（読み取り専用）
- 物理シミュレーション
- パーティクルエフェクト
- オーディオ再生

---

## その他

### テレインエディタ
- ブラシツール（上げる/下げる/スムーズ）
- テクスチャペインティング
- ハイトマップ入出力

### スケルトン/リグエディタ
- ボーン階層
- IK チェーン設定
- ウェイトペインティング

### Navmesh ジェネレーター
- シーンジオメトリからナビメッシュ生成
- パス可視化
- エージェントパラメータ設定

### 物理シミュレーター
- 重力シミュレーション
- コリジョン可視化
- リジッドボディ状態表示

### パーティクルプレビュー
- エミッター可視化
- パーティクル数表示
- 速度/ライフタイム編集
