#!/bin/bash
# 🚀 完全リビルドスクリプト - すべてのキャッシュをクリアして確実にリフレッシュ

set -e  # エラー時に停止

echo "🧹 Step 1: プロセスの停止..."
# 実行中のTauriプロセスを完全に停止
pkill -f "trunk serve" || true
pkill -f "cargo tauri dev" || true
pkill -f "berry-editor" || true
sleep 2

echo "🗑️  Step 2: すべてのキャッシュを削除..."
# Trunkのキャッシュ
rm -rf .trunk-cache
rm -rf dist

# Cargoのビルドキャッシュ
rm -rf target

# Tauri のキャッシュ
rm -rf src-tauri/target

# WebViewのキャッシュ（macOS）
rm -rf ~/Library/Caches/com.berry.editor
rm -rf ~/Library/WebKit/com.berry.editor

echo "🔨 Step 3: WASM をクリーンビルド..."
trunk clean
trunk build --release

echo "🦀 Step 4: Tauri をクリーンビルド..."
cd src-tauri
cargo clean
cargo build
cd ..

echo "🚀 Step 5: 開発サーバー起動..."
cd src-tauri
cargo tauri dev

echo "✅ 完了！ブラウザウィンドウが開いたら、DevTools で Computed Styles を確認してください。"
