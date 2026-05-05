//! Shell-related helpers shared across modules that build commands
//! for `sh -c` execution.

/// POSIX shell-quote a value so it can be safely interpolated into
/// a `sh -c` script. Wraps the input in single quotes and escapes any
/// embedded single quote via the canonical `'\''` (close, escaped,
/// reopen) pattern.
///
/// Use this anywhere a user-controlled string (file path, identifier,
/// hostname, etc.) is concatenated into a shell command. Without it, a
/// value containing a literal `'` would close the surrounding quoting
/// and let the trailing portion run as a separate shell command — a
/// classic command-injection bug.
///
/// ```
/// use berrycode::common::shell::shell_quote;
/// assert_eq!(shell_quote("plain"), "'plain'");
/// assert_eq!(shell_quote("a'b"), r"'a'\''b'");
/// assert_eq!(shell_quote(""), "''");
/// ```
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_value_round_trips_in_quotes() {
        assert_eq!(shell_quote("hello"), "'hello'");
        assert_eq!(shell_quote("a/b/c.app"), "'a/b/c.app'");
    }

    #[test]
    fn embedded_single_quote_is_escaped() {
        assert_eq!(shell_quote("a'b"), r"'a'\''b'");
        assert_eq!(shell_quote("'leading"), r"''\''leading'");
        assert_eq!(shell_quote("trailing'"), r"'trailing'\'''");
    }

    #[test]
    fn empty_string_yields_two_quotes() {
        // Important so the value still evaluates to an empty positional
        // arg rather than being elided when interpolated.
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn injection_payload_becomes_literal() {
        // The classic injection test: a value that tries to close the
        // wrapping quote and inject a command. After quoting, the
        // payload is contained — `; rm -rf /` ends up *inside* the
        // single-quoted argument, not as a shell statement.
        let evil = "/tmp/foo'; rm -rf /";
        let quoted = shell_quote(evil);
        // Exact-match assertion is the only safe check: substring
        // searches like `!contains("; ")` give false alarms because
        // the literal payload (correctly) lives *inside* the second
        // single-quoted segment of the close/escape/reopen sequence.
        assert_eq!(quoted, r"'/tmp/foo'\''; rm -rf /'");
        // Structural property: the result must always begin and end
        // with a single quote so the entire value is a single sh argv
        // entry. If either end is missing, the trailing portion would
        // be parsed as a separate command.
        assert!(quoted.starts_with('\''));
        assert!(quoted.ends_with('\''));
    }
}
