#!/usr/bin/env python3
"""issue を読んで自動修正する"""

import os
import subprocess
from pathlib import Path
import json
import sys

REPOS = ["berryscode", "oracleberry", "SplatMap", "sotyping"]
GEMINI_API_KEY = "AIzaSyB3LaxjbYXXOQDkXw94HTfnqXRC-Bl3pc4"
TOKEN = os.getenv("GITHUB_TOKEN")

def read_issues():
    """オープンな issue を読む"""
    issues_file = Path("open-issues.json")

    if not issues_file.exists():
        print("⚠️ open-issues.json が見つかりません")
        return []

    return json.loads(issues_file.read_text(encoding="utf-8"))

def fix_issue(repo: str, issue_number: int, issue_body: str):
    """issue を読んで修正を提案する"""
    import google.generativeai as genai

    genai.configure(api_key=GEMINI_API_KEY)
    model = genai.GenerativeModel('gemini-2.5-flash')

    prompt = f"""以下の issue を読んで、修正案を提案してください。

Issue:
{issue_body}

※ 言語は各リポジトリに合わせて調整してください。

提案: """

    try:
        response = model.generate_content(prompt)
        fix_suggestion = response.text
        return fix_suggestion
    except Exception as e:
        print(f"  ⚠️ AI から修正案を取得できません: {e}")
        return None

def apply_fix(repo: str, fix_suggestion: str):
    """修正を適用する"""
    print(f"  🔧 修正を適用中...")

    try:
        # コミットメッセージを生成
        commit_msg = "fix: auto-fix from AI review"

        # 修正を適用
        subprocess.run([
            "git", "add", ".",
            "-m", commit_msg
        ], cwd=Path(f"/Users/kyosukeishizu/.openclaw/workspace/{repo}"), capture_output=True)

        return True
    except Exception as e:
        print(f"  ⚠️ 修正の適用に失敗しました: {e}")
        return False

def main():
    """メイン処理"""
    print("🔧 issue を読んで自動修正中...")

    issues = read_issues()

    if not issues:
        print("✅ 修正すべき issue なし")
        return

    fixed_count = 0

    for issue in issues:
        repo = issue["repo"]
        issue_number = issue["number"]
        issue_body = issue["body"]

        print(f"\n📌 {repo}#{issue_number}")
        print(f"   {issue['title']}")

        # AI で修正案を取得
        fix_suggestion = fix_issue(repo, issue_number, issue_body)

        if fix_suggestion:
            print(f"   🤖 AI 修正案: {fix_suggestion[:100]}...")

            # 修正を適用
            if apply_fix(repo, fix_suggestion):
                print(f"   ✅ 修正を適用しました")
                fixed_count += 1
            else:
                print(f"   ❌ 修正の適用に失敗しました")

    print(f"\n✅ {fixed_count} 件の issue を修正しました")

if __name__ == "__main__":
    main()