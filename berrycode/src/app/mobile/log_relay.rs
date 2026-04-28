//! Mobile-runtime log relay.
//!
//! Subprocesses spawned by `runner` (simctl/devicectl/adb logcat) emit
//! lines on stdout/stderr; this module classifies each line by severity
//! and keeps a bounded ring buffer of recent entries for the panel to
//! render. Lifted from `app::run_panel`'s `Severity`/`parse_tracing_line`
//! pattern, but kept independent so the desktop console and the mobile
//! console can evolve separately (mobile gets `adb logcat` priority
//! letters and Bevy panic detection that desktop doesn't need).

use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    /// Bevy / Rust panic — surfaced specially because losing the panic
    /// trace is the #1 mobile-debugging-pain.
    Panic,
    /// Couldn't classify; treat as info-equivalent for filter purposes.
    Unknown,
}

impl LogSeverity {
    #[allow(dead_code)] // surface for future filter UI
    pub fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Panic => "PANIC",
            Self::Unknown => "—",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub severity: LogSeverity,
    pub text: String,
    /// Wall-clock-ish ordering. Not serialised — purely for the UI to
    /// show "X seconds ago" (future "since panic" timeline).
    #[allow(dead_code)]
    pub received_at: Instant,
}

/// Classify a single log line.
///
/// Heuristics, in priority order:
///   1. `panicked at` / `thread 'main' panicked` → Panic
///   2. `adb logcat` priority letter `E/`, `W/`, `I/`, `D/`, `V/` (with
///      a tag immediately following — bare `E/` lines exist in body text)
///   3. Tracing / RUST_LOG style `INFO module: …`, `WARN …`, etc.
///   4. Apple unified log markers from `xcrun simctl launch --console`
///      (`<Error>`, `<Warning>`).
///   5. Default: Unknown.
pub fn classify(line: &str) -> LogSeverity {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return LogSeverity::Unknown;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("panicked at") || lower.contains("thread '") && lower.contains("' panicked") {
        return LogSeverity::Panic;
    }

    // adb logcat: `<priority>/<tag>(  pid):` — we only need the leading
    // `X/`. Require an alphanumeric tag char after the slash so we don't
    // mis-classify e.g. `E/B = m * c²`.
    if let Some(rest) = adb_priority_tail(trimmed) {
        return rest;
    }

    // Apple unified log: lines start with timestamp then `<Error>`/`<Notice>`/etc.
    if trimmed.contains("<Error>") {
        return LogSeverity::Error;
    }
    if trimmed.contains("<Warning>") {
        return LogSeverity::Warn;
    }

    // RUST_LOG / tracing keywords. Search whole-word so the
    // `error_count = 0` substring doesn't promote info to error.
    for token in trimmed.split(|c: char| !c.is_ascii_alphabetic()) {
        match token {
            "ERROR" => return LogSeverity::Error,
            "WARN" | "WARNING" => return LogSeverity::Warn,
            "INFO" => return LogSeverity::Info,
            "DEBUG" => return LogSeverity::Debug,
            "TRACE" => return LogSeverity::Trace,
            _ => {}
        }
    }

    LogSeverity::Unknown
}

/// Return Some(severity) iff `line` opens with `X/Tag(<pid>)` or
/// `X/Tag:` where X is a logcat priority letter. Real logcat output is
/// always one of those two shapes; rejecting other shapes filters out
/// math / regex / file-path collisions like `E/B = m * c^2`.
fn adb_priority_tail(line: &str) -> Option<LogSeverity> {
    let mut chars = line.chars();
    let prio = chars.next()?;
    if chars.next()? != '/' {
        return None;
    }
    let tag_start = chars.next()?;
    if !tag_start.is_ascii_alphanumeric() {
        return None;
    }
    // Walk through the rest of the tag (alnum + `_` + `.`).
    let mut after_tag: Option<char> = None;
    for c in chars.by_ref() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            continue;
        }
        after_tag = Some(c);
        break;
    }
    let after_tag = after_tag?;
    // Real logcat: tag is immediately followed by `(pid)` or `:`. We
    // permit a small amount of whitespace before `(` because some
    // formatters pad tags to a fixed width with spaces.
    let is_logcat_shape = match after_tag {
        '(' | ':' => true,
        c if c.is_whitespace() => chars.find(|c| !c.is_whitespace()) == Some('('),
        _ => false,
    };
    if !is_logcat_shape {
        return None;
    }
    Some(match prio {
        'E' => LogSeverity::Error,
        'W' => LogSeverity::Warn,
        'I' => LogSeverity::Info,
        'D' => LogSeverity::Debug,
        'V' => LogSeverity::Trace,
        _ => return None,
    })
}

/// Bounded log buffer. Append-only from the producer side; the panel
/// reads via `iter()`. Capped at `cap` lines to avoid memory growth on
/// long-running sessions (chatty Bevy apps emit ~thousands of lines per
/// minute). The cap is enforced FIFO — oldest entries drop first.
#[derive(Debug)]
pub struct LogStream {
    buf: VecDeque<LogLine>,
    cap: usize,
}

impl LogStream {
    pub fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(cap.min(1024)),
            cap,
        }
    }

    pub fn push_line(&mut self, raw: String) {
        let severity = classify(&raw);
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(LogLine {
            severity,
            text: raw,
            received_at: Instant::now(),
        });
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.buf.iter()
    }
}

impl Default for LogStream {
    fn default() -> Self {
        // 4k lines is ~hundreds of KB at typical line lengths — small
        // enough to keep in egui's per-frame layout cost without
        // having to virtualise the list, large enough to capture a
        // panic stack inside a chatty session.
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_panic() {
        assert_eq!(
            classify("thread 'main' panicked at 'index out of bounds', src/lib.rs:42"),
            LogSeverity::Panic
        );
        assert_eq!(
            classify("panicked at /path/to/file.rs:1:1"),
            LogSeverity::Panic
        );
    }

    #[test]
    fn classify_adb_logcat_priorities() {
        assert_eq!(
            classify("E/AndroidRuntime( 1234): FATAL EXCEPTION"),
            LogSeverity::Error
        );
        assert_eq!(classify("W/Bevy   ( 1234): something"), LogSeverity::Warn);
        assert_eq!(classify("I/wgpu(1234): adapter ready"), LogSeverity::Info);
        assert_eq!(classify("D/Tag: message"), LogSeverity::Debug);
        assert_eq!(classify("V/Verbose:msg"), LogSeverity::Trace);
    }

    #[test]
    fn classify_does_not_misread_math() {
        // `E/B` is a math expression in a body line, not a logcat header
        // (no tag char after the slash since `B` is followed by ` = `,
        // which is fine — but the logcat shape requires whitespace/`(`/
        // `:` after the tag, and ` = m` does not parse as a tag with `=`
        // separator). Make sure we treat it as Unknown.
        assert_ne!(classify("E/B = m * c^2"), LogSeverity::Error);
    }

    #[test]
    fn classify_tracing_levels() {
        assert_eq!(
            classify("2026-04-29T10:00:00Z INFO bevy_winit: Window created"),
            LogSeverity::Info
        );
        assert_eq!(
            classify("2026-04-29T10:00:00Z WARN bevy_render: low VRAM"),
            LogSeverity::Warn
        );
        assert_eq!(
            classify("2026-04-29T10:00:00Z ERROR app: bad thing"),
            LogSeverity::Error
        );
    }

    #[test]
    fn classify_does_not_promote_substring_match() {
        // Substring `error` inside a longer word should NOT promote.
        assert_eq!(
            classify("error_count = 0 (this is fine)"),
            LogSeverity::Unknown
        );
    }

    #[test]
    fn classify_apple_unified_log() {
        assert_eq!(
            classify("2026-04-29 10:00:00.000  <Error>: Something failed"),
            LogSeverity::Error
        );
        assert_eq!(
            classify("2026-04-29 10:00:00.000  <Warning>: Slow operation"),
            LogSeverity::Warn
        );
    }

    #[test]
    fn log_stream_caps_growth() {
        let mut s = LogStream::new(3);
        s.push_line("a".into());
        s.push_line("b".into());
        s.push_line("c".into());
        s.push_line("d".into());
        assert_eq!(s.len(), 3);
        let texts: Vec<&str> = s.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["b", "c", "d"]);
    }

    #[test]
    fn log_stream_classifies_on_push() {
        let mut s = LogStream::new(8);
        s.push_line("E/AndroidRuntime(1): boom".into());
        s.push_line("INFO foo: ok".into());
        let sevs: Vec<_> = s.iter().map(|l| l.severity).collect();
        assert_eq!(sevs, vec![LogSeverity::Error, LogSeverity::Info]);
    }
}
