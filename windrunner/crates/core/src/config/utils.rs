/// Check if a Rust toolchain channel name is valid
pub fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, "stable" | "beta" | "nightly")
        || channel.starts_with("stable-")
        || channel.starts_with("beta-")
        || channel.starts_with("nightly-")
        || channel.starts_with("1.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_channel() {
        assert!(is_valid_channel("stable"));
        assert!(is_valid_channel("beta"));
        assert!(is_valid_channel("nightly"));
        assert!(is_valid_channel("stable-2023-01-01"));
        assert!(is_valid_channel("beta-2023-01-01"));
        assert!(is_valid_channel("nightly-2023-01-01"));
        assert!(is_valid_channel("1.75.0"));
        assert!(is_valid_channel("1.76.0-beta.1"));

        assert!(!is_valid_channel("invalid"));
        assert!(!is_valid_channel("2.0.0")); // Rust 2.x doesn't exist yet
    }
}
