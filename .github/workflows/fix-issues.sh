#!/bin/bash
# Ollama qwen2.5-coder:32b-instruct-q8_0 で issue を読んで自動修正する
# Build とテストが通るまで繰り返す

set -e

REPOS=("berryscode" "oracleberry" "SplatMap" "sotyping")
MAX_RETRIES=5

echo "🤖 Ollama qwen2.5-coder:32b-instruct-q8_0 で自動修正を開始..."

for repo in "${REPOS[@]}"; do
    echo "📌 $repo の修正を開始..."

    cd /Users/kyosukeishizu/.openclaw/workspace/$repo

    # オープンな issue を取得
    open_issues=$(gh issue list --repo "Oracleberry/$repo" --state=open --json number,title,body,labels --jq '.[] | select(.labels[]? | .name == "auto") | "\(.number):\(.title)\n\(.body)\n---"')

    if [ -z "$open_issues" ]; then
        echo "  ✅ 修正すべき issue なし"
        continue
    fi

    echo "  📝 {number} 個の issue を検出"

    # 修正を繰り返す
    retry_count=0
    while [ $retry_count -lt $MAX_RETRIES ]; do
        echo "  🔄 修正を試行中... ($((retry_count + 1))/$MAX_RETRIES)"

        # issue をテキストファイルに保存
        echo "$open_issues" > /tmp/issue-$(echo $repo | tr '/' '-').txt

        # Ollama で修正を提案
        fix_suggestion=$(echo "$open_issues" | ollama run qwen2.5-coder:32b-instruct-q8_0 --experimental-yolo <<EOF
以下の issue を読んで、修正案を提案してください。

Issue:
$(cat /tmp/issue-$(echo $repo | tr '/' '-').txt)

※ 各リポジトリの言語に合わせて調整してください。

修正案: 
EOF
)

        # 修正を適用
        echo "$fix_suggestion" | ollama run qwen2.5-coder:32b-instruct-q8_0 --experimental-yolo <<EOF
以下の修正案を実装してください。

修正案: 
$fix_suggestion

実装: 
EOF

        # 変更を commit
        git add .
        git commit -m "fix: auto-fix from qwen2.5-coder:32b-instruct-q8_0 (retry $((retry_count + 1)))" || echo "  ⚠️ 変更なし"

        # Build とテストを実行
        echo "  🔍 Build とテストを実行中..."

        if [ -f "package.json" ]; then
            npm ci && npm run build && npm run test
        elif [ -f "Cargo.toml" ]; then
            cargo build --release && cargo test
        elif [ -f "pyproject.toml" ]; then
            pip install -e . && python -m pytest
        elif [ -f "Makefile" ]; then
            make build && make test
        fi

        # テストが成功したら終了
        if [ $? -eq 0 ]; then
            echo "  ✅ Build とテストが成功しました"
            break
        else
            echo "  ⚠️ Build またはテストが失敗しました。再試行..."
            retry_count=$((retry_count + 1))
        fi
    done

    if [ $retry_count -eq $MAX_RETRIES ]; then
        echo "  ❌ 最大試行回数に達しました"
    fi
done

echo "✅ 自動修正が完了しました"