//! Web scraping utilities for aider

use crate::berrycode::Result;
use scraper::{Html, Selector};
use reqwest::blocking::Client;
use anyhow::anyhow;

/// Scrape content from a URL
pub fn scrape_url(url: &str) -> Result<String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; AiderBot/1.0)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }

    let body = response.text()?;
    let document = Html::parse_document(&body);

    // Try to extract main content
    let content = extract_main_content(&document);

    Ok(content)
}

/// Extract main content from HTML document
fn extract_main_content(document: &Html) -> String {
    let mut content = String::new();

    // Try common content selectors in order of preference
    let selectors = vec![
        "article",
        "main",
        ".content",
        "#content",
        ".post",
        ".article",
        "body",
    ];

    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if !text.trim().is_empty() {
                    content.push_str(&text);
                    content.push('\n');

                    // If we found substantial content, stop
                    if content.len() > 100 {
                        break;
                    }
                }
            }

            if content.len() > 100 {
                break;
            }
        }
    }

    // Fallback: extract all text
    if content.trim().is_empty() {
        content = document.root_element().text().collect::<Vec<_>>().join(" ");
    }

    // Clean up whitespace
    content = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    content
}

/// Extract code blocks from a URL
pub fn scrape_code_from_url(url: &str) -> Result<Vec<String>> {
    let client = Client::new();
    let response = client.get(url).send()?;
    let body = response.text()?;
    let document = Html::parse_document(&body);

    let mut code_blocks = Vec::new();

    // Try to find code blocks
    let code_selectors = vec!["pre code", "code", "pre", ".highlight"];

    for selector_str in code_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let code = element.text().collect::<Vec<_>>().join("");
                if !code.trim().is_empty() && code.len() > 10 {
                    code_blocks.push(code);
                }
            }
        }
    }

    Ok(code_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires network access
    fn test_scrape_url() {
        let result = scrape_url("https://example.com");
        assert!(result.is_ok());
    }
}
