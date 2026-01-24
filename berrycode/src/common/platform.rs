//! Platform detection and configuration
//!
//! This module provides runtime platform detection to enable
//! platform-specific behavior without conditional compilation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Web,
    Desktop,
    iOS,
    Android,
}

impl Platform {
    /// Detect the current platform at runtime
    pub fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Platform::Web
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[cfg(target_os = "ios")]
            {
                Platform::iOS
            }
            #[cfg(target_os = "android")]
            {
                Platform::Android
            }
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            {
                Platform::Desktop
            }
        }
    }

    /// Check if running on a mobile platform
    pub fn is_mobile(&self) -> bool {
        matches!(self, Platform::iOS | Platform::Android)
    }

    /// Check if running in a web browser
    pub fn is_web(&self) -> bool {
        matches!(self, Platform::Web)
    }

    /// Check if running as a native desktop app
    pub fn is_desktop(&self) -> bool {
        matches!(self, Platform::Desktop)
    }

    /// Get platform name as string
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Web => "Web",
            Platform::Desktop => "Desktop",
            Platform::iOS => "iOS",
            Platform::Android => "Android",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        // Should detect as Web when compiled for WASM
        #[cfg(target_arch = "wasm32")]
        assert_eq!(platform, Platform::Web);

        // Should detect as Desktop on standard targets (unless mobile)
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "ios"), not(target_os = "android")))]
        assert_eq!(platform, Platform::Desktop);
    }

    #[test]
    fn test_platform_checks() {
        let web = Platform::Web;
        assert!(web.is_web());
        assert!(!web.is_mobile());
        assert!(!web.is_desktop());

        let ios = Platform::iOS;
        assert!(ios.is_mobile());
        assert!(!ios.is_web());

        let desktop = Platform::Desktop;
        assert!(desktop.is_desktop());
        assert!(!desktop.is_mobile());
    }

    #[test]
    fn test_platform_name() {
        assert_eq!(Platform::Web.name(), "Web");
        assert_eq!(Platform::Desktop.name(), "Desktop");
        assert_eq!(Platform::iOS.name(), "iOS");
        assert_eq!(Platform::Android.name(), "Android");
    }
}
