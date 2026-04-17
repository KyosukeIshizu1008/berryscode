# Automated Testing in BerryCode

BerryCode leverages Rust's powerful testing framework to ensure the reliability and robustness of its components. This document outlines the current test suite, including new additions and modifications.

## Table of Contents

1. [Introduction](#introduction)
2. [Testing Framework](#testing-framework)
3. [Test Cases](#test-cases)
   - [Unit Tests](#unit-tests)
   - [Integration Tests](#integration-tests)
4. [New Test Cases](#new-test-cases)
5. [Future Extensions](#future-extensions)

## Introduction

BerryCode is designed to be a self-healing, 100% Rust-based development tool that integrates AI for code generation and error correction. The testing suite plays a crucial role in maintaining the stability and correctness of these features.

## Testing Framework

BerryCode uses the following tools and libraries for automated testing:

- **Rust's Standard Test Suite**: For writing and running unit and integration tests.
- **Wasm-Bindgen Test**: To test web components within the Rust environment.
- **Tokio**: For asynchronous testing of I/O operations.

## Test Cases

### Unit Tests

Unit tests are located in the `src` directory alongside the modules they test. They focus on individual functions and their behavior.

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(add(2, 2), 4);
    }
}
```

### Integration Tests

Integration tests are located in the `tests` directory. They test the interaction between different components of BerryCode.

Example:
```rust
#[tokio::test]
async fn test_cargo_check_integration() {
    let result = run_cargo_check().await.unwrap();

    if result.success {
        println!("✅ Cargo Check: Success");
        println!("Warnings: {}", result.warnings.len());
    } else {
        println!("❌ Cargo Check: Failed");
        for error in &result.errors {
            eprintln!("{}:{}:{} - {}",
                error.file, error.line, error.column, error.message);
        }
    }
}
```

## New Test Cases

### IME Cursor Position Handling Stress Test

**File Path**: `tests/ime_cursor_stress_test.rs`

**Purpose**: To ensure robustness of IME cursor position handling under frequent state changes.

```rust
use wasm_bindgen_test::*;
use web_sys::KeyboardEvent;
use web_sys::KeyboardEventInit;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_ime_cursor_position_handling_during_frequent_state_changes() {
    // Test implementation...
}
```

### Scroll Content Click Robustness Test

**File Path**: `tests/scroll_content_click_robustness_test.rs`

**Purpose**: To ensure robustness of scroll content click interactions in various UI states.

```rust
use wasm_bindgen_test::*;
use web_sys::MouseEvent;
use web_sys::MouseEventInit;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_scroll_content_click_handling_in_various_ui_states() {
    // Test implementation...
}
```

### Focus Management Chaos Test

**File Path**: `tests/focus_management_chaos_test.rs`

**Purpose**: To ensure stability of focus management during rapid UI state transitions.

```rust
use wasm_bindgen_test::*;
use web_sys::FocusEvent;
use web_sys::FocusEventInit;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_focus_management_during_rapid_ui_state_transitions() {
    // Test implementation...
}
```

## Future Extensions

### Incremental Checks

Integrate file watching to automatically run `cargo check` on file changes.

```rust
use crate::native::watcher::{FileWatcher, FileEvent};

pub async fn watch_and_check(repo_path: &str) -> anyhow::Result<()> {
    // Watch implementation...
}
```

### Test Range Optimization

Run only the tests related to changed files to reduce test execution time.

```rust
pub async fn run_tests_for_files(
    changed_files: Vec<String>,
) -> anyhow::Result<TestResult> {
    // Filter and run tests...
}
```

## Debugging Tips

### Log Verification

Set environment variables to control logging levels for better debugging.

```bash
RUST_LOG=debug cargo run
```

### JSON Output Inspection

Directly inspect `cargo check` output in JSON format for detailed error analysis.

```bash
cargo check --message-format=json | jq '.message.message'
```