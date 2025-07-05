//! Cross-platform browser launching utilities

use crate::error::{RazError, RazResult};
use std::process::Command;

/// Opens a URL in the default browser or a specified browser
pub fn open_browser(url: &str, browser: Option<&str>) -> RazResult<()> {
    let result = if let Some(browser_cmd) = browser {
        // Use specified browser
        open_with_browser(url, browser_cmd)
    } else {
        // Use default browser
        open_default_browser(url)
    };

    result.map_err(|e| RazError::execution(format!("Failed to open browser: {e}")))
}

/// Opens URL in the system's default browser
fn open_default_browser(url: &str) -> Result<(), std::io::Error> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(&["/C", "start", url]).spawn()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // Try common Linux browsers in order
        if Command::new("xdg-open").arg(url).spawn().is_ok() {
            return Ok(());
        }
        if Command::new("gnome-open").arg(url).spawn().is_ok() {
            return Ok(());
        }
        if Command::new("kde-open").arg(url).spawn().is_ok() {
            return Ok(());
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find a suitable browser launcher",
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Browser launching not supported on this platform",
        ))
    }
}

/// Opens URL with a specific browser
fn open_with_browser(url: &str, browser: &str) -> Result<(), std::io::Error> {
    // Handle common browser aliases
    let browser_cmd = match browser.to_lowercase().as_str() {
        "chrome" | "google-chrome" => {
            #[cfg(target_os = "macos")]
            {
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
            }
            #[cfg(target_os = "windows")]
            {
                "chrome.exe"
            }
            #[cfg(target_os = "linux")]
            {
                "google-chrome"
            }
        }
        "firefox" => {
            #[cfg(target_os = "macos")]
            {
                "/Applications/Firefox.app/Contents/MacOS/firefox"
            }
            #[cfg(target_os = "windows")]
            {
                "firefox.exe"
            }
            #[cfg(target_os = "linux")]
            {
                "firefox"
            }
        }
        "safari" => {
            #[cfg(target_os = "macos")]
            {
                "/Applications/Safari.app/Contents/MacOS/Safari"
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Safari is only available on macOS",
                ));
            }
        }
        "edge" | "microsoft-edge" => {
            #[cfg(target_os = "macos")]
            {
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
            }
            #[cfg(target_os = "windows")]
            {
                "msedge.exe"
            }
            #[cfg(target_os = "linux")]
            {
                "microsoft-edge"
            }
        }
        // If not a known alias, use as-is (could be full path or command in PATH)
        _ => browser,
    };

    Command::new(browser_cmd).arg(url).spawn()?;
    Ok(())
}

/// Extract URL from server output
pub fn extract_server_url(output: &str) -> Option<String> {
    // Common patterns for server URLs
    let patterns = [
        // "Serving at http://..."
        r"(?i)serving\s+at\s+(https?://[^\s]+)",
        // "listening on http://..."
        r"(?i)listening\s+on\s+(https?://[^\s]+)",
        // "Server running on http://..."
        r"(?i)server\s+running\s+on\s+(https?://[^\s]+)",
        // "Available at http://..."
        r"(?i)available\s+at\s+(https?://[^\s]+)",
        // Leptos pattern: "Listening on http://127.0.0.1:3000"
        r"(?i)Listening\s+on\s+(https?://[^\s]+)",
        // Just find any http(s) URL
        r"(https?://(?:localhost|127\.0\.0\.1|\*+):[0-9]+)",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(output) {
                if let Some(url) = captures.get(1) {
                    let mut url_str = url.as_str().to_string();
                    // Replace asterisks pattern with localhost using simple string replacement
                    if url_str.contains('*') {
                        // Replace consecutive asterisks with localhost
                        url_str = url_str
                            .chars()
                            .collect::<String>()
                            .replace("*", "")
                            .replace("://:", "://localhost:");
                        if !url_str.contains("localhost") {
                            url_str = url_str.replace("://", "://localhost");
                        }
                    }
                    return Some(url_str);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_server_url() {
        assert_eq!(
            extract_server_url("Serving at http://localhost:3000"),
            Some("http://localhost:3000".to_string())
        );

        assert_eq!(
            extract_server_url("listening on http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".to_string())
        );

        assert_eq!(
            extract_server_url("Serving at http://*********:3000"),
            Some("http://localhost:3000".to_string())
        );

        assert_eq!(extract_server_url("no url here"), None);
    }
}
