# BerryEditor Testing Guide

## Overview

BerryEditor has a **comprehensive 3-layer testing strategy**, all written in **100% Rust**.

No JavaScript. No compromises. 🦀

## Test Layers

### Layer 1: Unit Tests (Mathematical Correctness)

**Framework:** `wasm-bindgen-test`
**Test Count:** ~57 tests
**Location:** `tests/*.rs`

**What it tests:**
- ✅ Coordinate calculation (col ↔ x reversibility)
- ✅ IME composition events
- ✅ Virtual scroll (100k+ lines)
- ✅ Focus management
- ✅ Buffer operations

**Run:**
```bash
wasm-pack test --headless --firefox
```

### Layer 2: E2E Tests (Physical Rendering)

**Framework:** `fantoccini` (Rust WebDriver)
**Test Count:** 6 tests
**Location:** `tests/rendering_accuracy.rs`

**What it tests:**
- ✅ **Font rendering accuracy** (1px precision)
- ✅ ASCII character width verification
- ✅ Japanese (wide) character width verification
- ✅ Mixed text rendering
- ✅ Cursor position accuracy
- ✅ Multi-line rendering

**Run:**
```bash
./run_rendering_tests.sh
```

**Prerequisites:**
```bash
# Install geckodriver
brew install geckodriver  # macOS
sudo apt install firefox-geckodriver  # Ubuntu
```

### Layer 3: Backend Tests (Tauri Commands)

**Framework:** `cargo test`
**Location:** `src-tauri/src/*.rs`

**What it tests:**
- ✅ File system operations
- ✅ Native dialogs
- ✅ Git integration
- ✅ Search functionality

**Run:**
```bash
cd src-tauri
cargo test
```

## Quick Start

### Run All Tests (Excluding E2E)

```bash
./run_all_tests.sh
```

This runs:
1. Backend tests (cargo test)
2. Frontend unit tests (wasm-pack test)
3. Skips E2E (requires manual setup)

### Run E2E Tests

```bash
# Option 1: Automatic (recommended)
./run_rendering_tests.sh

# Option 2: Manual
# Terminal 1:
geckodriver --port 4444

# Terminal 2:
cargo tauri dev

# Terminal 3:
cargo test --test rendering_accuracy -- --test-threads=1 --nocapture
```

## Test Details

### Unit Tests

#### Coordinate Fidelity

**File:** `tests/coordinate_consistency_test.rs`

Tests that `col → x → col` conversion is perfectly reversible.

Example:
```rust
#[wasm_bindgen_test]
fn test_ascii_reversibility() {
    let line = "fn main() {";
    for col in 0..=line.len() {
        let x = calculate_x_position(line, col);
        let col_back = get_col_from_x(line, x);
        assert_eq!(col, col_back);  // MUST be reversible
    }
}
```

#### IME Composition

**File:** `tests/ime_composition_e2e_test.rs`

Tests Japanese input handling.

Critical check: No double-input during composition.

#### Virtual Scroll Stress

**File:** `tests/virtual_scroll_stress_test.rs`

Tests 100k+ line file handling.

**Known Bug:** `test_scroll_beyond_end` currently ignored (VirtualScroll bug detected).

#### Focus Management

**File:** `tests/focus_race_condition_test.rs`

Tests focus returns to editor after modal/palette close.

### E2E Tests

#### Font Rendering Accuracy

**File:** `tests/rendering_accuracy.rs`

The **most critical tests** for preventing cursor drift bugs.

**How it works:**
```rust
// 1. Type 10 'W' characters
client.send_keys("WWWWWWWWWW").await?;

// 2. Measure ACTUAL browser rendering
let actual_width = client.execute(
    "return document.querySelector('.berry-editor-line').getBoundingClientRect().width;"
).await?;

// 3. Compare with Rust calculation
let expected_width = CHAR_WIDTH_ASCII * 10.0;

// 4. Assert < 1px drift
assert!((actual_width - expected_width).abs() < 1.0);
```

**Why this matters:**

```
Scenario: macOS font rendering changes by 0.2px per character

Without E2E:
- Rust calc: 80.0px (8px × 10 chars)
- Browser: 82.0px (8.2px × 10 chars)
- Drift: 2.0px
- Result: Cursor clicks are OFF by 2px
- User: "This editor is broken!"

With E2E:
- Test FAILS immediately
- CI sends alert
- Dev adjusts CHAR_WIDTH_ASCII constant
- User: (never notices anything)
```

## Test Output

### Successful Run

```
╔════════════════════════════════════════════════════╗
║  BerryEditor Complete Test Suite (100% Rust)      ║
╚════════════════════════════════════════════════════╝

[1/3] Backend Tests (Tauri Commands)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Backend Tests: 14 passed

[2/3] Frontend Unit Tests (WASM)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Testing: Coordinate Fidelity, IME, Virtual Scroll, Focus...
✅ Frontend Tests: 57 passed

╔════════════════════════════════════════════════════╗
║  Test Summary                                      ║
╚════════════════════════════════════════════════════╝

Backend Tests:                ✅ 14 passed
Frontend Unit Tests:          ✅ 57 passed
E2E Rendering Tests:          ⚠️  0 (run separately)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
TOTAL:                        ✅ 71 tests passed

✅ Frontend: EXCELLENT COVERAGE
✅ Backend: PRODUCTION READY

💡 To run E2E rendering tests:
   ./run_rendering_tests.sh

✨ All quick tests completed!
```

### E2E Test Output

```
🦀 BerryEditor Rendering Accuracy Tests

Starting geckodriver on port 4444...
✅ geckodriver running (PID: 12345)
✅ Tauri dev server ready

🧪 Running Rendering Accuracy Tests...

running 6 tests

ASCII Font Rendering:
  Expected (Rust calc): 80.00px
  Actual (Browser):     80.10px
  Drift:                0.10px

test test_ascii_font_rendering_accuracy ... ok

Japanese Font Rendering:
  Expected (Rust calc): 65.00px
  Actual (Browser):     65.20px
  Drift:                0.20px

test test_japanese_font_rendering_accuracy ... ok

test result: ok. 6 passed; 0 failed; 0 ignored

✅ All rendering accuracy tests passed!
```

## CI/CD Integration

### GitHub Actions

Create `.github/workflows/test.yml`:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Run unit tests
        run: wasm-pack test --headless --firefox
      - name: Run backend tests
        run: cd src-tauri && cargo test

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Install geckodriver
        run: sudo apt install firefox-geckodriver
      - name: Run E2E tests
        run: ./run_rendering_tests.sh
```

## Troubleshooting

### wasm-pack test fails

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install Firefox (required for headless tests)
brew install firefox  # macOS
```

### geckodriver not found

```bash
brew install geckodriver  # macOS
sudo apt install firefox-geckodriver  # Ubuntu
```

### Port 4444 already in use

```bash
# Kill existing geckodriver
pkill geckodriver

# Or use different port
geckodriver --port 4445
```

### Tauri dev won't start

```bash
# Check port 8081
lsof -i :8081

# Kill conflicting process
pkill -f "trunk serve"
```

## Writing New Tests

### Unit Test

```rust
// tests/my_feature_test.rs
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_my_feature() {
    // Your test here
    assert_eq!(1 + 1, 2);
}
```

### E2E Test

```rust
// tests/rendering_accuracy.rs
#[tokio::test]
#[ignore]  // Requires geckodriver
async fn test_my_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Type text
    client.send_keys("Hello").await?;

    // Measure rendering
    let width: f64 = client.execute(
        "return document.querySelector('.berry-editor-line').getBoundingClientRect().width;",
        vec![]
    ).await?.as_f64().unwrap();

    // Assert
    assert!(width > 0.0);

    client.close().await?;
    Ok(())
}
```

## Best Practices

1. **Always run tests before commit**
   ```bash
   ./run_all_tests.sh
   ```

2. **Run E2E tests after major changes**
   ```bash
   ./run_rendering_tests.sh
   ```

3. **Check logs on failure**
   ```bash
   cat /tmp/frontend_test.log
   cat /tmp/backend_test.log
   ```

4. **Update constants if E2E fails**

   If font rendering tests fail, update in `src/core/virtual_editor.rs`:
   ```rust
   const CHAR_WIDTH_ASCII: f64 = 8.0;  // Adjust based on E2E results
   const CHAR_WIDTH_WIDE: f64 = 13.0;  // Adjust based on E2E results
   ```

## Summary

| Test Type | Count | Framework | Runtime | Purpose |
|-----------|-------|-----------|---------|---------|
| Unit | ~57 | wasm-bindgen-test | <10s | Math correctness |
| E2E | 6 | Fantoccini | ~30s | Physical accuracy |
| Backend | ~14 | cargo test | <5s | Tauri commands |
| **Total** | **~77** | **100% Rust** | **<1min** | **Complete coverage** |

---

**All tests are written in Rust. No JavaScript. Ever.** 🦀
