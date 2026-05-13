use axum::http::HeaderMap;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};

const DEFAULT_AUTH_SERVICE: &str = "https://auth.dhanur.me";

#[derive(Clone, Debug, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub role: String,
}

impl Default for AuthStatus {
    fn default() -> Self {
        Self {
            authenticated: false,
            role: "guest".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreditUseResult {
    pub success: Option<bool>,
    pub balance: Option<i64>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub limit: Option<i64>,
    pub unlimited: Option<bool>,
}

fn auth_service_base_url() -> String {
    std::env::var("AUTH_SERVICE_URL")
        .unwrap_or_else(|_| DEFAULT_AUTH_SERVICE.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn with_forward_headers(builder: reqwest::RequestBuilder, headers: &HeaderMap) -> reqwest::RequestBuilder {
    let mut req = builder
        .header(USER_AGENT, "Mozilla/5.0 (compatible; LinkrBackend/1.0)");

    if let Some(cookie) = headers.get("cookie").and_then(|value| value.to_str().ok()) {
        req = req.header("Cookie", cookie);
    }

    if let Some(authz) = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
    {
        req = req.header("Authorization", authz);
    }

    req
}

pub async fn debit_linkr_credit(headers: &HeaderMap, amount: i64) -> Result<CreditUseResult, anyhow::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/credits/use", auth_service_base_url());

    let request = client
        .post(&url)
        .json(&serde_json::json!({
            "service": "linkr",
            "amount": amount,
            "description": "shorten_url"
        }))
        .timeout(std::time::Duration::from_secs(4));

    let request = with_forward_headers(request, headers);
    let response = request.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("Auth service returned {}: {}", status, error_text);
        return Err(anyhow::anyhow!("Auth API failure: status {}", status));
    }

    let body = response.json::<CreditUseResult>().await?;
    Ok(body)
}

pub async fn fetch_auth_status(headers: &HeaderMap) -> AuthStatus {
    let client = reqwest::Client::new();
    let url = format!("{}/api/status", auth_service_base_url());

    let request = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(3));

    let request = with_forward_headers(request, headers);

    match request.send().await {
        Ok(response) if response.status().is_success() => response
            .json::<AuthStatus>()
            .await
            .unwrap_or_default(),
        Ok(response) => {
            tracing::warn!("Auth status check failed with HTTP {}", response.status());
            AuthStatus::default()
        }
        Err(e) => {
            tracing::error!("Failed to reach auth service: {}", e);
            AuthStatus::default()
        }
    }
}