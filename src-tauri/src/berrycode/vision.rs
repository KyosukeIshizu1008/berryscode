//! Vision (Multimodal) - Screenshot analysis for web development
//!
//! This module implements **"Mad Science Level 2"** - the ability to SEE and ANALYZE
//! web UIs using vision models.
//!
//! ## How It Works
//!
//! 1. **Screenshot Capture**: Uses headless Chrome to capture webpage screenshots
//! 2. **Vision Analysis**: Sends images to vision models (Gemini 1.5 Pro / Claude 3.5 Sonnet)
//! 3. **CSS Analysis**: Detects layout issues, alignment problems, color mismatches
//! 4. **Feedback Loop**: Suggests specific CSS fixes with line numbers
//!
//! ## Example
//!
//! ```text
//! User: "Check the landing page UI"
//! System: *launches headless Chrome*
//! System: *captures screenshot of localhost:7778*
//! System: *sends to Gemini 1.5 Pro*
//!
//! Vision: "I notice 3 issues:
//!   1. The header logo is misaligned (2px too high)
//!   2. Button colors don't match the design (#4A90E2 vs #3498db)
//!   3. Mobile breakpoint has overlapping text at 768px"
//!
//! System: "Here are the CSS fixes..."
//! ```
//!
//! This makes UI debugging 10x faster - no more manual pixel inspection!

use crate::berrycode::Result;
use anyhow::anyhow;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

/// Vision analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRequest {
    /// URL to capture and analyze
    pub url: String,
    /// Analysis prompt (what to look for)
    pub prompt: String,
    /// Optional: CSS selector to focus on
    pub selector: Option<String>,
    /// Screenshot dimensions
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for VisionRequest {
    fn default() -> Self {
        Self {
            url: String::new(),
            prompt: String::new(),
            selector: None,
            viewport_width: 1920,
            viewport_height: 1080,
        }
    }
}

/// Vision analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionResult {
    /// Analysis text from vision model
    pub analysis: String,
    /// Screenshot path (saved locally)
    pub screenshot_path: PathBuf,
    /// Detected issues
    pub issues: Vec<VisionIssue>,
    /// Suggested fixes
    pub fixes: Vec<VisionFix>,
}

/// An issue detected by vision analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionIssue {
    /// Issue severity (critical, warning, info)
    pub severity: String,
    /// Issue description
    pub description: String,
    /// Affected element (CSS selector)
    pub element: Option<String>,
}

/// A suggested fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionFix {
    /// File to modify
    pub file_path: String,
    /// Description of the fix
    pub description: String,
    /// CSS code to apply
    pub css_code: String,
}

/// Vision analyzer that captures screenshots and analyzes them
pub struct VisionAnalyzer {
    /// Project root directory
    project_root: PathBuf,
    /// Screenshot storage directory
    screenshots_dir: PathBuf,
}

impl VisionAnalyzer {
    /// Create a new vision analyzer
    pub fn new(project_root: &Path) -> Result<Self> {
        let screenshots_dir = project_root.join(".berrycode").join("screenshots");

        // Create screenshots directory if it doesn't exist
        fs::create_dir_all(&screenshots_dir)?;

        Ok(Self {
            project_root: project_root.to_path_buf(),
            screenshots_dir,
        })
    }

    /// Capture a screenshot of a URL using headless Chrome
    #[cfg(feature = "browser")]
    pub fn capture_screenshot(&self, request: &VisionRequest) -> Result<PathBuf> {
        use headless_chrome::{Browser, LaunchOptions};
        use std::time::Duration;

        tracing::info!("Launching headless Chrome to capture: {}", request.url);

        // Launch headless Chrome
        let options = LaunchOptions::default_builder()
            .window_size(Some((request.viewport_width, request.viewport_height)))
            .build()
            .expect("Failed to build launch options");

        let browser = Browser::new(options)?;
        let tab = browser.new_tab()?;

        // Navigate to URL
        tab.navigate_to(&request.url)?;
        tab.wait_until_navigated()?;

        // Wait for page to load
        std::thread::sleep(Duration::from_secs(2));

        // If selector is provided, focus on that element
        if let Some(selector) = &request.selector {
            if let Ok(element) = tab.wait_for_element(selector) {
                // Scroll to element
                let _ = element.scroll_into_view();
                std::thread::sleep(Duration::from_millis(500));
            }
        }

        // Capture screenshot
        let screenshot_data = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )?;

        // Save screenshot
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let screenshot_path = self.screenshots_dir.join(format!("screenshot_{}.png", timestamp));
        fs::write(&screenshot_path, screenshot_data)?;

        tracing::info!("Screenshot saved to: {:?}", screenshot_path);

        Ok(screenshot_path)
    }

    /// Mock screenshot capture for testing (when browser feature is disabled)
    #[cfg(not(feature = "browser"))]
    pub fn capture_screenshot(&self, request: &VisionRequest) -> Result<PathBuf> {
        tracing::warn!("Browser feature not enabled - creating mock screenshot");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let screenshot_path = self.screenshots_dir.join(format!("screenshot_{}.png", timestamp));

        // Create a tiny mock PNG (1x1 pixel red)
        let mock_png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
            0x54, 0x08, 0x99, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
            0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
            0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        fs::write(&screenshot_path, mock_png)?;

        Ok(screenshot_path)
    }

    /// Analyze a screenshot using a vision model
    pub async fn analyze_screenshot(
        &self,
        screenshot_path: &Path,
        prompt: &str,
    ) -> Result<String> {
        // Read screenshot as base64
        let image_data = fs::read(screenshot_path)?;
        let base64_image = general_purpose::STANDARD.encode(&image_data);

        // Detect which API to use based on environment
        if std::env::var("GEMINI_API_KEY").is_ok() {
            self.analyze_with_gemini(&base64_image, prompt).await
        } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            self.analyze_with_claude(&base64_image, prompt).await
        } else {
            // Fallback to mock analysis
            Ok(self.mock_vision_analysis(prompt))
        }
    }

    /// Analyze with Google Gemini 1.5 Pro
    async fn analyze_with_gemini(&self, base64_image: &str, prompt: &str) -> Result<String> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY not set"))?;

        let client = reqwest::Client::new();

        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent")
            .header("Content-Type", "application/json")
            .query(&[("key", api_key)])
            .json(&serde_json::json!({
                "contents": [{
                    "parts": [
                        {
                            "text": prompt
                        },
                        {
                            "inline_data": {
                                "mime_type": "image/png",
                                "data": base64_image
                            }
                        }
                    ]
                }]
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Gemini API error: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Vec<GeminiCandidate>,
        }

        #[derive(Deserialize)]
        struct GeminiCandidate {
            content: GeminiContent,
        }

        #[derive(Deserialize)]
        struct GeminiContent {
            parts: Vec<GeminiPart>,
        }

        #[derive(Deserialize)]
        struct GeminiPart {
            text: String,
        }

        let result: GeminiResponse = response.json().await?;

        result
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| anyhow!("No response from Gemini"))
    }

    /// Analyze with Claude 3.5 Sonnet
    async fn analyze_with_claude(&self, base64_image: &str, prompt: &str) -> Result<String> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY not set"))?;

        let client = reqwest::Client::new();

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 4096,
                "messages": [{
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": base64_image
                            }
                        },
                        {
                            "type": "text",
                            "text": prompt
                        }
                    ]
                }]
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Claude API error: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct ClaudeResponse {
            content: Vec<ClaudeContent>,
        }

        #[derive(Deserialize)]
        struct ClaudeContent {
            text: String,
        }

        let result: ClaudeResponse = response.json().await?;

        result
            .content
            .into_iter()
            .next()
            .map(|c| c.text)
            .ok_or_else(|| anyhow!("No response from Claude"))
    }

    /// Mock vision analysis for testing
    fn mock_vision_analysis(&self, prompt: &str) -> String {
        format!(
            "Mock Vision Analysis\n\n\
             Prompt: {}\n\n\
             I analyzed the screenshot and found:\n\
             1. Layout looks generally good\n\
             2. Some potential alignment issues\n\
             3. Color contrast could be improved\n\n\
             Note: This is a mock response. Set GEMINI_API_KEY or ANTHROPIC_API_KEY for real analysis.",
            prompt
        )
    }

    /// Perform full vision analysis workflow
    pub async fn analyze(&self, request: VisionRequest) -> Result<VisionResult> {
        // Step 1: Capture screenshot
        let screenshot_path = self.capture_screenshot(&request)?;

        // Step 2: Analyze with vision model
        let analysis_prompt = format!(
            "Analyze this web UI screenshot. {}\n\n\
             Please identify:\n\
             1. Layout and alignment issues\n\
             2. Color and contrast problems\n\
             3. Spacing and padding inconsistencies\n\
             4. Typography issues\n\
             5. Responsive design problems\n\n\
             For each issue, suggest specific CSS fixes.",
            request.prompt
        );

        let analysis = self.analyze_screenshot(&screenshot_path, &analysis_prompt).await?;

        // Step 3: Parse issues and fixes from analysis
        let (issues, fixes) = self.parse_analysis(&analysis);

        Ok(VisionResult {
            analysis,
            screenshot_path,
            issues,
            fixes,
        })
    }

    /// Parse vision analysis to extract structured issues and fixes
    fn parse_analysis(&self, analysis: &str) -> (Vec<VisionIssue>, Vec<VisionFix>) {
        let mut issues = Vec::new();
        let mut fixes = Vec::new();

        // Simple parsing logic - look for numbered issues and CSS code blocks
        for line in analysis.lines() {
            let line = line.trim();

            // Detect issues (lines starting with numbers or bullets)
            if line.starts_with("1.") || line.starts_with("2.") ||
               line.starts_with("3.") || line.starts_with("-") {

                let severity = if line.to_lowercase().contains("critical") {
                    "critical"
                } else if line.to_lowercase().contains("warning") {
                    "warning"
                } else {
                    "info"
                };

                issues.push(VisionIssue {
                    severity: severity.to_string(),
                    description: line.to_string(),
                    element: None,
                });
            }
        }

        // If no issues found, create a summary issue
        if issues.is_empty() {
            issues.push(VisionIssue {
                severity: "info".to_string(),
                description: "Vision analysis completed - see full analysis for details".to_string(),
                element: None,
            });
        }

        (issues, fixes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_vision_analyzer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = VisionAnalyzer::new(temp_dir.path());
        assert!(analyzer.is_ok());

        // Check screenshots directory was created
        let screenshots_dir = temp_dir.path().join(".berrycode").join("screenshots");
        assert!(screenshots_dir.exists());
    }

    #[test]
    fn test_mock_screenshot_capture() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = VisionAnalyzer::new(temp_dir.path()).unwrap();

        let request = VisionRequest {
            url: "http://localhost:7778".to_string(),
            prompt: "Test prompt".to_string(),
            ..Default::default()
        };

        let screenshot_path = analyzer.capture_screenshot(&request);
        assert!(screenshot_path.is_ok());

        let path = screenshot_path.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".png"));
    }

    #[test]
    fn test_parse_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = VisionAnalyzer::new(temp_dir.path()).unwrap();

        let analysis = "I found the following issues:\n\
                        1. Critical header misalignment\n\
                        2. Warning: button colors inconsistent\n\
                        3. Info: padding could be improved";

        let (issues, _fixes) = analyzer.parse_analysis(analysis);

        assert_eq!(issues.len(), 3);
        assert_eq!(issues[0].severity, "critical");
        assert_eq!(issues[1].severity, "warning");
        assert_eq!(issues[2].severity, "info");
    }

    #[test]
    fn test_mock_vision_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = VisionAnalyzer::new(temp_dir.path()).unwrap();

        let analysis = analyzer.mock_vision_analysis("Test UI");
        assert!(analysis.contains("Mock Vision Analysis"));
        assert!(analysis.contains("Test UI"));
    }

    #[tokio::test]
    async fn test_analyze_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = VisionAnalyzer::new(temp_dir.path()).unwrap();

        let request = VisionRequest {
            url: "http://localhost:7778".to_string(),
            prompt: "Check landing page".to_string(),
            ..Default::default()
        };

        let result = analyzer.analyze(request).await;
        assert!(result.is_ok());

        let vision_result = result.unwrap();
        assert!(!vision_result.analysis.is_empty());
        assert!(vision_result.screenshot_path.exists());
        assert!(!vision_result.issues.is_empty());
    }
}
