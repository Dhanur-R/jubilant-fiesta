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
    let original_url = payload.original_url.trim().to_string();

    if let Err(error_msg) = validation::validate_url(&original_url) {
        let template = FormPartialTemplate {
            error: Some(error_msg),
            value: Some(original_url),
        };
        return http_helpers::form_error_response(template, StatusCode::UNPROCESSABLE_ENTITY);
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

async fn create_short_code(
    state: &AppState,
    original_url: &str,
) -> Result<String, anyhow::Error> {
    // Check if URL already exists and reuse the code
    if let Ok(Some(existing_code)) = db::get_link_by_original_url(&state.supabase, original_url).await {
        return Ok(existing_code);
    }

    // Create new short code if URL doesn't exist
    for _ in 0..Config::SHORT_CODE_RETRY_ATTEMPTS {
        let candidate = nanoid!(6); // Using literal as nanoid! macro requires it
        if db::insert_link(&state.supabase, original_url, &candidate).await? {
            return Ok(candidate);
        }
    }
    Err(anyhow::anyhow!(
        "Failed to create unique short code after {} attempts",
        Config::SHORT_CODE_RETRY_ATTEMPTS
    ))
}
