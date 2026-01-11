//! Virtual Scrolling Implementation
//!
//! Renders only visible lines for optimal performance with large files.

/// Virtual scrolling manager for efficient rendering of large files
#[derive(Debug, Clone)]
pub struct VirtualScroll {
    /// Total number of lines in the document
    total_lines: usize,
    /// Height of the viewport in pixels
    viewport_height: f64,
    /// Height of each line in pixels
    line_height: f64,
    /// Current scroll position (top of viewport)
    scroll_top: f64,
    /// Cached visible range (start_line, end_line)
    visible_range: (usize, usize),
    /// ✅ IntelliJ Pro: Fixed overscan for WASM compatibility
    overscan: usize,
    /// ✅ Track scroll position for prefetch calculation
    last_scroll_pos: f64,
    scroll_velocity: f64,  // Estimated lines per second
    /// ✅ IntelliJ Pro: Prefetch range for async syntax highlighting
    prefetch_range: (usize, usize),
}

impl VirtualScroll {
    /// Create a new virtual scroll manager
    pub fn new(total_lines: usize, viewport_height: f64, line_height: f64) -> Self {
        let mut vs = Self {
            total_lines,
            viewport_height,
            line_height,
            scroll_top: 0.0,
            visible_range: (0, 0),
            overscan: 10,  // Fixed overscan for WASM compatibility
            last_scroll_pos: 0.0,
            scroll_velocity: 0.0,
            prefetch_range: (0, 0),
        };
        vs.calculate_visible_range();
        vs
    }

    /// Update scroll position (WASM-compatible version)
    pub fn set_scroll_top(&mut self, scroll_top: f64) {
        // ✅ FIX: Calculate maximum scroll position to prevent scrolling beyond content
        // 2行分の余裕を追加
        let content_height = self.total_lines as f64 * self.line_height;
        let max_scroll = (content_height - self.viewport_height + 2.0 * self.line_height).max(0.0);

        // ✅ FIX: Clamp scroll position to [0, max_scroll] range
        let new_scroll = scroll_top.max(0.0).min(max_scroll);

        // Estimate scroll velocity based on delta (simplified for WASM)
        let scroll_delta = (new_scroll - self.last_scroll_pos) / self.line_height;
        self.scroll_velocity = scroll_delta;

        self.last_scroll_pos = new_scroll;
        self.scroll_top = new_scroll;

        self.calculate_visible_range();
        self.calculate_prefetch_range();
    }

    /// Update the viewport height
    pub fn set_viewport_height(&mut self, viewport_height: f64) {
        self.viewport_height = viewport_height;
        self.calculate_visible_range();
    }

    /// Update the total number of lines
    pub fn set_total_lines(&mut self, total_lines: usize) {
        self.total_lines = total_lines;
        self.calculate_visible_range();
    }

    /// Calculate the visible range of lines
    fn calculate_visible_range(&mut self) {
        if self.total_lines == 0 {
            self.visible_range = (0, 0);
            return;
        }

        // Calculate first visible line
        let first_visible_raw = (self.scroll_top / self.line_height).floor() as usize;

        // ✅ FIX: Clamp first_visible to prevent index out of bounds when scrolled beyond end
        // Bug: If scroll_top exceeds total_height, first_visible could be > total_lines
        // Example: scroll_top=2,100,000, line_height=20, total_lines=100,000
        //   -> first_visible_raw = 105,000 (INVALID!)
        //   -> Must clamp to max valid line index (total_lines - 1)
        let first_visible = first_visible_raw.min(self.total_lines.saturating_sub(1));

        // Calculate last visible line
        let visible_lines = (self.viewport_height / self.line_height).ceil() as usize;
        let last_visible = (first_visible + visible_lines).min(self.total_lines);

        // Apply overscan
        let start = first_visible.saturating_sub(self.overscan);
        let end = (last_visible + self.overscan).min(self.total_lines);

        self.visible_range = (start, end);
    }


    /// ✅ IntelliJ Pro: Calculate prefetch range for async syntax highlighting
    /// Prefetches lines ahead in scroll direction for zero-latency rendering
    fn calculate_prefetch_range(&mut self) {
        if self.total_lines == 0 {
            self.prefetch_range = (0, 0);
            return;
        }

        let (vis_start, vis_end) = self.visible_range;

        // Determine scroll direction and prefetch ahead
        if self.scroll_velocity > 5.0 {
            // Scrolling down: prefetch lines below
            let prefetch_amount = (self.scroll_velocity.abs() * 0.5).ceil() as usize;
            let prefetch_start = vis_end;
            let prefetch_end = (vis_end + prefetch_amount).min(self.total_lines);
            self.prefetch_range = (prefetch_start, prefetch_end);
        } else if self.scroll_velocity < -5.0 {
            // Scrolling up: prefetch lines above
            let prefetch_amount = (self.scroll_velocity.abs() * 0.5).ceil() as usize;
            let prefetch_start = vis_start.saturating_sub(prefetch_amount);
            let prefetch_end = vis_start;
            self.prefetch_range = (prefetch_start, prefetch_end);
        } else {
            // Not scrolling: no prefetch needed
            self.prefetch_range = (0, 0);
        }
    }

    /// Get the current visible range (inclusive start, exclusive end)
    pub fn visible_range(&self) -> (usize, usize) {
        self.visible_range
    }

    /// ✅ IntelliJ Pro: Get prefetch range for async syntax highlighting
    /// Returns lines that should be prefetched ahead of visible range
    pub fn prefetch_range(&self) -> (usize, usize) {
        self.prefetch_range
    }

    /// ✅ IntelliJ Pro: Get current scroll velocity (lines per second)
    pub fn scroll_velocity(&self) -> f64 {
        self.scroll_velocity
    }

    /// Get the Y offset for a specific line number
    pub fn get_line_offset(&self, line: usize) -> f64 {
        line as f64 * self.line_height
    }

    /// Get the total scrollable height
    pub fn total_height(&self) -> f64 {
        self.total_lines as f64 * self.line_height
    }

    /// Check if a line is currently visible
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.visible_range.0 && line < self.visible_range.1
    }

    /// Get the line at a specific Y coordinate
    pub fn line_at_y(&self, y: f64) -> usize {
        let line = ((self.scroll_top + y) / self.line_height).floor() as usize;
        line.min(self.total_lines.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_scroll_creation() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        assert_eq!(vs.total_lines, 1000);
        assert_eq!(vs.viewport_height, 600.0);
        assert_eq!(vs.line_height, 20.0);
    }

    #[test]
    fn test_visible_range_calculation() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        let (start, end) = vs.visible_range();

        // With overscan of 5, should render from line 0 to ~40
        // (600px / 20px = 30 visible lines, +5 overscan on each side)
        assert_eq!(start, 0);
        assert!(end > 30 && end <= 40);
    }

    #[test]
    fn test_scroll_position_update() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);

        // Scroll down by 200px (10 lines)
        vs.set_scroll_top(200.0);
        let (start, end) = vs.visible_range();

        // ✅ Fixed: overscan is actually 10, not 5
        // first_visible = 200px / 20px = 10
        // start = 10 - 10 = 0
        // Should start at line 0 (10 - 10 overscan)
        assert_eq!(start, 0);
        assert!(end >= 40 && end <= 50); // 10 + 30 visible + 10 overscan
    }

    #[test]
    fn test_line_offset() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert_eq!(vs.get_line_offset(0), 0.0);
        assert_eq!(vs.get_line_offset(10), 200.0);
        assert_eq!(vs.get_line_offset(50), 1000.0);
    }

    #[test]
    fn test_total_height() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        assert_eq!(vs.total_height(), 20000.0);
    }

    #[test]
    fn test_is_line_visible() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert!(vs.is_line_visible(0));
        assert!(vs.is_line_visible(30));
        assert!(!vs.is_line_visible(500));
    }

    #[test]
    fn test_line_at_y() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert_eq!(vs.line_at_y(0.0), 0);
        assert_eq!(vs.line_at_y(20.0), 1);
        assert_eq!(vs.line_at_y(100.0), 5);
    }

    #[test]
    fn test_empty_document() {
        let vs = VirtualScroll::new(0, 600.0, 20.0);
        assert_eq!(vs.visible_range(), (0, 0));
        assert_eq!(vs.total_height(), 0.0);
    }

    #[test]
    fn test_viewport_resize() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);
        let (_, end1) = vs.visible_range();

        vs.set_viewport_height(1200.0);
        let (_, end2) = vs.visible_range();

        // Larger viewport should show more lines
        assert!(end2 > end1);
    }

    #[test]
    fn test_negative_scroll_clamping() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);
        vs.set_scroll_top(-100.0);

        // Should clamp to 0
        assert_eq!(vs.scroll_top, 0.0);
        assert_eq!(vs.visible_range().0, 0);
    }
}
