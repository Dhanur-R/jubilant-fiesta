use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub async fn middleware(&self, req: Request, next: Next) -> Response {
        let ip = extract_ip(&req);

        if !self.check_rate_limit(ip).await {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many requests. Please slow down.",
            )
                .into_response();
        }

        next.run(req).await
    }

    async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Periodically purge stale IPs to prevent unbounded memory growth
        if requests.len() > 1000 {
            requests.retain(|_, v| {
                v.retain(|&time| now.duration_since(time) < self.window);
                !v.is_empty()
            });
        }

        let entry = requests.entry(ip).or_insert_with(Vec::new);

        entry.retain(|&time| now.duration_since(time) < self.window);

        if entry.len() >= self.max_requests {
            return false;
        }

        entry.push(now);
        true
    }
}

fn extract_ip(req: &Request) -> IpAddr {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| {
            req.extensions()
                .get::<std::net::SocketAddr>()
                .map(|addr| addr.ip())
        })
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

pub async fn security_headers(req: Request, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com https://unpkg.com https://dhanur.me; style-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https://dhanur.me; connect-src 'self'; frame-ancestors 'none';"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    response
}
