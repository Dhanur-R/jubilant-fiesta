use std::time::Duration;

/// Configuration constants for the linkr application
pub struct Config;

impl Config {
    /// Number of retry attempts when generating unique short codes
    pub const SHORT_CODE_RETRY_ATTEMPTS: usize = 5;

    /// Number of days of inactivity before links are deleted (1 year)
    pub const LINK_INACTIVE_DAYS: i64 = 365;

    /// Maximum allowed URL length
    pub const URL_MAX_LENGTH: usize = 2048;

    /// Maximum number of requests per time window for rate limiting
    pub const RATE_LIMIT_MAX_REQUESTS: usize = 10;

    /// Rate limit time window in seconds
    pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

    /// Cleanup task interval in hours
    pub const CLEANUP_INTERVAL_HOURS: u64 = 24;

    /// Default port if not specified in environment
    pub const DEFAULT_PORT: u16 = 3000;

    /// Default RUST_LOG value
    pub const DEFAULT_RUST_LOG: &'static str = "linkr=info,tower_http=info";

    /// Get rate limit window as Duration
    pub fn rate_limit_window() -> Duration {
        Duration::from_secs(Self::RATE_LIMIT_WINDOW_SECS)
    }

    /// Get cleanup interval as Duration
    pub fn cleanup_interval() -> Duration {
        Duration::from_secs(Self::CLEANUP_INTERVAL_HOURS * 60 * 60)
    }
}
