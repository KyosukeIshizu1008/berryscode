# Troubleshooting Guide

## Common Issues

### ❌ "Tauri API not available" Error

**Symptoms:**
```
berry_invoke error: read_dir Error: Tauri API not available
```

**Cause:**
You're running in **browser mode** (`trunk serve`) but the code is trying to use **Tauri APIs** (file system, native OS features).

**Solution:**
Run the desktop app instead:

```bash
./run_desktop.sh
# OR
cargo tauri dev
```

**Why this happens:**
- Browser mode: WASM runs in the browser sandbox (no file system access)
- Desktop mode: Tauri provides a native window with full OS APIs

---

### ❌ Font Loading Error: "Failed to decode downloaded font"

**Symptoms:**
```
Failed to decode downloaded font: http://localhost:8081/codicon.ttf
OTS parsing error: invalid sfntVersion: 1008821359
```

**Cause:**
The browser's CORS/MIME type handling is interfering with font loading.

**Solution:**
Run the desktop app:

```bash
./run_desktop.sh
```

The Tauri desktop app handles font loading correctly via the `asset:` protocol.

**Verification:**
- Desktop app: Font loads via `asset://` protocol ✅
- Browser: Font loads via `http://localhost` ⚠️ (may fail due to CORS)

---

### ❌ "No tab data available for rendering"

**Symptoms:**
```
⚠️ No tab data available for rendering
```

**Cause:**
File tree failed to load because Tauri APIs are unavailable in browser mode.

**Solution:**
1. Run desktop app: `./run_desktop.sh`
2. OR manually open a file (Ctrl+O) in browser mode

---

### ❌ WebSocket Connection Failed

**Symptoms:**
```
WebSocket connection to 'ws://localhost:8081/.well-known/trunk/ws' failed
```

**Cause:**
This is Trunk's hot-reload WebSocket. It's non-critical and can be ignored.

**Solution:**
No action needed. This doesn't affect functionality.

---

## Mode Comparison

| Feature | Browser Mode (`trunk serve`) | Desktop App (`cargo tauri dev`) |
|---------|------------------------------|----------------------------------|
| File Tree | ❌ No access | ✅ Full access |
| File Save/Load | ❌ Download only | ✅ Native save |
| Font Loading | ⚠️ CORS issues | ✅ Asset protocol |
| Performance | ⚠️ Good | ✅ Better |
| Canvas Rendering | ✅ Works | ✅ Works |
| LSP Support | ❌ No backend | ✅ Full support |

**Recommendation:** Always use **Desktop App mode** for development.

---

## Quick Start Checklist

1. ✅ Install dependencies:
   ```bash
   cargo install trunk
   cargo install tauri-cli
   ```

2. ✅ Run desktop app:
   ```bash
   ./run_desktop.sh
   ```

3. ✅ Verify:
   - Native window opens (not a browser tab)
   - File tree shows project files
   - No console errors about "Tauri API not available"

---

## Still Having Issues?

1. Check Rust version: `rustc --version` (should be 1.70+)
2. Check Trunk version: `trunk --version` (should be 0.17+)
3. Check Tauri CLI: `cargo tauri --version` (should be 1.5+)
4. Clean and rebuild:
   ```bash
   cargo clean
   rm -rf dist
   cargo tauri dev
   ```

5. Check logs:
   - Desktop app: `/tmp/tauri_dev.log`
   - Browser: Browser DevTools Console
