use askama::Template;
use axum::response::{Html, IntoResponse, Response};
use axum::http::StatusCode;

/// Renders a template to HTML
///
/// Handles template rendering errors by returning a fallback error message
pub fn render_html<T: Template>(template: T) -> Html<String> {
    Html(template.render().unwrap_or_else(|_| "Template error".into()))
}

/// Creates an error response with the form partial
///
/// Used for validation errors and other form-related errors
pub fn form_error_response<T: Template>(
    template: T,
    status: StatusCode,
) -> Response {
    (status, render_html(template)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    #[derive(Template)]
    #[template(source = "Hello {{ name }}!", ext = "html")]
    struct TestTemplate {
        name: String,
    }

    #[test]
    fn test_render_html_success() {
        let template = TestTemplate {
            name: "World".to_string(),
        };
        let result = render_html(template);
        assert_eq!(result.0, "Hello World!");
    }
}
