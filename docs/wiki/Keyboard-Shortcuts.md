# Keyboard Shortcuts / キーボードショートカット

[English](#english) | [日本語](#japanese)

---

<a name="english"></a>

## English

### Panel Switch

| Shortcut | Panel |
|----------|-------|
| `Ctrl+1` | Explorer |
| `Ctrl+2` | Search |
| `Ctrl+3` | Git |
| `Ctrl+4` | Terminal |
| `Ctrl+5` | Settings |
| `Ctrl+6` | ECS Inspector |
| `Ctrl+7` | Bevy Templates |
| `Ctrl+8` | Asset Browser |
| `Ctrl+9` | Scene Editor |

### Editor

| Shortcut | Action |
|----------|--------|
| `Cmd+S` | Save |
| `Cmd+Z` | Undo |
| `Cmd+Shift+Z` | Redo |
| `Cmd+Shift+D` | Duplicate line |
| `Cmd+F` | Find |
| `Cmd+H` | Replace |
| `Cmd+Shift+F` | Format (rustfmt) |
| `Cmd+Space` | LSP completions |
| `Ctrl+Shift+M` | Macro expand |

### Code Navigation

| Shortcut | Action |
|----------|--------|
| `F12` | Go to definition |
| `Cmd+Click` | Go to definition |
| `Shift+F12` | Find references |

### Scene Editor

| Shortcut | Action |
|----------|--------|
| `W` | Gizmo: Move |
| `E` | Gizmo: Rotate |
| `R` | Gizmo: Scale |
| `Delete` | Delete entity |
| `Cmd+D` | Duplicate entity |

### Debug

| Shortcut | Action |
|----------|--------|
| `F5` | Start/Continue debug |
| `F9` | Toggle breakpoint |
| `F10` | Step over |
| `F11` | Step into |
| `Shift+F11` | Step out |

> Note: macOS uses `Cmd`, Linux/Windows uses `Ctrl`. Keybindings are customizable in Settings.

---

<a name="japanese"></a>

## 日本語

## パネル切替

| ショートカット | パネル |
|--------------|--------|
| `Ctrl+1` | Explorer |
| `Ctrl+2` | Search |
| `Ctrl+3` | Git |
| `Ctrl+4` | Terminal |
| `Ctrl+5` | Settings |
| `Ctrl+6` | ECS Inspector |
| `Ctrl+7` | Bevy Templates |
| `Ctrl+8` | Asset Browser |
| `Ctrl+9` | Scene Editor |

---

## エディタ操作

| ショートカット | 動作 |
|--------------|------|
| `Cmd+S` | 保存 |
| `Cmd+Z` | 元に戻す |
| `Cmd+Shift+Z` | やり直し |
| `Cmd+Shift+D` | 行を複製 |
| `Cmd+F` | 検索ダイアログ |
| `Cmd+H` | 置換ダイアログ |
| `Cmd+Shift+F` | フォーマット (rustfmt) |
| `Cmd+Space` | LSP 補完 |
| `Ctrl+Shift+M` | マクロ展開 |

---

## コードナビゲーション

| ショートカット | 動作 |
|--------------|------|
| `F12` | 定義に移動 |
| `Cmd+Click` | 定義に移動 |
| `Shift+F12` | 参照を検索 |
| `Ctrl+Shift+P` | コマンドパレット |

---

## シーンエディタ

| ショートカット | 動作 |
|--------------|------|
| `W` | ギズモ: 移動モード |
| `E` | ギズモ: 回転モード |
| `R` | ギズモ: スケールモード |
| `Delete` | エンティティ削除 |
| `Cmd+D` | エンティティ複製 |

---

## デバッグ

| ショートカット | 動作 |
|--------------|------|
| `F5` | デバッグ開始/続行 |
| `F9` | ブレークポイントトグル |
| `F10` | ステップオーバー |
| `F11` | ステップイン |
| `Shift+F11` | ステップアウト |

---

## Vim Mode (有効時)

### Normal モード

| キー | 動作 |
|------|------|
| `h/j/k/l` | 左/下/上/右 |
| `w/b/e` | ワード移動 |
| `0` / `$` | 行頭/行末 |
| `gg` / `G` | ファイル先頭/末尾 |
| `f`+char / `F`+char | 文字検索 (前方/後方) |
| `%` | 対応するカッコへ |
| `{` / `}` | 段落移動 |
| `dd` | 行削除 |
| `yy` | 行コピー |
| `p` | ペースト |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `.` | 前の操作を繰り返し |
| `/` / `?` | 検索 (前方/後方) |
| `n` / `N` | 次/前の検索結果 |

### オペレータ

| オペレータ | 動作 |
|-----------|------|
| `d` | 削除 |
| `c` | 変更 (削除+Insert) |
| `y` | コピー |
| `>` | インデント |
| `<` | デデント |
| `~` | 大文字/小文字切替 |

### テキストオブジェクト

| オブジェクト | 範囲 |
|-------------|------|
| `iw` / `aw` | ワード |
| `i"` / `a"` | ダブルクォート内 |
| `i(` / `a(` | カッコ内 |
| `i{` / `a{` | 波カッコ内 |
| `i[` / `a[` | 角カッコ内 |

### モード切替

| キー | 遷移先 |
|------|--------|
| `i` | Insert (カーソル前) |
| `a` | Insert (カーソル後) |
| `o` | Insert (新しい行) |
| `v` | Visual (文字) |
| `V` | Visual (行) |
| `Ctrl+V` | Visual (矩形) |
| `:` | Command |
| `Esc` | Normal に戻る |

---

## 注意

- macOS では `Cmd` キー、Linux/Windows では `Ctrl` キーを使用
- キーバインドは Settings パネルからカスタマイズ可能
