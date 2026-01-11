//! Canvas Rendering Engine
//!
//! このモジュールだけがweb-sysの直接使用を許可されています。
//! 全てのCanvas描画操作はここに集約します。

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use crate::theme::{EditorTheme, RUSTROVER_DARCULA};

/// Dirty region tracking for optimized rendering
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyRegion {
    pub min_line: usize,
    pub max_line: usize,
    pub flags: DirtyFlags,
}

/// Flags indicating what parts need redrawing
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyFlags {
    bits: u8,
}

impl DirtyFlags {
    pub const NONE: u8 = 0b0000;
    pub const TEXT: u8 = 0b0001;
    pub const CURSOR: u8 = 0b0010;
    pub const SELECTION: u8 = 0b0100;
    pub const GUTTER: u8 = 0b1000;
    pub const ALL: u8 = 0b1111;

    pub fn new() -> Self {
        Self { bits: Self::NONE }
    }

    pub fn all() -> Self {
        Self { bits: Self::ALL }
    }

    pub fn with_text(mut self) -> Self {
        self.bits |= Self::TEXT;
        self
    }

    pub fn with_cursor(mut self) -> Self {
        self.bits |= Self::CURSOR;
        self
    }

    pub fn with_selection(mut self) -> Self {
        self.bits |= Self::SELECTION;
        self
    }

    pub fn with_gutter(mut self) -> Self {
        self.bits |= Self::GUTTER;
        self
    }

    pub fn has_text(&self) -> bool {
        self.bits & Self::TEXT != 0
    }

    pub fn has_cursor(&self) -> bool {
        self.bits & Self::CURSOR != 0
    }

    pub fn has_selection(&self) -> bool {
        self.bits & Self::SELECTION != 0
    }

    pub fn has_gutter(&self) -> bool {
        self.bits & Self::GUTTER != 0
    }
}

/// Git diff status for a line (IntelliJ-style gutter indicators)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitLineStatus {
    Unmodified,
    Added,     // Green bar - newly added line
    Modified,  // Yellow bar - modified line
    Deleted,   // Red bar - deleted line marker
}

/// Cursor animation for smooth movement (IntelliJ-style)
#[derive(Debug, Clone)]
pub struct CursorAnimation {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub start_time: f64,
    pub duration: f64, // milliseconds
}

impl CursorAnimation {
    pub fn new(from: (f64, f64), to: (f64, f64), now: f64) -> Self {
        Self {
            from,
            to,
            start_time: now,
            duration: 100.0, // 100ms IntelliJ-style animation
        }
    }

    /// Ease-out quadratic easing function (smooth deceleration)
    pub fn ease_out_quad(t: f64) -> f64 {
        t * (2.0 - t)
    }

    /// Get current interpolated position
    pub fn current_position(&self, now: f64) -> (f64, f64) {
        let elapsed = now - self.start_time;
        if elapsed >= self.duration {
            return self.to;
        }

        let t = (elapsed / self.duration).min(1.0).max(0.0);
        let eased = Self::ease_out_quad(t);

        let x = self.from.0 + (self.to.0 - self.from.0) * eased;
        let y = self.from.1 + (self.to.1 - self.from.1) * eased;
        (x, y)
    }

    /// Check if animation is finished
    pub fn is_finished(&self, now: f64) -> bool {
        now - self.start_time >= self.duration
    }
}

/// IntelliJ Darculaカラースキーム (Backward compatibility)
pub const COLOR_BACKGROUND: &str = "#1E1F22";  // Editor background (pixel-perfect)
pub const COLOR_FOREGROUND: &str = "#BCBEC4";  // Default text (pixel-perfect)
pub const COLOR_CURSOR: &str = "#BBBBBB";      // Caret
pub const COLOR_SELECTION: &str = "#214283";   // Selection
pub const COLOR_GUTTER_BG: &str = "#313335";   // Gutter background
pub const COLOR_GUTTER_FG: &str = "#4B5059";   // Line numbers (pixel-perfect)
pub const COLOR_LINE_HIGHLIGHT: &str = "#26282E"; // Current line (pixel-perfect)

/// フォント設定
pub const FONT_FAMILY: &str = "JetBrains Mono";
pub const FONT_SIZE: f64 = 13.0;  // RustRover actual size (smaller and crisper)
pub const LINE_HEIGHT: f64 = 20.0; // RustRover standard line height
pub const LETTER_SPACING: f64 = 0.0; // No extra spacing for sharp rendering

/// トークンの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,        // fn, pub, struct, let, mut (orange)
    KeywordImport,  // use, mod (blue)
    FunctionDef,    // function definition names (yellow)
    FunctionCall,   // function calls identifier() (bright blue)
    Type,           // String, usize, custom types (purple-pink)
    Module,         // module/crate names identifier:: (tan/orange)
    Identifier,     // variable/field names (white)
    String,         // string literals (green)
    Number,         // numeric literals (cyan)
    Comment,        // comments (gray)
    DocComment,     // /// doc comments (dark green)
    Attribute,      // #[derive] (yellow)
    Macro,          // println!, vec! (blue)
    Constant,       // CONSTANTS (purple)
    Punctuation,    // symbols, operators (white)
    HtmlTag,        // HTML tags <div> (orange)
    HtmlAttribute,  // HTML attributes class="..." (cyan)
    CssSelector,    // CSS selectors .class, #id (yellow)
    CssProperty,    // CSS properties color, margin (orange)
}

/// シンタックストークン
#[derive(Debug, Clone)]
struct SyntaxToken {
    text: String,
    kind: TokenKind,
}

/// Convert TokenKind to string representation for caching
fn token_kind_to_str(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Keyword => "keyword",
        TokenKind::KeywordImport => "keyword_import",
        TokenKind::FunctionDef => "function_def",
        TokenKind::FunctionCall => "function_call",
        TokenKind::Type => "type",
        TokenKind::Module => "module",
        TokenKind::Identifier => "identifier",
        TokenKind::String => "string",
        TokenKind::Number => "number",
        TokenKind::Comment => "comment",
        TokenKind::DocComment => "doc_comment",
        TokenKind::Attribute => "attribute",
        TokenKind::Macro => "macro",
        TokenKind::Constant => "constant",
        TokenKind::Punctuation => "punctuation",
        TokenKind::HtmlTag => "html_tag",
        TokenKind::HtmlAttribute => "html_attribute",
        TokenKind::CssSelector => "css_selector",
        TokenKind::CssProperty => "css_property",
    }.to_string()
}

/// Convert string representation back to TokenKind
fn token_kind_from_str(s: &str) -> TokenKind {
    match s {
        "keyword" => TokenKind::Keyword,
        "keyword_import" => TokenKind::KeywordImport,
        "function_def" => TokenKind::FunctionDef,
        "function_call" => TokenKind::FunctionCall,
        "type" => TokenKind::Type,
        "module" => TokenKind::Module,
        "identifier" => TokenKind::Identifier,
        "string" => TokenKind::String,
        "number" => TokenKind::Number,
        "comment" => TokenKind::Comment,
        "doc_comment" => TokenKind::DocComment,
        "attribute" => TokenKind::Attribute,
        "macro" => TokenKind::Macro,
        "constant" => TokenKind::Constant,
        "punctuation" => TokenKind::Punctuation,
        "html_tag" => TokenKind::HtmlTag,
        "html_attribute" => TokenKind::HtmlAttribute,
        "css_selector" => TokenKind::CssSelector,
        "css_property" => TokenKind::CssProperty,
        _ => TokenKind::Identifier, // Fallback for unknown kinds
    }
}

/// Canvas描画エンジン
pub struct CanvasRenderer {
    context: CanvasRenderingContext2d,
    char_width_ascii: f64,
    char_width_wide: f64,
    line_height: f64,
    gutter_width: f64,
}

impl CanvasRenderer {
    /// Canvas要素から描画エンジンを作成
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        // ✅ Canvasコンテキストオプション設定（ネイティブアプリの質感に近づける）
        use wasm_bindgen::JsValue;
        let context_options = js_sys::Object::new();

        // alpha: false - 背景が透けないことを明示してテキスト品質向上
        let _ = js_sys::Reflect::set(
            &context_options,
            &JsValue::from_str("alpha"),
            &JsValue::from_bool(false)
        );

        // desynchronized: true - 低遅延モードでカーソルの動きをキビキビと
        let _ = js_sys::Reflect::set(
            &context_options,
            &JsValue::from_str("desynchronized"),
            &JsValue::from_bool(true)
        );

        let context = canvas
            .get_context_with_context_options("2d", &context_options)
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("2d context is None")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        // ✅ Rust側の役割: 内部解像度をDPRに合わせて「にじみ」を完全に消す
        // devicePixelRatioを取得（Macなら通常 2.0、Windows HDPIなら 1.5 など）
        let window = web_sys::window().ok_or("no global window")?;
        let dpr = window.device_pixel_ratio();

        #[cfg(debug_assertions)]
        web_sys::console::log_1(&format!("🎨 CanvasRenderer: DPR = {}, applying transform", dpr).into());

        // ✅ setTransform() で描画コンテキストをスケール
        // これが「黄金の組み合わせ」の核心部分：
        // 1. Tailwind (CSS) が「表示サイズ」を制御 → 例: 1000px × 600px
        // 2. virtual_editor.rs が「内部バッファ」を DPR 倍に設定 → 例: 2000 × 1200 ピクセル
        // 3. この set_transform(dpr, dpr) が「描画座標系」を調整
        //    → Rust側は論理座標（1000×600）で描画するだけで、自動的に高解像度になる
        //
        // 結果: ソースコードの文字が「クッキリ」表示される！
        context
            .set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)
            .map_err(|_| "Failed to set transform")?;

        // フォント品質設定（DPR適用後に設定）
        // Weight 300 (Light) - RustRoverの見本に合わせた軽量なフォント
        let font_string = format!("300 {}px '{}'", FONT_SIZE, FONT_FAMILY);

        #[cfg(debug_assertions)]
        web_sys::console::log_1(&format!("🎨 CanvasRenderer::new() - Setting font: {}", font_string).into());

        context.set_font(&font_string);

        // ✅ 設定直後にフォントを読み取って確認
        #[cfg(debug_assertions)]
        {
            let actual_font = context.font();
            web_sys::console::log_1(&format!("🎨 CanvasRenderer::new() - Font after set_font(): {}", actual_font).into());
        }

        // 高品質なテキストレンダリングを有効化
        context.set_image_smoothing_enabled(false); // Disable for sharper edges
        context.set_text_baseline("alphabetic");

        // テキストレンダリング品質の最適化
        // optimizeSpeed: エディタではピクセル整合性と描画速度を優先
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("fontKerning"),
            &JsValue::from_str("normal")
        );
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("textRendering"),
            &JsValue::from_str("optimizeSpeed")
        );

        // Letter spacing for ultra-crisp rendering
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("letterSpacing"),
            &JsValue::from_str(&format!("{}px", LETTER_SPACING))
        );

        // macOS/WebKit向けフォントスムージング最適化
        let _ = js_sys::Reflect::set(
            &context,
            &JsValue::from_str("imageSmoothingQuality"),
            &JsValue::from_str("high")
        );

        // 文字幅を実測
        let char_width_ascii = context
            .measure_text("M")
            .map_err(|_| "Failed to measure ASCII char")?
            .width();

        let char_width_wide = context
            .measure_text("あ")
            .map_err(|_| "Failed to measure wide char")?
            .width();

        Ok(Self {
            context,
            char_width_ascii,
            char_width_wide,
            line_height: LINE_HEIGHT,
            gutter_width: 55.0,
        })
    }

    /// ASCII文字幅を取得
    pub fn char_width_ascii(&self) -> f64 {
        self.char_width_ascii
    }

    /// 全角文字幅を取得
    pub fn char_width_wide(&self) -> f64 {
        self.char_width_wide
    }

    /// 行の高さを取得
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// ガター幅を取得
    pub fn gutter_width(&self) -> f64 {
        self.gutter_width
    }

    /// Canvas全体をクリア
    pub fn clear(&self, width: f64, height: f64) {
        let theme = EditorTheme::current();
        self.context.set_fill_style(&theme.bg_editor.into());
        self.context.fill_rect(0.0, 0.0, width, height);
    }

    /// 🚀 PERFORMANCE: Clear only a specific rectangular region
    pub fn clear_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        let theme = EditorTheme::current();
        self.context.set_fill_style(&theme.bg_editor.into());
        self.context.fill_rect(x, y, width, height);
    }

    /// 🚀 PERFORMANCE: Clear only specific lines (for text updates)
    pub fn clear_lines(&self, start_line: usize, end_line: usize, scroll_top: f64, canvas_width: f64) {
        let y_start = start_line as f64 * self.line_height - scroll_top;
        let height = (end_line - start_line) as f64 * self.line_height;

        // Clear from end of gutter to right edge
        self.clear_rect(self.gutter_width, y_start, canvas_width - self.gutter_width, height);
    }

    /// 🚀 PERFORMANCE: Clear cursor region only (very small rectangle)
    pub fn clear_cursor_region(&self, line: usize, col: usize, scroll_top: f64, line_text: &str) {
        // ✅ IntelliJ風: 余白を20.0pxに統一
        let x = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(line_text, col)).round();
        let y = (line as f64 * self.line_height - scroll_top).round();

        // Clear a small rectangle around cursor (4px wide to account for 2px line width + margins)
        self.clear_rect(x - 2.0, y, 4.0, self.line_height);
    }

    /// 行番号ガターを描画 (IntelliJ Darcula exact reproduction)
    pub fn draw_gutter(&self, start_line: usize, end_line: usize, height: f64) {
        // ✅ IntelliJ風: ガター背景（本体より少し暗い #313335）
        self.context.set_fill_style(&"#313335".into());
        self.context.fill_rect(0.0, 0.0, self.gutter_width, height);

        // ✅ IntelliJ風: 境界線を1px描画するだけで引き締まる（#444444）
        self.context.set_stroke_style(&"#444444".into());
        self.context.set_line_width(1.0);
        self.context.begin_path();
        self.context.move_to(self.gutter_width, 0.0);
        self.context.line_to(self.gutter_width, height);
        self.context.stroke();

        // 行番号
        self.context.set_fill_style(&COLOR_GUTTER_FG.into());
        self.context.set_text_align("right");

        for line_num in start_line..end_line {
            // ✅ IntelliJ風: ベースラインを微調整（LINE_HEIGHT * 0.8）
            let y = ((line_num - start_line) as f64 * self.line_height + self.line_height * 0.8).round();
            let _ = self.context.fill_text(
                &(line_num + 1).to_string(),
                self.gutter_width - 10.0,
                y,
            );
        }

        self.context.set_text_align("left");
    }

    /// 🚀 ABSOLUTE BEAUTY: ガターとエディタの間に微細な影を落とし、階層を作る
    ///
    /// この影により、ガターが「手前」に、エディタ本体が「奥」にあるような
    /// 奥行き感（Elevation）が生まれ、高級時計のような質感を実現します。
    pub fn draw_elevation_shadow(&self, height: f64) {
        // ガターの右端に微細な影を落とす（シンプル版）
        // TODO: LinearGradient API が安定したらグラデーション版に戻す

        // 4段階の透明度でグラデーション効果を模倣
        let shadow_steps = [
            (0.0, "rgba(0,0,0,0.2)"),
            (1.0, "rgba(0,0,0,0.15)"),
            (2.0, "rgba(0,0,0,0.08)"),
            (3.0, "rgba(0,0,0,0.02)"),
        ];

        for (offset, color) in shadow_steps.iter() {
            self.context.set_fill_style(&(*color).into());
            self.context.fill_rect(self.gutter_width + offset, 0.0, 1.0, height);
        }
    }

    /// 🚀 ABSOLUTE BEAUTY: 行番号のスタイルを洗練させる
    ///
    /// IntelliJの美しさの秘密は「情報の階層化」にあります：
    /// - アクティブ行の行番号は明るく、太字で
    /// - 非アクティブ行は小さく、半透明で控えめに
    /// - 背景には微細なグラデーションで「質感」を加える
    pub fn draw_refined_gutter(&self, start_line: usize, end_line: usize, active_line: usize, height: f64) {
        // 🎨 背景（グラデーション効果を持つ単色）
        // TODO: LinearGradient API が安定したらグラデーション版に戻す
        // 現在は中間色を使用
        self.context.set_fill_style(&"#2F3032".into());
        self.context.fill_rect(0.0, 0.0, self.gutter_width, height);

        // 🎨 境界線（右側のセパレーター）
        self.context.set_stroke_style(&"#444444".into());
        self.context.set_line_width(1.0);
        self.context.begin_path();
        self.context.move_to(self.gutter_width, 0.0);
        self.context.line_to(self.gutter_width, height);
        self.context.stroke();

        // 🎨 行番号の描画
        self.context.set_text_align("right");

        for line_num in start_line..end_line {
            let is_active = line_num == active_line;
            let y = ((line_num - start_line) as f64 * self.line_height + self.line_height * 0.8).round();

            // アクティブ行 vs 非アクティブ行で視覚的に差別化
            if is_active {
                // 🚀 RustRover: アクティブ行も Light (300) で統一、シャープで美しく
                self.context.set_fill_style(&"#A9B7C6".into());
                let _ = self.context.set_font("300 12px 'JetBrains Mono'");
            } else {
                // 🚀 RustRover: 非アクティブ行も Light (300)、半透明
                self.context.set_fill_style(&"rgba(96, 103, 112, 0.5)".into());
                let _ = self.context.set_font("300 11px 'JetBrains Mono'");
            }

            let _ = self.context.fill_text(
                &(line_num + 1).to_string(),
                self.gutter_width - 12.0,
                y
            );
        }

        self.context.set_text_align("left");

        // 🚀 RustRover: 行番号描画後、エディタのメインフォントに戻す
        let _ = self.context.set_font(&format!("300 {}px '{}'", FONT_SIZE, FONT_FAMILY));

        // 🎨 最後に影を重ねて奥行きを出す
        self.draw_elevation_shadow(height);
    }

    /// Draw Git diff indicators in the gutter (IntelliJ-style colored bars)
    ///
    /// # Arguments
    /// * `git_status` - Closure that returns GitLineStatus for a given line number
    /// * `start_line` - First visible line
    /// * `end_line` - Last visible line
    /// * `scroll_top` - Current scroll position
    pub fn draw_git_diff_indicators<F>(&self, git_status: F, start_line: usize, end_line: usize, scroll_top: f64)
    where
        F: Fn(usize) -> GitLineStatus,
    {
        const INDICATOR_WIDTH: f64 = 3.0; // 3px wide colored bar
        const INDICATOR_X: f64 = 3.0;     // 3px from left edge

        for line_num in start_line..end_line {
            let status = git_status(line_num);

            if status == GitLineStatus::Unmodified {
                continue; // No indicator for unmodified lines
            }

            // IntelliJ-style colors
            let color = match status {
                GitLineStatus::Added => "#6A8759",    // IntelliJ green for additions
                GitLineStatus::Modified => "#CA8435", // IntelliJ orange/yellow for modifications
                GitLineStatus::Deleted => "#BC3F3C",  // IntelliJ red for deletions
                GitLineStatus::Unmodified => continue,
            };

            let y = ((line_num - start_line) as f64 * self.line_height - scroll_top).round();

            self.context.set_fill_style(&color.into());
            self.context.fill_rect(
                INDICATOR_X,
                y,
                INDICATOR_WIDTH,
                self.line_height
            );
        }
    }

    /// 🚀 NEW: アクティブ行のハイライト背景を描画
    pub fn draw_active_line_highlight(&self, line: usize, scroll_top: f64, canvas_width: f64) {
        let y = (line as f64 * self.line_height - scroll_top).round();

        // IntelliJ Darculaのアクティブ行背景色（本体よりわずかに明るい #323232）
        self.context.set_fill_style(&COLOR_LINE_HIGHLIGHT.into());
        // ガターの右端から画面右端まで塗る
        self.context.fill_rect(self.gutter_width + 1.0, y, canvas_width - self.gutter_width, self.line_height);
    }

    /// 🚀 NEW: インデントガイドを描画（コード構造の可視化）
    pub fn draw_indent_guides(&self, line_text: &str, y_offset: f64) {
        // 行頭の空白文字をカウント
        let space_count = line_text.chars().take_while(|c| *c == ' ').count();
        let tab_size = 4; // TODO: 設定から取得

        if space_count < tab_size {
            return; // インデントがない場合は描画しない
        }

        // 非常に控えめな色（エディタ背景より少し明るい）
        self.context.set_stroke_style(&"#373737".into());
        self.context.set_line_width(1.0);

        // タブサイズごとに垂直線を描画
        for i in (tab_size..=space_count).step_by(tab_size) {
            let x = (self.gutter_width + 20.0 + (i as f64 * self.char_width_ascii)).round();
            self.context.begin_path();
            self.context.move_to(x, y_offset);
            self.context.line_to(x, y_offset + self.line_height);
            self.context.stroke();
        }
    }

    /// テキスト行を描画 (IntelliJ-style baseline and spacing)
    pub fn draw_line(&self, _line_num: usize, y_offset: f64, text: &str, color: &str) {
        // 🚀 RUSTROVER: 300(Light) を指定することで、暗い背景でも文字が太らずクッキリします
        let _ = self.context.set_font("300 13px 'JetBrains Mono'");

        // ✅ IntelliJ風: Gutterとテキストの間に明確なセパレータ空間を作る（20.0px）
        let x = (self.gutter_width + 20.0).round();

        // ✅ IntelliJ風: フォントサイズに合わせてベースラインを微調整（LINE_HEIGHT * 0.8）
        let y = (y_offset + self.line_height * 0.8).round();

        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x, y);
    }

    /// シンタックスハイライト付きでテキスト行を描画
    /// 🚀 PERFORMANCE: Token cache to avoid re-tokenizing every frame (60 FPS!)
    /// language: Some("rust") の場合はRustトークナイザーを使用、Noneの場合は単色
    /// 🎨 NEW: インデントガイドも描画
    pub fn draw_line_highlighted(&self, buffer: &mut crate::buffer::TextBuffer, line_idx: usize, y_offset: f64, text: &str, theme: &EditorTheme, language: Option<&str>) {
        // 🚀 NEW: インデントガイドを最初に描画（テキストの下になる）
        self.draw_indent_guides(text, y_offset);

        // ✅ IntelliJ風: ピクセルグリッドに合わせて整数に丸める（シャープなレンダリング）
        let x_base = (self.gutter_width + 20.0).round();
        let y = (y_offset + self.line_height * 0.8).round();

        // 言語が指定されていない場合は単色で描画
        if language.is_none() {
            self.context.set_fill_style(&COLOR_FOREGROUND.into());
            let _ = self.context.fill_text(text, x_base, y);
            return;
        }

        // 🚀 PERFORMANCE: Check token cache first
        let tokens = if let Some(cached_tokens) = buffer.get_cached_tokens(line_idx) {
            // Cache hit! Convert cached representation to SyntaxToken
            cached_tokens.iter().map(|(text, kind_str)| {
                SyntaxToken {
                    text: text.clone(),
                    kind: token_kind_from_str(kind_str),
                }
            }).collect()
        } else {
            // Cache miss - tokenize and cache result
            let tokens = match language {
                Some("rust") => self.tokenize_rust(text),
                Some("javascript" | "js" | "typescript" | "ts") => self.tokenize_javascript(text),
                Some("html" | "htm") => self.tokenize_html(text),
                Some("css") => self.tokenize_css(text),
                _ => {
                    // サポートされていない言語は単色で描画
                    self.context.set_fill_style(&COLOR_FOREGROUND.into());
                    let _ = self.context.fill_text(text, x_base, y);
                    return;
                }
            };

            // Store in cache for next frame
            let cache_repr: Vec<(String, String)> = tokens.iter().map(|t| {
                (t.text.clone(), token_kind_to_str(&t.kind))
            }).collect();
            buffer.cache_tokens(line_idx, cache_repr);

            tokens
        };

        let mut x_offset = 0.0;

        for token in tokens {
            let color = match token.kind {
                TokenKind::Keyword => theme.syntax_keyword,
                TokenKind::KeywordImport => theme.syntax_keyword_import,
                TokenKind::FunctionDef => theme.syntax_function_def,
                TokenKind::FunctionCall => theme.syntax_function_call,
                TokenKind::Type => theme.syntax_type,
                TokenKind::Module => theme.syntax_module,
                TokenKind::Identifier => theme.syntax_identifier,
                TokenKind::String => theme.syntax_string,
                TokenKind::Number => theme.syntax_number,
                TokenKind::Comment => theme.syntax_comment,
                TokenKind::DocComment => theme.syntax_doc_comment,
                TokenKind::Attribute => theme.syntax_attribute,
                TokenKind::Macro => theme.syntax_macro,
                TokenKind::Constant => theme.syntax_constant,
                TokenKind::Punctuation => theme.syntax_identifier,
                TokenKind::HtmlTag => theme.syntax_keyword,        // Orange for HTML tags
                TokenKind::HtmlAttribute => theme.syntax_number,   // Cyan for attributes
                TokenKind::CssSelector => theme.syntax_function_def, // Yellow for CSS selectors
                TokenKind::CssProperty => theme.syntax_keyword,    // Orange for CSS properties
            };

            // 🎨 ABSOLUTE BEAUTY: キーワード・関数に微細なグロー効果を追加
            // 暗いテーマでシンタックスハイライトが「浮かび上がる」ような美しさを実現
            let should_glow = matches!(
                token.kind,
                TokenKind::Keyword | TokenKind::KeywordImport | TokenKind::FunctionDef |
                TokenKind::FunctionCall | TokenKind::Type | TokenKind::Macro
            );

            if should_glow {
                // グロー効果のための設定
                self.context.set_shadow_blur(2.0);
                self.context.set_shadow_color(color);
                // shadowOffsetX/Y は 0 のまま（テキストの真下に発光）
            }

            self.context.set_fill_style(&color.into());
            // X座標も整数に丸める
            let _ = self.context.fill_text(&token.text, (x_base + x_offset).round(), y);

            // グロー効果をリセット（次のトークンに影響しないように）
            if should_glow {
                self.context.set_shadow_blur(0.0);
            }

            // 次のトークンの位置を計算
            x_offset += self.measure_text(&token.text);
        }
    }

    /// Rustコードをトークンに分解
    fn tokenize_rust(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = line.chars().collect();
        let mut prev_token_was_fn = false;

        while current_pos < chars.len() {
            // Safety: Store position to detect infinite loop
            let pos_before = current_pos;
            // ドキュメントコメント ///
            if current_pos + 2 < chars.len()
                && chars[current_pos] == '/'
                && chars[current_pos + 1] == '/'
                && chars[current_pos + 2] == '/' {
                let comment: String = chars[current_pos..].iter().collect();
                tokens.push(SyntaxToken {
                    text: comment,
                    kind: TokenKind::DocComment,
                });
                break;
            }

            // 通常のコメント //
            if current_pos + 1 < chars.len() && chars[current_pos] == '/' && chars[current_pos + 1] == '/' {
                let comment: String = chars[current_pos..].iter().collect();
                tokens.push(SyntaxToken {
                    text: comment,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // 文字列リテラル
            if chars[current_pos] == '"' {
                let mut end = current_pos + 1;
                while end < chars.len() && chars[end] != '"' {
                    if chars[end] == '\\' && end + 1 < chars.len() {
                        end += 2;
                    } else {
                        end += 1;
                    }
                }
                if end < chars.len() {
                    end += 1;
                }
                let string_lit: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: string_lit,
                    kind: TokenKind::String,
                });
                current_pos = end;
                continue;
            }

            // 属性
            if chars[current_pos] == '#' && current_pos + 1 < chars.len() && chars[current_pos + 1] == '[' {
                let mut end = current_pos + 2;
                let mut bracket_count = 1;
                while end < chars.len() && bracket_count > 0 {
                    if chars[end] == '[' {
                        bracket_count += 1;
                    } else if chars[end] == ']' {
                        bracket_count -= 1;
                    }
                    end += 1;
                }
                let attr: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: attr,
                    kind: TokenKind::Attribute,
                });
                current_pos = end;
                continue;
            }

            // 数値
            if chars[current_pos].is_ascii_digit() {
                let mut end = current_pos;
                while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.' || chars[end] == '_') {
                    end += 1;
                }
                let number: String = chars[current_pos..end].iter().collect();
                tokens.push(SyntaxToken {
                    text: number,
                    kind: TokenKind::Number,
                });
                current_pos = end;
                continue;
            }

            // 識別子/キーワード
            if chars[current_pos].is_alphabetic() || chars[current_pos] == '_' {
                let mut end = current_pos;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                let ident: String = chars[current_pos..end].iter().collect();

                // マクロ呼び出しチェック identifier!
                let is_macro = end < chars.len() && chars[end] == '!';
                if is_macro {
                    end += 1;
                    let macro_call: String = chars[current_pos..end].iter().collect();
                    tokens.push(SyntaxToken {
                        text: macro_call,
                        kind: TokenKind::Macro,
                    });
                    current_pos = end;
                    continue;
                }

                // モジュール名チェック identifier::
                let is_module = end + 1 < chars.len() && chars[end] == ':' && chars[end + 1] == ':';

                // 関数呼び出しチェック identifier(
                // 空白をスキップして (  をチェック
                let mut peek = end;
                while peek < chars.len() && chars[peek].is_whitespace() {
                    peek += 1;
                }
                let is_function_call = peek < chars.len() && chars[peek] == '(';

                // 関数定義名検出: `fn` の直後の識別子
                let kind = if prev_token_was_fn {
                    prev_token_was_fn = false;
                    TokenKind::FunctionDef
                } else if is_module {
                    TokenKind::Module
                } else if is_function_call {
                    TokenKind::FunctionCall
                } else {
                    match ident.as_str() {
                        // Import keywords (blue)
                        "use" | "mod" => TokenKind::KeywordImport,

                        // Regular keywords (orange)
                        "fn" | "pub" | "struct" | "enum" | "impl" | "trait" | "type" | "let" | "mut" |
                        "const" | "static" | "if" | "else" | "match" | "for" | "while" | "loop" |
                        "return" | "break" | "continue" | "crate" | "self" | "Self" |
                        "super" | "as" | "in" | "ref" | "move" | "unsafe" | "async" | "await" |
                        "dyn" | "where" | "true" | "false" => {
                            // `fn` キーワードを記憶
                            if ident == "fn" {
                                prev_token_was_fn = true;
                            }
                            TokenKind::Keyword
                        }

                        // 型
                        "String" | "str" | "usize" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" |
                        "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" | "char" |
                        "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" | "HashMap" | "HashSet" => TokenKind::Type,

                        // 大文字始まりは型と判断
                        _ if ident.chars().next().unwrap().is_uppercase() => TokenKind::Type,

                        // 全大文字は定数と判断
                        _ if ident.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit()) && ident.len() > 1 => TokenKind::Constant,

                        _ => TokenKind::Identifier,
                    }
                };

                tokens.push(SyntaxToken {
                    text: ident,
                    kind,
                });
                current_pos = end;
                continue;
            }

            // その他の文字（記号など）
            tokens.push(SyntaxToken {
                text: chars[current_pos].to_string(),
                kind: TokenKind::Punctuation,
            });
            current_pos += 1;

            // Safety: Ensure current_pos advanced to prevent infinite loop
            #[cfg(debug_assertions)]
            if current_pos == pos_before {
                web_sys::console::error_1(&format!(
                    "⚠️ INFINITE LOOP DETECTED in tokenize_rust at pos {} char '{}'",
                    current_pos,
                    chars.get(current_pos).unwrap_or(&'?')
                ).into());
                current_pos += 1; // Force advance
            }
        }

        tokens
    }

    /// 指定座標にテキストを描画（IME未確定文字用）
    pub fn draw_text_at(&self, x: f64, y: f64, text: &str, color: &str) {
        // ✅ IntelliJ風: ベースライン調整を適用
        let x_rounded = x.round();
        let y_rounded = y.round();

        self.context.set_fill_style(&color.into());
        let _ = self.context.fill_text(text, x_rounded, y_rounded);
    }

    /// カーソルを描画（縦線）
    /// line_text: カーソルがある行のテキスト全体
    pub fn draw_cursor(&self, line: usize, col: usize, scroll_top: f64, line_text: &str,
                       prev_line: usize, prev_col: usize, move_timestamp: f64, prev_line_text: &str) {
        // Calculate target position
        let target_x = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(line_text, col)).round();
        let target_y = (line as f64 * self.line_height - scroll_top).round();

        // TODO: Performance API integration for smooth cursor animation
        // Currently disabled due to web_sys API compatibility
        let now = 0.0;

        // Calculate animated position if cursor recently moved
        let (x, y) = if move_timestamp > 0.0 {
            let elapsed = now - move_timestamp;
            let duration = 100.0; // 100ms animation like IntelliJ

            if elapsed < duration {
                // Animation in progress - interpolate
                let prev_x = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(prev_line_text, prev_col)).round();
                let prev_y = (prev_line as f64 * self.line_height - scroll_top).round();

                let t = (elapsed / duration).min(1.0).max(0.0);
                let eased = CursorAnimation::ease_out_quad(t);

                let x = prev_x + (target_x - prev_x) * eased;
                let y = prev_y + (target_y - prev_y) * eased;
                (x, y)
            } else {
                // Animation finished
                (target_x, target_y)
            }
        } else {
            // No animation (first render or no movement)
            (target_x, target_y)
        };

        self.context.set_stroke_style(&COLOR_CURSOR.into());
        self.context.set_line_width(2.0);
        self.context.begin_path();
        self.context.move_to(x, y);
        self.context.line_to(x, y + self.line_height);
        self.context.stroke();
    }

    /// 選択範囲を描画 (IntelliJ-style spacing)
    /// get_line_text: 行番号から行のテキストを取得するクロージャ（日本語などマルチバイト文字の幅を正確に計算するため）
    pub fn draw_selection<F>(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        scroll_top: f64,
        get_line_text: F,
    ) where
        F: Fn(usize) -> String,
    {
        self.context.set_fill_style(&COLOR_SELECTION.into());

        if start_line == end_line {
            // 単一行の選択
            let line_text = get_line_text(start_line);
            // ✅ IntelliJ風: 余白を20.0pxに統一
            let x_start = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(&line_text, start_col)).round();
            let x_end = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(&line_text, end_col)).round();
            let y = (start_line as f64 * self.line_height - scroll_top).round();

            self.context
                .fill_rect(x_start, y, x_end - x_start, self.line_height);
        } else {
            // 複数行の選択
            // 最初の行: start_colから行末まで
            let first_line_text = get_line_text(start_line);
            // 🚀 PERFORMANCE: Avoid Vec<char> allocation - use chars().count() directly
            let first_line_len = first_line_text.chars().count();
            // ✅ IntelliJ風: 余白を20.0pxに統一
            let x_start = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(&first_line_text, start_col)).round();
            let x_end_first = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(&first_line_text, first_line_len)).round();
            let y_first = (start_line as f64 * self.line_height - scroll_top).round();

            self.context.fill_rect(
                x_start,
                y_first,
                x_end_first - x_start,
                self.line_height,
            );

            // 🚀 PERFORMANCE: Middle lines - full width selection
            // Avoid get_line_text() call entirely for middle lines
            // They are fully selected, so we can use a fixed large width
            for line in (start_line + 1)..end_line {
                // ✅ IntelliJ風: 余白を20.0pxに統一
                let x_start_middle = (self.gutter_width + 20.0).round();
                let y_middle = (line as f64 * self.line_height - scroll_top).round();

                // Draw selection with a large fixed width (10000px covers most lines)
                // This eliminates both String allocation and text measurement
                self.context.fill_rect(
                    x_start_middle,
                    y_middle,
                    10000.0, // Large enough to cover any reasonable line width
                    self.line_height,
                );
            }

            // 最後の行: 行頭からend_colまで
            let last_line_text = get_line_text(end_line);
            // ✅ IntelliJ風: 余白を20.0pxに統一
            let x_start_last = (self.gutter_width + 20.0).round();
            let x_end_last = (self.gutter_width + 20.0 + self.calculate_x_offset_from_text(&last_line_text, end_col)).round();
            let y_last = (end_line as f64 * self.line_height - scroll_top).round();

            self.context.fill_rect(
                x_start_last,
                y_last,
                x_end_last - x_start_last,
                self.line_height,
            );
        }
    }

    /// 文字列の幅を計算（ASCII + 全角混在対応）
    /// 実際のテキストから、指定された列位置までの幅を測定
    fn calculate_x_offset_from_text(&self, line_text: &str, col: usize) -> f64 {
        // 列位置までの文字列を取得
        let chars: Vec<char> = line_text.chars().collect();
        let end_col = col.min(chars.len());

        if end_col == 0 {
            return 0.0;
        }

        // カーソル位置までの文字列
        let text_up_to_cursor: String = chars[0..end_col].iter().collect();

        // 実際の幅を測定
        self.measure_text(&text_up_to_cursor)
    }

    /// 文字列の幅を計算（後方互換性のため残す、非推奨）
    #[allow(dead_code)]
    fn calculate_x_offset(&self, col: usize) -> f64 {
        // 簡易実装（ASCII幅のみ）
        col as f64 * self.char_width_ascii
    }

    /// 指定したテキストの実際の幅を測定
    pub fn measure_text(&self, text: &str) -> f64 {
        match self.context.measure_text(text) {
            Ok(metrics) => metrics.width(),
            Err(_) => 0.0,
        }
    }

    /// 現在のフォント設定を取得
    pub fn get_font(&self) -> String {
        self.context.font()
    }

    /// HTMLをトークンに分解
    fn tokenize_html(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = line.chars().collect();

        // HTML comments
        if line.trim_start().starts_with("<!--") {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Comment,
            });
            return tokens;
        }

        while current_pos < chars.len() {
            // Safety: Store position to detect infinite loop
            let pos_before = current_pos;
            // Skip whitespace
            if chars[current_pos].is_whitespace() {
                let start = current_pos;
                while current_pos < chars.len() && chars[current_pos].is_whitespace() {
                    current_pos += 1;
                }
                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Identifier,
                });
                continue;
            }

            // HTML tags
            if chars[current_pos] == '<' {
                let start = current_pos;
                current_pos += 1;

                // Find tag end
                while current_pos < chars.len() && chars[current_pos] != '>' {
                    current_pos += 1;
                }

                if current_pos < chars.len() {
                    current_pos += 1; // Include '>'
                    tokens.push(SyntaxToken {
                        text: chars[start..current_pos].iter().collect(),
                        kind: TokenKind::HtmlTag,
                    });
                }
                continue;
            }

            // String literals
            if chars[current_pos] == '"' || chars[current_pos] == '\'' {
                let quote = chars[current_pos];
                let start = current_pos;
                current_pos += 1;

                while current_pos < chars.len() && chars[current_pos] != quote {
                    if chars[current_pos] == '\\' && current_pos + 1 < chars.len() {
                        current_pos += 2;
                    } else {
                        current_pos += 1;
                    }
                }

                if current_pos < chars.len() {
                    current_pos += 1; // Include closing quote
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::String,
                });
                continue;
            }

            // Default: identifier
            let start = current_pos;
            while current_pos < chars.len()
                && !chars[current_pos].is_whitespace()
                && chars[current_pos] != '<'
                && chars[current_pos] != '>'
                && chars[current_pos] != '"'
                && chars[current_pos] != '\'' {
                current_pos += 1;
            }

            if current_pos > start {
                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Identifier,
                });
            } else {
                current_pos += 1;
            }

            // Safety: Ensure current_pos advanced to prevent infinite loop
            #[cfg(debug_assertions)]
            if current_pos == pos_before {
                web_sys::console::error_1(&format!(
                    "⚠️ INFINITE LOOP DETECTED in tokenize_html at pos {} char '{}'",
                    current_pos,
                    chars.get(current_pos).unwrap_or(&'?')
                ).into());
                current_pos += 1; // Force advance
            }
        }

        if tokens.is_empty() {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Identifier,
            });
        }

        tokens
    }

    /// CSSをトークンに分解
    fn tokenize_css(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = line.chars().collect();

        // CSS comments
        if line.trim_start().starts_with("/*") {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Comment,
            });
            return tokens;
        }

        // CSS keywords
        let css_keywords = [
            "color", "background", "margin", "padding", "border", "width", "height",
            "display", "position", "top", "left", "right", "bottom", "flex", "grid",
            "font", "text", "line", "opacity", "transform", "transition", "animation",
        ];

        while current_pos < chars.len() {
            // Safety: Store position to detect infinite loop
            let pos_before = current_pos;
            // Skip whitespace
            if chars[current_pos].is_whitespace() {
                let start = current_pos;
                while current_pos < chars.len() && chars[current_pos].is_whitespace() {
                    current_pos += 1;
                }
                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Identifier,
                });
                continue;
            }

            // CSS selectors (., #, :)
            if chars[current_pos] == '.' || chars[current_pos] == '#' || chars[current_pos] == ':' {
                let start = current_pos;
                current_pos += 1;

                while current_pos < chars.len()
                    && (chars[current_pos].is_alphanumeric() || chars[current_pos] == '-' || chars[current_pos] == '_') {
                    current_pos += 1;
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::CssSelector,
                });
                continue;
            }

            // String literals
            if chars[current_pos] == '"' || chars[current_pos] == '\'' {
                let quote = chars[current_pos];
                let start = current_pos;
                current_pos += 1;

                while current_pos < chars.len() && chars[current_pos] != quote {
                    if chars[current_pos] == '\\' && current_pos + 1 < chars.len() {
                        current_pos += 2;
                    } else {
                        current_pos += 1;
                    }
                }

                if current_pos < chars.len() {
                    current_pos += 1;
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::String,
                });
                continue;
            }

            // Numbers (including hex colors)
            if chars[current_pos].is_ascii_digit() || (chars[current_pos] == '#' && current_pos + 1 < chars.len() && chars[current_pos + 1].is_ascii_hexdigit()) {
                let start = current_pos;

                if chars[current_pos] == '#' {
                    current_pos += 1;
                    while current_pos < chars.len() && chars[current_pos].is_ascii_hexdigit() {
                        current_pos += 1;
                    }
                } else {
                    while current_pos < chars.len() && (chars[current_pos].is_ascii_digit() || chars[current_pos] == '.') {
                        current_pos += 1;
                    }
                    // CSS units
                    while current_pos < chars.len() && chars[current_pos].is_alphabetic() {
                        current_pos += 1;
                    }
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers and keywords
            if chars[current_pos].is_alphabetic() || chars[current_pos] == '-' {
                let start = current_pos;

                while current_pos < chars.len()
                    && (chars[current_pos].is_alphanumeric() || chars[current_pos] == '-') {
                    current_pos += 1;
                }

                let word: String = chars[start..current_pos].iter().collect();
                let is_keyword = css_keywords.iter().any(|&kw| word.starts_with(kw));

                tokens.push(SyntaxToken {
                    text: word,
                    kind: if is_keyword { TokenKind::CssProperty } else { TokenKind::Identifier },
                });
                continue;
            }

            // Punctuation
            let start = current_pos;
            current_pos += 1;
            tokens.push(SyntaxToken {
                text: chars[start..current_pos].iter().collect(),
                kind: TokenKind::Punctuation,
            });

            // Safety: Ensure current_pos advanced to prevent infinite loop
            #[cfg(debug_assertions)]
            if current_pos == pos_before {
                web_sys::console::error_1(&format!(
                    "⚠️ INFINITE LOOP DETECTED in tokenize_css at pos {} char '{}'",
                    current_pos,
                    chars.get(current_pos).unwrap_or(&'?')
                ).into());
                current_pos += 1; // Force advance
            }
        }

        if tokens.is_empty() {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Identifier,
            });
        }

        tokens
    }

    /// JavaScriptをトークンに分解
    fn tokenize_javascript(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = line.chars().collect();

        // Single-line comments
        if line.trim_start().starts_with("//") {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Comment,
            });
            return tokens;
        }

        // JavaScript keywords
        let js_keywords = [
            "const", "let", "var", "function", "async", "await", "return",
            "if", "else", "for", "while", "do", "switch", "case", "break",
            "continue", "try", "catch", "finally", "throw", "new", "this",
            "class", "extends", "import", "export", "from", "default",
            "typeof", "instanceof", "in", "of", "void", "delete", "yield",
            "true", "false", "null", "undefined",
        ];

        while current_pos < chars.len() {
            // Safety: Store position to detect infinite loop
            let pos_before = current_pos;
            // Skip whitespace
            if chars[current_pos].is_whitespace() {
                let start = current_pos;
                while current_pos < chars.len() && chars[current_pos].is_whitespace() {
                    current_pos += 1;
                }
                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Identifier,
                });
                continue;
            }

            // Template literals
            if chars[current_pos] == '`' {
                let start = current_pos;
                current_pos += 1;

                while current_pos < chars.len() && chars[current_pos] != '`' {
                    if chars[current_pos] == '\\' && current_pos + 1 < chars.len() {
                        current_pos += 2;
                    } else {
                        current_pos += 1;
                    }
                }

                if current_pos < chars.len() {
                    current_pos += 1;
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::String,
                });
                continue;
            }

            // String literals
            if chars[current_pos] == '"' || chars[current_pos] == '\'' {
                let quote = chars[current_pos];
                let start = current_pos;
                current_pos += 1;

                while current_pos < chars.len() && chars[current_pos] != quote {
                    if chars[current_pos] == '\\' && current_pos + 1 < chars.len() {
                        current_pos += 2;
                    } else {
                        current_pos += 1;
                    }
                }

                if current_pos < chars.len() {
                    current_pos += 1;
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::String,
                });
                continue;
            }

            // Numbers
            if chars[current_pos].is_ascii_digit() {
                let start = current_pos;

                while current_pos < chars.len() && (chars[current_pos].is_ascii_digit() || chars[current_pos] == '.') {
                    current_pos += 1;
                }

                tokens.push(SyntaxToken {
                    text: chars[start..current_pos].iter().collect(),
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers and keywords
            if chars[current_pos].is_alphabetic() || chars[current_pos] == '_' || chars[current_pos] == '$' {
                let start = current_pos;

                while current_pos < chars.len()
                    && (chars[current_pos].is_alphanumeric() || chars[current_pos] == '_' || chars[current_pos] == '$') {
                    current_pos += 1;
                }

                let word: String = chars[start..current_pos].iter().collect();
                let is_keyword = js_keywords.contains(&word.as_str());

                // Check if it's a function call (followed by '(')
                let is_function_call = current_pos < chars.len() && {
                    let mut peek = current_pos;
                    while peek < chars.len() && chars[peek].is_whitespace() {
                        peek += 1;
                    }
                    peek < chars.len() && chars[peek] == '('
                };

                tokens.push(SyntaxToken {
                    text: word,
                    kind: if is_keyword {
                        TokenKind::Keyword
                    } else if is_function_call {
                        TokenKind::FunctionCall
                    } else {
                        TokenKind::Identifier
                    },
                });
                continue;
            }

            // Punctuation
            let start = current_pos;
            current_pos += 1;
            tokens.push(SyntaxToken {
                text: chars[start..current_pos].iter().collect(),
                kind: TokenKind::Punctuation,
            });

            // Safety: Ensure current_pos advanced to prevent infinite loop
            #[cfg(debug_assertions)]
            if current_pos == pos_before {
                web_sys::console::error_1(&format!(
                    "⚠️ INFINITE LOOP DETECTED in tokenize_javascript at pos {} char '{}'",
                    current_pos,
                    chars.get(current_pos).unwrap_or(&'?')
                ).into());
                current_pos += 1; // Force advance
            }
        }

        if tokens.is_empty() {
            tokens.push(SyntaxToken {
                text: line.to_string(),
                kind: TokenKind::Identifier,
            });
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constants() {
        assert_eq!(COLOR_BACKGROUND, "#1E1F22");  // Pixel-perfect editor background
        assert_eq!(COLOR_FOREGROUND, "#BCBEC4");  // Pixel-perfect default text color
    }

    #[test]
    fn test_font_constants() {
        assert_eq!(FONT_FAMILY, "JetBrains Mono");
        assert_eq!(FONT_SIZE, 13.0);
        assert_eq!(LINE_HEIGHT, 20.0);
        assert_eq!(LETTER_SPACING, 0.0);
    }
}
