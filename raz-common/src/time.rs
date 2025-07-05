//! Time and duration utilities

use chrono::{DateTime, Local, Utc};
use std::time::{Duration, SystemTime};

/// Time utilities for consistent time handling
pub struct TimeUtils;

impl TimeUtils {
    /// Get current time in UTC
    pub fn now_utc() -> DateTime<Utc> {
        Utc::now()
    }

    /// Get current time in local timezone
    pub fn now_local() -> DateTime<Local> {
        Local::now()
    }

    /// Format a duration in a human-readable way
    pub fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();

        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3600 {
            let mins = secs / 60;
            let secs = secs % 60;
            if secs > 0 {
                format!("{mins}m {secs}s")
            } else {
                format!("{mins}m")
            }
        } else {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            if mins > 0 {
                format!("{hours}h {mins}m")
            } else {
                format!("{hours}h")
            }
        }
    }

    /// Format a timestamp in a consistent way
    pub fn format_timestamp(time: DateTime<Local>) -> String {
        time.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Format a timestamp as relative time (e.g., "2 hours ago")
    pub fn format_relative(time: DateTime<Local>) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(time);

        if duration.num_seconds() < 0 {
            return "in the future".to_string();
        }

        if duration.num_seconds() < 60 {
            "just now".to_string()
        } else if duration.num_minutes() < 60 {
            let mins = duration.num_minutes();
            format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
        } else if duration.num_hours() < 24 {
            let hours = duration.num_hours();
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else if duration.num_days() < 30 {
            let days = duration.num_days();
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else {
            time.format("%Y-%m-%d").to_string()
        }
    }

    /// Convert SystemTime to DateTime<Local>
    pub fn system_time_to_local(time: SystemTime) -> DateTime<Local> {
        let datetime: DateTime<Utc> = time.into();
        datetime.with_timezone(&Local)
    }

    /// Parse ISO 8601 timestamp
    pub fn parse_iso8601(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
    }
}

/// Extension trait for measuring elapsed time
pub trait Elapsed {
    /// Get elapsed time since this instant
    fn elapsed_formatted(&self) -> String;
}

impl Elapsed for std::time::Instant {
    fn elapsed_formatted(&self) -> String {
        TimeUtils::format_duration(self.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(TimeUtils::format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(
            TimeUtils::format_duration(Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(TimeUtils::format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(
            TimeUtils::format_duration(Duration::from_secs(3750)),
            "1h 2m"
        );
    }

    #[test]
    fn test_format_relative() {
        let now = Local::now();
        assert_eq!(TimeUtils::format_relative(now), "just now");

        let two_hours_ago = now - chrono::Duration::hours(2);
        assert_eq!(TimeUtils::format_relative(two_hours_ago), "2 hours ago");
    }
}
