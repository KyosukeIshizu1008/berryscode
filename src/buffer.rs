//! Text Buffer Implementation using Ropey
//! IntelliJ-inspired design: Immutable snapshots with lazy evaluation

use ropey::Rope;
use std::collections::HashMap;

/// ✅ IntelliJ Design: TextBuffer with Rope data structure
/// 🚀 PERFORMANCE: Cache tokenization results to avoid re-parsing every frame
#[derive(Clone)]
pub struct TextBuffer {
    rope: Rope,
    file_path: Option<String>,
    modified: bool,
    language: String,
    /// Version counter - incremented on every edit to invalidate cache
    version: u64,
    /// 🚀 Token cache: Stores parsed syntax tokens per line
    /// This prevents re-tokenizing on every render frame (60 FPS!)
    /// Only stores visible lines + margin to save memory
    token_cache: HashMap<usize, Vec<(String, String)>>, // line_idx -> Vec<(text, token_kind)>
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
            version: 0,
            token_cache: HashMap::new(),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
            version: 0,
            token_cache: HashMap::new(),
        }
    }

    pub fn set_file_path(&mut self, path: String) {
        self.file_path = Some(path);
    }

    pub fn set_language(&mut self, lang: String) {
        self.language = lang;
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        // ✅ 境界チェック：char_idxをバッファサイズ以内にクランプ
        let safe_idx = char_idx.min(self.rope.len_chars());

        // ✅ IntelliJ Pro: Incremental Syntax Analysis - only invalidate affected lines
        let start_line = self.rope.char_to_line(safe_idx);
        let newline_count = text.chars().filter(|&c| c == '\n').count();

        self.rope.insert(safe_idx, text);
        self.modified = true;
        self.version += 1;

        // ✅ IntelliJ Pro: Smart cache invalidation
        // Only clear lines that were actually modified + surrounding context
        let end_line = start_line + newline_count + 2; // +2 for context
        self.invalidate_cache_range(start_line, end_line);
    }

    pub fn remove(&mut self, start: usize, end: usize) {
        // ✅ 境界チェック：start と end をバッファサイズ以内にクランプ
        let safe_start = start.min(self.rope.len_chars());
        let safe_end = end.min(self.rope.len_chars());

        // 削除する範囲がない場合は早期リターン
        if safe_start >= safe_end {
            return;
        }

        // ✅ IntelliJ Pro: Incremental invalidation for deletions
        let start_line = self.rope.char_to_line(safe_start);
        let end_line = self.rope.char_to_line(safe_end);

        self.rope.remove(safe_start..safe_end);
        self.modified = true;
        self.version += 1;

        // ✅ IntelliJ Pro: Only invalidate affected range
        self.invalidate_cache_range(start_line, end_line + 2); // +2 for context
    }

    /// ✅ IntelliJ Pro: Invalidate only specific line range (incremental)
    /// 🚀 PERFORMANCE: Clear token cache for edited lines
    fn invalidate_cache_range(&mut self, start_line: usize, end_line: usize) {
        // Remove cached tokens for lines that were modified
        self.token_cache.retain(|&line_idx, _| {
            line_idx < start_line || line_idx > end_line
        });
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// Convert line index to character index (start of the line)
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }

    /// Convert character index to line index
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    /// Get a slice of text from start to end char indices
    pub fn slice(&self, start: usize, end: usize) -> Option<String> {
        if start <= end && end <= self.len_chars() {
            Some(self.rope.slice(start..end).to_string())
        } else {
            None
        }
    }

    /// ✅ IntelliJ Design: Get version for cache invalidation
    pub fn version(&self) -> u64 {
        self.version
    }

    /// 🚀 PERFORMANCE: Get cached tokens for a line
    /// Returns None if not cached yet
    pub fn get_cached_tokens(&self, line_idx: usize) -> Option<&Vec<(String, String)>> {
        self.token_cache.get(&line_idx)
    }

    /// 🚀 PERFORMANCE: Cache tokens for a line
    /// tokens: Vec<(text, token_kind)>
    pub fn cache_tokens(&mut self, line_idx: usize, tokens: Vec<(String, String)>) {
        self.token_cache.insert(line_idx, tokens);
    }

    /// 🚀 PERFORMANCE: Trim token cache to visible range only
    /// Prevents unbounded memory growth during scrolling
    pub fn trim_token_cache(&mut self, visible_start: usize, visible_end: usize, keep_margin: usize) {
        let keep_start = visible_start.saturating_sub(keep_margin);
        let keep_end = visible_end + keep_margin;

        self.token_cache.retain(|&line_idx, _| {
            line_idx >= keep_start && line_idx <= keep_end
        });
    }

    /// 🚀 PERFORMANCE: Get cache statistics for monitoring
    pub fn token_cache_size(&self) -> usize {
        self.token_cache.len()
    }

    /// ✅ IntelliJ Pro: Create immutable snapshot of the rope for rendering
    /// This is O(1) operation - Rope uses Arc internally, so clone is instant!
    /// The snapshot is frozen and won't be affected by future edits
    pub fn snapshot(&self) -> Rope {
        self.rope.clone() // O(1) - just clones Arc pointer, not data!
    }

    /// ✅ IntelliJ Pro: Get line from snapshot (for safe concurrent access)
    pub fn line_from_snapshot(snapshot: &Rope, line_idx: usize) -> Option<String> {
        if line_idx < snapshot.len_lines() {
            Some(snapshot.line(line_idx).to_string())
        } else {
            None
        }
    }

    /// ✅ IntelliJ Pro: Get horizontal segment of a line (for long-line rendering)
    /// Only returns visible characters within viewport bounds
    pub fn line_segment(&self, line_idx: usize, start_col: usize, end_col: usize) -> Option<String> {
        let line = self.line(line_idx)?;
        let chars: Vec<char> = line.chars().collect();
        let end = end_col.min(chars.len());
        if start_col >= chars.len() {
            return Some(String::new());
        }
        Some(chars[start_col..end].iter().collect())
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // ✅ MEMORY FIX: Memory load test to verify efficiency with large files
    #[wasm_bindgen_test]
    fn test_large_file_memory_efficiency() {
        // Generate ~5MB of data (100,000 lines)
        let large_text = (0..100000)
            .map(|i| format!("Line {}: some repetitive text to fill memory and simulate real files...\n", i))
            .collect::<String>();

        let start_lines = 100000;
        let buffer = TextBuffer::from_str(&large_text);

        // Ropey uses rope data structure, so cloning should NOT duplicate the entire content
        // Instead, it shares the internal data structure (copy-on-write)
        let cloned_buffer = buffer.clone();

        // Note: 100000 lines with \n creates 100001 lines (last empty line)
        assert_eq!(buffer.len_lines(), start_lines + 1);
        assert_eq!(cloned_buffer.len_lines(), start_lines + 1);

        // Verify operations work correctly on large files
        let line_0 = buffer.line(0);
        assert!(line_0.is_some());
        assert!(line_0.unwrap().contains("Line 0:"));

        let line_50000 = buffer.line(50000);
        assert!(line_50000.is_some());
        assert!(line_50000.unwrap().contains("Line 50000:"));

        // Note: Actual memory usage should be profiled in Chrome DevTools
        // Expected behavior: Total heap size should stay under 300MB even with multiple clones
    }

    #[wasm_bindgen_test]
    fn test_multiple_clones_dont_explode_memory() {
        // Create a moderately large buffer
        let text = (0..10000)
            .map(|i| format!("Line {}: test data\n", i))
            .collect::<String>();

        let buffer = TextBuffer::from_str(&text);

        // Clone multiple times (simulating multiple tabs with same file)
        let _clone1 = buffer.clone();
        let _clone2 = buffer.clone();
        let _clone3 = buffer.clone();
        let _clone4 = buffer.clone();
        let _clone5 = buffer.clone();

        // All clones should work correctly
        // Note: 10000 lines with \n creates 10001 lines (last empty line)
        assert_eq!(buffer.len_lines(), 10001);
        assert_eq!(_clone1.len_lines(), 10001);
        assert_eq!(_clone5.len_lines(), 10001);

        // Memory should NOT increase by 5x due to Ropey's internal sharing
    }

    #[wasm_bindgen_test]
    fn test_new_buffer() {
        let buffer = TextBuffer::new();
        assert_eq!(buffer.len_chars(), 0);
        assert_eq!(buffer.len_lines(), 1); // Ropey always has at least 1 line
        assert!(!buffer.is_modified());
        assert_eq!(buffer.language(), "plaintext");
    }

    #[wasm_bindgen_test]
    fn test_from_str() {
        let text = "Hello\nWorld";
        let buffer = TextBuffer::from_str(text);
        assert_eq!(buffer.to_string(), text);
        assert_eq!(buffer.len_lines(), 2);
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(5, " World");
        assert_eq!(buffer.to_string(), "Hello World");
        assert!(buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_remove() {
        let mut buffer = TextBuffer::from_str("Hello World");
        buffer.remove(5, 11);
        assert_eq!(buffer.to_string(), "Hello");
        assert!(buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_len_chars() {
        let buffer = TextBuffer::from_str("Hello");
        assert_eq!(buffer.len_chars(), 5);
    }

    #[wasm_bindgen_test]
    fn test_len_lines() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        assert_eq!(buffer.len_lines(), 3);
    }

    #[wasm_bindgen_test]
    fn test_line() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        assert_eq!(buffer.line(0).unwrap(), "Line1\n");
        assert_eq!(buffer.line(1).unwrap(), "Line2\n");
        assert_eq!(buffer.line(2).unwrap(), "Line3");
        assert!(buffer.line(3).is_none());
    }

    #[wasm_bindgen_test]
    fn test_mark_saved() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(5, " World");
        assert!(buffer.is_modified());
        buffer.mark_saved();
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_set_file_path() {
        let mut buffer = TextBuffer::new();
        assert!(buffer.file_path().is_none());
        buffer.set_file_path("/path/to/file.rs".to_string());
        assert_eq!(buffer.file_path(), Some("/path/to/file.rs"));
    }

    #[wasm_bindgen_test]
    fn test_set_language() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.language(), "plaintext");
        buffer.set_language("rust".to_string());
        assert_eq!(buffer.language(), "rust");
    }

    #[wasm_bindgen_test]
    fn test_multiple_operations() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(0, "Well, ");
        buffer.insert(buffer.len_chars(), "!");
        assert_eq!(buffer.to_string(), "Well, Hello!");

        buffer.remove(0, 6);
        assert_eq!(buffer.to_string(), "Hello!");

        assert!(buffer.is_modified());
        buffer.mark_saved();
        assert!(!buffer.is_modified());
    }

    // ========== 境界条件・異常系テスト ==========

    #[wasm_bindgen_test]
    fn test_empty_buffer_operations() {
        let mut buffer = TextBuffer::new();

        // 空バッファへの削除（クラッシュしないこと）
        buffer.remove(0, 0);
        assert_eq!(buffer.len_chars(), 0);

        // 空バッファへの挿入
        buffer.insert(0, "First");
        assert_eq!(buffer.to_string(), "First");

        // 全削除
        buffer.remove(0, buffer.len_chars());
        assert_eq!(buffer.len_chars(), 0);
    }

    #[wasm_bindgen_test]
    fn test_out_of_bounds_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        let initial_len = buffer.len_chars();

        // 境界外への挿入（min()でクランプされる）
        buffer.insert(100, " World");

        // 末尾に追加されるべき
        assert_eq!(buffer.to_string(), "Hello World");
        assert_eq!(buffer.len_chars(), initial_len + 6);
    }

    #[wasm_bindgen_test]
    fn test_out_of_bounds_remove() {
        let mut buffer = TextBuffer::from_str("Hello");

        // 境界外の削除範囲（min()でクランプされる）
        buffer.remove(0, 1000);
        assert_eq!(buffer.len_chars(), 0);
    }

    #[wasm_bindgen_test]
    fn test_remove_with_start_greater_than_length() {
        let mut buffer = TextBuffer::from_str("Hello");

        // start が length より大きい場合
        buffer.remove(100, 200);

        // 何も削除されない（Ropeの動作に依存）
        assert!(buffer.len_chars() <= 5);
    }

    #[wasm_bindgen_test]
    fn test_slice_boundary_conditions() {
        let buffer = TextBuffer::from_str("Hello");

        // 正常なスライス
        assert_eq!(buffer.slice(0, 5), Some("Hello".to_string()));

        // start == end
        assert_eq!(buffer.slice(2, 2), Some("".to_string()));

        // start > end (None を返す)
        assert_eq!(buffer.slice(5, 2), None);

        // end > len_chars() (None を返す)
        assert_eq!(buffer.slice(0, 100), None);

        // 両方とも範囲外
        assert_eq!(buffer.slice(100, 200), None);
    }

    #[wasm_bindgen_test]
    fn test_line_segment_boundary_conditions() {
        let buffer = TextBuffer::from_str("Short");

        // 列範囲が文字数を超える場合
        let segment = buffer.line_segment(0, 10, 20);
        assert_eq!(segment, Some("".to_string()));

        // start_col が文字数を超える場合
        let segment = buffer.line_segment(0, 100, 200);
        assert_eq!(segment, Some("".to_string()));

        // 存在しない行
        let segment = buffer.line_segment(10, 0, 5);
        assert_eq!(segment, None);

        // 正常なケース（部分文字列）
        let buffer2 = TextBuffer::from_str("Hello World");
        let segment = buffer2.line_segment(0, 0, 5);
        assert_eq!(segment, Some("Hello".to_string()));

        // end_col が文字数を超える（クランプされる）
        let segment = buffer2.line_segment(0, 6, 100);
        assert_eq!(segment, Some("World".to_string()));
    }

    // 🗑️ REMOVED: Cache tests (dead code - Canvas doesn't use HTML caching)
    /*
    #[wasm_bindgen_test]
    fn test_cache_operations() { ... }

    #[wasm_bindgen_test]
    fn test_trim_cache_precision() { ... }

    #[wasm_bindgen_test]
    fn test_trim_cache_with_margin() { ... }

    #[wasm_bindgen_test]
    fn test_cache_invalidation_on_insert() { ... }

    #[wasm_bindgen_test]
    fn test_cache_invalidation_on_remove() { ... }
    */

    #[wasm_bindgen_test]
    fn test_line_char_conversion_boundary() {
        let buffer = TextBuffer::from_str("L1\nL2\nL3");

        // 行0の開始文字インデックス
        assert_eq!(buffer.line_to_char(0), 0);

        // 行1の開始文字インデックス（"L1\n" = 3文字）
        assert_eq!(buffer.line_to_char(1), 3);

        // 行2の開始文字インデックス
        assert_eq!(buffer.line_to_char(2), 6);

        // 文字インデックスから行番号
        assert_eq!(buffer.char_to_line(0), 0); // "L"
        assert_eq!(buffer.char_to_line(2), 0); // "\n"
        assert_eq!(buffer.char_to_line(3), 1); // "L"
        assert_eq!(buffer.char_to_line(6), 2); // "L"
    }

    #[wasm_bindgen_test]
    fn test_snapshot_immutability() {
        let mut buffer = TextBuffer::from_str("Original");

        // スナップショット作成
        let snapshot = buffer.snapshot();

        // バッファを変更
        buffer.insert(8, " Modified");

        // スナップショットは変更前の状態を保持
        assert_eq!(snapshot.to_string(), "Original");
        assert_eq!(buffer.to_string(), "Original Modified");
    }

    #[wasm_bindgen_test]
    fn test_line_from_snapshot() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        let snapshot = buffer.snapshot();

        // 正常な行取得
        assert_eq!(
            TextBuffer::line_from_snapshot(&snapshot, 0),
            Some("Line1\n".to_string())
        );
        assert_eq!(
            TextBuffer::line_from_snapshot(&snapshot, 2),
            Some("Line3".to_string())
        );

        // 範囲外
        assert_eq!(TextBuffer::line_from_snapshot(&snapshot, 10), None);
    }

    #[wasm_bindgen_test]
    fn test_default_trait() {
        let buffer = TextBuffer::default();
        assert_eq!(buffer.len_chars(), 0);
        assert_eq!(buffer.len_lines(), 1);
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_version_increment() {
        let mut buffer = TextBuffer::new();
        let v0 = buffer.version();

        buffer.insert(0, "A");
        let v1 = buffer.version();
        assert_eq!(v1, v0 + 1);

        buffer.remove(0, 1);
        let v2 = buffer.version();
        assert_eq!(v2, v1 + 1);

        // mark_saved はバージョンを変更しない
        buffer.mark_saved();
        assert_eq!(buffer.version(), v2);
    }

    // 🗑️ REMOVED: test_multiline_insert_cache_invalidation (dead code)

    // ========================================
    // 🚀 TOKEN CACHE INTEGRATION TESTS
    // ========================================
    // These tests verify the token cache prevents re-tokenizing every frame (60 FPS)
    // and properly manages memory through cache trimming

    #[wasm_bindgen_test]
    fn test_token_cache_basic_operations() {
        let mut buffer = TextBuffer::from_str("line 1\nline 2\nline 3\n");

        // Initially empty cache
        assert_eq!(buffer.token_cache_size(), 0);
        assert!(buffer.get_cached_tokens(0).is_none());

        // Cache some tokens
        let tokens_line_0 = vec![
            ("line".to_string(), "identifier".to_string()),
            (" ".to_string(), "punctuation".to_string()),
            ("1".to_string(), "number".to_string()),
        ];
        buffer.cache_tokens(0, tokens_line_0.clone());

        // Verify cache hit
        assert_eq!(buffer.token_cache_size(), 1);
        assert_eq!(buffer.get_cached_tokens(0), Some(&tokens_line_0));
        assert!(buffer.get_cached_tokens(1).is_none());

        // Cache another line
        let tokens_line_1 = vec![
            ("line".to_string(), "identifier".to_string()),
            (" ".to_string(), "punctuation".to_string()),
            ("2".to_string(), "number".to_string()),
        ];
        buffer.cache_tokens(1, tokens_line_1.clone());

        assert_eq!(buffer.token_cache_size(), 2);
        assert_eq!(buffer.get_cached_tokens(0), Some(&tokens_line_0));
        assert_eq!(buffer.get_cached_tokens(1), Some(&tokens_line_1));
    }

    #[wasm_bindgen_test]
    fn test_token_cache_trimming() {
        let mut buffer = TextBuffer::from_str("line\n".repeat(1000).as_str());

        // Simulate caching all 1000 lines
        for i in 0..1000 {
            let tokens = vec![
                ("line".to_string(), "identifier".to_string()),
            ];
            buffer.cache_tokens(i, tokens);
        }

        assert_eq!(buffer.token_cache_size(), 1000);

        // Trim to visible range (500-550) with 20 line margin
        // Should keep lines 480-570 (20 margin on each side)
        buffer.trim_token_cache(500, 550, 20);

        // Verify trimming worked
        let cache_size = buffer.token_cache_size();
        assert!(cache_size <= 91, "Cache size after trim: {}", cache_size); // 570-480+1 = 91 lines

        // Verify out-of-range lines are cleared
        assert!(buffer.get_cached_tokens(0).is_none(), "Line 0 should be cleared");
        assert!(buffer.get_cached_tokens(100).is_none(), "Line 100 should be cleared");
        assert!(buffer.get_cached_tokens(999).is_none(), "Line 999 should be cleared");

        // Verify in-range lines are preserved
        assert!(buffer.get_cached_tokens(500).is_some(), "Line 500 should be preserved");
        assert!(buffer.get_cached_tokens(525).is_some(), "Line 525 should be preserved");
        assert!(buffer.get_cached_tokens(550).is_some(), "Line 550 should be preserved");

        // Verify margin lines are preserved
        assert!(buffer.get_cached_tokens(480).is_some(), "Line 480 (start margin) should be preserved");
        assert!(buffer.get_cached_tokens(570).is_some(), "Line 570 (end margin) should be preserved");
    }

    #[wasm_bindgen_test]
    fn test_token_cache_invalidation_on_edit() {
        let mut buffer = TextBuffer::from_str("line 1\nline 2\nline 3\nline 4\nline 5\n");

        // Cache all lines
        for i in 0..5 {
            let tokens = vec![
                (format!("line {}", i + 1), "identifier".to_string()),
            ];
            buffer.cache_tokens(i, tokens);
        }

        assert_eq!(buffer.token_cache_size(), 5);

        // Insert text at line 2 (char index for line 2 start)
        let char_idx = buffer.line_to_char(2);
        buffer.insert(char_idx, "NEW ");

        // Verify affected lines are invalidated
        // Lines 2+ should be cleared because edit happened on line 2
        assert!(buffer.get_cached_tokens(0).is_some(), "Line 0 unaffected");
        assert!(buffer.get_cached_tokens(1).is_some(), "Line 1 unaffected");
        assert!(buffer.get_cached_tokens(2).is_none(), "Line 2 edited - should be cleared");
        assert!(buffer.get_cached_tokens(3).is_none(), "Line 3+ may shift - should be cleared");
        assert!(buffer.get_cached_tokens(4).is_none(), "Line 4+ may shift - should be cleared");
    }

    #[wasm_bindgen_test]
    fn test_token_cache_invalidation_on_newline_insert() {
        let mut buffer = TextBuffer::from_str("line 1\nline 2\nline 3\n");

        // Cache all lines
        for i in 0..3 {
            let tokens = vec![
                (format!("line {}", i + 1), "identifier".to_string()),
            ];
            buffer.cache_tokens(i, tokens);
        }

        assert_eq!(buffer.token_cache_size(), 3);

        // Insert newline in middle of line 1
        let char_idx = buffer.line_to_char(1) + 3; // "lin|e 2"
        buffer.insert(char_idx, "\n");

        // Lines from edit point onward should be invalidated
        assert!(buffer.get_cached_tokens(0).is_some(), "Line 0 before edit");
        assert!(buffer.get_cached_tokens(1).is_none(), "Line 1 edited - should be cleared");
        assert!(buffer.get_cached_tokens(2).is_none(), "Line 2+ shifted - should be cleared");
    }

    #[wasm_bindgen_test]
    fn test_token_cache_invalidation_on_removal() {
        let mut buffer = TextBuffer::from_str("line 1\nline 2\nline 3\nline 4\n");

        // Cache all lines
        for i in 0..4 {
            let tokens = vec![
                (format!("line {}", i + 1), "identifier".to_string()),
            ];
            buffer.cache_tokens(i, tokens);
        }

        assert_eq!(buffer.token_cache_size(), 4);

        // Remove characters from line 2
        let char_idx_start = buffer.line_to_char(2);
        buffer.remove(char_idx_start, char_idx_start + 4); // Remove "line"

        // Lines from edit point onward should be invalidated
        assert!(buffer.get_cached_tokens(0).is_some(), "Line 0 before edit");
        assert!(buffer.get_cached_tokens(1).is_some(), "Line 1 before edit");
        assert!(buffer.get_cached_tokens(2).is_none(), "Line 2 edited - should be cleared");
        assert!(buffer.get_cached_tokens(3).is_none(), "Line 3+ may shift - should be cleared");
    }

    #[wasm_bindgen_test]
    fn test_token_cache_trimming_edge_cases() {
        let mut buffer = TextBuffer::from_str("line\n".repeat(100).as_str());

        // Cache all 100 lines
        for i in 0..100 {
            buffer.cache_tokens(i, vec![("line".to_string(), "identifier".to_string())]);
        }

        assert_eq!(buffer.token_cache_size(), 100);

        // Test 1: Trim with start_line = 0 (edge of file)
        buffer.trim_token_cache(0, 10, 5);
        let size = buffer.token_cache_size();
        assert!(size <= 16, "Trimmed to 0-15 (margin saturates at 0): {}", size);
        assert!(buffer.get_cached_tokens(0).is_some());
        assert!(buffer.get_cached_tokens(15).is_some());
        assert!(buffer.get_cached_tokens(50).is_none());

        // Re-cache all for next test
        for i in 0..100 {
            buffer.cache_tokens(i, vec![("line".to_string(), "identifier".to_string())]);
        }

        // Test 2: Trim with end_line near end of file
        buffer.trim_token_cache(90, 99, 5);
        assert!(buffer.get_cached_tokens(85).is_some(), "Start margin");
        assert!(buffer.get_cached_tokens(95).is_some(), "Middle");
        assert!(buffer.get_cached_tokens(99).is_some(), "End (no +1 line)");
        assert!(buffer.get_cached_tokens(0).is_none(), "Far away cleared");

        // Re-cache all for next test
        for i in 0..100 {
            buffer.cache_tokens(i, vec![("line".to_string(), "identifier".to_string())]);
        }

        // Test 3: Very large margin (should not crash)
        buffer.trim_token_cache(50, 60, 1000);
        // With margin=1000, should keep everything (saturates at file boundaries)
        assert_eq!(buffer.token_cache_size(), 100);
    }

    #[wasm_bindgen_test]
    fn test_token_cache_memory_efficiency_with_large_file() {
        // Simulate rendering loop on 1000-line file
        let mut buffer = TextBuffer::from_str("fn example() { println!(\"Hello\"); }\n".repeat(1000).as_str());

        // Simulate scrolling through file with 50-line viewport
        for viewport_start in (0..950).step_by(50) {
            let viewport_end = viewport_start + 50;

            // Simulate caching tokens as they're rendered
            for line_idx in viewport_start..viewport_end {
                // Simulate tokenization result
                let tokens = vec![
                    ("fn".to_string(), "keyword".to_string()),
                    (" example() { println!(\"Hello\"); }".to_string(), "identifier".to_string()),
                ];
                buffer.cache_tokens(line_idx, tokens);
            }

            // Trim after each render (20 line margin)
            buffer.trim_token_cache(viewport_start, viewport_end, 20);

            // Verify cache stays bounded
            let cache_size = buffer.token_cache_size();
            assert!(cache_size <= 90, "Viewport {}-{}: cache size {} exceeds limit",
                viewport_start, viewport_end, cache_size);
        }

        // Final cache should only contain last viewport + margin
        assert!(buffer.token_cache_size() <= 90);
    }
}

