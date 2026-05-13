use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    Json,
};
use askama::Template;
use nanoid::nanoid;
use serde::Serialize;

use crate::{config::Config, db, http_helpers, models::ShortenForm, validation, AppState};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    error: Option<String>,
    value: Option<String>,
}

#[derive(Template)]
#[template(path = "partials/form.html")]
struct FormPartialTemplate {
    error: Option<String>,
    value: Option<String>,
}

#[derive(Template)]
#[template(path = "partials/result.html")]
struct ResultPartialTemplate {
    short_url: String,
    short_code: String,
}

pub async fn index() -> impl IntoResponse {
    let template = IndexTemplate {
        error: None,
        value: None,
    };

    http_helpers::render_html(template)
}

pub async fn shorten(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(payload): Form<ShortenForm>,
) -> impl IntoResponse {
    let original_url = validation::normalize_url(&payload.original_url);

    if let Err(error_msg) = validation::validate_url(&original_url) {
        let template = FormPartialTemplate {
            error: Some(error_msg),
            value: Some(original_url),
        };
        return http_helpers::form_error_response(template, StatusCode::UNPROCESSABLE_ENTITY);
    }

    // Gate URL creation behind authy credits for both guests and authenticated users.
    let auth_status = crate::auth::fetch_auth_status(&headers).await;
    tracing::debug!(
        authenticated = auth_status.authenticated,
        role = %auth_status.role,
        "authorizing shorten request"
    );

    match crate::auth::debit_linkr_credit(&headers, 1).await {
        Ok(result) => {
            let allowed = result.success.unwrap_or(false) || result.unlimited.unwrap_or(false);
            if !allowed {
                let message = match result.code.as_deref() {
                    Some("GUEST_DAILY_LIMIT") => "Guest daily limit reached for link creation. Sign in for higher limits.".to_string(),
                    _ => result
                        .error
                        .unwrap_or_else(|| "You do not have enough credits to shorten this link.".to_string()),
                };

                let template = FormPartialTemplate {
                    error: Some(message),
                    value: Some(original_url),
                };
                return http_helpers::form_error_response(template, StatusCode::PAYMENT_REQUIRED);
            }
        }
        Err(_) => {
            let template = FormPartialTemplate {
                error: Some("Could not validate credits right now. Please try again in a moment.".to_string()),
                value: Some(original_url),
            };
            return http_helpers::form_error_response(template, StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    let short_code = match create_short_code(&state, &original_url).await {
        Ok(code) => code,
        Err(_) => {
            let template = FormPartialTemplate {
                error: Some("Could not create a short link. Try again.".to_string()),
                value: Some(original_url),
            };
            return http_helpers::form_error_response(template, StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let base_url = state
        .public_base_url
        .clone()
        .or_else(|| derive_base_url(&headers));

    let short_url = match base_url {
        Some(base) => format!("{}/{}", base.trim_end_matches('/'), short_code),
        None => format!("/{}", short_code),
    };

    http_helpers::render_html(ResultPartialTemplate {
        short_url,
        short_code,
    })
    .into_response()
}

pub async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    // Validate short code format (nanoid: alphanumeric + _ and -, length 6)
    if code.is_empty()
        || code.len() > 20
        || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }

    match db::get_link(&state.supabase, &code).await {
        Ok(Some(link)) => {
            // Update last_accessed in background (keeps link alive)
            let supabase = state.supabase.clone();
            let code_clone = code.clone();
            tokio::spawn(async move {
                let _ = db::update_last_accessed(&supabase, &code_clone).await;
            });

            Redirect::to(&link.original_url).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response(),
    }
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

fn derive_base_url(headers: &HeaderMap) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?;
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");
    Some(format!("{}://{}", proto, host))
}

// In routes.rs
async fn create_short_code(
    state: &AppState,
    original_url: &str,
) -> Result<String, anyhow::Error> {
    if let Ok(Some(existing_code)) = db::get_link_by_original_url(&state.supabase, original_url).await {
        return Ok(existing_code);
    }

    for attempt in 1..=Config::SHORT_CODE_RETRY_ATTEMPTS {
        let candidate = nanoid!(6);
        match db::insert_link(&state.supabase, original_url, &candidate).await {
            Ok(true) => return Ok(candidate),
            Ok(false) => {
                // Short-code collision occurred (409), loop continues to try next ID
                tracing::warn!("short_code collision on attempt {}, retrying", attempt);
                continue;
            }
            Err(e) => {
                // Log underlying Supabase/Reqwest transport errors for diagnostic visibility
                tracing::error!("Supabase execution failure during short_code generation: {:#}", e);
                return Err(e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed to create unique short code after {} attempts",
        Config::SHORT_CODE_RETRY_ATTEMPTS
    ))
}
