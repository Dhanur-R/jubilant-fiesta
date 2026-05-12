use chrono::Utc;
use reqwest::Client;
use serde_json::json;

use crate::models::Link;

pub struct SupabaseClient {
    pub client: Client,
    pub base_url: String,
    pub api_key: String,
}

impl SupabaseClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    /// Build a request with authentication headers
    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
    }
}

pub async fn get_link_by_original_url(
    supabase: &SupabaseClient,
    original_url: &str,
) -> Result<Option<String>, anyhow::Error> {
    let path = format!("/rest/v1/links?original_url=eq.{}&select=short_code", urlencoding::encode(original_url));
    let response = supabase
        .build_request(reqwest::Method::GET, &path)
        .send()
        .await?;

    if response.status().is_success() {
        let links: Vec<serde_json::Value> = response.json().await?;
        if let Some(link) = links.first() {
            if let Some(code) = link.get("short_code").and_then(|v| v.as_str()) {
                return Ok(Some(code.to_string()));
            }
        }
    }
    Ok(None)
}

pub async fn insert_link(
    supabase: &SupabaseClient,
    original_url: &str,
    short_code: &str,
) -> Result<bool, anyhow::Error> {
    let payload = json!({
        "original_url": original_url,
        "short_code": short_code
    });

    let response = supabase
        .build_request(reqwest::Method::POST, "/rest/v1/links")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    if status.as_u16() == 201 {
        return Ok(true);
    }
    if status.as_u16() == 409 {
        return Ok(false); // short_code collision, caller should retry
    }

    let body = response.text().await.unwrap_or_default();
    Err(anyhow::anyhow!("Supabase insert failed ({}): {}", status, body))
}

pub async fn update_last_accessed(
    supabase: &SupabaseClient,
    short_code: &str,
) -> Result<(), anyhow::Error> {
    let payload = json!({
        "last_accessed": chrono::Utc::now().to_rfc3339()
    });
    
    let path = format!("/rest/v1/links?short_code=eq.{}", urlencoding::encode(short_code));

    supabase
        .build_request(reqwest::Method::PATCH, &path)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    Ok(())
}

pub async fn get_link(
    supabase: &SupabaseClient,
    short_code: &str,
) -> Result<Option<Link>, anyhow::Error> {
    let path = format!("/rest/v1/links?short_code=eq.{}&select=*", urlencoding::encode(short_code));

    let response = supabase
        .build_request(reqwest::Method::GET, &path)
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let links: Vec<Link> = response.json().await?;
    Ok(links.into_iter().next())
}

pub async fn cleanup_unused_links(
    supabase: &SupabaseClient,
) -> Result<u64, anyhow::Error> {
    let cutoff_date = Utc::now() - chrono::TimeDelta::days(365);
    let cutoff_rfc = cutoff_date.to_rfc3339();
    let cutoff_iso = urlencoding::encode(&cutoff_rfc);

    // Delete links where:
    // - never accessed AND created more than 1 year ago, OR
    // - last accessed more than 1 year ago
    let path = format!(
        "/rest/v1/links?or=(and(last_accessed.is.null,created_at.lt.{}),last_accessed.lt.{})",
        cutoff_iso, cutoff_iso
    );

    let response = supabase
        .build_request(reqwest::Method::DELETE, &path)
        .header("Prefer", "return=headers-only, count=exact")
        .send()
        .await?;

    let count = response
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split('/')
                .nth(1)
                .and_then(|n| n.parse::<u64>().ok())
        })
        .unwrap_or(0);

    Ok(count)
}
