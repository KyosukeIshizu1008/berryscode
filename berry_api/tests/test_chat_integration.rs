/// Integration test: real Ollama server call via LlmClient
///
/// Run with:
///   cargo test -p berry-api --test test_chat_integration -- --nocapture
use berry_api::llm::{LlmClient, Role};
use futures::StreamExt;

// =========================================================
// chat_stream()
// =========================================================

#[tokio::test]
async fn test_chat_stream_simple_response() {
    let client = LlmClient::new().expect("LlmClient::new() failed");

    let mut stream = client
        .chat_stream(
            "Reply with exactly the word PONG and nothing else.".to_string(),
            Role::Coder,
            false,
            None,
        )
        .await
        .expect("chat_stream() failed");

    let mut full_response = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => full_response.push_str(&text),
            Err(e) => panic!("Stream error: {}", e),
        }
    }

    assert!(!full_response.is_empty(), "Got empty response");
    println!("✅ chat_stream response: {}", full_response.trim());
}

// =========================================================
// classify_with_router() — accuracy test
// =========================================================

/// Run all cases and print results. Counts correct/total.
#[tokio::test]
async fn test_router_accuracy() {
    let client = LlmClient::new().expect("LlmClient::new() failed");

    // (message, expected_role, description)
    let cases: &[(&str, Role, &str)] = &[
        // coder
        ("Rustでバグを修正して", Role::Coder, "バグ修正"),
        (
            "この関数にエラーハンドリングを追加して",
            Role::Coder,
            "エラーハンドリング追加",
        ),
        ("パニックしているのを直して", Role::Coder, "パニック修正"),
        (
            "新しいAPIエンドポイントを実装して",
            Role::Coder,
            "新機能実装",
        ),
        ("リファクタリングして", Role::Coder, "リファクタ"),
        (
            "Fix the compilation error in main.rs",
            Role::Coder,
            "compile error",
        ),
        (
            "Implement a binary search function",
            Role::Coder,
            "implementation",
        ),
        // architect
        (
            "システム設計のアドバイスをして",
            Role::Architect,
            "システム設計",
        ),
        (
            "モジュール構成をどう設計すべきか",
            Role::Architect,
            "モジュール設計",
        ),
        (
            "この機能のアーキテクチャを考えて",
            Role::Architect,
            "アーキテクチャ",
        ),
        (
            "How should I structure this codebase?",
            Role::Architect,
            "structure",
        ),
        // vision
        (
            "スクリーンショットのUIを確認して",
            Role::Vision,
            "UI screenshot",
        ),
        ("このレイアウトのズレを指摘して", Role::Vision, "layout"),
        (
            "Check the UI screenshot for alignment issues",
            Role::Vision,
            "screenshot EN",
        ),
        // summarizer
        ("このコードを要約して", Role::Summarizer, "コード要約"),
        ("このエラーログを解説して", Role::Summarizer, "ログ解説"),
        (
            "このファイルが何をしているか教えて",
            Role::Summarizer,
            "ファイル説明",
        ),
        (
            "Explain what this module does",
            Role::Summarizer,
            "explain EN",
        ),
        (
            "Summarize this build output",
            Role::Summarizer,
            "summarize EN",
        ),
        // summarizer — 設計+教えて context trap (should NOT route to architect)
        (
            "このプロジェクトの概要と設計教えて",
            Role::Summarizer,
            "設計+教えて=summarizer",
        ),
        (
            "このプロジェクトの設計を教えて",
            Role::Summarizer,
            "設計を教えて=summarizer",
        ),
        (
            "設計を説明して",
            Role::Summarizer,
            "設計を説明して=summarizer",
        ),
        (
            "Explain the architecture of this project",
            Role::Summarizer,
            "explain architecture EN",
        ),
        // cli_git
        ("コミットメッセージを作って", Role::CliGit, "コミット"),
        ("ブランチを切って", Role::CliGit, "ブランチ"),
        ("Generate a git commit message", Role::CliGit, "commit EN"),
        ("How do I rebase interactively?", Role::CliGit, "rebase EN"),
        // reviewer
        (
            "セキュリティ脆弱性をチェックして",
            Role::Reviewer,
            "セキュリティ",
        ),
        ("このunsafeブロックを監査して", Role::Reviewer, "unsafe監査"),
        (
            "Audit this code for vulnerabilities",
            Role::Reviewer,
            "audit EN",
        ),
        (
            "Check for memory safety issues",
            Role::Reviewer,
            "memory safety EN",
        ),
        // doc_rag
        ("READMEを書いて", Role::DocRag, "README"),
        ("tokioクレートの使い方を教えて", Role::DocRag, "crate使い方"),
        ("Write the API documentation", Role::DocRag, "doc EN"),
    ];

    let mut correct = 0usize;
    let mut total = 0usize;

    for (message, expected, desc) in cases {
        let got = client.classify_with_router(message).await;
        let ok = &got == expected;
        if ok {
            correct += 1;
        }
        total += 1;

        let mark = if ok { "✅" } else { "❌" };
        println!(
            "{} [{}] \"{}\" → {:?}  (expected {:?})",
            mark, desc, message, got, expected
        );
    }

    let accuracy = correct as f64 / total as f64 * 100.0;
    println!("\n📊 Accuracy: {}/{} ({:.1}%)", correct, total, accuracy);

    // Assert minimum 80% accuracy
    assert!(
        accuracy >= 80.0,
        "Router accuracy {:.1}% is below 80% threshold ({}/{})",
        accuracy,
        correct,
        total
    );
}

// =========================================================
// from_router_reply robustness (current parsing behavior)
// =========================================================
// These call the live model with edge-case messages to expose weaknesses
#[tokio::test]
async fn test_router_edge_cases() {
    let client = LlmClient::new().expect("LlmClient::new() failed");

    let edge_cases: &[(&str, Role, &str)] = &[
        // Very short / vague
        ("助けて", Role::Coder, "vague: 助けて"),
        ("直して", Role::Coder, "vague: 直して"),
        // Mixed intent
        (
            "バグを直してドキュメントも更新して",
            Role::Coder,
            "mixed: coder+doc",
        ),
        (
            "設計を確認してコードも書いて",
            Role::Architect,
            "mixed: architect+coder",
        ),
        // Unusual phrasing
        (
            "gitのコミットメッセージをいい感じに",
            Role::CliGit,
            "unusual: commit",
        ),
        ("このクラッシュ何？", Role::Coder, "casual: crash"),
        ("なんかエラー出てる", Role::Coder, "casual: error"),
        // English edge cases
        ("it's broken", Role::Coder, "casual EN: broken"),
        ("help me", Role::Coder, "vague EN"),
        ("write some tests for this", Role::Coder, "tests → coder"),
    ];

    let mut correct = 0usize;
    for (msg, expected, desc) in edge_cases {
        let got = client.classify_with_router(msg).await;
        let ok = &got == expected;
        if ok {
            correct += 1;
        }
        let mark = if ok { "✅" } else { "❌" };
        println!("{} [{}] \"{}\" → {:?}", mark, desc, msg, got);
    }
    let total = edge_cases.len();
    let accuracy = correct as f64 / total as f64 * 100.0;
    println!(
        "\n📊 Edge case accuracy: {}/{} ({:.1}%)",
        correct, total, accuracy
    );

    assert!(
        accuracy >= 70.0,
        "Edge case accuracy {:.1}% is below 70% threshold ({}/{})",
        accuracy,
        correct,
        total
    );
}

// =========================================================
// Context-trap / negation tests (critical correctness checks)
// =========================================================
/// These test the hardest cases: negation, mixed intent, topic≠operation
#[tokio::test]
async fn test_context_traps() {
    let client = LlmClient::new().expect("LlmClient::new() failed");

    let cases: &[(&str, Role, &str)] = &[
        // negation traps: mentioned word is NOT the goal
        (
            "設計は変えなくていい、バグだけ直して",
            Role::Coder,
            "negation: 設計 but goal=fix",
        ),
        (
            "設計はそのままでいいのでエラーを修正して",
            Role::Coder,
            "negation: 設計 but goal=error fix",
        ),
        (
            "コミットはまだしないで。コードを直して",
            Role::Coder,
            "negation: commit but goal=fix",
        ),
        (
            "コミットはまだしないで。まず脆弱性を探して",
            Role::Reviewer,
            "negation: commit, goal=security",
        ),
        (
            "pushする前にセキュリティチェックして",
            Role::Reviewer,
            "before push: security",
        ),
        // topic≠operation traps: topic word ≠ intended action
        (
            "GitのREADMEを書いて",
            Role::DocRag,
            "topic=git but action=write docs",
        ),
        (
            "tokioクレートの使い方を教えて",
            Role::DocRag,
            "crate usage → doc_rag not summarizer",
        ),
        (
            "How do I use the tokio crate?",
            Role::DocRag,
            "crate usage EN → doc_rag",
        ),
        // summarizer vs doc_rag boundary
        (
            "このコードが何をしているか教えて",
            Role::Summarizer,
            "explain code → summarizer not doc",
        ),
        (
            "このエラーの意味を教えて",
            Role::Summarizer,
            "explain error → summarizer",
        ),
        // coder vs reviewer boundary
        (
            "セキュリティの脆弱性を直して",
            Role::Reviewer,
            "fix+security → reviewer",
        ),
        (
            "このunsafeを安全に書き直して",
            Role::Reviewer,
            "rewrite unsafe → reviewer",
        ),
        // coder vs cli_git boundary
        (
            "テストを書いてコミットして",
            Role::Coder,
            "write tests first → coder",
        ),
    ];

    let mut correct = 0usize;
    for (msg, expected, desc) in cases {
        let got = client.classify_with_router(msg).await;
        let ok = &got == expected;
        if ok {
            correct += 1;
        }
        let mark = if ok { "✅" } else { "❌" };
        println!(
            "{} [{}] \"{}\" → {:?}  (expected {:?})",
            mark, desc, msg, got, expected
        );
    }
    let total = cases.len();
    let accuracy = correct as f64 / total as f64 * 100.0;
    println!(
        "\n📊 Context-trap accuracy: {}/{} ({:.1}%)",
        correct, total, accuracy
    );

    assert!(
        accuracy >= 75.0,
        "Context-trap accuracy {:.1}% is below 75% threshold ({}/{})",
        accuracy,
        correct,
        total
    );
}

// =========================================================
// 設計 disambiguation: architect vs summarizer
// =========================================================
/// '設計' + explain-verb  → summarizer
/// '設計' + action-verb   → architect
#[tokio::test]
async fn test_sekkei_disambiguation() {
    let client = LlmClient::new().expect("LlmClient::new() failed");

    let cases: &[(&str, Role, &str)] = &[
        // summarizer — 教えて / 説明して / 概要 dominates
        (
            "このプロジェクトの概要と設計教えて",
            Role::Summarizer,
            "概要と設計+教えて → summarizer",
        ),
        (
            "このプロジェクトの設計を教えて",
            Role::Summarizer,
            "設計+教えて → summarizer",
        ),
        (
            "設計を教えて",
            Role::Summarizer,
            "設計を教えて → summarizer",
        ),
        (
            "設計を説明して",
            Role::Summarizer,
            "設計を説明して → summarizer",
        ),
        (
            "アーキテクチャを教えて",
            Role::Summarizer,
            "アーキテクチャ+教えて → summarizer",
        ),
        (
            "Explain the architecture of this project",
            Role::Summarizer,
            "explain architecture → summarizer",
        ),
        (
            "Describe the current design",
            Role::Summarizer,
            "describe design → summarizer",
        ),
        // architect — して / 考えて / すべきか / 提案して dominates
        (
            "システム設計のアドバイスをして",
            Role::Architect,
            "設計アドバイス → architect",
        ),
        (
            "どんな設計にすべきか",
            Role::Architect,
            "設計すべきか → architect",
        ),
        ("設計を考えて", Role::Architect, "設計を考えて → architect"),
        (
            "設計を提案して",
            Role::Architect,
            "設計を提案して → architect",
        ),
        (
            "How should I design this module?",
            Role::Architect,
            "how to design → architect",
        ),
    ];

    let mut correct = 0usize;
    for (msg, expected, desc) in cases {
        let got = client.classify_with_router(msg).await;
        let ok = &got == expected;
        if ok {
            correct += 1;
        }
        let mark = if ok { "✅" } else { "❌" };
        println!(
            "{} [{}] \"{}\" → {:?}  (expected {:?})",
            mark, desc, msg, got, expected
        );
    }
    let total = cases.len();
    let accuracy = correct as f64 / total as f64 * 100.0;
    println!(
        "\n📊 設計 disambiguation accuracy: {}/{} ({:.1}%)",
        correct, total, accuracy
    );

    assert!(
        accuracy >= 80.0,
        "設計 disambiguation accuracy {:.1}% is below 80% threshold ({}/{})",
        accuracy,
        correct,
        total
    );
}
