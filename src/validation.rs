use std::net::IpAddr;
use url::Url;

use crate::config::Config;

/// Normalizes a URL to a canonical form for consistent storage and deduplication.
///
/// - Auto-prepends `https://` if no scheme is provided
/// - Lowercases scheme and host
/// - Removes default ports (80 for http, 443 for https)
/// - Removes fragments (#section)
/// - Strips trailing slash on root path (no query)
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Auto-prepend https:// if no scheme
    let url_str = if !trimmed.contains("://") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };

    // Parse and reconstruct for canonical form
    let Ok(mut parsed) = Url::parse(&url_str) else {
        return url_str;
    };

    // Remove default ports
    if matches!(
        (parsed.scheme(), parsed.port()),
        ("http", Some(80)) | ("https", Some(443))
    ) {
        let _ = parsed.set_port(None);
    }

    // Remove fragment
    parsed.set_fragment(None);

    let mut result = parsed.to_string();

    // Strip trailing slash for root path with no query
    // (Url::parse always adds "/" to root, making example.com/ and example.com equivalent)
    if parsed.path() == "/" && parsed.query().is_none() {
        result.truncate(result.len() - 1);
    }

    result
}

/// Validates a URL for shortening
///
/// Checks:
/// - URL is not empty
/// - Length is within limits
/// - Starts with http:// or https://
/// - Has a valid host
/// - Host is not private/internal
/// - Port is standard (80 or 443)
pub fn validate_url(url_str: &str) -> Result<(), String> {
    // Check if empty
    let url_str = url_str.trim();
    if url_str.is_empty() {
        return Err("URL cannot be empty.".to_string());
    }

    // Check length
    if url_str.len() > Config::URL_MAX_LENGTH {
        return Err(format!(
            "URL is too long (max {} characters).",
            Config::URL_MAX_LENGTH
        ));
    }

    // Check protocol
    if !(url_str.starts_with("http://") || url_str.starts_with("https://")) {
        return Err("Enter a valid http(s) URL.".to_string());
    }

    // Parse URL
    let parsed_url = Url::parse(url_str).map_err(|_| "Invalid URL format.".to_string())?;

    // Check host
    let host = parsed_url
        .host_str()
        .ok_or_else(|| "URL must have a valid host.".to_string())?;

    // Check if host is private/internal
    if is_private_or_internal(host) {
        return Err("Cannot shorten internal/private URLs.".to_string());
    }

    // Check port
    if let Some(port) = parsed_url.port() {
        if port != 80 && port != 443 {
            return Err("Cannot shorten URLs with non-standard ports.".to_string());
        }
    }

    Ok(())
}

/// Checks if a host is private, internal, or local
///
/// Returns true for:
/// - localhost
/// - .local domains
/// - Private IP ranges (10.x, 172.16-31.x, 192.168.x)
/// - Loopback addresses (127.x)
/// - Link-local addresses (169.254.x)
/// - 0.0.0.0
fn is_private_or_internal(host: &str) -> bool {
    // Check for localhost
    if host == "localhost" || host == "0.0.0.0" {
        return true;
    }

    // Check for .local domain
    if host.ends_with(".local") {
        return true;
    }

    // Try to parse as IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        // Use standard library methods for IP classification
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback()      // 127.x.x.x
                    || ipv4.is_private()     // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
                    || ipv4.is_link_local()  // 169.254.x.x
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() || ipv6.is_unique_local() || ipv6.is_multicast()
            }
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_https_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("https://example.com/path").is_ok());
        assert!(validate_url("https://subdomain.example.com").is_ok());
    }

    #[test]
    fn test_valid_http_url() {
        assert!(validate_url("http://example.com").is_ok());
    }

    #[test]
    fn test_empty_url_rejected() {
        assert!(validate_url("").is_err());
        assert!(validate_url("   ").is_err());
    }

    #[test]
    fn test_too_long_url_rejected() {
        let long_url = format!("https://example.com/{}", "a".repeat(3000));
        assert!(validate_url(&long_url).is_err());
    }

    #[test]
    fn test_localhost_rejected() {
        assert!(validate_url("http://localhost").is_err());
        assert!(validate_url("http://localhost:3000").is_err());
    }

    #[test]
    fn test_private_ip_10_rejected() {
        assert!(validate_url("http://10.0.0.1").is_err());
        assert!(validate_url("http://10.1.2.3").is_err());
        assert!(validate_url("http://10.255.255.255").is_err());
    }

    #[test]
    fn test_private_ip_192_168_rejected() {
        assert!(validate_url("http://192.168.1.1").is_err());
        assert!(validate_url("http://192.168.0.1").is_err());
    }

    #[test]
    fn test_private_ip_172_16_31_rejected() {
        assert!(validate_url("http://172.16.0.1").is_err());
        assert!(validate_url("http://172.20.1.1").is_err());
        assert!(validate_url("http://172.31.255.255").is_err());
    }

    #[test]
    fn test_link_local_169_254_rejected() {
        assert!(validate_url("http://169.254.0.1").is_err());
    }

    #[test]
    fn test_loopback_127_rejected() {
        assert!(validate_url("http://127.0.0.1").is_err());
        assert!(validate_url("http://127.0.1.1").is_err());
    }

    #[test]
    fn test_local_domain_rejected() {
        assert!(validate_url("http://machine.local").is_err());
        assert!(validate_url("http://server.local/path").is_err());
    }

    #[test]
    fn test_zero_address_rejected() {
        assert!(validate_url("http://0.0.0.0").is_err());
    }

    #[test]
    fn test_non_standard_port_rejected() {
        assert!(validate_url("http://example.com:8080").is_err());
        assert!(validate_url("https://example.com:3000").is_err());
    }

    #[test]
    fn test_standard_ports_allowed() {
        // Explicit standard ports should be allowed
        assert!(validate_url("http://example.com:80").is_ok());
        assert!(validate_url("https://example.com:443").is_ok());
    }

    #[test]
    fn test_invalid_protocol() {
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("example.com").is_err());
    }

    // --- normalize_url tests ---

    #[test]
    fn test_normalize_adds_scheme() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("example.com/path"), "https://example.com/path");
    }

    #[test]
    fn test_normalize_trailing_slash() {
        // Root trailing slash stripped
        assert_eq!(normalize_url("https://example.com/"), "https://example.com");
        // Non-root trailing slash preserved (semantically different)
        assert_eq!(normalize_url("https://example.com/path/"), "https://example.com/path/");
    }

    #[test]
    fn test_normalize_default_ports() {
        assert_eq!(normalize_url("https://example.com:443/path"), "https://example.com/path");
        assert_eq!(normalize_url("http://example.com:80/path"), "http://example.com/path");
        // Non-default port preserved
        assert_eq!(normalize_url("https://example.com:8080/path"), "https://example.com:8080/path");
    }

    #[test]
    fn test_normalize_fragment_removed() {
        assert_eq!(normalize_url("https://example.com/page#section"), "https://example.com/page");
    }

    #[test]
    fn test_normalize_lowercases_host() {
        assert_eq!(normalize_url("https://EXAMPLE.COM/Path"), "https://example.com/Path");
    }

    #[test]
    fn test_normalize_preserves_query() {
        assert_eq!(normalize_url("https://example.com/path?q=1&b=2"), "https://example.com/path?q=1&b=2");
    }

    #[test]
    fn test_normalize_dedup_equivalents() {
        // All of these should normalize to the same canonical URL
        let canonical = normalize_url("https://example.com");
        assert_eq!(normalize_url("https://example.com/"), canonical);
        assert_eq!(normalize_url("https://EXAMPLE.COM"), canonical);
        assert_eq!(normalize_url("https://example.com:443"), canonical);
        assert_eq!(normalize_url("example.com"), canonical);
        assert_eq!(normalize_url("  example.com  "), canonical);
        assert_eq!(normalize_url("https://example.com#about"), canonical);
    }
}
