//! Japanese Cursor Position Test with Canvas measureText
//!
//! Tests that cursor position is correctly calculated for Japanese text
//! using actual Canvas measureText() instead of Unicode width heuristics.

use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use berry_editor::core::canvas_renderer::CanvasRenderer;
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

fn create_test_canvas() -> HtmlCanvasElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    canvas.set_width(1000);
    canvas.set_height(800);
    canvas
}

#[wasm_bindgen_test]
fn test_japanese_text_width_measurement() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    // ASCII文字の幅
    let ascii_text = "hello";
    let ascii_width = renderer.measure_text(ascii_text);

    // 日本語（全角）文字の幅
    let japanese_text = "こんにちは";
    let japanese_width = renderer.measure_text(japanese_text);

    // 日本語文字は幅が広いはず
    assert!(
        japanese_width > ascii_width,
        "Japanese text should be wider than ASCII. ASCII: {}, Japanese: {}",
        ascii_width,
        japanese_width
    );

    // 1文字あたりの平均幅を比較
    let ascii_avg = ascii_width / 5.0;
    let japanese_avg = japanese_width / 5.0;

    assert!(
        japanese_avg > ascii_avg * 1.5,
        "Japanese char should be at least 1.5x wider than ASCII. ASCII avg: {}, Japanese avg: {}",
        ascii_avg,
        japanese_avg
    );
}

#[wasm_bindgen_test]
fn test_mixed_text_width_measurement() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    // 混在テキスト
    let mixed_text = "hello世界";

    // 部分ごとに測定
    let hello_width = renderer.measure_text("hello");
    let sekai_width = renderer.measure_text("世界");
    let total_width = renderer.measure_text(mixed_text);

    // 合計はほぼ一致するはず（小さな誤差は許容）
    let expected_total = hello_width + sekai_width;
    let diff = (total_width - expected_total).abs();

    assert!(
        diff < 1.0,
        "Mixed text width should equal sum of parts. Expected: {}, Got: {}, Diff: {}",
        expected_total,
        total_width,
        diff
    );
}

#[wasm_bindgen_test]
fn test_cursor_position_after_japanese_input() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    // "日本語test" というテキスト
    let text = "日本語test";

    // カーソルが位置3（"日本語"の後、"test"の前）にある場合
    let cursor_col = 3;

    // カーソル位置までのテキスト
    let text_before_cursor: String = text.chars().take(cursor_col).collect();
    assert_eq!(text_before_cursor, "日本語");

    // 幅を測定
    let width_before_cursor = renderer.measure_text(&text_before_cursor);

    // ASCII部分だけの幅と比較
    let ascii_width = renderer.measure_text("test");

    // "日本語"の幅は"test"より広いはず
    assert!(
        width_before_cursor > ascii_width,
        "Japanese '日本語' should be wider than ASCII 'test'. Japanese: {}, ASCII: {}",
        width_before_cursor,
        ascii_width
    );
}

#[wasm_bindgen_test]
fn test_cursor_position_progression_with_japanese() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    let text = "あいうえお";

    // カーソルが0から5まで進むにつれて、幅が単調増加するはず
    let mut prev_width = 0.0;

    for col in 0..=5 {
        let text_before: String = text.chars().take(col).collect();
        let width = renderer.measure_text(&text_before);

        assert!(
            width >= prev_width,
            "Width should increase monotonically. Col: {}, Prev: {}, Current: {}",
            col,
            prev_width,
            width
        );

        prev_width = width;
    }

    // 最後の幅は0より大きい
    assert!(prev_width > 0.0, "Final width should be positive");
}

#[wasm_bindgen_test]
fn test_empty_text_before_cursor() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    // カーソルが行頭（col=0）の場合
    let text = "日本語";
    let text_before: String = text.chars().take(0).collect();

    let width = renderer.measure_text(&text_before);

    assert_eq!(width, 0.0, "Width before cursor at col=0 should be 0");
}

#[wasm_bindgen_test]
fn test_emoji_text_width() {
    let canvas = create_test_canvas();
    let renderer = CanvasRenderer::new(canvas).expect("Failed to create renderer");

    // 絵文字テスト
    let emoji_text = "🎉🎊";
    let ascii_text = "ab";

    let emoji_width = renderer.measure_text(emoji_text);
    let ascii_width = renderer.measure_text(ascii_text);

    // 絵文字の幅はASCIIより広いはず
    assert!(
        emoji_width > ascii_width,
        "Emoji should be wider than ASCII. Emoji: {}, ASCII: {}",
        emoji_width,
        ascii_width
    );
}
